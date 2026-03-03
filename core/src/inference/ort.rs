//! ORT segmenter backend implementation.
//! Created by M&K (c)2026 VetCoders

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use ndarray::Array;
use ort::value::Value;

use super::backend::{
    BackendCapabilities, BackendHealth, PromptPoint, PromptSet, SegmenterBackend,
};
use super::hf_cache;
use crate::imaging::normalize;
use crate::imaging::types::{ColorSpace, Frame, Mask};

pub struct OrtBackend {
    session: Option<ort::session::Session>,
    model_name: String,
}

impl OrtBackend {
    pub fn new() -> Self {
        Self {
            session: None,
            model_name: String::new(),
        }
    }

    fn resolve_model_path(repo: &str, filename: &str) -> Result<PathBuf> {
        if let Ok(path) = std::env::var("MUDD_MODEL_PATH") {
            let model_path = PathBuf::from(&path);
            if model_path.exists() {
                return Ok(model_path);
            }
            tracing::warn!("MUDD_MODEL_PATH={path} does not exist, falling back to HF cache");
        }

        let snapshot = hf_cache::find_snapshot(repo, &[filename]).with_context(|| {
            format!(
                "model not found in HF cache: {repo}/{filename}. \
                 Download with: huggingface-cli download {repo} {filename}"
            )
        })?;
        let model_path = snapshot.join(filename);
        if !model_path.exists() {
            anyhow::bail!(
                "snapshot found but missing model file: {}",
                model_path.display()
            );
        }

        tracing::info!("resolved model from HF cache: {}", model_path.display());
        Ok(model_path)
    }
}

impl Default for OrtBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl SegmenterBackend for OrtBackend {
    fn init_from_path(&mut self, model_path: &str) -> Result<()> {
        tracing::info!("initializing ORT backend from: {model_path}");

        let session = ort::session::Session::builder()?
            .with_execution_providers([ort::ep::CoreML::default().build()])?
            .with_intra_threads(1)?
            .commit_from_file(model_path)
            .with_context(|| format!("failed to load ONNX model: {model_path}"))?;

        for (i, input) in session.inputs().iter().enumerate() {
            tracing::info!("  input[{i}]: {} {:?}", input.name(), input.dtype());
        }
        for (i, output) in session.outputs().iter().enumerate() {
            tracing::info!("  output[{i}]: {} {:?}", output.name(), output.dtype());
        }

        self.model_name = Path::new(model_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        self.session = Some(session);

        tracing::info!("ORT backend ready");
        Ok(())
    }

    fn init_from_hf(&mut self, repo: &str, filename: &str) -> Result<()> {
        let model_path = Self::resolve_model_path(repo, filename)?;
        let model_path = model_path
            .to_str()
            .context("invalid model path from HF cache")?;
        self.init_from_path(model_path)
    }

    fn segment(&mut self, frame: &Frame, prompts: &[PromptSet]) -> Result<Vec<Mask>> {
        let session = self
            .session
            .as_mut()
            .context("ORT backend not initialized — call init() first")?;

        let mut flat_points = Vec::new();
        for prompt in prompts {
            #[allow(unreachable_patterns)]
            match prompt {
                PromptSet::Points(pts) => flat_points.extend_from_slice(pts),
                _ => anyhow::bail!("prompt type not supported by ORT backend"),
            }
        }

        let rgb_frame = if frame.colorspace == ColorSpace::Grayscale {
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

        let model_size = 1024u32;
        let resized = normalize::resize(&rgb_frame, model_size, model_size)?;

        let h = model_size as usize;
        let w = model_size as usize;
        let mut tensor_data = vec![0.0f32; 3 * h * w];

        for y in 0..h {
            for x in 0..w {
                let src_idx = (y * w + x) * 3;
                tensor_data[y * w + x] = resized.data[src_idx] as f32 / 255.0;
                tensor_data[h * w + y * w + x] = resized.data[src_idx + 1] as f32 / 255.0;
                tensor_data[2 * h * w + y * w + x] = resized.data[src_idx + 2] as f32 / 255.0;
            }
        }

        let input_tensor = Array::from_shape_vec((1, 3, h, w), tensor_data)
            .context("failed to create input tensor")?;

        let scale_x = model_size as f32 / frame.width as f32;
        let scale_y = model_size as f32 / frame.height as f32;

        let point_coords: Vec<f32> = flat_points
            .iter()
            .flat_map(|p: &PromptPoint| [p.x * scale_x, p.y * scale_y])
            .collect();
        let point_labels: Vec<i64> = flat_points.iter().map(|p| p.label).collect();

        let n_points = flat_points.len();
        let coords_tensor = Array::from_shape_vec((1, n_points, 2), point_coords)
            .context("failed to create coords tensor")?;
        let labels_tensor = Array::from_shape_vec((1, n_points), point_labels)
            .context("failed to create labels tensor")?;

        let image_value = Value::from_array(input_tensor)?;
        let coords_value = Value::from_array(coords_tensor)?;
        let labels_value = Value::from_array(labels_tensor)?;

        let outputs = session.run([
            (&image_value).into(),
            (&coords_value).into(),
            (&labels_value).into(),
        ])?;

        let mask_output = outputs
            .get("masks")
            .or_else(|| outputs.get("output"))
            .context("model output missing 'masks' tensor")?;

        let (mask_shape_raw, mask_data) = mask_output
            .try_extract_tensor::<f32>()
            .context("failed to extract mask tensor")?;

        let mask_shape: Vec<usize> = mask_shape_raw.iter().map(|&d| d as usize).collect();

        let mask_h = *mask_shape.last().unwrap_or(&h);
        let mask_w = if mask_shape.len() >= 2 {
            mask_shape[mask_shape.len() - 2]
        } else {
            w
        };

        let binary_mask: Vec<u8> = mask_data
            .iter()
            .take(mask_h * mask_w)
            .map(|&v| {
                let sigmoid = 1.0 / (1.0 + (-v).exp());
                if sigmoid > 0.5 { 255 } else { 0 }
            })
            .collect();

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

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            gpu: true,
            batch: false,
            name: "ort",
            model_formats: vec!["onnx".to_string()],
        }
    }

    fn health(&self) -> BackendHealth {
        let ready = self.session.is_some();
        BackendHealth {
            ready,
            model_name: if ready {
                Some(self.model_name.clone())
            } else {
                None
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Context;

    use super::*;
    use crate::imaging::types::FrameSource;

    #[test]
    #[ignore = "requires a local ONNX model (set MUDD_TEST_MODEL_PATH)"]
    fn ort_backend_parity_smoke() -> Result<()> {
        let model_path = std::env::var("MUDD_TEST_MODEL_PATH")
            .context("set MUDD_TEST_MODEL_PATH to run this parity test")?;

        let mut backend = OrtBackend::new();
        backend.init_from_path(&model_path)?;

        let frame = Frame {
            data: vec![127u8; 64 * 64],
            width: 64,
            height: 64,
            colorspace: ColorSpace::Grayscale,
            source: FrameSource::Image {
                path: "synthetic".to_string(),
            },
        };

        let prompts = vec![PromptSet::Points(vec![PromptPoint {
            x: 32.0,
            y: 32.0,
            label: 1,
        }])];

        let masks = backend.segment(&frame, &prompts)?;
        assert!(!masks.is_empty());
        assert!(!masks[0].data.is_empty());
        Ok(())
    }
}
