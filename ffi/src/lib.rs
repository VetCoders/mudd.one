// mudd-ffi — UniFFI bridge for mudd.one
// Created by M&K (c)2026 VetCoders

uniffi::setup_scaffolding!();

use mudd_core::imaging::types::{ColorSpace, FilterType, Frame, FrameSource, Mask, Roi};

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
// Conversion helpers (owned — no unnecessary clones)
// ═══════════════════════════════════════════════════════════

fn ffi_to_frame(f: FfiFrame) -> Result<Frame, MuddError> {
    let colorspace = match f.channels {
        1 => ColorSpace::Grayscale,
        3 => ColorSpace::Rgb,
        4 => ColorSpace::Rgba,
        _ => {
            return Err(MuddError::Core {
                msg: format!("unsupported channel count: {}", f.channels),
            });
        }
    };
    Ok(Frame {
        data: f.data,
        width: f.width,
        height: f.height,
        colorspace,
        source: FrameSource::Image {
            path: String::new(),
        },
    })
}

fn frame_to_ffi(f: Frame) -> FfiFrame {
    FfiFrame {
        channels: f.colorspace.channels() as u8,
        data: f.data,
        width: f.width,
        height: f.height,
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

fn ffi_to_mask(m: FfiMask) -> Mask {
    Mask {
        data: m.data,
        width: m.width,
        height: m.height,
        label: m.label,
    }
}

// ═══════════════════════════════════════════════════════════
// Engine
// ═══════════════════════════════════════════════════════════

#[uniffi::export]
pub fn init_engine(model_path: String) -> Result<(), MuddError> {
    mudd_core::inference::segmentation::init(&model_path)?;
    Ok(())
}

#[uniffi::export]
pub fn init_engine_from_hf(repo: String, filename: String) -> Result<(), MuddError> {
    mudd_core::inference::segmentation::init_from_hf(&repo, &filename)?;
    Ok(())
}

#[uniffi::export]
pub fn is_engine_ready() -> bool {
    mudd_core::inference::segmentation::is_initialized()
}

#[uniffi::export]
pub fn engine_model_name() -> Option<String> {
    mudd_core::inference::segmentation::model_name()
}

// ═══════════════════════════════════════════════════════════
// Input loading
// ═══════════════════════════════════════════════════════════

#[uniffi::export]
pub fn load_file(path: String) -> Result<Vec<FfiFrame>, MuddError> {
    let seq = mudd_core::dicom::reader::load_file(&path)?;
    Ok(seq.frames.into_iter().map(frame_to_ffi).collect())
}

// ═══════════════════════════════════════════════════════════
// ROI detection + crop
// ═══════════════════════════════════════════════════════════

#[uniffi::export]
pub fn detect_roi(frame: FfiFrame) -> Result<FfiRoi, MuddError> {
    let f = ffi_to_frame(frame)?;
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
    let f = ffi_to_frame(frame)?;
    let r = Roi {
        x: roi.x,
        y: roi.y,
        width: roi.width,
        height: roi.height,
    };
    let cropped = mudd_core::imaging::crop::crop_frame(&f, &r)?;
    Ok(frame_to_ffi(cropped))
}

// ═══════════════════════════════════════════════════════════
// Filters
// ═══════════════════════════════════════════════════════════

#[uniffi::export]
pub fn apply_filter(frame: FfiFrame, filter_type: FfiFilterType) -> Result<FfiFrame, MuddError> {
    let f = ffi_to_frame(frame)?;
    let ft = ffi_to_filter(&filter_type);
    let result = mudd_core::imaging::filters::apply_filter(&f, ft)?;
    Ok(frame_to_ffi(result))
}

#[uniffi::export]
pub fn apply_filters(
    frame: FfiFrame,
    filter_types: Vec<FfiFilterType>,
) -> Result<FfiFrame, MuddError> {
    let f = ffi_to_frame(frame)?;
    let fts: Vec<FilterType> = filter_types.iter().map(ffi_to_filter).collect();
    let result = mudd_core::imaging::filters::apply_filters(&f, &fts)?;
    Ok(frame_to_ffi(result))
}

// ═══════════════════════════════════════════════════════════
// Segmentation
// ═══════════════════════════════════════════════════════════

#[uniffi::export]
pub fn segment_frame(
    frame: FfiFrame,
    prompts: Vec<FfiPromptPoint>,
) -> Result<Vec<FfiMask>, MuddError> {
    use mudd_core::inference::segmentation::{PromptPoint, PromptSet};

    let f = ffi_to_frame(frame)?;
    let pts: Vec<PromptPoint> = prompts
        .iter()
        .map(|p| PromptPoint {
            x: p.x,
            y: p.y,
            label: p.label,
        })
        .collect();
    let masks = mudd_core::inference::segmentation::segment_frame(&f, &[PromptSet::Points(pts)])?;
    Ok(masks
        .into_iter()
        .map(|m| FfiMask {
            data: m.data,
            width: m.width,
            height: m.height,
            label: m.label,
        })
        .collect())
}

// ═══════════════════════════════════════════════════════════
// Export
// ═══════════════════════════════════════════════════════════

#[derive(uniffi::Enum)]
pub enum FfiExportFormat {
    Coco,
    Yolo,
}

#[derive(uniffi::Enum)]
pub enum FfiImageFormat {
    Png,
    Jpeg,
    Tiff,
}

/// One frame with optional masks, ready for export
#[derive(uniffi::Record)]
pub struct FfiExportItem {
    pub frame: FfiFrame,
    pub masks: Vec<FfiMask>,
    pub frame_index: u32,
}

#[uniffi::export]
pub fn export_dataset(
    output_dir: String,
    format: FfiExportFormat,
    image_format: FfiImageFormat,
    items: Vec<FfiExportItem>,
) -> Result<u32, MuddError> {
    use mudd_core::imaging::types::FrameMetadata;
    use mudd_core::pipeline::contracts::{
        AnnotatedFrame, ExportConfig, ExportFormat, ExportItem, ImageExportFormat, ProcessedFrame,
    };

    let total = items.len() as u32;

    let core_config = ExportConfig {
        format: match format {
            FfiExportFormat::Coco => ExportFormat::Coco,
            FfiExportFormat::Yolo => ExportFormat::Yolo,
        },
        output_dir: output_dir.clone(),
        image_format: match image_format {
            FfiImageFormat::Png => ImageExportFormat::Png,
            FfiImageFormat::Jpeg => ImageExportFormat::Jpeg,
            FfiImageFormat::Tiff => ImageExportFormat::Tiff,
        },
        include_metadata: false,
    };

    let core_items: Vec<ExportItem> = items
        .into_iter()
        .map(|item| {
            let frame = ffi_to_frame(item.frame)?;
            let annotation = if item.masks.is_empty() {
                None
            } else {
                Some(AnnotatedFrame {
                    frame: frame.clone(),
                    masks: item.masks.into_iter().map(ffi_to_mask).collect(),
                })
            };
            Ok(ExportItem {
                processed: ProcessedFrame {
                    frame,
                    filters_applied: Vec::new(),
                },
                annotation,
                metadata: FrameMetadata {
                    frame_index: item.frame_index as usize,
                    total_frames: total as usize,
                    ..Default::default()
                },
            })
        })
        .collect::<Result<Vec<_>, MuddError>>()?;

    match core_config.format {
        ExportFormat::Coco => mudd_core::export::coco::export_coco(&core_config, &core_items)?,
        ExportFormat::Yolo => mudd_core::export::yolo::export_yolo(&core_config, &core_items)?,
        ExportFormat::Custom => {
            return Err(MuddError::Core {
                msg: "custom export format not implemented".to_string(),
            });
        }
    }

    Ok(total)
}
