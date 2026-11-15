use std::fmt::Write as _;
use std::io::Cursor;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};

use anyhow::{Context, Result};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use image::{ImageEncoder as _, ImageFormat};

static NEXT_IMAGE_ID: AtomicU32 = AtomicU32::new(1);

#[derive(Debug, Clone, Copy)]
pub struct KittyImageOptions {
    pub max_width_cells: u16,
    pub max_height_cells: Option<u16>,
}

fn transmit_png(out: &mut String, png: &[u8], first: &str, continuation: &str) {
    let chunk_size = 3 * 1024;
    let n_chunks = png.len().div_ceil(chunk_size);
    for (i, start) in (0..png.len()).step_by(chunk_size).enumerate() {
        let chunk = &png[start..(start + chunk_size).min(png.len())];
        let control = if i == 0 { first } else { continuation };
        let separator = if control.is_empty() { "" } else { "," };
        let more = if i + 1 < n_chunks { 1 } else { 0 };
        write!(out, "\u{1b}_G{control}{separator}m={more};").unwrap();
        STANDARD.encode_string(chunk, out);
        out.push_str("\u{1b}\\");
    }
}

fn encode_rgba(image: &image::RgbaImage) -> Result<Vec<u8>> {
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png).write_image(
        image.as_raw(),
        image.width(),
        image.height(),
        image::ExtendedColorType::Rgba8,
    )?;
    Ok(png)
}

fn png_bytes(image: image::DynamicImage) -> Result<Vec<u8>> {
    encode_rgba(&image.to_rgba8())
}

fn frame_delay_ms(frame: &image::Frame) -> u32 {
    let (numerator, denominator) = frame.delay().numer_denom_ms();
    numerator.div_ceil(denominator).max(10)
}

fn changed_rect(
    previous: &image::RgbaImage,
    current: &image::RgbaImage,
) -> Option<(u32, u32, u32, u32)> {
    let (width, height) = current.dimensions();
    let (mut left, mut top, mut right, mut bottom) = (width, height, 0, 0);
    let mut changed = false;
    for (index, (before, after)) in previous.pixels().zip(current.pixels()).enumerate() {
        if before == after {
            continue;
        }
        let x = index as u32 % width;
        let y = index as u32 / width;
        left = left.min(x);
        top = top.min(y);
        right = right.max(x + 1);
        bottom = bottom.max(y + 1);
        changed = true;
    }
    changed.then_some((left, top, right - left, bottom - top))
}

// Kitty graphics protocol: https://sw.kovidgoyal.net/kitty/graphics-protocol/
// APC format: ESC _ G <key=val,...> ; <base64-payload> ESC \
// a=T transmit+display, t=d direct data, f=100 PNG format.
// Large images split into chunks; m=1 means more chunks follow, m=0 is last.
// c/r set max display width/height in cells (r=0 means unconstrained height).
pub fn render_image(path: &Path, opts: KittyImageOptions) -> Result<String> {
    let reader = image::ImageReader::open(path)
        .and_then(|reader| reader.with_guessed_format())
        .with_context(|| format!("failed to open image: {}", path.display()))?;
    if reader.format() == Some(ImageFormat::Gif) {
        let bytes = std::fs::read(path)?;
        let (frames, loops) = decode_gif(&bytes)?;
        return render_gif_frames(&frames, loops, opts);
    }
    let dyn_image = reader.decode()?;
    let png = png_bytes(dyn_image.clone())
        .with_context(|| format!("failed to encode image: {}", path.display()))?;
    render_image_png(&dyn_image, &png, opts)
}

pub(crate) fn decode_gif(bytes: &[u8]) -> Result<(Vec<image::Frame>, u32)> {
    use image::AnimationDecoder as _;

    let decoder = image::codecs::gif::GifDecoder::new(Cursor::new(bytes))?;
    let loops = match decoder.loop_count() {
        image::metadata::LoopCount::Infinite => 1,
        image::metadata::LoopCount::Finite(count) => count.get().saturating_add(1),
    };
    let frames = decoder.into_frames().collect_frames()?;
    Ok((frames, loops))
}

fn render_png(
    width: u32,
    height: u32,
    png: &[u8],
    opts: KittyImageOptions,
    id: Option<u32>,
) -> String {
    let width_cells = opts.max_width_cells.max(1);
    let max_height = opts.max_height_cells.unwrap_or(0);
    let id = id.map(|id| format!(",i={id}")).unwrap_or_default();
    let first = format!("a=T,t=d,f=100{id},s={width},v={height},c={width_cells},r={max_height}");
    let mut out = String::new();
    transmit_png(&mut out, png, &first, "");
    out
}

pub fn render_image_png(
    dyn_image: &image::DynamicImage,
    png: &[u8],
    opts: KittyImageOptions,
) -> Result<String> {
    Ok(render_png(
        dyn_image.width(),
        dyn_image.height(),
        png,
        opts,
        None,
    ))
}

pub fn render_gif_frames(
    frames: &[image::Frame],
    loops: u32,
    opts: KittyImageOptions,
) -> Result<String> {
    let first_frame = frames
        .first()
        .ok_or_else(|| anyhow::anyhow!("GIF has no frames"))?;
    let (width, height) = first_frame.buffer().dimensions();
    if frames.len() == 1 {
        return Ok(render_png(
            width,
            height,
            &encode_rgba(first_frame.buffer())?,
            opts,
            None,
        ));
    }

    let id = NEXT_IMAGE_ID.fetch_add(1, Ordering::Relaxed);
    let mut out = render_png(
        width,
        height,
        &encode_rgba(first_frame.buffer())?,
        opts,
        Some(id),
    );

    for (index, frame) in frames.iter().enumerate().skip(1) {
        let current = frame.buffer();
        // Overwriting only the changed rectangle avoids uploading identical pixels.
        let (x, y, w, h) =
            changed_rect(frames[index - 1].buffer(), current).unwrap_or((0, 0, 1, 1));
        let png = if (x, y, w, h) == (0, 0, width, height) {
            encode_rgba(current)?
        } else {
            let region =
                image::RgbaImage::from_fn(w, h, |dx, dy| *current.get_pixel(x + dx, y + dy));
            encode_rgba(&region)?
        };
        let first = format!(
            "a=f,t=d,f=100,i={id},x={x},y={y},s={w},v={h},c={index},X=1,z={}",
            frame_delay_ms(frame)
        );
        let continuation = format!("a=f,i={id}");
        transmit_png(&mut out, &png, &first, &continuation);
    }
    write!(
        out,
        "\u{1b}_Ga=a,i={id},r=1,z={}\u{1b}\\",
        frame_delay_ms(first_frame)
    )
    .unwrap();
    write!(out, "\u{1b}_Ga=a,i={id},s=3,v={loops}\u{1b}\\").unwrap();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn animation_transmits_only_the_changed_rectangle() {
        let previous = image::RgbaImage::new(4, 4);
        let mut current = previous.clone();
        current.put_pixel(2, 1, image::Rgba([255, 0, 0, 255]));
        let frames = vec![image::Frame::new(previous), image::Frame::new(current)];
        let output = render_gif_frames(
            &frames,
            1,
            KittyImageOptions {
                max_width_cells: 4,
                max_height_cells: Some(4),
            },
        )
        .unwrap();
        assert!(output.contains("x=2,y=1,s=1,v=1,c=1,X=1"));
        let frame = output
            .split("\u{1b}_G")
            .find(|command| command.starts_with("a=f,"))
            .unwrap();
        let payload = frame
            .split(';')
            .nth(1)
            .unwrap()
            .trim_end_matches("\u{1b}\\");
        let png = STANDARD.decode(payload).unwrap();
        let decoded = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(decoded.dimensions(), (1, 1));
        assert_eq!(decoded.get_pixel(0, 0), &image::Rgba([255, 0, 0, 255]));
    }
}
