use anyhow::Result;

use super::types::{FilterType, Frame};

/// Apply a filter to a frame, returning a new frame
pub fn apply_filter(frame: &Frame, filter: FilterType) -> Result<Frame> {
    match filter {
        FilterType::HistogramEqualization => histogram_equalization(frame),
        FilterType::ContrastStretch => contrast_stretch(frame),
        FilterType::AdaptiveThreshold => adaptive_threshold(frame),
        FilterType::Canny => canny_edge(frame),
        FilterType::GaussianBlur => gaussian_blur(frame),
    }
}

fn histogram_equalization(_frame: &Frame) -> Result<Frame> {
    // TODO: implement via imageproc::contrast::equalize_histogram
    todo!("histogram equalization")
}

fn contrast_stretch(_frame: &Frame) -> Result<Frame> {
    // TODO: implement via imageproc::contrast::stretch_contrast
    todo!("contrast stretch")
}

fn adaptive_threshold(_frame: &Frame) -> Result<Frame> {
    // TODO: implement via imageproc::contrast::adaptive_threshold
    todo!("adaptive threshold")
}

fn canny_edge(_frame: &Frame) -> Result<Frame> {
    // TODO: implement via imageproc::edges::canny
    todo!("canny edge detection")
}

fn gaussian_blur(_frame: &Frame) -> Result<Frame> {
    // TODO: implement via imageproc::filter::gaussian_blur_f32
    todo!("gaussian blur")
}
