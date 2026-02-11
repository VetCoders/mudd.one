use anyhow::{Result, bail};

use super::types::{Frame, Roi};

/// Crop a frame to the given ROI
pub fn crop_frame(frame: &Frame, roi: &Roi) -> Result<Frame> {
    let channels = frame.colorspace.channels();
    let src_stride = frame.width as usize * channels;

    if roi.x + roi.width > frame.width || roi.y + roi.height > frame.height {
        bail!(
            "ROI ({},{} {}x{}) exceeds frame dimensions ({}x{})",
            roi.x,
            roi.y,
            roi.width,
            roi.height,
            frame.width,
            frame.height
        );
    }

    let dst_stride = roi.width as usize * channels;
    let mut data = vec![0u8; dst_stride * roi.height as usize];

    for row in 0..roi.height as usize {
        let src_offset = (roi.y as usize + row) * src_stride + roi.x as usize * channels;
        let dst_offset = row * dst_stride;
        data[dst_offset..dst_offset + dst_stride]
            .copy_from_slice(&frame.data[src_offset..src_offset + dst_stride]);
    }

    Ok(Frame {
        data,
        width: roi.width,
        height: roi.height,
        colorspace: frame.colorspace,
        source: frame.source.clone(),
    })
}
