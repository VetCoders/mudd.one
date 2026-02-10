use anyhow::Result;

use crate::pipeline::contracts::{ExportConfig, ExportItem};

/// Export items in COCO JSON format
pub fn export_coco(_config: &ExportConfig, _items: &[ExportItem]) -> Result<()> {
    // TODO: generate COCO JSON with images + annotations arrays
    todo!("COCO export")
}
