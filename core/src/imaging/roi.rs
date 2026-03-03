use anyhow::Result;
use image::{GrayImage, Luma};
use imageproc::contours::{BorderType, find_contours};

use super::types::{ColorSpace, Frame, Roi};

/// Detect the ultrasound ROI from a frame using classical CV:
/// grayscale → binary threshold (Otsu) → morphological close → largest connected component → bbox
///
/// This crops out the vendor UI overlay (menus, text, depth scale) leaving
/// only the actual ultrasound image area.
pub fn detect_roi(frame: &Frame) -> Result<Roi> {
    // Convert to grayscale if needed
    let gray_data = match frame.colorspace {
        ColorSpace::Grayscale => frame.data.clone(),
        _ => {
            let channels = frame.colorspace.channels();
            frame
                .data
                .chunks_exact(channels)
                .map(|px| {
                    // BT.601 luma
                    (0.299 * px[0] as f32 + 0.587 * px[1] as f32 + 0.114 * px[2] as f32) as u8
                })
                .collect()
        }
    };

    let gray = GrayImage::from_raw(frame.width, frame.height, gray_data)
        .ok_or_else(|| anyhow::anyhow!("failed to create grayscale image"))?;

    // Otsu threshold
    let threshold = otsu_threshold(&gray);
    let binary: GrayImage = GrayImage::from_fn(frame.width, frame.height, |x, y| {
        if gray.get_pixel(x, y).0[0] > threshold {
            Luma([255])
        } else {
            Luma([0])
        }
    });

    // Morphological close (dilate then erode) to fill small gaps
    let closed = morphological_close(&binary, 5);

    // Find contours and pick the largest by bounding box area
    let contours = find_contours::<u32>(&closed);

    let mut best_roi = Roi {
        x: 0,
        y: 0,
        width: frame.width,
        height: frame.height,
    };
    let mut best_area = 0u64;

    for contour in &contours {
        if contour.border_type == BorderType::Hole {
            continue;
        }

        let (mut min_x, mut min_y) = (u32::MAX, u32::MAX);
        let (mut max_x, mut max_y) = (0u32, 0u32);

        for point in &contour.points {
            let px = point.x;
            let py = point.y;
            min_x = min_x.min(px);
            min_y = min_y.min(py);
            max_x = max_x.max(px);
            max_y = max_y.max(py);
        }

        let w = max_x.saturating_sub(min_x);
        let h = max_y.saturating_sub(min_y);
        let area = w as u64 * h as u64;

        if area > best_area {
            best_area = area;
            best_roi = Roi {
                x: min_x,
                y: min_y,
                width: w,
                height: h,
            };
        }
    }

    // Sanity check: ROI should be at least 10% of frame area
    let frame_area = frame.width as u64 * frame.height as u64;
    if best_area < frame_area / 10 {
        tracing::warn!(
            "detected ROI too small ({best_area} vs frame {frame_area}), returning full frame"
        );
        return Ok(Roi {
            x: 0,
            y: 0,
            width: frame.width,
            height: frame.height,
        });
    }

    tracing::info!(
        "detected ROI: ({},{}) {}x{} (area: {best_area})",
        best_roi.x,
        best_roi.y,
        best_roi.width,
        best_roi.height
    );

    Ok(best_roi)
}

/// Otsu's threshold method
fn otsu_threshold(img: &GrayImage) -> u8 {
    let mut histogram = [0u32; 256];
    for pixel in img.pixels() {
        histogram[pixel.0[0] as usize] += 1;
    }

    let total = img.width() as f64 * img.height() as f64;
    let mut sum = 0.0f64;
    for (i, &count) in histogram.iter().enumerate() {
        sum += i as f64 * count as f64;
    }

    let mut sum_b = 0.0f64;
    let mut w_b = 0.0f64;
    let mut max_variance = 0.0f64;
    let mut threshold = 0u8;

    for (i, &count) in histogram.iter().enumerate() {
        w_b += count as f64;
        if w_b == 0.0 {
            continue;
        }

        let w_f = total - w_b;
        if w_f == 0.0 {
            break;
        }

        sum_b += i as f64 * count as f64;
        let mean_b = sum_b / w_b;
        let mean_f = (sum - sum_b) / w_f;
        let variance = w_b * w_f * (mean_b - mean_f).powi(2);

        if variance > max_variance {
            max_variance = variance;
            threshold = i as u8;
        }
    }

    threshold
}

/// Simple morphological close: dilate then erode with a square kernel
fn morphological_close(img: &GrayImage, kernel_size: u32) -> GrayImage {
    let dilated = dilate(img, kernel_size);
    erode(&dilated, kernel_size)
}

fn dilate(img: &GrayImage, kernel_size: u32) -> GrayImage {
    let half = (kernel_size / 2) as i32;
    let (w, h) = img.dimensions();
    GrayImage::from_fn(w, h, |x, y| {
        let mut max_val = 0u8;
        for dy in -half..=half {
            for dx in -half..=half {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx >= 0 && nx < w as i32 && ny >= 0 && ny < h as i32 {
                    max_val = max_val.max(img.get_pixel(nx as u32, ny as u32).0[0]);
                }
            }
        }
        Luma([max_val])
    })
}

fn erode(img: &GrayImage, kernel_size: u32) -> GrayImage {
    let half = (kernel_size / 2) as i32;
    let (w, h) = img.dimensions();
    GrayImage::from_fn(w, h, |x, y| {
        let mut min_val = 255u8;
        for dy in -half..=half {
            for dx in -half..=half {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx >= 0 && nx < w as i32 && ny >= 0 && ny < h as i32 {
                    min_val = min_val.min(img.get_pixel(nx as u32, ny as u32).0[0]);
                }
            }
        }
        Luma([min_val])
    })
}
