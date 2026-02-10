use anyhow::Result;
use dicom::dictionary_std::tags;

use crate::imaging::types::FrameMetadata;

/// Extract metadata from a DICOM file
pub fn extract_metadata(path: &str) -> Result<FrameMetadata> {
    let obj = dicom::object::open_file(path)?;

    let patient_id = obj
        .element_opt(tags::PATIENT_ID)?
        .map(|e| e.to_str().map(|s| s.to_string()))
        .transpose()?;

    let study_date = obj
        .element_opt(tags::STUDY_DATE)?
        .map(|e| e.to_str().map(|s| s.to_string()))
        .transpose()?;

    let modality = obj
        .element_opt(tags::MODALITY)?
        .map(|e| e.to_str().map(|s| s.to_string()))
        .transpose()?;

    let total_frames = obj
        .element_opt(tags::NUMBER_OF_FRAMES)?
        .map(|e| e.to_str().map(|s| s.parse::<usize>().unwrap_or(1)))
        .transpose()?
        .unwrap_or(1);

    let pixel_spacing = obj
        .element_opt(tags::PIXEL_SPACING)?
        .and_then(|e| {
            let s = e.to_str().ok()?;
            let parts: Vec<&str> = s.split('\\').collect();
            if parts.len() == 2 {
                Some((
                    parts[0].parse::<f64>().ok()?,
                    parts[1].parse::<f64>().ok()?,
                ))
            } else {
                None
            }
        });

    Ok(FrameMetadata {
        patient_id,
        study_date,
        modality,
        pixel_spacing,
        frame_index: 0,
        total_frames,
    })
}
