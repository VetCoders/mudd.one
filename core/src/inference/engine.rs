use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use anyhow::{Context, Result};

static ENGINE: OnceLock<Mutex<OrtEngine>> = OnceLock::new();

pub struct OrtEngine {
    session: ort::session::Session,
    model_name: String,
}

/// Initialize the ONNX inference engine with a model file
pub fn init(model_path: &str) -> Result<()> {
    if ENGINE.get().is_some() {
        tracing::warn!("engine already initialized, skipping");
        return Ok(());
    }

    tracing::info!("initializing ORT engine from: {model_path}");

    let session = ort::session::Session::builder()?
        .with_execution_providers([ort::ep::CoreML::default().build()])?
        .with_intra_threads(1)?
        .commit_from_file(model_path)
        .with_context(|| format!("failed to load ONNX model: {model_path}"))?;

    // Log model I/O info
    for (i, input) in session.inputs().iter().enumerate() {
        tracing::info!("  input[{i}]: {} {:?}", input.name(), input.dtype());
    }
    for (i, output) in session.outputs().iter().enumerate() {
        tracing::info!("  output[{i}]: {} {:?}", output.name(), output.dtype());
    }

    let model_name = Path::new(model_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let engine = OrtEngine {
        session,
        model_name,
    };

    ENGINE
        .set(Mutex::new(engine))
        .map_err(|_| anyhow::anyhow!("engine already initialized (race)"))?;

    tracing::info!("ORT engine ready");
    Ok(())
}

/// Initialize from HuggingFace cache or env var
pub fn init_from_hf(repo: &str, filename: &str) -> Result<()> {
    let path = resolve_model_path(repo, filename)?;
    let path_str = path.to_str().context("invalid model path")?;
    init(path_str)
}

pub fn is_initialized() -> bool {
    ENGINE.get().is_some()
}

pub fn model_name() -> Option<String> {
    ENGINE
        .get()
        .and_then(|m| m.lock().ok())
        .map(|e| e.model_name.clone())
}

/// Access the engine (panics if not initialized)
pub(crate) fn engine() -> &'static Mutex<OrtEngine> {
    ENGINE
        .get()
        .expect("engine not initialized — call init() first")
}

impl OrtEngine {
    pub fn session(&self) -> &ort::session::Session {
        &self.session
    }

    pub fn session_mut(&mut self) -> &mut ort::session::Session {
        &mut self.session
    }
}

/// Resolve model path: env var → HuggingFace cache → error
///
/// HF cache layout: ~/.cache/huggingface/hub/models--{repo}/snapshots/{hash}/{filename}
fn resolve_model_path(repo: &str, filename: &str) -> Result<PathBuf> {
    // 1. Explicit env var
    if let Ok(path) = std::env::var("MUDD_MODEL_PATH") {
        let p = PathBuf::from(&path);
        if p.exists() {
            return Ok(p);
        }
        tracing::warn!("MUDD_MODEL_PATH={path} does not exist, falling back to HF cache");
    }

    // 2. HuggingFace cache
    let hf_bases = hf_cache_bases();
    let repo_dir = format!("models--{}", repo.replace('/', "--"));

    for base in &hf_bases {
        let snapshots_dir = base.join(&repo_dir).join("snapshots");
        if !snapshots_dir.exists() {
            continue;
        }

        // Pick most recently modified snapshot
        let mut best: Option<(PathBuf, std::time::SystemTime)> = None;
        if let Ok(entries) = std::fs::read_dir(&snapshots_dir) {
            for entry in entries.flatten() {
                let candidate = entry.path().join(filename);
                if candidate.exists() {
                    let mtime = entry
                        .metadata()
                        .and_then(|m| m.modified())
                        .unwrap_or(std::time::UNIX_EPOCH);
                    if best.as_ref().map_or(true, |(_, t)| mtime > *t) {
                        best = Some((candidate, mtime));
                    }
                }
            }
        }

        if let Some((path, _)) = best {
            tracing::info!("resolved model from HF cache: {}", path.display());
            return Ok(path);
        }
    }

    anyhow::bail!(
        "model not found: {repo}/{filename}. \
         Download with: huggingface-cli download {repo} {filename}"
    )
}

/// HuggingFace cache base directories (priority order)
fn hf_cache_bases() -> Vec<PathBuf> {
    let mut bases = Vec::new();

    for var in ["MUDD_HF_CACHE", "HUGGINGFACE_HUB_CACHE", "HF_HUB_CACHE"] {
        if let Ok(val) = std::env::var(var) {
            bases.push(PathBuf::from(val));
        }
    }

    if let Ok(hf_home) = std::env::var("HF_HOME") {
        bases.push(PathBuf::from(hf_home).join("hub"));
    }

    if let Some(home) = directories::BaseDirs::new() {
        bases.push(home.home_dir().join(".cache/huggingface/hub"));
    }

    bases
}
