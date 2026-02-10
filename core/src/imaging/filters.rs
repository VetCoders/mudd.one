use anyhow::{Result, Context};
use image::GrayImage;
use imageproc::contrast;
use imageproc::edges;
use imageproc::filter;

use super::types::{ColorSpace, FilterType, Frame};

/// Apply a filter to a frame, returning a new frame
pub fn apply_filter(frame: &Frame, filter_type: FilterType) -> Result<Frame> {
    match filter_type {
        FilterType::HistogramEqualization => histogram_equalization(frame),
        FilterType::ContrastStretch => contrast_stretch(frame),
        FilterType::AdaptiveThreshold => adaptive_threshold(frame),
        FilterType::Canny => canny_edge(frame),
        FilterType::GaussianBlur => gaussian_blur(frame),
    }
}

/// Apply multiple filters in sequence
pub fn apply_filters(frame: &Frame, filters: &[FilterType]) -> Result<Frame> {
    let mut result = frame.clone();
    for &f in filters {
        result = apply_filter(&result, f)?;
    }
    Ok(result)
}

fn to_gray_image(frame: &Frame) -> Result<GrayImage> {
    let gray_data = match frame.colorspace {
        ColorSpace::Grayscale => frame.data.clone(),
        _ => {
            let channels = frame.colorspace.channels();
            frame
                .data
                .chunks_exact(channels)
                .map(|px| (0.299 * px[0] as f32 + 0.587 * px[1] as f32 + 0.114 * px[2] as f32) as u8)
                .collect()
        }
    };

    GrayImage::from_raw(frame.width, frame.height, gray_data)
        .context("failed to create grayscale image for filter")
}

fn gray_to_frame(img: GrayImage, source: &Frame) -> Frame {
    Frame {
        width: img.width(),
        height: img.height(),
        data: img.into_raw(),
        colorspace: ColorSpace::Grayscale,
        source: source.source.clone(),
    }
}

fn histogram_equalization(frame: &Frame) -> Result<Frame> {
    let gray = to_gray_image(frame)?;
    let equalized = contrast::equalize_histogram(&gray);
    Ok(gray_to_frame(equalized, frame))
}

fn contrast_stretch(frame: &Frame) -> Result<Frame> {
    let gray = to_gray_image(frame)?;
    let stretched = contrast::stretch_contrast(&gray, 0, 255, 0, 255);
    Ok(gray_to_frame(stretched, frame))
}

fn adaptive_threshold(frame: &Frame) -> Result<Frame> {
    let gray = to_gray_image(frame)?;
    let block_radius = 15; // 31x31 window
    let result = contrast::adaptive_threshold(&gray, block_radius);
    Ok(gray_to_frame(result, frame))
}

fn canny_edge(frame: &Frame) -> Result<Frame> {
    let gray = to_gray_image(frame)?;
    let result = edges::canny(&gray, 50.0, 150.0);
    Ok(gray_to_frame(result, frame))
}

fn gaussian_blur(frame: &Frame) -> Result<Frame> {
    let gray = to_gray_image(frame)?;
    let sigma = 2.0f32;
    let blurred: GrayImage = filter::gaussian_blur_f32(&gray, sigma);
    Ok(gray_to_frame(blurred, frame))
}
