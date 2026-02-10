use std::path::Path;

use anyhow::{Context, Result};

use crate::pipeline::contracts::{ExportConfig, ExportItem, ImageExportFormat};

/// Export items in YOLO format:
/// - Per-image .txt with: class_id x_center y_center width height (normalized 0-1)
/// - classes.txt with class names
/// - images/ directory with exported frames
pub fn export_yolo(config: &ExportConfig, items: &[ExportItem]) -> Result<()> {
    let output_dir = Path::new(&config.output_dir);
    let images_dir = output_dir.join("images");
    let labels_dir = output_dir.join("labels");
    std::fs::create_dir_all(&images_dir).context("failed to create images directory")?;
    std::fs::create_dir_all(&labels_dir).context("failed to create labels directory")?;

    let ext = match config.image_format {
        ImageExportFormat::Png => "png",
        ImageExportFormat::Jpeg => "jpg",
        ImageExportFormat::Tiff => "tiff",
    };

    for item in items {
        let stem = format!("frame_{:06}", item.metadata.frame_index);
        let frame = &item.processed.frame;

        // Save image
        let image_path = images_dir.join(format!("{stem}.{ext}"));
        save_frame(&frame.data, frame.width, frame.height, frame.colorspace, &image_path, config.image_format)?;

        // Generate label file
        let label_path = labels_dir.join(format!("{stem}.txt"));
        let mut label_content = String::new();

        if let Some(annotated) = &item.annotation {
            for mask in &annotated.masks {
                let bbox = mask_to_normalized_bbox(mask, frame.width, frame.height);
                // class_id x_center y_center width height
                label_content.push_str(&format!(
                    "0 {:.6} {:.6} {:.6} {:.6}\n",
                    bbox.0, bbox.1, bbox.2, bbox.3
                ));
            }
        }

        std::fs::write(&label_path, &label_content)
            .with_context(|| format!("failed to write label: {}", label_path.display()))?;
    }

    // Write classes.txt
    let classes_path = output_dir.join("classes.txt");
    std::fs::write(&classes_path, "ultrasound_roi\n").context("failed to write classes.txt")?;

    tracing::info!(
        "exported YOLO dataset: {} items → {}",
        items.len(),
        output_dir.display()
    );

    Ok(())
}

/// Convert binary mask to normalized YOLO bbox (x_center, y_center, width, height)
fn mask_to_normalized_bbox(
    mask: &crate::imaging::types::Mask,
    frame_w: u32,
    frame_h: u32,
) -> (f64, f64, f64, f64) {
    let (mut min_x, mut min_y) = (mask.width, mask.height);
    let (mut max_x, mut max_y) = (0u32, 0u32);

    for y in 0..mask.height {
        for x in 0..mask.width {
            if mask.data[(y * mask.width + x) as usize] > 127 {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }

    let w = (max_x - min_x) as f64;
    let h = (max_y - min_y) as f64;
    let cx = min_x as f64 + w / 2.0;
    let cy = min_y as f64 + h / 2.0;

    (
        cx / frame_w as f64,
        cy / frame_h as f64,
        w / frame_w as f64,
        h / frame_h as f64,
    )
}

fn save_frame(
    data: &[u8],
    width: u32,
    height: u32,
    colorspace: crate::imaging::types::ColorSpace,
    path: &Path,
    format: ImageExportFormat,
) -> Result<()> {
    use crate::imaging::types::ColorSpace;

    let img_format = match format {
        ImageExportFormat::Png => image::ImageFormat::Png,
        ImageExportFormat::Jpeg => image::ImageFormat::Jpeg,
        ImageExportFormat::Tiff => image::ImageFormat::Tiff,
    };

    match colorspace {
        ColorSpace::Grayscale => {
            let img: image::GrayImage = image::ImageBuffer::from_raw(width, height, data.to_vec())
                .context("failed to create grayscale buffer")?;
            img.save_with_format(path, img_format)?;
        }
        ColorSpace::Rgb => {
            let img: image::RgbImage = image::ImageBuffer::from_raw(width, height, data.to_vec())
                .context("failed to create RGB buffer")?;
            img.save_with_format(path, img_format)?;
        }
        ColorSpace::Rgba => {
            let img: image::RgbaImage = image::ImageBuffer::from_raw(width, height, data.to_vec())
                .context("failed to create RGBA buffer")?;
            img.save_with_format(path, img_format)?;
        }
    }

    Ok(())
}
