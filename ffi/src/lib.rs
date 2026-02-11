// mudd-ffi — UniFFI bridge for mudd.one
// Created by M&K (c)2026 VetCoders

uniffi::setup_scaffolding!();

use mudd_core::imaging::types::{ColorSpace, FilterType, Frame, FrameSource, Roi};

// ═══════════════════════════════════════════════════════════
// FFI error type (UniFFI requires a proper error enum)
// ═══════════════════════════════════════════════════════════

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum MuddError {
    #[error("{msg}")]
    Core { msg: String },
}

impl From<anyhow::Error> for MuddError {
    fn from(e: anyhow::Error) -> Self {
        MuddError::Core {
            msg: format!("{e:#}"),
        }
    }
}

// ═══════════════════════════════════════════════════════════
// FFI types — flat representations for Swift
// ═══════════════════════════════════════════════════════════

#[derive(uniffi::Record)]
pub struct FfiFrame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub channels: u8, // 1=gray, 3=rgb, 4=rgba
}

#[derive(uniffi::Record)]
pub struct FfiRoi {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(uniffi::Record)]
pub struct FfiMask {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub label: String,
}

#[derive(uniffi::Record)]
pub struct FfiPromptPoint {
    pub x: f32,
    pub y: f32,
    pub label: i64,
}

#[derive(uniffi::Enum)]
pub enum FfiFilterType {
    HistogramEqualization,
    ContrastStretch,
    AdaptiveThreshold,
    Canny,
    GaussianBlur,
}

// ═══════════════════════════════════════════════════════════
// Conversion helpers
// ═══════════════════════════════════════════════════════════

fn ffi_to_frame(f: &FfiFrame) -> Frame {
    let colorspace = match f.channels {
        1 => ColorSpace::Grayscale,
        3 => ColorSpace::Rgb,
        4 => ColorSpace::Rgba,
        _ => ColorSpace::Rgb,
    };
    Frame {
        data: f.data.clone(),
        width: f.width,
        height: f.height,
        colorspace,
        source: FrameSource::Image {
            path: String::new(),
        },
    }
}

fn frame_to_ffi(f: &Frame) -> FfiFrame {
    FfiFrame {
        data: f.data.clone(),
        width: f.width,
        height: f.height,
        channels: f.colorspace.channels() as u8,
    }
}

fn ffi_to_filter(f: &FfiFilterType) -> FilterType {
    match f {
        FfiFilterType::HistogramEqualization => FilterType::HistogramEqualization,
        FfiFilterType::ContrastStretch => FilterType::ContrastStretch,
        FfiFilterType::AdaptiveThreshold => FilterType::AdaptiveThreshold,
        FfiFilterType::Canny => FilterType::Canny,
        FfiFilterType::GaussianBlur => FilterType::GaussianBlur,
    }
}

// ═══════════════════════════════════════════════════════════
// Engine
// ═══════════════════════════════════════════════════════════

#[uniffi::export]
pub fn init_engine(model_path: String) -> Result<(), MuddError> {
    mudd_core::inference::engine::init(&model_path)?;
    Ok(())
}

#[uniffi::export]
pub fn init_engine_from_hf(repo: String, filename: String) -> Result<(), MuddError> {
    mudd_core::inference::engine::init_from_hf(&repo, &filename)?;
    Ok(())
}

#[uniffi::export]
pub fn is_engine_ready() -> bool {
    mudd_core::inference::engine::is_initialized()
}

#[uniffi::export]
pub fn engine_model_name() -> Option<String> {
    mudd_core::inference::engine::model_name()
}

// ═══════════════════════════════════════════════════════════
// Input loading
// ═══════════════════════════════════════════════════════════

#[uniffi::export]
pub fn load_file(path: String) -> Result<Vec<FfiFrame>, MuddError> {
    let seq = mudd_core::dicom::reader::load_file(&path)?;
    Ok(seq.frames.iter().map(frame_to_ffi).collect())
}

// ═══════════════════════════════════════════════════════════
// ROI detection + crop
// ═══════════════════════════════════════════════════════════

#[uniffi::export]
pub fn detect_roi(frame: FfiFrame) -> Result<FfiRoi, MuddError> {
    let f = ffi_to_frame(&frame);
    let roi = mudd_core::imaging::roi::detect_roi(&f)?;
    Ok(FfiRoi {
        x: roi.x,
        y: roi.y,
        width: roi.width,
        height: roi.height,
    })
}

#[uniffi::export]
pub fn crop_frame(frame: FfiFrame, roi: FfiRoi) -> Result<FfiFrame, MuddError> {
    let f = ffi_to_frame(&frame);
    let r = Roi {
        x: roi.x,
        y: roi.y,
        width: roi.width,
        height: roi.height,
    };
    let cropped = mudd_core::imaging::crop::crop_frame(&f, &r)?;
    Ok(frame_to_ffi(&cropped))
}

// ═══════════════════════════════════════════════════════════
// Filters
// ═══════════════════════════════════════════════════════════

#[uniffi::export]
pub fn apply_filter(frame: FfiFrame, filter_type: FfiFilterType) -> Result<FfiFrame, MuddError> {
    let f = ffi_to_frame(&frame);
    let ft = ffi_to_filter(&filter_type);
    let result = mudd_core::imaging::filters::apply_filter(&f, ft)?;
    Ok(frame_to_ffi(&result))
}

#[uniffi::export]
pub fn apply_filters(
    frame: FfiFrame,
    filter_types: Vec<FfiFilterType>,
) -> Result<FfiFrame, MuddError> {
    let f = ffi_to_frame(&frame);
    let fts: Vec<FilterType> = filter_types.iter().map(ffi_to_filter).collect();
    let result = mudd_core::imaging::filters::apply_filters(&f, &fts)?;
    Ok(frame_to_ffi(&result))
}

// ═══════════════════════════════════════════════════════════
// Segmentation
// ═══════════════════════════════════════════════════════════

#[uniffi::export]
pub fn segment_frame(
    frame: FfiFrame,
    prompts: Vec<FfiPromptPoint>,
) -> Result<Vec<FfiMask>, MuddError> {
    let f = ffi_to_frame(&frame);
    let pts: Vec<mudd_core::inference::segmentation::PromptPoint> = prompts
        .iter()
        .map(|p| mudd_core::inference::segmentation::PromptPoint {
            x: p.x,
            y: p.y,
            label: p.label,
        })
        .collect();
    let masks = mudd_core::inference::segmentation::segment_frame(&f, &pts)?;
    Ok(masks
        .iter()
        .map(|m| FfiMask {
            data: m.data.clone(),
            width: m.width,
            height: m.height,
            label: m.label.clone(),
        })
        .collect())
}
