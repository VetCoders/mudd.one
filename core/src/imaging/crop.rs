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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imaging::types::{ColorSpace, FrameSource};

    fn test_frame() -> Frame {
        // 4x4 grayscale, sequential values 0..16
        Frame {
            data: (0u8..16).collect(),
            width: 4,
            height: 4,
            colorspace: ColorSpace::Grayscale,
            source: FrameSource::Image {
                path: String::new(),
            },
        }
    }

    #[test]
    fn crop_center_2x2() {
        let frame = test_frame();
        let roi = Roi {
            x: 1,
            y: 1,
            width: 2,
            height: 2,
        };
        let cropped = crop_frame(&frame, &roi).unwrap();
        assert_eq!(cropped.width, 2);
        assert_eq!(cropped.height, 2);
        // row0: pixels (1,1)=5, (2,1)=6
        // row1: pixels (1,2)=9, (2,2)=10
        assert_eq!(cropped.data, vec![5, 6, 9, 10]);
    }

    #[test]
    fn crop_full_frame() {
        let frame = test_frame();
        let roi = Roi {
            x: 0,
            y: 0,
            width: 4,
            height: 4,
        };
        let cropped = crop_frame(&frame, &roi).unwrap();
        assert_eq!(cropped.data, frame.data);
    }

    #[test]
    fn crop_exceeds_bounds() {
        let frame = test_frame();
        let roi = Roi {
            x: 3,
            y: 3,
            width: 2,
            height: 2,
        };
        assert!(crop_frame(&frame, &roi).is_err());
    }

    #[test]
    fn crop_rgb_frame() {
        // 3x2 RGB
        let frame = Frame {
            data: vec![
                10, 20, 30, 40, 50, 60, 70, 80, 90, // row 0: 3 pixels
                100, 110, 120, 130, 140, 150, 160, 170, 180, // row 1: 3 pixels
            ],
            width: 3,
            height: 2,
            colorspace: ColorSpace::Rgb,
            source: FrameSource::Image {
                path: String::new(),
            },
        };
        let roi = Roi {
            x: 1,
            y: 0,
            width: 2,
            height: 2,
        };
        let cropped = crop_frame(&frame, &roi).unwrap();
        assert_eq!(cropped.width, 2);
        assert_eq!(cropped.height, 2);
        assert_eq!(
            cropped.data,
            vec![40, 50, 60, 70, 80, 90, 130, 140, 150, 160, 170, 180]
        );
    }
}
