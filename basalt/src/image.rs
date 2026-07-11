use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    ffi::OsStr,
    fs,
    panic::{catch_unwind, AssertUnwindSafe},
    path::{Path, PathBuf},
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    thread,
    time::Duration,
};

use basalt_core::obsidian::Vault;
use image::DynamicImage;
use ratatui::layout::Size;
use ratatui_image::{picker::Picker, sliced::SlicedProtocol, Resize};

use crate::note_editor::{ast::ImageSource, ImageEntry};

const MAX_FETCH_BYTES: u64 = 32 * 1024 * 1024;

const FETCH_TIMEOUT: Duration = Duration::from_secs(10);

const MAX_IMAGE_DIM: u32 = 1600;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ImageKey {
    Path(PathBuf),
    Url(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelSize {
    pub width: u32,
    pub height: u32,
}

/// Keyed with the note's folder applied, so `img.png` in two folders stays two images.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Target {
    Embed(String),
    Key(ImageKey),
}

impl Target {
    fn new(source: &ImageSource, note_dir: &Path) -> Self {
        match source {
            ImageSource::Embed(name) => Self::Embed(name.clone()),
            ImageSource::Path(path) => Self::Key(ImageKey::Path(note_dir.join(path))),
            ImageSource::Url(url) => Self::Key(ImageKey::Url(url.clone())),
        }
    }
}

struct LoadRequest {
    target: Target,
    vault: Vault,
}

struct EncodeJob {
    key: ImageKey,
    size: Size,
    image: Arc<DynamicImage>,
}

struct EncodeDone {
    key: ImageKey,
    size: Size,
    protocol: Option<SlicedProtocol>,
}

enum Decoded {
    Ready {
        target: Target,
        key: ImageKey,
        image: DynamicImage,
    },
    Failed(Target),
}

enum Encoded {
    Encoding,
    Ready {
        protocol: Box<SlicedProtocol>,
        shown: bool,
    },
    Failed,
}

struct Cached {
    image: Arc<DynamicImage>,
    protocols: HashMap<Size, Encoded>,
}

pub struct ImageStore {
    font_size: Size,
    vault: Vault,
    sources: HashMap<Target, ImageEntry>,
    entries: HashMap<ImageKey, Cached>,
    deferred_transmit: bool,
    decode_jobs: Sender<LoadRequest>,
    decoded: Receiver<Decoded>,
    encode_jobs: Sender<EncodeJob>,
    encoded: Receiver<EncodeDone>,
}

impl ImageStore {
    pub fn new(picker: Picker) -> Self {
        let font_size = Size::new(picker.font_size().width, picker.font_size().height);

        let (decode_jobs, decode_rx) = mpsc::channel::<LoadRequest>();
        let (decoded_tx, decoded) = mpsc::channel::<Decoded>();
        thread::spawn(move || {
            let agent: ureq::Agent = ureq::Agent::config_builder()
                .timeout_global(Some(FETCH_TIMEOUT))
                .build()
                .into();
            while let Ok(LoadRequest { target, vault }) = decode_rx.recv() {
                let decoded = resolve(&target, &vault)
                    .and_then(|key| {
                        load_image(&key, &agent).map(|image| Decoded::Ready {
                            target: target.clone(),
                            key,
                            image,
                        })
                    })
                    .unwrap_or(Decoded::Failed(target));
                if decoded_tx.send(decoded).is_err() {
                    break;
                }
            }
        });

        let (encode_jobs, encode_rx) = mpsc::channel::<EncodeJob>();
        let (encoded_tx, encoded) = mpsc::channel();
        thread::spawn(move || {
            while let Ok(EncodeJob { key, size, image }) = encode_rx.recv() {
                // `Scale`, unlike `Fit`, upscales a small source to fill the reserved box.
                let protocol = catch_unwind(AssertUnwindSafe(|| {
                    let image = (*image).clone();
                    SlicedProtocol::new_with_resize(&picker, image, size, Resize::Scale(None)).ok()
                }))
                .unwrap_or(None);
                if encoded_tx
                    .send(EncodeDone {
                        key,
                        size,
                        protocol,
                    })
                    .is_err()
                {
                    break;
                }
            }
        });

        Self {
            font_size,
            vault: Vault::default(),
            sources: HashMap::new(),
            entries: HashMap::new(),
            deferred_transmit: false,
            decode_jobs,
            decoded,
            encode_jobs,
            encoded,
        }
    }

    pub fn set_vault(&mut self, vault: &Vault) {
        if self.vault.path != vault.path {
            self.vault = vault.clone();
            self.sources
                .retain(|target, _| matches!(target, Target::Key(_)));
        }
    }

    pub fn request(&mut self, source: &ImageSource, note_dir: &Path) {
        if let Entry::Vacant(slot) = self.sources.entry(Target::new(source, note_dir)) {
            let _ = self.decode_jobs.send(LoadRequest {
                target: slot.key().clone(),
                vault: self.vault.clone(),
            });
            slot.insert(ImageEntry::Loading);
        }
    }

    pub fn forget_failed(&mut self) {
        self.sources.retain(|_, entry| *entry != ImageEntry::Failed);
    }

    pub fn poll(&mut self) {
        while let Ok(decoded) = self.decoded.try_recv() {
            match decoded {
                Decoded::Ready { target, key, image } => {
                    let size = PixelSize {
                        width: image.width(),
                        height: image.height(),
                    };
                    self.sources.insert(
                        target,
                        ImageEntry::Decoded {
                            key: key.clone(),
                            size,
                        },
                    );
                    self.entries.entry(key).or_insert(Cached {
                        image: Arc::new(image),
                        protocols: HashMap::new(),
                    });
                }
                Decoded::Failed(target) => {
                    self.sources.insert(target, ImageEntry::Failed);
                }
            }
        }

        while let Ok(EncodeDone {
            key,
            size,
            protocol,
        }) = self.encoded.try_recv()
        {
            if let Some(cached) = self.entries.get_mut(&key) {
                let encoded = match protocol {
                    Some(protocol) => Encoded::Ready {
                        protocol: Box::new(protocol),
                        shown: false,
                    },
                    None => Encoded::Failed,
                };
                cached.protocols.insert(size, encoded);
            }
        }
    }

    pub fn font_size(&self) -> Size {
        self.font_size
    }

    pub fn entry(&self, source: &ImageSource, note_dir: &Path) -> ImageEntry {
        self.sources
            .get(&Target::new(source, note_dir))
            .cloned()
            .unwrap_or(ImageEntry::Loading)
    }

    pub fn is_shown(&self, key: &ImageKey, size: Size) -> bool {
        matches!(
            self.entries
                .get(key)
                .and_then(|cached| cached.protocols.get(&size)),
            Some(Encoded::Ready { shown: true, .. })
        )
    }

    /// Keeps encodings for laid-out sizes only, so resizes do not pile them up.
    pub fn retain_sizes<'k>(&mut self, live: impl IntoIterator<Item = (&'k ImageKey, Size)>) {
        let live: HashSet<_> = live.into_iter().collect();
        self.entries.iter_mut().for_each(|(key, cached)| {
            cached
                .protocols
                .retain(|size, _| live.contains(&(key, *size)))
        });
    }

    pub fn set_deferred_transmit(&mut self, deferred: bool) {
        self.deferred_transmit = deferred;
    }

    pub fn is_busy(&self) -> bool {
        self.deferred_transmit
            || self
                .sources
                .values()
                .any(|entry| *entry == ImageEntry::Loading)
            || self
                .entries
                .values()
                .flat_map(|cached| cached.protocols.values())
                .any(|encoded| matches!(encoded, Encoded::Encoding))
    }

    /// Marks the protocol as shown, since the caller draws it.
    pub fn protocol_at(&mut self, key: &ImageKey, size: Size) -> Option<&SlicedProtocol> {
        let cached = self.entries.get_mut(key)?;
        match cached.protocols.entry(size) {
            Entry::Occupied(slot) => match slot.into_mut() {
                Encoded::Ready { protocol, shown } => {
                    *shown = true;
                    Some(protocol)
                }
                Encoded::Encoding | Encoded::Failed => None,
            },
            Entry::Vacant(slot) => {
                slot.insert(Encoded::Encoding);
                let _ = self.encode_jobs.send(EncodeJob {
                    key: key.clone(),
                    size,
                    image: Arc::clone(&cached.image),
                });
                None
            }
        }
    }
}

fn resolve(target: &Target, vault: &Vault) -> Option<ImageKey> {
    match target {
        Target::Key(key) => Some(key.clone()),
        Target::Embed(name) => vault
            .notes()
            .into_iter()
            .find(|note| note.path().file_name() == Some(OsStr::new(name)))
            .map(|note| ImageKey::Path(note.path().to_path_buf())),
    }
}

fn load_image(key: &ImageKey, agent: &ureq::Agent) -> Option<DynamicImage> {
    let image = image::load_from_memory(&read_bytes(key, agent)?).ok()?;
    Some(downscale(image, MAX_IMAGE_DIM))
}

fn read_bytes(key: &ImageKey, agent: &ureq::Agent) -> Option<Vec<u8>> {
    match key {
        ImageKey::Path(path) => fs::read(path).ok(),
        ImageKey::Url(url) => agent
            .get(url)
            .call()
            .ok()?
            .body_mut()
            .with_config()
            .limit(MAX_FETCH_BYTES)
            .read_to_vec()
            .ok(),
    }
}

fn downscale(image: DynamicImage, max_dim: u32) -> DynamicImage {
    if image.width().max(image.height()) <= max_dim {
        image
    } else {
        image.resize(max_dim, max_dim, image::imageops::FilterType::Nearest)
    }
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
