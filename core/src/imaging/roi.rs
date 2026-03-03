use anyhow::Result;
use image::{GrayImage, Luma};
use imageproc::contours::{BorderType, find_contours};
use imageproc::contrast::otsu_level;
use imageproc::distance_transform::Norm;
use imageproc::morphology::close;

use super::types::{Frame, Roi};

/// Detect the ultrasound ROI from a frame using classical CV:
/// grayscale → binary threshold (Otsu) → morphological close → largest connected component → bbox
///
/// This crops out the vendor UI overlay (menus, text, depth scale) leaving
/// only the actual ultrasound image area.
pub fn detect_roi(frame: &Frame) -> Result<Roi> {
    // Convert to grayscale using centralized BT.601 conversion
    let gray_frame = super::normalize::to_grayscale(frame)?;

    let gray = GrayImage::from_raw(gray_frame.width, gray_frame.height, gray_frame.data)
        .ok_or_else(|| anyhow::anyhow!("failed to create grayscale image"))?;

    // Otsu threshold via imageproc
    let threshold = otsu_level(&gray);
    let binary: GrayImage = GrayImage::from_fn(frame.width, frame.height, |x, y| {
        if gray.get_pixel(x, y).0[0] > threshold {
            Luma([255])
        } else {
            Luma([0])
        }
    });

    // Morphological close via imageproc (dilate then erode, 5x5 square kernel)
    let closed = close(&binary, Norm::LInf, 2);

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
