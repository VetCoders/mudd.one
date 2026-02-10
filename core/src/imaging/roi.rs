use anyhow::Result;

use super::types::{Frame, Roi};

/// Detect the ultrasound ROI from a frame by classical CV:
/// grayscale → binary threshold (Otsu) → morphological close → largest connected component → bbox
pub fn detect_roi(frame: &Frame) -> Result<Roi> {
    // TODO: implement classical CV pipeline
    // For now, return full frame as ROI
    Ok(Roi {
        x: 0,
        y: 0,
        width: frame.width,
        height: frame.height,
    })
}
