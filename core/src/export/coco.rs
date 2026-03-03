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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imaging::types::Mask;

    fn mask_with_rect(w: u32, h: u32, x0: u32, y0: u32, x1: u32, y1: u32) -> Mask {
        let mut data = vec![0u8; (w * h) as usize];
        for y in y0..=y1 {
            for x in x0..=x1 {
                data[(y * w + x) as usize] = 255;
            }
        }
        Mask {
            data,
            width: w,
            height: h,
            label: "test".to_string(),
        }
    }

    #[test]
    fn bbox_known_rect() {
        let mask = mask_with_rect(10, 10, 2, 3, 5, 7);
        let bbox = mask_to_bbox(&mask);
        assert_eq!(bbox[0], 2.0); // x
        assert_eq!(bbox[1], 3.0); // y
        assert_eq!(bbox[2], 3.0); // width (5-2)
        assert_eq!(bbox[3], 4.0); // height (7-3)
    }

    #[test]
    fn bbox_empty_mask() {
        let mask = Mask {
            data: vec![0u8; 100],
            width: 10,
            height: 10,
            label: "empty".to_string(),
        };
        let bbox = mask_to_bbox(&mask);
        assert_eq!(bbox, [0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn bbox_single_pixel() {
        let mask = mask_with_rect(10, 10, 5, 5, 5, 5);
        let bbox = mask_to_bbox(&mask);
        // single pixel: max == min, so w=0, h=0
        assert_eq!(bbox[2], 0.0);
        assert_eq!(bbox[3], 0.0);
    }

    #[test]
    fn bbox_full_frame() {
        let mask = mask_with_rect(8, 6, 0, 0, 7, 5);
        let bbox = mask_to_bbox(&mask);
        assert_eq!(bbox[0], 0.0);
        assert_eq!(bbox[1], 0.0);
        assert_eq!(bbox[2], 7.0); // 7-0
        assert_eq!(bbox[3], 5.0); // 5-0
    }

    #[test]
    fn rle_simple_mask() {
        // 3x2 mask: row0=[0,255,0], row1=[0,255,0]
        let mask = Mask {
            data: vec![0, 255, 0, 0, 255, 0],
            width: 3,
            height: 2,
            label: "test".to_string(),
        };
        let seg = mask_to_rle(&mask);
        // RLE is column-major, starts counting from background (false)
        // col0: [0,0] → 2 bg
        // col1: [255,255] → transition → 2 fg
        // col2: [0,0] → transition → 2 bg
        // Result: [2, 2, 2] = 2 bg, 2 fg, 2 bg
        match seg {
            CocoSegmentation::Rle { counts, size } => {
                assert_eq!(size, [2, 3]); // [height, width]
                assert_eq!(counts, vec![2, 2, 2]);
            }
        }
    }

    #[test]
    fn export_coco_skips_degenerate_masks() {
        use crate::imaging::types::{ColorSpace, FrameMetadata, FrameSource};
        use crate::pipeline::contracts::{
            AnnotatedFrame, ExportConfig, ExportFormat, ExportItem, ImageExportFormat,
            ProcessedFrame,
        };

        let dir = tempfile::tempdir().unwrap();
        let frame = crate::imaging::types::Frame {
            data: vec![128u8; 100],
            width: 10,
            height: 10,
            colorspace: ColorSpace::Grayscale,
            source: FrameSource::Image {
                path: String::new(),
            },
        };

        let single_pixel_mask = mask_with_rect(10, 10, 5, 5, 5, 5);
        let real_mask = mask_with_rect(10, 10, 1, 1, 4, 4);

        let config = ExportConfig {
            format: ExportFormat::Coco,
            output_dir: dir.path().to_str().unwrap().to_string(),
            image_format: ImageExportFormat::Png,
            include_metadata: false,
        };

        let items = vec![ExportItem {
            processed: ProcessedFrame {
                frame: frame.clone(),
                filters_applied: vec![],
            },
            annotation: Some(AnnotatedFrame {
                frame: frame.clone(),
                masks: vec![single_pixel_mask, real_mask],
            }),
            metadata: FrameMetadata::default(),
        }];

        export_coco(&config, &items).unwrap();

        let json_path = dir.path().join("annotations.json");
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&json_path).unwrap()).unwrap();

        let annotations = json["annotations"].as_array().unwrap();
        // Only the real mask should survive, single-pixel skipped
        assert_eq!(annotations.len(), 1);
        assert!(annotations[0]["area"].as_f64().unwrap() > 0.0);
    }
}
