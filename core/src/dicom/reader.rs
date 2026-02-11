use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::imaging::types::{ColorSpace, Frame, FrameSequence, FrameSource};

/// Supported image file extensions
const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "bmp", "tiff", "tif"];
/// Supported DICOM extensions
const DICOM_EXTENSIONS: &[&str] = &["dcm", "dicom"];
/// Supported video extensions
const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mov", "avi", "wmv", "mkv"];

/// Detect input type from file extension and load accordingly
pub fn load_file(path: &str) -> Result<FrameSequence> {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    if DICOM_EXTENSIONS.contains(&ext.as_str()) {
        load_dicom(path)
    } else if VIDEO_EXTENSIONS.contains(&ext.as_str()) {
        crate::video::extractor::load_video(path)
    } else if IMAGE_EXTENSIONS.contains(&ext.as_str()) {
        load_image(path)
    } else {
        // Try DICOM first (many DICOM files have no extension)
        load_dicom(path).or_else(|_| bail!("unsupported file format: .{ext}"))
    }
}

/// Load a DICOM file and extract frames
pub fn load_dicom(path: &str) -> Result<FrameSequence> {
    let obj = dicom::object::open_file(path)
        .with_context(|| format!("failed to open DICOM file: {path}"))?;
    let pixel_data = dicom_pixeldata::PixelDecoder::decode_pixel_data(&obj)
        .context("failed to decode DICOM pixel data")?;

    let cols = pixel_data.columns() as u32;
    let rows = pixel_data.rows() as u32;
    let n_frames = pixel_data.number_of_frames() as usize;
    let samples = pixel_data.samples_per_pixel();

    let colorspace = match samples {
        1 => ColorSpace::Grayscale,
        3 => ColorSpace::Rgb,
        4 => ColorSpace::Rgba,
        _ => ColorSpace::Grayscale,
    };

    let mut frames = Vec::with_capacity(n_frames);

    for i in 0..n_frames {
        let data: Vec<u8> = pixel_data
            .to_vec_frame(i as u32)
            .with_context(|| format!("failed to extract frame {i}/{n_frames}"))?;

        frames.push(Frame {
            data,
            width: cols,
            height: rows,
            colorspace,
            source: FrameSource::Dicom {
                path: path.to_string(),
            },
        });
    }

    tracing::info!("loaded DICOM: {cols}x{rows}, {n_frames} frames, {samples} samples/pixel");
    Ok(FrameSequence { frames, fps: None })
}

/// Load a regular image file (PNG, JPEG, BMP, TIFF)
pub fn load_image(path: &str) -> Result<FrameSequence> {
    let img = image::open(path).with_context(|| format!("failed to open image: {path}"))?;

    let (width, height) = (img.width(), img.height());

    // Determine if grayscale or color
    let (data, colorspace) = match img.color() {
        image::ColorType::L8 | image::ColorType::L16 => {
            (img.to_luma8().into_raw(), ColorSpace::Grayscale)
        }
        _ => (img.to_rgb8().into_raw(), ColorSpace::Rgb),
    };

    tracing::info!("loaded image: {width}x{height}, {colorspace:?}");

    Ok(FrameSequence {
        frames: vec![Frame {
            data,
            width,
            height,
            colorspace,
            source: FrameSource::Image {
                path: path.to_string(),
            },
        }],
        fps: None,
    })
}
