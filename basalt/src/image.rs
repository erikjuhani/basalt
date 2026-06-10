//! Loading, caching and resolution of inline note images.
//!
//! Two background worker threads keep the UI responsive: one decodes/fetches
//! image bytes into frames, the other (owning the [`Picker`]) resizes and
//! encodes frames into a sliceable terminal-graphics protocol. The UI thread
//! only ever renders already-encoded frames, so it never blocks on image work
//! even while a GIF animates. Encoding to [`SlicedProtocol`] lets a partially
//! scrolled image render just its visible rows.

use std::{
    collections::HashMap,
    fs,
    io::Cursor,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant},
};

use image::{codecs::gif::GifDecoder, AnimationDecoder, DynamicImage, ImageFormat};
use ratatui::layout::Size;
use ratatui_image::{picker::Picker, sliced::SlicedProtocol};

use crate::note_editor::ast::ImageSource;

/// Cap on remote image downloads to keep a stray URL from exhausting memory.
const MAX_FETCH_BYTES: u64 = 32 * 1024 * 1024;

/// Frame delay assumed when a GIF frame reports none.
const DEFAULT_FRAME_DELAY: Duration = Duration::from_millis(100);

/// Most GIF frames to decode. Screen-recording GIFs can have thousands of
/// frames; decoding them all is slow and holds gigabytes in memory, so playback
/// is capped to the first stretch and loops.
const MAX_FRAMES: usize = 60;

/// Longest side, in pixels, that decoded frames are downscaled to. Bounds memory
/// and keeps encoding cheap; the picker downsizes further to the cell area.
const MAX_FRAME_DIM: u32 = 640;

/// A resolved, canonical location an image can be loaded from. Doubles as the
/// cache key so the same file or URL is only ever decoded once.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ImageKey {
    Path(PathBuf),
    Url(String),
}

/// One decoded frame of an image. Still images are a single zero-delay frame.
struct Frame {
    image: DynamicImage,
    delay: Duration,
}

/// A frame handed to the encode worker, returned as [`EncodeDone`].
struct EncodeJob {
    key: ImageKey,
    index: usize,
    size: Size,
    image: DynamicImage,
}

struct EncodeDone {
    key: ImageKey,
    index: usize,
    size: Size,
    protocol: SlicedProtocol,
}

enum Entry {
    Loading,
    Ready {
        frames: Vec<Frame>,
        dims: (u32, u32),
        /// When playback started, for time-based frame selection.
        started: Instant,
        /// Per-frame protocols, encoded for `area`. Filled by the encode worker.
        protocols: Vec<Option<Box<SlicedProtocol>>>,
        /// Per-frame: an encode is in flight, so it is not requested again.
        pending: Vec<bool>,
        /// The cell size the cached protocols were encoded for. A change clears
        /// the cache so frames are re-encoded for the new size.
        area: Option<Size>,
        /// Last frame returned ready, shown while the next one encodes.
        shown: Option<usize>,
    },
    Failed,
}

pub struct ImageStore {
    /// Terminal cell size in pixels, cached so the picker can live on the worker.
    font_size: (u16, u16),
    entries: HashMap<ImageKey, Entry>,
    decode_jobs: Sender<ImageKey>,
    decoded: Receiver<(ImageKey, Option<Vec<Frame>>)>,
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
        let font_size = picker
            .as_ref()
            .map(|picker| {
                let size = picker.font_size();
                (size.width, size.height)
            })
            .unwrap_or((10, 20));

        let (decode_jobs, decode_rx) = mpsc::channel::<ImageKey>();
        let (decoded_tx, decoded) = mpsc::channel();
        thread::spawn(move || {
            while let Ok(key) = decode_rx.recv() {
                let frames = load(&key).ok();
                if decoded_tx.send((key, frames)).is_err() {
                    break;
                }
            }
        });

        let (encode_jobs, encode_rx) = mpsc::channel::<EncodeJob>();
        let (encoded_tx, encoded) = mpsc::channel();
        thread::spawn(move || {
            let Some(picker) = picker else { return };
            while let Ok(job) = encode_rx.recv() {
                if let Ok(protocol) = SlicedProtocol::new(&picker, job.image, Some(job.size)) {
                    let done = EncodeDone {
                        key: job.key,
                        index: job.index,
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
            entries: HashMap::new(),
            decode_jobs,
            decoded,
            encode_jobs,
            encoded,
        }
    }

    /// Resolves an image source to a cache key. Embeds are matched by file name
    /// anywhere in the vault (Obsidian-style); relative paths are joined against
    /// the note's directory; URLs pass through unchanged.
    pub fn resolve(source: &ImageSource, note_dir: &Path, vault_root: &Path) -> Option<ImageKey> {
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

    /// Queues a load for `key` unless it is already known. Idempotent.
    pub fn request(&mut self, key: ImageKey) {
        if self.entries.contains_key(&key) {
            return;
        }
        self.entries.insert(key.clone(), Entry::Loading);
        let _ = self.decode_jobs.send(key);
    }

    /// Drains finished decode and encode work into the cache. Call once per frame
    /// before reading sizes or protocols.
    pub fn poll(&mut self) {
        while let Ok((key, decoded)) = self.decoded.try_recv() {
            let entry = match decoded {
                Some(frames) if !frames.is_empty() => {
                    let dims = (frames[0].image.width(), frames[0].image.height());
                    let count = frames.len();
                    Entry::Ready {
                        frames,
                        dims,
                        started: Instant::now(),
                        protocols: (0..count).map(|_| None).collect(),
                        pending: vec![false; count],
                        area: None,
                        shown: None,
                    }
                }
                _ => Entry::Failed,
            };
            self.entries.insert(key, entry);
        }

        while let Ok(job) = self.encoded.try_recv() {
            if let Some(Entry::Ready {
                protocols,
                pending,
                area,
                shown,
                ..
            }) = self.entries.get_mut(&job.key)
            {
                if let Some(slot) = pending.get_mut(job.index) {
                    *slot = false;
                }
                // Discard if the area changed while the frame was encoding.
                if *area == Some(job.size) {
                    if let Some(slot) = protocols.get_mut(job.index) {
                        *slot = Some(Box::new(job.protocol));
                    }
                    // Keep a ready frame to fall back on while later frames
                    // are still encoding (the worker lags the playhead).
                    *shown = Some(job.index);
                }
            }
        }
    }

    /// Terminal cell size in pixels, used to size images.
    pub fn font_size(&self) -> (u16, u16) {
        self.font_size
    }

    /// Pixel dimensions of a decoded image, if it is ready.
    pub fn dims(&self, key: &ImageKey) -> Option<(u32, u32)> {
        match self.entries.get(key) {
            Some(Entry::Ready { dims, .. }) => Some(*dims),
            _ => None,
        }
    }

    /// Whether any ready image has more than one frame, so the run loop knows to
    /// redraw frequently enough to animate it.
    pub fn is_animating(&self) -> bool {
        self.entries
            .values()
            .any(|entry| matches!(entry, Entry::Ready { frames, .. } if frames.len() > 1))
    }

    /// The encoded protocol to draw for `key` at `now`, encoded to `size`.
    ///
    /// Returns the current frame once the encode worker has it ready, otherwise
    /// the previously shown frame (so playback never blanks while a frame
    /// encodes), and requests any not-yet-encoded frame in the background.
    pub fn protocol_at(
        &mut self,
        key: &ImageKey,
        now: Instant,
        size: Size,
    ) -> Option<&SlicedProtocol> {
        let Some(Entry::Ready {
            frames,
            started,
            protocols,
            pending,
            area,
            shown,
            ..
        }) = self.entries.get_mut(key)
        else {
            return None;
        };

        // A resized cell area invalidates every cached encoding.
        if *area != Some(size) {
            *area = Some(size);
            protocols.iter_mut().for_each(|slot| *slot = None);
            pending.iter_mut().for_each(|slot| *slot = false);
            *shown = None;
        }

        let index = current_frame_index(frames, now.saturating_duration_since(*started));

        // Hand the frame off to the encode worker if it is neither ready nor
        // already being encoded.
        if protocols[index].is_none() && !pending[index] {
            pending[index] = true;
            let _ = self.encode_jobs.send(EncodeJob {
                key: key.clone(),
                index,
                size,
                image: frames[index].image.clone(),
            });
        }

        let target = if protocols[index].is_some() {
            *shown = Some(index);
            index
        } else {
            (*shown).filter(|&i| protocols[i].is_some())?
        };
        protocols[target].as_deref()
    }
}

/// The frame to display for the given elapsed time, looping over frame delays.
fn current_frame_index(frames: &[Frame], elapsed: Duration) -> usize {
    if frames.len() <= 1 {
        return 0;
    }

    let delay = |frame: &Frame| {
        if frame.delay.is_zero() {
            DEFAULT_FRAME_DELAY
        } else {
            frame.delay
        }
    };

    let total: Duration = frames.iter().map(delay).sum();
    if total.is_zero() {
        return 0;
    }

    let mut offset = Duration::from_nanos((elapsed.as_nanos() % total.as_nanos()) as u64);
    for (index, frame) in frames.iter().enumerate() {
        let frame_delay = delay(frame);
        if offset < frame_delay {
            return index;
        }
        offset -= frame_delay;
    }
    frames.len() - 1
}

fn load(key: &ImageKey) -> Result<Vec<Frame>, Box<dyn std::error::Error>> {
    let bytes = match key {
        ImageKey::Path(path) => fs::read(path)?,
        ImageKey::Url(url) => ureq::get(url)
            .call()?
            .body_mut()
            .with_config()
            .limit(MAX_FETCH_BYTES)
            .read_to_vec()?,
    };

    // Animated GIFs decode to their frames (capped); everything else is a
    // single still frame.
    if image::guess_format(&bytes).ok() == Some(ImageFormat::Gif) {
        let decoder = GifDecoder::new(Cursor::new(bytes))?;
        let frames = decoder
            .into_frames()
            .take(MAX_FRAMES)
            .map(|frame| {
                let frame = frame?;
                Ok(Frame {
                    delay: Duration::from(frame.delay()),
                    image: downscale(DynamicImage::ImageRgba8(frame.into_buffer())),
                })
            })
            .collect::<Result<Vec<_>, image::ImageError>>()?;
        Ok(frames)
    } else {
        Ok(vec![Frame {
            image: downscale(image::load_from_memory(&bytes)?),
            delay: Duration::ZERO,
        }])
    }
}

/// Shrinks an image so its longest side is at most [`MAX_FRAME_DIM`], preserving
/// aspect ratio. Smaller images are returned unchanged.
fn downscale(image: DynamicImage) -> DynamicImage {
    if image.width().max(image.height()) <= MAX_FRAME_DIM {
        image
    } else {
        image.resize(
            MAX_FRAME_DIM,
            MAX_FRAME_DIM,
            image::imageops::FilterType::Nearest,
        )
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

    fn frames(delays_ms: &[u64]) -> Vec<Frame> {
        delays_ms
            .iter()
            .map(|ms| Frame {
                image: DynamicImage::new_rgba8(1, 1),
                delay: Duration::from_millis(*ms),
            })
            .collect()
    }

    #[test]
    fn frame_index_cycles_over_delays() {
        let frames = frames(&[100, 100, 100]);
        assert_eq!(current_frame_index(&frames, Duration::from_millis(0)), 0);
        assert_eq!(current_frame_index(&frames, Duration::from_millis(150)), 1);
        assert_eq!(current_frame_index(&frames, Duration::from_millis(250)), 2);
        // Loops back around after the total duration.
        assert_eq!(current_frame_index(&frames, Duration::from_millis(350)), 0);
    }

    #[test]
    fn single_frame_is_static() {
        let frames = frames(&[0]);
        assert_eq!(current_frame_index(&frames, Duration::from_secs(5)), 0);
    }

    #[test]
    fn zero_delay_frames_use_default() {
        // Two frames with no delay split the default-delay cycle evenly.
        let frames = frames(&[0, 0]);
        assert_eq!(current_frame_index(&frames, Duration::ZERO), 0);
        assert_eq!(
            current_frame_index(&frames, DEFAULT_FRAME_DELAY + Duration::from_millis(1)),
            1
        );
    }
}
