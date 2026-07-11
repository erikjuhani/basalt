//! Loading, caching and resolution of inline note images.
//!
//! Background worker threads keep the UI responsive: a decode worker resolves an
//! image source against the vault (a filesystem walk), fetches and decodes it,
//! and an encode worker (owning the [`Picker`]) encodes it into a sliceable
//! terminal-graphics protocol. The UI thread only reads already-resolved and
//! already-encoded results, so opening a note never blocks on image work.
//! Encoding to [`SlicedProtocol`] lets a partially scrolled image render just
//! its visible rows. Animated GIFs render only their first frame.

use std::{
    collections::{HashMap, HashSet},
    fs,
    io::Cursor,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

use image::{codecs::gif::GifDecoder, AnimationDecoder, DynamicImage, ImageFormat};
use ratatui::layout::Size;
use ratatui_image::{picker::Picker, sliced::SlicedProtocol, Resize};

use crate::note_editor::ast::ImageSource;

/// Cap on remote image downloads to keep a stray URL from exhausting memory.
const MAX_FETCH_BYTES: u64 = 32 * 1024 * 1024;

/// Longest side, in pixels, an image is downscaled to before encoding.
const MAX_IMAGE_DIM: u32 = 1600;

/// A resolved, canonical location an image can be loaded from. Doubles as the
/// cache key so the same file or URL is only ever decoded once.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ImageKey {
    Path(PathBuf),
    Url(String),
}

/// A request to resolve, fetch and decode a source, sent to the decode worker
/// so the vault walk never runs on the UI thread.
struct LoadRequest {
    source: ImageSource,
    note_dir: PathBuf,
    vault_root: PathBuf,
}

/// An image handed to the encode worker, returned as [`EncodeDone`].
struct EncodeJob {
    key: ImageKey,
    size: Size,
    image: DynamicImage,
}

struct EncodeDone {
    key: ImageKey,
    size: Size,
    protocol: SlicedProtocol,
}

/// The decode worker's result for a requested source.
enum Decoded {
    Ready {
        source: ImageSource,
        key: ImageKey,
        image: DynamicImage,
    },
    Failed(ImageSource),
}

/// A per-size graphics-protocol encoding: in flight or ready to draw.
enum Encoded {
    Pending,
    Ready(Box<SlicedProtocol>),
}

/// A decoded image and its cached encodings, one per cell size it is drawn at.
/// The same image can appear at several sizes (e.g. sized and unsized copies),
/// so each is cached independently rather than churning a single slot.
struct Cached {
    image: DynamicImage,
    dims: (u32, u32),
    protocols: HashMap<(u16, u16), Encoded>,
}

pub struct ImageStore {
    /// Terminal cell size in pixels, cached so the picker can live on the worker.
    font_size: (u16, u16),
    /// Whether an encoder (graphics-capable picker) exists. Without one, images
    /// never transmit, so `pending_transmit` must not spin the run loop.
    has_encoder: bool,
    /// How each requested source resolved to a cache key.
    resolved: HashMap<ImageSource, ImageKey>,
    /// Sources the worker could not resolve or decode.
    failed: HashSet<ImageSource>,
    /// Sources already sent to the worker, so requests stay idempotent.
    requested: HashSet<ImageSource>,
    /// Decoded images keyed by location, so a file is only decoded once.
    entries: HashMap<ImageKey, Cached>,
    /// (key, size) pairs already transmitted to the terminal, so a heavy first
    /// transmit is spread one per frame rather than flooding a single draw.
    shown: HashSet<(ImageKey, (u16, u16))>,
    /// Whether a visible image still needs a first transmit or is encoding, so
    /// the run loop redraws promptly rather than waiting a full tick.
    pending_transmit: bool,
    decode_jobs: Sender<LoadRequest>,
    decoded: Receiver<Decoded>,
    encode_jobs: Sender<EncodeJob>,
    encoded: Receiver<EncodeDone>,
}

impl Default for ImageStore {
    fn default() -> Self {
        Self::new(None)
    }
}

impl ImageStore {
    /// Builds a store backed by decode and encode worker threads. `picker` is
    /// `None` when the terminal has no graphics protocol (or under a headless
    /// test backend), in which case images degrade to text placeholders.
    pub fn new(picker: Option<Picker>) -> Self {
        let has_encoder = picker.is_some();
        let font_size = picker
            .as_ref()
            .map(|picker| {
                let size = picker.font_size();
                (size.width, size.height)
            })
            .unwrap_or((10, 20));

        let (decode_jobs, decode_rx) = mpsc::channel::<LoadRequest>();
        let (decoded_tx, decoded) = mpsc::channel::<Decoded>();
        thread::spawn(move || {
            while let Ok(request) = decode_rx.recv() {
                let decoded =
                    resolve_source(&request.source, &request.note_dir, &request.vault_root)
                        .and_then(|key| {
                            load_image(&key).map(|image| Decoded::Ready {
                                source: request.source.clone(),
                                key,
                                image,
                            })
                        })
                        .unwrap_or(Decoded::Failed(request.source));
                if decoded_tx.send(decoded).is_err() {
                    break;
                }
            }
        });

        let (encode_jobs, encode_rx) = mpsc::channel::<EncodeJob>();
        let (encoded_tx, encoded) = mpsc::channel();
        thread::spawn(move || {
            let Some(picker) = picker else { return };
            while let Ok(job) = encode_rx.recv() {
                // `Scale`, unlike `Fit`, upscales a source smaller than the
                // reserved box so the image fills it instead of leaving a gap.
                if let Ok(protocol) = SlicedProtocol::new_with_resize(
                    &picker,
                    job.image,
                    job.size,
                    Resize::Scale(None),
                ) {
                    let done = EncodeDone {
                        key: job.key,
                        size: job.size,
                        protocol,
                    };
                    if encoded_tx.send(done).is_err() {
                        break;
                    }
                }
            }
        });

        Self {
            font_size,
            has_encoder,
            resolved: HashMap::new(),
            failed: HashSet::new(),
            requested: HashSet::new(),
            entries: HashMap::new(),
            shown: HashSet::new(),
            pending_transmit: false,
            decode_jobs,
            decoded,
            encode_jobs,
            encoded,
        }
    }

    /// Queues resolution and decoding of `source` on the worker unless it has
    /// already been requested. Idempotent, so it is cheap to call every frame.
    pub fn request(&mut self, source: ImageSource, note_dir: PathBuf, vault_root: PathBuf) {
        if !self.requested.insert(source.clone()) {
            return;
        }
        let _ = self.decode_jobs.send(LoadRequest {
            source,
            note_dir,
            vault_root,
        });
    }

    /// Drains finished decode and encode work into the cache. Call once per frame
    /// before reading resolutions, sizes or protocols.
    pub fn poll(&mut self) {
        while let Ok(decoded) = self.decoded.try_recv() {
            match decoded {
                Decoded::Ready { source, key, image } => {
                    let dims = (image.width(), image.height());
                    self.resolved.insert(source, key.clone());
                    self.entries.entry(key).or_insert(Cached {
                        image,
                        dims,
                        protocols: HashMap::new(),
                    });
                }
                Decoded::Failed(source) => {
                    self.failed.insert(source);
                }
            }
        }

        while let Ok(job) = self.encoded.try_recv() {
            if let Some(cached) = self.entries.get_mut(&job.key) {
                cached.protocols.insert(
                    (job.size.width, job.size.height),
                    Encoded::Ready(Box::new(job.protocol)),
                );
            }
        }
    }

    /// Terminal cell size in pixels.
    pub fn font_size(&self) -> (u16, u16) {
        self.font_size
    }

    /// The cache key `source` resolved to, if resolution has finished.
    pub fn resolved(&self, source: &ImageSource) -> Option<ImageKey> {
        self.resolved.get(source).cloned()
    }

    /// Whether `source` could not be resolved or decoded.
    pub fn is_failed(&self, source: &ImageSource) -> bool {
        self.failed.contains(source)
    }

    /// Pixel dimensions of a decoded image, if it is ready.
    pub fn dims(&self, key: &ImageKey) -> Option<(u32, u32)> {
        self.entries.get(key).map(|cached| cached.dims)
    }

    /// Whether `key` has already been transmitted to the terminal at `size`, so
    /// re-drawing it is cheap. A fresh (key, size) is an expensive transmit.
    pub fn is_shown(&self, key: &ImageKey, size: Size) -> bool {
        self.shown
            .contains(&(key.clone(), (size.width, size.height)))
    }

    /// Records that `key` has been transmitted at `size`.
    pub fn mark_shown(&mut self, key: ImageKey, size: Size) {
        self.shown.insert((key, (size.width, size.height)));
    }

    /// Flags that a visible image still needs a transmit, so the run loop redraws
    /// soon. Ignored without an encoder, where images never transmit.
    pub fn set_pending_transmit(&mut self, pending: bool) {
        self.pending_transmit = pending && self.has_encoder;
    }

    /// Whether a visible image still needs transmitting.
    pub fn pending_transmit(&self) -> bool {
        self.pending_transmit
    }

    /// The encoded protocol to draw for `key` at `size`. Encodes on the worker
    /// on first use (and after a resize), returning it once ready.
    pub fn protocol_at(&mut self, key: &ImageKey, size: Size) -> Option<&SlicedProtocol> {
        use std::collections::hash_map::Entry;

        let cached = self.entries.get_mut(key)?;
        let dims = (size.width, size.height);
        match cached.protocols.entry(dims) {
            Entry::Occupied(slot) => match slot.into_mut() {
                Encoded::Ready(protocol) => Some(protocol),
                Encoded::Pending => None,
            },
            Entry::Vacant(slot) => {
                slot.insert(Encoded::Pending);
                let _ = self.encode_jobs.send(EncodeJob {
                    key: key.clone(),
                    size,
                    image: cached.image.clone(),
                });
                None
            }
        }
    }
}

/// Resolves an image source to a cache key. Embeds are matched by file name
/// anywhere in the vault (Obsidian-style); relative paths are joined against the
/// note's directory; URLs pass through unchanged.
fn resolve_source(source: &ImageSource, note_dir: &Path, vault_root: &Path) -> Option<ImageKey> {
    match source {
        ImageSource::Url(url) => Some(ImageKey::Url(url.clone())),
        ImageSource::Path(path) => {
            let path = Path::new(path);
            let resolved = if path.is_absolute() {
                path.to_path_buf()
            } else {
                note_dir.join(path)
            };
            Some(ImageKey::Path(resolved))
        }
        ImageSource::Embed(name) => find_in_vault(vault_root, name).map(ImageKey::Path),
    }
}

/// Reads and decodes an image. Animated GIFs decode to their first frame only.
fn load_image(key: &ImageKey) -> Option<DynamicImage> {
    let bytes = read_bytes(key).ok()?;
    let image = if image::guess_format(&bytes).ok() == Some(ImageFormat::Gif) {
        first_gif_frame(&bytes)?
    } else {
        image::load_from_memory(&bytes).ok()?
    };
    Some(downscale(image, MAX_IMAGE_DIM))
}

fn read_bytes(key: &ImageKey) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    Ok(match key {
        ImageKey::Path(path) => fs::read(path)?,
        ImageKey::Url(url) => ureq::get(url)
            .call()?
            .body_mut()
            .with_config()
            .limit(MAX_FETCH_BYTES)
            .read_to_vec()?,
    })
}

fn first_gif_frame(bytes: &[u8]) -> Option<DynamicImage> {
    let decoder = GifDecoder::new(Cursor::new(bytes)).ok()?;
    let frame = decoder.into_frames().next()?.ok()?;
    Some(DynamicImage::ImageRgba8(frame.into_buffer()))
}

/// Shrinks an image so its longest side is at most `max_dim`, preserving aspect
/// ratio. Smaller images are returned unchanged.
fn downscale(image: DynamicImage, max_dim: u32) -> DynamicImage {
    if image.width().max(image.height()) <= max_dim {
        image
    } else {
        image.resize(max_dim, max_dim, image::imageops::FilterType::Nearest)
    }
}

/// Depth-first search for a file named `name` within the vault, skipping hidden
/// directories. Returns the first match.
fn find_in_vault(vault_root: &Path, name: &str) -> Option<PathBuf> {
    let mut stack = vec![vault_root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                if !entry.file_name().to_string_lossy().starts_with('.') {
                    stack.push(entry.path());
                }
            } else if entry.file_name().to_string_lossy() == name {
                return Some(entry.path());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downscale_caps_the_longest_side() {
        let image = downscale(DynamicImage::new_rgba8(2000, 1000), MAX_IMAGE_DIM);
        assert_eq!(image.width().max(image.height()), MAX_IMAGE_DIM);
    }

    #[test]
    fn downscale_leaves_small_images_untouched() {
        let image = downscale(DynamicImage::new_rgba8(320, 240), MAX_IMAGE_DIM);
        assert_eq!((image.width(), image.height()), (320, 240));
    }
}
