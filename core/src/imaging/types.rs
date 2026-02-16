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

#[cfg(test)]
mod tests {
    use super::*;

    fn gray_frame(w: u32, h: u32) -> Frame {
        Frame {
            data: vec![128u8; (w * h) as usize],
            width: w,
            height: h,
            colorspace: ColorSpace::Grayscale,
            source: FrameSource::Image {
                path: String::new(),
            },
        }
    }

    #[test]
    fn colorspace_channels() {
        assert_eq!(ColorSpace::Grayscale.channels(), 1);
        assert_eq!(ColorSpace::Rgb.channels(), 3);
        assert_eq!(ColorSpace::Rgba.channels(), 4);
    }

    #[test]
    fn frame_stride_grayscale() {
        let f = gray_frame(100, 50);
        assert_eq!(f.stride(), 100);
    }

    #[test]
    fn frame_stride_rgb() {
        let f = Frame {
            data: vec![0u8; 300 * 200 * 3],
            width: 300,
            height: 200,
            colorspace: ColorSpace::Rgb,
            source: FrameSource::Image {
                path: String::new(),
            },
        };
        assert_eq!(f.stride(), 900);
    }

    #[test]
    fn frame_pixel_count() {
        let f = gray_frame(640, 480);
        assert_eq!(f.pixel_count(), 307200);
    }

    #[test]
    fn frame_metadata_default() {
        let m = FrameMetadata::default();
        assert_eq!(m.frame_index, 0);
        assert_eq!(m.total_frames, 0);
        assert!(m.patient_id.is_none());
    }
}
