use anyhow::Result;

use crate::pipeline::contracts::{ExportConfig, ExportItem};

/// Export items in YOLO format (per-image .txt with class + normalized bbox/polygon)
pub fn export_yolo(_config: &ExportConfig, _items: &[ExportItem]) -> Result<()> {
    // TODO: generate YOLO txt per image + classes.txt
    todo!("YOLO export")
}
