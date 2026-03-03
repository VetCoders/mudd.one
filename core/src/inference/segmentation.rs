//! Segmentation dispatcher — delegates to active SegmenterBackend.
//! Created by M&K (c)2026 VetCoders

use std::sync::{Mutex, OnceLock};

use anyhow::{Context, Result};

use crate::imaging::types::{Frame, Mask};

pub use super::backend::{
    BackendCapabilities, BackendHealth, PromptPoint, PromptSet, SegmenterBackend,
};

static BACKEND: OnceLock<Mutex<Option<Box<dyn SegmenterBackend>>>> = OnceLock::new();

fn backend_lock() -> &'static Mutex<Option<Box<dyn SegmenterBackend>>> {
    BACKEND.get_or_init(|| Mutex::new(None))
}

#[cfg(feature = "backend-ort")]
pub fn init(model_path: &str) -> Result<()> {
    let mut backend = Box::new(super::ort::OrtBackend::new());
    backend.init_from_path(model_path)?;
    let mut guard = backend_lock().lock().unwrap();
    *guard = Some(backend);
    tracing::info!("segmenter backend ready (ort)");
    Ok(())
}

#[cfg(not(feature = "backend-ort"))]
pub fn init(model_path: &str) -> Result<()> {
    let _ = model_path;
    anyhow::bail!("no inference backend compiled — enable backend-ort or backend-candle feature")
}

#[cfg(feature = "backend-ort")]
pub fn init_from_hf(repo: &str, filename: &str) -> Result<()> {
    let mut backend = Box::new(super::ort::OrtBackend::new());
    backend.init_from_hf(repo, filename)?;
    let mut guard = backend_lock().lock().unwrap();
    *guard = Some(backend);
    tracing::info!("segmenter backend ready (ort, from HF)");
    Ok(())
}

#[cfg(not(feature = "backend-ort"))]
pub fn init_from_hf(repo: &str, filename: &str) -> Result<()> {
    let _ = (repo, filename);
    anyhow::bail!("no inference backend compiled — enable backend-ort or backend-candle feature")
}

pub fn replace_backend(backend: Box<dyn SegmenterBackend>) -> Result<()> {
    let mut guard = backend_lock().lock().unwrap();
    *guard = Some(backend);
    Ok(())
}

pub fn is_initialized() -> bool {
    backend_lock().lock().ok().is_some_and(|g| g.is_some())
}

pub fn model_name() -> Option<String> {
    backend_lock()
        .lock()
        .ok()
        .and_then(|g| g.as_ref().and_then(|b| b.health().model_name))
}

pub fn capabilities() -> Option<BackendCapabilities> {
    backend_lock()
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|b| b.capabilities()))
}

pub fn health() -> Option<BackendHealth> {
    backend_lock()
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|b| b.health()))
}

pub fn segment_frame(frame: &Frame, prompts: &[PromptSet]) -> Result<Vec<Mask>> {
    let mut guard = backend_lock()
        .lock()
        .map_err(|_| anyhow::anyhow!("backend lock poisoned"))?;
    let backend = guard
        .as_mut()
        .context("segmenter not initialized — call init() first")?;
    backend.segment(frame, prompts)
}
