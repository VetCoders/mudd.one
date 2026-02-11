use std::path::Path;

use anyhow::{Context, Result};
use serde::Serialize;

use crate::pipeline::contracts::{ExportConfig, ExportItem, ImageExportFormat};

/// Export items in COCO JSON format
pub fn export_coco(config: &ExportConfig, items: &[ExportItem]) -> Result<()> {
    let output_dir = Path::new(&config.output_dir);
    let images_dir = output_dir.join("images");
    std::fs::create_dir_all(&images_dir).context("failed to create images directory")?;

    let mut coco = CocoDataset {
        images: Vec::with_capacity(items.len()),
        annotations: Vec::new(),
        categories: vec![CocoCategory {
            id: 1,
            name: "ultrasound_roi".to_string(),
        }],
    };

    let mut annotation_id = 1u64;
    let ext = match config.image_format {
        ImageExportFormat::Png => "png",
        ImageExportFormat::Jpeg => "jpg",
        ImageExportFormat::Tiff => "tiff",
    };

    for (i, item) in items.iter().enumerate() {
        let image_id = (i + 1) as u64;
        let filename = format!("frame_{:06}.{ext}", item.metadata.frame_index);
        let frame = &item.processed.frame;

        // Save image
        let image_path = images_dir.join(&filename);
        save_frame_to_file(frame, &image_path, config.image_format)?;

        coco.images.push(CocoImage {
            id: image_id,
            file_name: filename,
            width: frame.width,
            height: frame.height,
        });

        // Convert masks to annotations (skip empty masks)
        if let Some(annotated) = &item.annotation {
            for mask in &annotated.masks {
                let bbox = mask_to_bbox(mask);
                // Skip degenerate annotations (empty mask → zero-area bbox)
                if bbox[2] <= 0.0 || bbox[3] <= 0.0 {
                    continue;
                }
                let segmentation = mask_to_rle(mask);

                coco.annotations.push(CocoAnnotation {
                    id: annotation_id,
                    image_id,
                    category_id: 1,
                    bbox,
                    area: bbox[2] * bbox[3],
                    segmentation,
                    iscrowd: 0,
                });
                annotation_id += 1;
            }
        }
    }

    // Write JSON
    let json_path = output_dir.join("annotations.json");
    let json = serde_json::to_string_pretty(&coco).context("failed to serialize COCO JSON")?;
    std::fs::write(&json_path, json).context("failed to write annotations.json")?;

    tracing::info!(
        "exported COCO dataset: {} images, {} annotations → {}",
        coco.images.len(),
        coco.annotations.len(),
        output_dir.display()
    );

    Ok(())
}

fn save_frame_to_file(
    frame: &crate::imaging::types::Frame,
    path: &Path,
    format: ImageExportFormat,
) -> Result<()> {
    use crate::imaging::types::ColorSpace;
    use image::ColorType;

    let color_type = match frame.colorspace {
        ColorSpace::Grayscale => ColorType::L8,
        ColorSpace::Rgb => ColorType::Rgb8,
        ColorSpace::Rgba => ColorType::Rgba8,
    };

    let img_format = match format {
        ImageExportFormat::Png => image::ImageFormat::Png,
        ImageExportFormat::Jpeg => image::ImageFormat::Jpeg,
        ImageExportFormat::Tiff => image::ImageFormat::Tiff,
    };

    let img = image::ImageBuffer::from_raw(frame.width, frame.height, frame.data.clone())
        .context("failed to create image buffer for export")?;

    match color_type {
        ColorType::L8 => {
            let gray: image::GrayImage = img;
            gray.save_with_format(path, img_format)?;
        }
        ColorType::Rgb8 => {
            let rgb: image::RgbImage =
                image::ImageBuffer::from_raw(frame.width, frame.height, frame.data.clone())
                    .context("failed to create RGB buffer")?;
            rgb.save_with_format(path, img_format)?;
        }
        _ => {
            let rgba: image::RgbaImage =
                image::ImageBuffer::from_raw(frame.width, frame.height, frame.data.clone())
                    .context("failed to create RGBA buffer")?;
            rgba.save_with_format(path, img_format)?;
        }
    }

    Ok(())
}

/// Extract bounding box [x, y, width, height] from a binary mask.
/// Returns [0,0,0,0] for empty masks (no active pixels).
fn mask_to_bbox(mask: &crate::imaging::types::Mask) -> [f64; 4] {
    let (mut min_x, mut min_y) = (mask.width as f64, mask.height as f64);
    let (mut max_x, mut max_y) = (0.0f64, 0.0f64);
    let mut found = false;

    for y in 0..mask.height {
        for x in 0..mask.width {
            if mask.data[(y * mask.width + x) as usize] > 127 {
                min_x = min_x.min(x as f64);
                min_y = min_y.min(y as f64);
                max_x = max_x.max(x as f64);
                max_y = max_y.max(y as f64);
                found = true;
            }
        }
    }

    if !found {
        return [0.0, 0.0, 0.0, 0.0];
    }

    [min_x, min_y, max_x - min_x, max_y - min_y]
}

/// Simple RLE encoding of binary mask (COCO compressed RLE)
fn mask_to_rle(mask: &crate::imaging::types::Mask) -> CocoSegmentation {
    let mut counts = Vec::new();
    let mut current = false;
    let mut count = 0u64;

    // COCO RLE is column-major
    for x in 0..mask.width {
        for y in 0..mask.height {
            let val = mask.data[(y * mask.width + x) as usize] > 127;
            if val == current {
                count += 1;
            } else {
                counts.push(count);
                count = 1;
                current = val;
            }
        }
    }
    counts.push(count);

    CocoSegmentation::Rle {
        counts,
        size: [mask.height as u64, mask.width as u64],
    }
}

#[derive(Serialize)]
struct CocoDataset {
    images: Vec<CocoImage>,
    annotations: Vec<CocoAnnotation>,
    categories: Vec<CocoCategory>,
}

#[derive(Serialize)]
struct CocoImage {
    id: u64,
    file_name: String,
    width: u32,
    height: u32,
}

#[derive(Serialize)]
struct CocoAnnotation {
    id: u64,
    image_id: u64,
    category_id: u64,
    bbox: [f64; 4],
    area: f64,
    segmentation: CocoSegmentation,
    iscrowd: u8,
}

#[derive(Serialize)]
#[serde(untagged)]
enum CocoSegmentation {
    Rle { counts: Vec<u64>, size: [u64; 2] },
}

#[derive(Serialize)]
struct CocoCategory {
    id: u64,
    name: String,
}
