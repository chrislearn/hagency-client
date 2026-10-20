//! Application-owned chat working sets. These are admission limits, not RSS or
//! whole-driver VRAM limits (fonts, swapchains and driver bookkeeping are separate).
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicUsize, Ordering},
    },
};
use makepad_widgets::{
    makepad_draw::image_cache::{self, ImageCache, ImageCacheEntry, ImageError},
    image_cache::ImageCacheImpl,
    widget::WidgetWeakRef,
    *,
};

pub const ENCODED_BYTES: usize = 64 * 1024 * 1024;
pub const IMAGE_FILE_BYTES: usize = 8 * 1024 * 1024;
pub const IMAGE_PIXELS_BYTES: usize = 16 * 1024 * 1024;
pub const IMAGE_WORKING_BYTES: usize = 128 * 1024 * 1024;
pub const GPU_TEXTURE_BYTES: u64 = 256 * 1024 * 1024;
pub const GPU_INSTANCE_BYTES: usize = 32 * 1024 * 1024;

/// Chat I/O should not create one runtime thread per CPU on large workstations.
/// Blocking database/search work queues behind a separate, finite worker pool.
pub fn bounded_async_runtime() -> std::io::Result<tokio::runtime::Runtime> {
    let workers = std::thread::available_parallelism()
        .map_or(2, usize::from)
        .min(4);
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(workers)
        .max_blocking_threads(8)
        .thread_keep_alive(std::time::Duration::from_secs(5))
        .build()
}

#[derive(Default)]
struct EncodedLedger(Vec<Weak<[u8]>>);
impl EncodedLedger {
    fn admit(&mut self, data: &Arc<[u8]>) -> bool {
        self.0.retain(|entry| entry.strong_count() != 0);
        let weak = Arc::downgrade(data);
        if self.0.iter().any(|entry| entry.ptr_eq(&weak)) {
            return true;
        }
        let bytes = self
            .0
            .iter()
            .filter_map(Weak::upgrade)
            .map(|v| v.len())
            .sum::<usize>();
        if data.len() > IMAGE_FILE_BYTES
            || self.0.len() >= 512
            || bytes.saturating_add(data.len()) > ENCODED_BYTES
        {
            return false;
        }
        self.0.push(weak);
        true
    }
}
static ENCODED: Mutex<EncodedLedger> = Mutex::new(EncodedLedger(Vec::new()));

/// Weak accounting follows the last Arc, including callbacks, decoders and viewers.
pub fn admit_encoded(data: &Arc<[u8]>) -> bool {
    ENCODED.lock().unwrap().admit(data)
}

static IMAGE_WORKING: AtomicUsize = AtomicUsize::new(0);
#[derive(Debug)]
pub struct ImagePermit(usize);
impl ImagePermit {
    pub fn pixel_limit(&self) -> usize {
        self.0 / 6
    }

    fn reserve(bytes: usize) -> Result<Self, ImageError> {
        IMAGE_WORKING
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |held| {
                held.checked_add(bytes)
                    .filter(|total| *total <= IMAGE_WORKING_BYTES)
            })
            .map_err(|held| too_large(held.saturating_add(bytes), IMAGE_WORKING_BYTES))?;
        Ok(Self(bytes))
    }
}
impl Drop for ImagePermit {
    fn drop(&mut self) {
        IMAGE_WORKING.fetch_sub(self.0, Ordering::AcqRel);
    }
}

pub fn reserve_viewer(cx: &mut Cx, data: &[u8]) -> Result<ImagePermit, ImageError> {
    let bytes = image_cost(data, Path::new(""))?;
    if !gpu_image_fits(
        cx.texture_pool_bytes(),
        IMAGE_WORKING.load(Ordering::Acquire),
        bytes,
    ) {
        return Err(too_large(bytes, IMAGE_WORKING_BYTES));
    }
    ImagePermit::reserve(bytes)
}

fn gpu_image_fits(allocated: u64, held: usize, next: usize) -> bool {
    // Include every admitted decode that may upload later. Counting resident
    // images twice is conservative and avoids overbooking before the next Draw.
    allocated.saturating_add(held.saturating_add(next) as u64 / 2) <= GPU_TEXTURE_BYTES
}

struct ImageLease {
    key: PathBuf,
    _permit: ImagePermit,
    owners: Vec<ImageOwner>,
    texture: Option<Texture>,
}
struct ImageOwner {
    widget: WidgetWeakRef,
    bound: bool,
}
impl ImageOwner {
    fn new(widget: &WidgetRef) -> Self {
        Self {
            widget: widget.downgrade(),
            bound: false,
        }
    }
}
#[derive(Default)]
pub struct ImageBudget {
    images: Vec<ImageLease>,
    pub refused: u64,
    pub evicted: u64,
}

impl ImageBudget {
    /// A widget changing to another asset releases its old claim. Weak references
    /// do not keep invisible PortalList rows alive.
    fn detach(&mut self, widget: &WidgetRef) {
        for image in &mut self.images {
            image.owners.retain(|owner| owner.widget != *widget);
        }
    }
}

/// Retire only images with no widget readers. Pending decodes keep their charge
/// until completion, even if the requesting row has disappeared.
pub fn collect_images(cx: &mut Cx) {
    ENCODED
        .lock()
        .unwrap()
        .0
        .retain(|entry| entry.strong_count() != 0);
    if !cx.has_global::<ImageBudget>() {
        return;
    }
    let mut budget = std::mem::take(cx.get_global::<ImageBudget>());
    for image in &mut budget.images {
        if image.texture.is_none() && cx.has_global::<ImageCache>() {
            image.texture = match cx.get_global::<ImageCache>().map.get(&image.key) {
                Some(ImageCacheEntry::Loaded(texture)) => Some(texture.clone()),
                _ => None,
            };
        }
        image.owners.retain_mut(|owner| {
            owner.widget.upgrade().is_some_and(|widget| {
                let image_ref = widget.as_image();
                image_ref.borrow().is_some_and(|widget| {
                    // While decoding, a live owner still has a pending request.
                    if let Some(texture) = image.texture.as_ref() {
                        if widget.get_texture(0).as_ref() == Some(texture) {
                            owner.bound = true;
                            true
                        } else {
                            !owner.bound
                        }
                    } else {
                        true
                    }
                })
            })
        });
    }
    let mut index = 0;
    while index < budget.images.len() {
        let image = &budget.images[index];
        let pending = cx.has_global::<ImageCache>()
            && matches!(
                cx.get_global::<ImageCache>().map.get(&image.key),
                Some(ImageCacheEntry::Loading(..))
            );
        if pending || (image.texture.is_some() && !image.owners.is_empty()) {
            index += 1;
            continue;
        }
        let image = budget.images.swap_remove(index);
        if let Some(texture) = image.texture {
            texture.release(cx);
        }
        image_cache::evict_image_from_cache(cx, &image.key);
        budget.evicted += 1;
    }
    *cx.get_global::<ImageBudget>() = budget;
}

pub fn release_image(cx: &mut Cx, image: &ImageRef) {
    if cx.has_global::<ImageBudget>() {
        cx.get_global::<ImageBudget>().detach(image);
    }
    image.set_texture(cx, None);
}

pub fn image_budget_bytes(cx: &mut Cx) -> usize {
    let _ = cx;
    IMAGE_WORKING.load(Ordering::Acquire)
}

/// Keep a viewer's clone pinned when it takes over a thumbnail texture.
pub fn pin_texture(cx: &mut Cx, image: &ImageRef, texture: &Texture) {
    if cx.has_global::<ImageBudget>() {
        let budget = cx.get_global::<ImageBudget>();
        let index = budget
            .images
            .iter()
            .position(|entry| entry.texture.as_ref() == Some(texture));
        budget.detach(image);
        if let Some(index) = index {
            let entry = &mut budget.images[index];
            entry.owners.push(ImageOwner::new(image));
        }
    }
}

pub fn load_image(
    img: &ImageRef,
    cx: &mut Cx,
    key: &Path,
    data: Arc<[u8]>,
) -> Result<(), ImageError> {
    if !cx.has_global::<ImageBudget>() {
        cx.set_global(ImageBudget::default());
    }
    // Register a reader before collecting: an already decoded image can be
    // handed to a new widget in this very draw.
    let existing = cx
        .get_global::<ImageBudget>()
        .images
        .iter()
        .position(|entry| entry.key == key);
    if let Some(index) = existing {
        let cached =
            cx.has_global::<ImageCache>() && cx.get_global::<ImageCache>().map.contains_key(key);
        if !cached {
            // Failed/externally evicted keys may decode again. Recheck their
            // headers so a changed response cannot reuse a smaller old lease.
            let next = image_cost(&data, key)?;
            let reserved = cx.get_global::<ImageBudget>().images[index]._permit.0;
            if next > reserved {
                return Err(too_large(next, reserved));
            }
        }
        let budget = cx.get_global::<ImageBudget>();
        budget.detach(img);
        budget.images[index].owners.push(ImageOwner::new(img));
        let pixels = budget.images[index]._permit.pixel_limit();
        return image_cache::with_image_decode_limit(pixels, || {
            img.load_image_from_data_async(cx, key, data)
        });
    }
    let bytes = image_cost(&data, key)?;
    if !admit_encoded(&data) {
        return Err(too_large(data.len(), ENCODED_BYTES));
    }
    cx.get_global::<ImageBudget>().detach(img);
    collect_images(cx);
    let held = image_budget_bytes(cx);
    // Reserve decoder/composition output and upload/retirement headroom.
    if held.saturating_add(bytes) > IMAGE_WORKING_BYTES
        || !gpu_image_fits(cx.texture_pool_bytes(), held, bytes)
        || cx.get_global::<ImageBudget>().images.len() >= 256
    {
        cx.get_global::<ImageBudget>().refused += 1;
        return Err(too_large(held.saturating_add(bytes), IMAGE_WORKING_BYTES));
    }
    let budget = cx.get_global::<ImageBudget>();
    budget.detach(img);
    budget.images.push(ImageLease {
        key: key.into(),
        _permit: ImagePermit::reserve(bytes)?,
        owners: vec![ImageOwner::new(img)],
        texture: None,
    });
    let result = image_cache::with_image_decode_limit(bytes / 6, || {
        img.load_image_from_data_async(cx, key, data)
    });
    if result.is_err() {
        release_image(cx, img);
        collect_images(cx);
    }
    result
}

fn too_large(bytes: usize, limit: usize) -> ImageError {
    ImageError::DataTooLarge { bytes, limit }
}

/// Check animation atlas padding too: the renderer packs even a tiny animation
/// into a full-width atlas. Header parsing allocates no pixel buffers.
pub fn image_cost(data: &[u8], key: &Path) -> Result<usize, ImageError> {
    if data.len() > IMAGE_FILE_BYTES {
        return Err(too_large(data.len(), IMAGE_FILE_BYTES));
    }
    // SVG has unbounded path/filter expansion; chat previews require a raster
    // thumbnail. The original attachment remains downloadable.
    if image_cache::looks_like_svg(data) {
        return Err(ImageError::UnsupportedFormat);
    }
    if data.starts_with(&[0, 0, 1, 0]) {
        validate_ico(data)?;
    }
    let (w, h) = image_cache::image_size_by_data(data, key)?;
    let frames = animation_frames(data).ok_or(ImageError::UnsupportedFormat)?;
    let animated_png =
        data.starts_with(b"\x89PNG\r\n\x1a\n") && png_animation_frames(data)?.is_some();
    let pixels = if frames > 1 || animated_png {
        let columns = (Cx::max_texture_width() / w.max(1)).max(1);
        columns
            .checked_mul(w)
            .and_then(|width| width.checked_mul(frames.div_ceil(columns)))
            .and_then(|pixels| pixels.checked_mul(h))
    } else {
        w.checked_mul(h)
    };
    let bytes = pixels
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| too_large(usize::MAX, IMAGE_PIXELS_BYTES))?;
    if bytes > IMAGE_PIXELS_BYTES {
        return Err(too_large(bytes, IMAGE_PIXELS_BYTES));
    }
    // Include decoder scratch/composition, final pixels and mips. This is an
    // admission estimate, not a claim about allocator/driver RSS accounting.
    bytes
        .checked_mul(6)
        .ok_or_else(|| too_large(usize::MAX, IMAGE_WORKING_BYTES))
}

fn validate_ico(data: &[u8]) -> Result<(), ImageError> {
    let invalid = || ImageError::UnsupportedFormat;
    let count =
        u16::from_le_bytes(data.get(4..6).ok_or_else(invalid)?.try_into().unwrap()) as usize;
    if count == 0 || count > 256 {
        return Err(invalid());
    }
    for index in 0..count {
        let entry = data
            .get(6 + index * 16..6 + (index + 1) * 16)
            .ok_or_else(invalid)?;
        let width = if entry[0] == 0 { 256 } else { entry[0] as u32 };
        let height = if entry[1] == 0 { 256 } else { entry[1] as u32 };
        let length = u32::from_le_bytes(entry[8..12].try_into().unwrap()) as usize;
        let offset = u32::from_le_bytes(entry[12..16].try_into().unwrap()) as usize;
        let payload = data
            .get(offset..offset.checked_add(length).ok_or_else(invalid)?)
            .ok_or_else(invalid)?;
        if payload.starts_with(b"\x89PNG\r\n\x1a\n") {
            let dimensions = payload.get(16..24).ok_or_else(invalid)?;
            if u32::from_be_bytes(dimensions[..4].try_into().unwrap()) != width
                || u32::from_be_bytes(dimensions[4..].try_into().unwrap()) != height
                || png_animation_frames(payload)?.is_some()
            {
                return Err(invalid());
            }
        } else {
            // The renderer rewrites the DIB height, but uses its stored width.
            let stored_width =
                u32::from_le_bytes(payload.get(4..8).ok_or_else(invalid)?.try_into().unwrap());
            if stored_width != width {
                return Err(invalid());
            }
        }
    }
    Ok(())
}

fn animation_frames(data: &[u8]) -> Option<usize> {
    if data.starts_with(b"GIF8") {
        let packed = *data.get(10)?;
        let mut pos = 13
            + if packed & 0x80 != 0 {
                3 * (1usize << ((packed & 7) + 1))
            } else {
                0
            };
        let mut frames = 0usize;
        let mut blocks = 0usize;
        while pos < data.len() {
            match *data.get(pos)? {
                0x3b => return (frames > 0).then_some(frames),
                0x21 => {
                    pos += 2;
                }
                0x2c => {
                    frames += 1;
                    let word = |offset| -> Option<usize> {
                        Some(
                            u16::from_le_bytes(data.get(offset..offset + 2)?.try_into().ok()?)
                                as usize,
                        )
                    };
                    let (canvas_w, canvas_h) = (word(6)?, word(8)?);
                    let (left, top, width, height) = (
                        word(pos + 1)?,
                        word(pos + 3)?,
                        word(pos + 5)?,
                        word(pos + 7)?,
                    );
                    if width == 0
                        || height == 0
                        || left.checked_add(width)? > canvas_w
                        || top.checked_add(height)? > canvas_h
                    {
                        return None;
                    }
                    let packed = *data.get(pos + 9)?;
                    pos += 10
                        + if packed & 0x80 != 0 {
                            3 * (1usize << ((packed & 7) + 1))
                        } else {
                            0
                        };
                    pos += 1; // LZW minimum code size
                }
                _ => return None,
            }
            loop {
                blocks += 1;
                if blocks > 65536 {
                    return None;
                }
                let length = *data.get(pos)? as usize;
                pos = pos.checked_add(length + 1)?;
                if pos > data.len() {
                    return None;
                }
                if length == 0 {
                    break;
                }
            }
            if frames > 4096 {
                return None;
            }
        }
        return None;
    }
    if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        return png_animation_frames(data)
            .ok()
            .map(|count| count.unwrap_or(1));
    }
    if data.starts_with(b"RIFF") && data.get(8..12) == Some(b"WEBP") {
        let mut pos = 12usize;
        let mut frames = 0;
        let mut canvas = None;
        let mut chunks = 0usize;
        let word24 = |offset: usize| -> Option<usize> {
            let bytes = data.get(offset..offset + 3)?;
            Some(bytes[0] as usize | ((bytes[1] as usize) << 8) | ((bytes[2] as usize) << 16))
        };
        while pos + 8 <= data.len() {
            chunks += 1;
            if chunks > 4096 {
                return None;
            }
            let length = u32::from_le_bytes(data.get(pos + 4..pos + 8)?.try_into().ok()?) as usize;
            if data.get(pos..pos + 4)? == b"VP8X" {
                if length != 10 || canvas.is_some() {
                    return None;
                }
                canvas = Some((word24(pos + 12)? + 1, word24(pos + 15)? + 1));
            }
            if data.get(pos..pos + 4)? == b"ANMF" {
                if length < 16 {
                    return None;
                }
                let (width, height) = canvas?;
                if (word24(pos + 8)? * 2).checked_add(word24(pos + 14)? + 1)? > width
                    || (word24(pos + 11)? * 2).checked_add(word24(pos + 17)? + 1)? > height
                {
                    return None;
                }
                frames += 1;
            }
            pos = pos.checked_add(length.checked_add(8 + (length & 1))?)?;
            if pos > data.len() || frames > 4096 {
                return None;
            }
        }
        return Some(frames.max(1));
    }
    Some(1)
}

fn png_animation_frames(data: &[u8]) -> Result<Option<usize>, ImageError> {
    let mut pos = 8usize;
    let mut frames = None;
    let mut canvas = None;
    let mut controls = 0usize;
    let mut chunks = 0usize;
    while pos + 12 <= data.len() {
        chunks += 1;
        if chunks > 4096 {
            return Err(ImageError::UnsupportedFormat);
        }
        let length = u32::from_be_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
        let end = pos
            .checked_add(length)
            .and_then(|end| end.checked_add(12))
            .ok_or(ImageError::UnsupportedFormat)?;
        if end > data.len() {
            return Err(ImageError::UnsupportedFormat);
        }
        let kind = &data[pos + 4..pos + 8];
        if kind == b"IHDR" {
            if length != 13 || canvas.is_some() {
                return Err(ImageError::UnsupportedFormat);
            }
            canvas = Some((
                u32::from_be_bytes(data[pos + 8..pos + 12].try_into().unwrap()),
                u32::from_be_bytes(data[pos + 12..pos + 16].try_into().unwrap()),
            ));
        }
        if kind == b"acTL" {
            if frames.is_some() {
                return Err(ImageError::UnsupportedFormat);
            }
            if length != 8 {
                return Err(ImageError::UnsupportedFormat);
            }
            let count = u32::from_be_bytes(data[pos + 8..pos + 12].try_into().unwrap()) as usize;
            if count == 0 || count > 4096 {
                return Err(ImageError::UnsupportedFormat);
            }
            frames = Some(count);
        }
        if kind == b"fcTL" {
            if length != 26 {
                return Err(ImageError::UnsupportedFormat);
            }
            controls += 1;
            let word = |offset| u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap());
            let (width, height, left, top) = (
                word(pos + 12),
                word(pos + 16),
                word(pos + 20),
                word(pos + 24),
            );
            let (canvas_w, canvas_h) = canvas.ok_or(ImageError::UnsupportedFormat)?;
            if width == 0
                || height == 0
                || left.checked_add(width).is_none_or(|end| end > canvas_w)
                || top.checked_add(height).is_none_or(|end| end > canvas_h)
                || frames.is_none_or(|frames| controls > frames)
            {
                return Err(ImageError::UnsupportedFormat);
            }
        }
        pos = end;
    }
    Ok(frames)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn icon_directory_cannot_hide_a_larger_png_payload() {
        let mut icon = vec![0, 0, 1, 0, 1, 0, 1, 1, 0, 0, 1, 0, 32, 0];
        let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
        png.extend(4096u32.to_be_bytes());
        png.extend(4096u32.to_be_bytes());
        icon.extend((png.len() as u32).to_le_bytes());
        icon.extend(22u32.to_le_bytes());
        icon.extend(png);
        assert!(image_cost(&icon, Path::new("icon.ico")).is_err());
    }
    #[test]
    fn gpu_admission_includes_pending_uploads() {
        assert!(gpu_image_fits(
            GPU_TEXTURE_BYTES - 12 * 1024 * 1024,
            12 * 1024 * 1024,
            12 * 1024 * 1024
        ));
        assert!(!gpu_image_fits(
            GPU_TEXTURE_BYTES - 12 * 1024 * 1024,
            24 * 1024 * 1024,
            12 * 1024 * 1024
        ));
    }
    #[test]
    fn pending_decode_keeps_its_budget_until_completion_even_without_readers() {
        let mut cx = Cx::new(Box::new(|_, _| {}));
        cx.set_global(ImageCache::new());
        let key = PathBuf::from("pending.png");
        cx.get_global::<ImageCache>()
            .map
            .insert(key.clone(), ImageCacheEntry::Loading(16, 16));
        cx.set_global(ImageBudget {
            images: vec![ImageLease {
                key: key.clone(),
                _permit: ImagePermit::reserve(3072).unwrap(),
                owners: vec![],
                texture: None,
            }],
            ..Default::default()
        });
        collect_images(&mut cx);
        assert_eq!(cx.get_global::<ImageBudget>().images.len(), 1);
        cx.get_global::<ImageCache>().map.remove(&key);
        collect_images(&mut cx);
        assert!(cx.get_global::<ImageBudget>().images.is_empty());
    }
    #[test]
    fn encoded_budget_counts_live_clones_and_deduplicates() {
        let mut ledger = EncodedLedger::default();
        let data: Arc<[u8]> = vec![0; IMAGE_FILE_BYTES].into();
        assert!(ledger.admit(&data));
        assert!(ledger.admit(&data.clone()));
        let mut held = vec![data];
        for _ in 1..8 {
            let next = vec![0; IMAGE_FILE_BYTES].into();
            assert!(ledger.admit(&next));
            held.push(next);
        }
        assert!(!ledger.admit(&Arc::from([1u8])));
        held.pop();
        assert!(ledger.admit(&Arc::from([1u8])));
        assert!(!ledger.admit(&vec![0; IMAGE_FILE_BYTES + 1].into()));
    }
    #[test]
    fn animation_headers_do_not_undercharge_or_scan_compressed_payload_as_frames() {
        let gif = b"GIF89a\x01\0\x01\0\0\0\0\x2c\0\0\0\0\x01\0\x01\0\0\x02\x02\x2c\x2c\0\x3b";
        assert_eq!(animation_frames(gif), Some(1));
        assert_eq!(animation_frames(&gif[..gif.len() - 2]), None);
        let mut huge_frame = gif.to_vec();
        huge_frame[18..20].copy_from_slice(&65535u16.to_le_bytes());
        assert_eq!(
            animation_frames(&huge_frame),
            None,
            "small logical canvas cannot hide a huge decoded subframe"
        );
        let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\x08acTL\0\0\x10\0\0\0\0\0\0\0\0\0".to_vec();
        assert_eq!(animation_frames(&png), Some(4096));
        png[18] = 0x20;
        assert_eq!(animation_frames(&png), None);
    }
}
