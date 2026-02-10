use serde::{Deserialize, Serialize};

use crate::imaging::types::{FilterType, Frame, FrameMetadata, Mask, Roi};

/// After ROI detection and cropping
#[derive(Debug, Clone)]
pub struct CroppedFrame {
    pub frame: Frame,
    pub roi: Roi,
    pub original_width: u32,
    pub original_height: u32,
}

/// After filter application
#[derive(Debug, Clone)]
pub struct ProcessedFrame {
    pub frame: Frame,
    pub filters_applied: Vec<FilterType>,
}

/// After segmentation/annotation
#[derive(Debug, Clone)]
pub struct AnnotatedFrame {
    pub frame: Frame,
    pub masks: Vec<Mask>,
}

/// Ready for export
#[derive(Debug, Clone)]
pub struct ExportItem {
    pub processed: ProcessedFrame,
    pub annotation: Option<AnnotatedFrame>,
    pub metadata: FrameMetadata,
}

/// Export format configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    pub format: ExportFormat,
    pub output_dir: String,
    pub image_format: ImageExportFormat,
    pub include_metadata: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ExportFormat {
    Coco,
    Yolo,
    Custom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ImageExportFormat {
    Png,
    Jpeg,
    Tiff,
}
