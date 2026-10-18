//! Image normalization and cover cropping shared by all hosts.
use serde::{Serialize, Deserialize};
use crate::document::Cover;
pub const MAX_FILE: u64 = 12 * 1024 * 1024;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Asset {
    pub id: String,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub mime: String,
    pub bytes: u64,
}
pub fn prepare_image(bytes: &[u8], name: &str) -> Result<(Asset, Vec<u8>), String> {
    if bytes.len() as u64 > MAX_FILE {
        return Err("Choose a JPEG or PNG image up to 12 MB.".into());
    }
    let format =
        image::guess_format(&bytes).map_err(|_| "Choose a JPEG or PNG image up to 12 MB.")?;
    if !matches!(format, image::ImageFormat::Jpeg | image::ImageFormat::Png) {
        return Err("Choose a JPEG or PNG image up to 12 MB.".into());
    }
    let mut reader = image::ImageReader::with_format(std::io::Cursor::new(&bytes), format);
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(128 * 1024 * 1024);
    limits.max_image_width = Some(12000);
    limits.max_image_height = Some(12000);
    reader.limits(limits);
    use image::ImageDecoder;
    let mut decoder = reader.into_decoder().map_err(|e| e.to_string())?;
    let (w, h) = decoder.dimensions();
    if w == 0 || h == 0 || (w as u64 * h as u64) > 24_000_000 {
        return Err("Choose an image with at most 24 megapixels.".into());
    }
    let orientation = decoder.orientation().map_err(|e| e.to_string())?;
    let mut decoded = image::DynamicImage::from_decoder(decoder).map_err(|e| e.to_string())?;
    decoded.apply_orientation(orientation);
    let resized = if decoded.width() > 2048 || decoded.height() > 2048 {
        decoded.resize(2048, 2048, image::imageops::FilterType::Lanczos3)
    } else {
        decoded
    };
    let mut output = std::io::Cursor::new(Vec::new());
    resized
        .write_to(&mut output, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    let output = output.into_inner();
    if output.len() as u64 > MAX_FILE {
        return Err("This image is too large after processing.".into());
    }
    let id = blake3::hash(&output).to_hex().to_string();
    let asset = Asset {
        id: id.clone(),
        name: name.chars().filter(|c| !c.is_control()).take(160).collect(),
        width: resized.width(),
        height: resized.height(),
        mime: "image/png".into(),
        bytes: output.len() as u64,
    };

    Ok((asset, output))
}

pub fn crop_cover(bytes: &[u8], cover: &Cover, square: bool) -> Result<Vec<u8>, String> {
    if cover.focal_x > 1000 || cover.focal_y > 1000 { return Err("Invalid cover focal point".into()); }
    // Work from the same bounded, normalized asset representation used by import.
    let (_, normalized) = prepare_image(bytes, "cover")?;
    let image = image::load_from_memory(&normalized).map_err(|e| e.to_string())?;
    let ratio = if square { 1.0 } else { 2.35 };
    let (w, h) = (image.width(), image.height());
    let (cw, ch) = if w as f64 / h as f64 > ratio {
        ((h as f64 * ratio) as u32, h)
    } else {
        (w, (w as f64 / ratio) as u32)
    };
    let x = ((w - cw) as f64 * cover.focal_x as f64 / 1000.0) as u32;
    let y = ((h - ch) as f64 * cover.focal_y as f64 / 1000.0) as u32;
    let cropped = image.crop_imm(x, y, cw.max(1), ch.max(1));
    let mut output = std::io::Cursor::new(Vec::new());
    cropped
        .write_to(&mut output, ::image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(output.into_inner())
}
