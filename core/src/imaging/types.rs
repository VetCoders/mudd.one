use serde::{Deserialize, Serialize};

/// Color space of a frame
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorSpace {
    Grayscale,
    Rgb,
    Rgba,
}

impl ColorSpace {
    pub fn channels(&self) -> usize {
        match self {
            Self::Grayscale => 1,
            Self::Rgb => 3,
            Self::Rgba => 4,
        }
    }
}

/// Source of a frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FrameSource {
    Dicom { path: String },
    Video { path: String, frame_index: usize },
    Image { path: String },
}

/// A single image frame
#[derive(Debug, Clone)]
pub struct Frame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub colorspace: ColorSpace,
    pub source: FrameSource,
}

impl Frame {
    pub fn stride(&self) -> usize {
        self.width as usize * self.colorspace.channels()
    }

    pub fn pixel_count(&self) -> usize {
        self.width as usize * self.height as usize
    }
}

/// Sequence of frames (from video or multiframe DICOM)
#[derive(Debug)]
pub struct FrameSequence {
    pub frames: Vec<Frame>,
    pub fps: Option<f64>,
}

/// Region of interest bounding box
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Roi {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Binary segmentation mask
#[derive(Debug, Clone)]
pub struct Mask {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub label: String,
}

/// Available filter types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterType {
    HistogramEqualization,
    ContrastStretch,
    AdaptiveThreshold,
    Canny,
    GaussianBlur,
}

/// Frame metadata extracted from DICOM or video
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FrameMetadata {
    pub patient_id: Option<String>,
    pub study_date: Option<String>,
    pub modality: Option<String>,
    pub pixel_spacing: Option<(f64, f64)>,
    pub frame_index: usize,
    pub total_frames: usize,
}
