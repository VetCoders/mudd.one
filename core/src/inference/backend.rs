//! Segmenter backend trait — runtime-agnostic inference abstraction.
//! Created by M&K (c)2026 VetCoders

use anyhow::Result;

use crate::imaging::types::{Frame, Mask};

/// Prompt input — extensible beyond just points
#[derive(Debug, Clone)]
pub enum PromptSet {
    /// SAM-style point prompts (foreground/background)
    Points(Vec<PromptPoint>),
    // Future:
    // Box { x1: f32, y1: f32, x2: f32, y2: f32 },
    // MaskHint { data: Vec<u8>, width: u32, height: u32 },
}

/// Single point prompt (x, y in pixel coords, label: 1=foreground, 0=background)
#[derive(Debug, Clone, Copy)]
pub struct PromptPoint {
    pub x: f32,
    pub y: f32,
    pub label: i64,
}

/// What a backend can do
#[derive(Debug, Clone)]
pub struct BackendCapabilities {
    pub gpu: bool,
    pub batch: bool,
    pub name: &'static str,
    pub model_formats: Vec<String>,
}

/// Backend health/readiness
#[derive(Debug, Clone)]
pub struct BackendHealth {
    pub ready: bool,
    pub model_name: Option<String>,
}

/// Abstraction over inference runtime.
///
/// Each implementor owns the full path from raw Frame to Mask output,
/// including any model-specific preprocessing (resize, normalize, tensor layout).
pub trait SegmenterBackend: Send {
    /// Initialize from a local model file path
    fn init_from_path(&mut self, model_path: &str) -> Result<()>;

    /// Initialize from a HuggingFace repo + filename
    fn init_from_hf(&mut self, repo: &str, filename: &str) -> Result<()>;

    /// Run segmentation: frame + prompts -> masks
    fn segment(&mut self, frame: &Frame, prompts: &[PromptSet]) -> Result<Vec<Mask>>;

    /// Query backend capabilities
    fn capabilities(&self) -> BackendCapabilities;

    /// Query backend health/readiness
    fn health(&self) -> BackendHealth;
}
