use anyhow::Result;

use super::types::{ColorSpace, Frame};

/// Convert frame to grayscale
pub fn to_grayscale(frame: &Frame) -> Result<Frame> {
    if frame.colorspace == ColorSpace::Grayscale {
        return Ok(frame.clone());
    }

    let channels = frame.colorspace.channels();
    let pixel_count = frame.pixel_count();
    let mut gray = Vec::with_capacity(pixel_count);

    for i in 0..pixel_count {
        let offset = i * channels;
        let r = frame.data[offset] as f32;
        let g = frame.data[offset + 1] as f32;
        let b = frame.data[offset + 2] as f32;
        // ITU-R BT.601 luma
        gray.push((0.299 * r + 0.587 * g + 0.114 * b) as u8);
    }

    Ok(Frame {
        data: gray,
        width: frame.width,
        height: frame.height,
        colorspace: ColorSpace::Grayscale,
        source: frame.source.clone(),
    })
}

/// Resize frame to target dimensions
pub fn resize(frame: &Frame, target_width: u32, target_height: u32) -> Result<Frame> {
    let channels = frame.colorspace.channels() as u32;
    let img = image::ImageBuffer::from_raw(frame.width, frame.height, frame.data.clone())
        .ok_or_else(|| anyhow::anyhow!("failed to create image buffer"))?;

    let resized: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> = match channels {
        1 => {
            let gray: image::GrayImage = img;
            let r = image::imageops::resize(
                &gray,
                target_width,
                target_height,
                image::imageops::FilterType::Lanczos3,
            );
            image::DynamicImage::ImageLuma8(r).to_rgba8()
        }
        3 => {
            let rgb: image::RgbImage =
                image::ImageBuffer::from_raw(frame.width, frame.height, frame.data.clone())
                    .ok_or_else(|| anyhow::anyhow!("failed to create rgb buffer"))?;
            let r = image::imageops::resize(
                &rgb,
                target_width,
                target_height,
                image::imageops::FilterType::Lanczos3,
            );
            image::DynamicImage::ImageRgb8(r).to_rgba8()
        }
        _ => {
            let rgba: image::RgbaImage =
                image::ImageBuffer::from_raw(frame.width, frame.height, frame.data.clone())
                    .ok_or_else(|| anyhow::anyhow!("failed to create rgba buffer"))?;
            image::imageops::resize(
                &rgba,
                target_width,
                target_height,
                image::imageops::FilterType::Lanczos3,
            )
        }
    };

    // Convert back to original colorspace
    let data = match frame.colorspace {
        ColorSpace::Grayscale => {
            let gray = image::DynamicImage::ImageRgba8(resized).to_luma8();
            gray.into_raw()
        }
        ColorSpace::Rgb => {
            let rgb = image::DynamicImage::ImageRgba8(resized).to_rgb8();
            rgb.into_raw()
        }
        ColorSpace::Rgba => resized.into_raw(),
    };

    Ok(Frame {
        data,
        width: target_width,
        height: target_height,
        colorspace: frame.colorspace,
        source: frame.source.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imaging::types::FrameSource;

    #[test]
    fn grayscale_noop() {
        let frame = Frame {
            data: vec![100u8; 16],
            width: 4,
            height: 4,
            colorspace: ColorSpace::Grayscale,
            source: FrameSource::Image {
                path: String::new(),
            },
        };
        let result = to_grayscale(&frame).unwrap();
        assert_eq!(result.colorspace, ColorSpace::Grayscale);
        assert_eq!(result.data, frame.data);
    }

    #[test]
    fn rgb_to_grayscale_bt601() {
        // Single pixel: pure red (255,0,0) → luma = 0.299*255 ≈ 76
        let frame = Frame {
            data: vec![255, 0, 0],
            width: 1,
            height: 1,
            colorspace: ColorSpace::Rgb,
            source: FrameSource::Image {
                path: String::new(),
            },
        };
        let result = to_grayscale(&frame).unwrap();
        assert_eq!(result.colorspace, ColorSpace::Grayscale);
        assert_eq!(result.data.len(), 1);
        assert_eq!(result.data[0], 76); // 0.299 * 255 = 76.245
    }

    #[test]
    fn resize_dimensions() {
        let frame = Frame {
            data: vec![128u8; 64],
            width: 8,
            height: 8,
            colorspace: ColorSpace::Grayscale,
            source: FrameSource::Image {
                path: String::new(),
            },
        };
        let result = resize(&frame, 4, 4).unwrap();
        assert_eq!(result.width, 4);
        assert_eq!(result.height, 4);
        assert_eq!(result.colorspace, ColorSpace::Grayscale);
        assert_eq!(result.data.len(), 16);
    }

    #[test]
    fn resize_preserves_colorspace_rgb() {
        let frame = Frame {
            data: vec![128u8; 8 * 8 * 3],
            width: 8,
            height: 8,
            colorspace: ColorSpace::Rgb,
            source: FrameSource::Image {
                path: String::new(),
            },
        };
        let result = resize(&frame, 4, 4).unwrap();
        assert_eq!(result.colorspace, ColorSpace::Rgb);
        assert_eq!(result.data.len(), 4 * 4 * 3);
    }
}
