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
        save_frame(
            &frame.data,
            frame.width,
            frame.height,
            frame.colorspace,
            &image_path,
            config.image_format,
        )?;

        // Generate label file
        let label_path = labels_dir.join(format!("{stem}.txt"));
        let mut label_content = String::new();

        if let Some(annotated) = &item.annotation {
            for mask in &annotated.masks {
                if let Some(bbox) = mask_to_normalized_bbox(mask, frame.width, frame.height) {
                    // class_id x_center y_center width height
                    label_content.push_str(&format!(
                        "0 {:.6} {:.6} {:.6} {:.6}\n",
                        bbox.0, bbox.1, bbox.2, bbox.3
                    ));
                }
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

/// Convert binary mask to normalized YOLO bbox (x_center, y_center, width, height).
/// Returns None for empty masks (no active pixels).
fn mask_to_normalized_bbox(
    mask: &crate::imaging::types::Mask,
    frame_w: u32,
    frame_h: u32,
) -> Option<(f64, f64, f64, f64)> {
    let (mut min_x, mut min_y) = (mask.width, mask.height);
    let (mut max_x, mut max_y) = (0u32, 0u32);
    let mut found = false;

    for y in 0..mask.height {
        for x in 0..mask.width {
            if mask.data[(y * mask.width + x) as usize] > 127 {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
                found = true;
            }
        }
    }

    if !found || frame_w == 0 || frame_h == 0 {
        return None;
    }

    let w = (max_x - min_x + 1) as f64;
    let h = (max_y - min_y + 1) as f64;

    let cx = min_x as f64 + w / 2.0;
    let cy = min_y as f64 + h / 2.0;

    Some((
        cx / frame_w as f64,
        cy / frame_h as f64,
        w / frame_w as f64,
        h / frame_h as f64,
    ))
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
    use image::ColorType;

    let color_type = match colorspace {
        ColorSpace::Grayscale => ColorType::L8,
        ColorSpace::Rgb => ColorType::Rgb8,
        ColorSpace::Rgba => ColorType::Rgba8,
    };

    let img_format = match format {
        ImageExportFormat::Png => image::ImageFormat::Png,
        ImageExportFormat::Jpeg => image::ImageFormat::Jpeg,
        ImageExportFormat::Tiff => image::ImageFormat::Tiff,
    };

    image::save_buffer_with_format(path, data, width, height, color_type, img_format)
        .with_context(|| format!("failed to save frame to {}", path.display()))?;

    Ok(())
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
    fn normalized_bbox_known_rect() {
        // 10x10 mask, rect at (2,2)-(6,8) on 100x100 frame
        let mask = mask_with_rect(10, 10, 2, 2, 6, 8);
        let bbox = mask_to_normalized_bbox(&mask, 100, 100).unwrap();
        // w=6-2+1=5, h=8-2+1=7, cx=2+2.5=4.5, cy=2+3.5=5.5
        assert!((bbox.0 - 0.045).abs() < 1e-9); // cx/100
        assert!((bbox.1 - 0.055).abs() < 1e-9); // cy/100
        assert!((bbox.2 - 0.05).abs() < 1e-9); // w/100
        assert!((bbox.3 - 0.07).abs() < 1e-9); // h/100
    }

    #[test]
    fn normalized_bbox_empty_mask() {
        let mask = Mask {
            data: vec![0u8; 100],
            width: 10,
            height: 10,
            label: "empty".to_string(),
        };
        assert!(mask_to_normalized_bbox(&mask, 100, 100).is_none());
    }

    #[test]
    fn normalized_bbox_single_pixel_valid() {
        let mask = mask_with_rect(10, 10, 5, 5, 5, 5);
        // single pixel: w=1, h=1 — valid 1x1 bbox
        let bbox = mask_to_normalized_bbox(&mask, 100, 100).unwrap();
        assert!((bbox.2 - 0.01).abs() < 1e-9); // 1/100
        assert!((bbox.3 - 0.01).abs() < 1e-9); // 1/100
    }

    #[test]
    fn normalized_bbox_zero_frame_dims() {
        let mask = mask_with_rect(10, 10, 1, 1, 5, 5);
        assert!(mask_to_normalized_bbox(&mask, 0, 100).is_none());
        assert!(mask_to_normalized_bbox(&mask, 100, 0).is_none());
    }

    #[test]
    fn export_yolo_end_to_end() {
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

        let real_mask = mask_with_rect(10, 10, 1, 1, 4, 4);
        let degenerate_mask = mask_with_rect(10, 10, 5, 5, 5, 5);

        let config = ExportConfig {
            format: ExportFormat::Yolo,
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
                masks: vec![real_mask, degenerate_mask],
            }),
            metadata: FrameMetadata::default(),
        }];

        export_yolo(&config, &items).unwrap();

        // Both masks valid: single-pixel is 1x1, real mask is 4x4
        let label = std::fs::read_to_string(dir.path().join("labels/frame_000000.txt")).unwrap();
        let lines: Vec<&str> = label.lines().collect();
        assert_eq!(lines.len(), 2, "both masks should produce labels");
        assert!(lines[0].starts_with("0 "), "should start with class_id 0");
        assert!(lines[1].starts_with("0 "), "should start with class_id 0");

        // Check classes.txt
        let classes = std::fs::read_to_string(dir.path().join("classes.txt")).unwrap();
        assert_eq!(classes.trim(), "ultrasound_roi");
    }
}
