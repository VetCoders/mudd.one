use anyhow::{Context, Result};
use ndarray::Array;
use ort::value::Value;

use crate::imaging::normalize;
use crate::imaging::types::{ColorSpace, Frame, Mask};

/// Prompt point for segmentation (x, y in pixel coords, label: 1=foreground, 0=background)
#[derive(Debug, Clone, Copy)]
pub struct PromptPoint {
    pub x: f32,
    pub y: f32,
    pub label: i64,
}

/// Run segmentation inference on a frame
///
/// The model is expected to be a SAM-style encoder that takes:
/// - input: [1, 3, H, W] float32 normalized image
/// - output: feature embeddings or mask logits
///
/// Actual I/O depends on the chosen model (UltraSam, SAM2, etc.)
pub fn segment_frame(frame: &Frame, prompts: &[PromptPoint]) -> Result<Vec<Mask>> {
    if !super::engine::is_initialized() {
        anyhow::bail!("inference engine not initialized — call init() first");
    }

    let mut engine_guard = super::engine::engine().lock().unwrap();
    let session = engine_guard.session_mut();

    // Preprocess: ensure RGB, normalize to [0,1]
    let rgb_frame = if frame.colorspace == ColorSpace::Grayscale {
        // Convert grayscale to RGB by tripling channels
        let mut rgb_data = Vec::with_capacity(frame.data.len() * 3);
        for &pixel in &frame.data {
            rgb_data.push(pixel);
            rgb_data.push(pixel);
            rgb_data.push(pixel);
        }
        Frame {
            data: rgb_data,
            width: frame.width,
            height: frame.height,
            colorspace: ColorSpace::Rgb,
            source: frame.source.clone(),
        }
    } else {
        frame.clone()
    };

    // Resize to model input size (1024x1024 for SAM-style models)
    let model_size = 1024u32;
    let resized = normalize::resize(&rgb_frame, model_size, model_size)?;

    // Convert to [1, 3, H, W] float32 tensor (CHW format, normalized 0-1)
    let h = model_size as usize;
    let w = model_size as usize;
    let mut tensor_data = vec![0.0f32; 3 * h * w];

    for y in 0..h {
        for x in 0..w {
            let src_idx = (y * w + x) * 3;
            tensor_data[y * w + x] = resized.data[src_idx] as f32 / 255.0; // R
            tensor_data[h * w + y * w + x] = resized.data[src_idx + 1] as f32 / 255.0; // G
            tensor_data[2 * h * w + y * w + x] = resized.data[src_idx + 2] as f32 / 255.0; // B
        }
    }

    let input_tensor = Array::from_shape_vec((1, 3, h, w), tensor_data)
        .context("failed to create input tensor")?;

    // Scale prompt points to model input size
    let scale_x = model_size as f32 / frame.width as f32;
    let scale_y = model_size as f32 / frame.height as f32;

    let point_coords: Vec<f32> = prompts
        .iter()
        .flat_map(|p| [p.x * scale_x, p.y * scale_y])
        .collect();
    let point_labels: Vec<i64> = prompts.iter().map(|p| p.label).collect();

    let n_points = prompts.len();
    let coords_tensor = Array::from_shape_vec((1, n_points, 2), point_coords)
        .context("failed to create coords tensor")?;
    let labels_tensor = Array::from_shape_vec((1, n_points), point_labels)
        .context("failed to create labels tensor")?;

    // Run inference using ort Value API (same pattern as CodeScribe silero_ort.rs)
    let image_value = Value::from_array(input_tensor)?;
    let coords_value = Value::from_array(coords_tensor)?;
    let labels_value = Value::from_array(labels_tensor)?;

    let outputs = session.run([
        (&image_value).into(),
        (&coords_value).into(),
        (&labels_value).into(),
    ])?;

    // Extract mask from output
    // SAM models typically output: masks [1, N, H, W], iou_predictions [1, N]
    let mask_output = outputs
        .get("masks")
        .or_else(|| outputs.get("output"))
        .context("model output missing 'masks' tensor")?;

    let (mask_shape_raw, mask_data) = mask_output
        .try_extract_tensor::<f32>()
        .context("failed to extract mask tensor")?;

    let mask_shape: Vec<usize> = mask_shape_raw.iter().map(|&d| d as usize).collect();

    // Take best mask (highest IoU, typically index 0 or the last one)
    let mask_h = *mask_shape.last().unwrap_or(&h);
    let mask_w = if mask_shape.len() >= 2 {
        mask_shape[mask_shape.len() - 2]
    } else {
        w
    };

    // Binarize mask: sigmoid > 0.5
    let binary_mask: Vec<u8> = mask_data
        .iter()
        .take(mask_h * mask_w)
        .map(|&v| {
            let sigmoid = 1.0 / (1.0 + (-v).exp());
            if sigmoid > 0.5 { 255 } else { 0 }
        })
        .collect();

    // Resize mask back to original frame dimensions if needed
    let final_mask = if mask_w != frame.width as usize || mask_h != frame.height as usize {
        let mask_frame = Frame {
            data: binary_mask,
            width: mask_w as u32,
            height: mask_h as u32,
            colorspace: ColorSpace::Grayscale,
            source: frame.source.clone(),
        };
        let resized_mask = normalize::resize(&mask_frame, frame.width, frame.height)?;
        resized_mask.data
    } else {
        binary_mask
    };

    Ok(vec![Mask {
        data: final_mask,
        width: frame.width,
        height: frame.height,
        label: "segmentation".to_string(),
    }])
}
