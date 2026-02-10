use anyhow::Result;

use crate::imaging::types::{Frame, Mask};

/// Run segmentation inference on a frame, returning a binary mask
pub fn segment_frame(frame: &Frame, _prompt_points: &[(f32, f32)]) -> Result<Mask> {
    if !super::engine::is_initialized() {
        anyhow::bail!("inference engine not initialized");
    }

    let _engine = super::engine::engine().lock().unwrap();

    // TODO: implement model-specific preprocessing, inference, postprocessing
    // Pattern: preprocess frame → ndarray tensor → session.run() → extract mask
    // Will depend on chosen model (UltraSam, SAM2, etc.)

    Ok(Mask {
        data: vec![0u8; frame.pixel_count()],
        width: frame.width,
        height: frame.height,
        label: "segmentation".to_string(),
    })
}
