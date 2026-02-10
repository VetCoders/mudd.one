use std::sync::{Mutex, OnceLock};

use anyhow::{Result, Context};

static ENGINE: OnceLock<Mutex<OrtEngine>> = OnceLock::new();

pub struct OrtEngine {
    session: ort::session::Session,
}

/// Initialize the ONNX inference engine with a model file
pub fn init(model_path: &str) -> Result<()> {
    if ENGINE.get().is_some() {
        return Ok(());
    }

    let session = ort::session::Session::builder()?
        .with_execution_providers([ort::ep::CoreML::default().build()])?
        .with_intra_threads(1)?
        .commit_from_file(model_path)
        .context("failed to load ONNX model")?;

    let engine = OrtEngine { session };

    ENGINE
        .set(Mutex::new(engine))
        .map_err(|_| anyhow::anyhow!("engine already initialized"))?;

    Ok(())
}

pub fn is_initialized() -> bool {
    ENGINE.get().is_some()
}

/// Access the engine (panics if not initialized)
pub(crate) fn engine() -> &'static Mutex<OrtEngine> {
    ENGINE.get().expect("engine not initialized — call init() first")
}

impl OrtEngine {
    pub fn session(&self) -> &ort::session::Session {
        &self.session
    }
}
