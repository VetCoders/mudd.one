use anyhow::{Context, Result};

use crate::imaging::types::{ColorSpace, Frame, FrameSequence, FrameSource};

/// Load all frames from a video file
pub fn load_video(path: &str) -> Result<FrameSequence> {
    ffmpeg_next::init().context("failed to initialize ffmpeg")?;

    let mut ictx =
        ffmpeg_next::format::input(path).with_context(|| format!("failed to open video: {path}"))?;

    let stream_index = ictx
        .streams()
        .best(ffmpeg_next::media::Type::Video)
        .context("no video stream found")?
        .index();

    let stream = ictx.stream(stream_index).context("stream not found")?;
    let fps = f64::from(stream.avg_frame_rate());
    let total_frames_hint = stream.frames() as usize;

    let codec_par = stream.parameters();
    let decoder_codec = ffmpeg_next::codec::context::Context::from_parameters(codec_par)?;
    let mut decoder = decoder_codec.decoder().video()?;

    let mut scaler = ffmpeg_next::software::scaling::Context::get(
        decoder.format(),
        decoder.width(),
        decoder.height(),
        ffmpeg_next::format::Pixel::RGB24,
        decoder.width(),
        decoder.height(),
        ffmpeg_next::software::scaling::Flags::BILINEAR,
    )?;

    let capacity = if total_frames_hint > 0 {
        total_frames_hint
    } else {
        256
    };
    let mut frames = Vec::with_capacity(capacity);
    let mut frame_index = 0usize;
    let path_str = path.to_string();

    // Decode packets
    for (stream_idx, packet) in ictx.packets() {
        if stream_idx.index() != stream_index {
            continue;
        }

        decoder.send_packet(&packet)?;
        decode_pending(&mut decoder, &mut scaler, &mut frames, &mut frame_index, &path_str)?;
    }

    // Flush decoder (get remaining frames)
    decoder.send_eof()?;
    decode_pending(&mut decoder, &mut scaler, &mut frames, &mut frame_index, &path_str)?;

    tracing::info!(
        "loaded video: {}x{}, {} frames, {:.1} fps",
        decoder.width(),
        decoder.height(),
        frames.len(),
        fps
    );

    Ok(FrameSequence {
        frames,
        fps: Some(fps),
    })
}

/// Drain all pending decoded frames from the decoder
fn decode_pending(
    decoder: &mut ffmpeg_next::decoder::Video,
    scaler: &mut ffmpeg_next::software::scaling::Context,
    frames: &mut Vec<Frame>,
    frame_index: &mut usize,
    path: &str,
) -> Result<()> {
    let mut decoded = ffmpeg_next::frame::Video::empty();

    while decoder.receive_frame(&mut decoded).is_ok() {
        let mut rgb_frame = ffmpeg_next::frame::Video::empty();
        scaler.run(&decoded, &mut rgb_frame)?;

        let width = rgb_frame.width();
        let height = rgb_frame.height();
        let data = rgb_frame.data(0);
        let stride = rgb_frame.stride(0);

        // Copy row by row (stride may differ from width*3)
        let row_bytes = width as usize * 3;
        let mut pixel_data = Vec::with_capacity(row_bytes * height as usize);
        for row in 0..height as usize {
            let row_start = row * stride;
            pixel_data.extend_from_slice(&data[row_start..row_start + row_bytes]);
        }

        frames.push(Frame {
            data: pixel_data,
            width,
            height,
            colorspace: ColorSpace::Rgb,
            source: FrameSource::Video {
                path: path.to_string(),
                frame_index: *frame_index,
            },
        });

        *frame_index += 1;
    }

    Ok(())
}
