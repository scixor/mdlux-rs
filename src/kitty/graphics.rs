use std::io::Cursor;
use std::path::Path;

use anyhow::{Context, Result};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use image::ImageFormat;

#[derive(Debug, Clone, Copy)]
pub struct KittyImageOptions {
    pub max_width_cells: u16,
    pub max_height_cells: Option<u16>,
}

fn chunk_bytes(input: &[u8], max_chunk_size: usize) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let mut start = 0;
    while start < input.len() {
        let end = (start + max_chunk_size).min(input.len());
        out.push(&input[start..end]);
        start = end;
    }
    out
}

pub fn render_image(path: &Path, opts: KittyImageOptions) -> Result<String> {
    let dyn_image =
        image::open(path).with_context(|| format!("failed to open image: {}", path.display()))?;
    let mut png = Vec::new();
    {
        let mut cursor = Cursor::new(&mut png);
        dyn_image
            .write_to(&mut cursor, ImageFormat::Png)
            .context("failed to encode image as png")?;
    }

    let (w, h) = (dyn_image.width(), dyn_image.height());
    let width_cells = opts.max_width_cells.max(1);
    let max_height = opts.max_height_cells.unwrap_or(0);

    let chunks = chunk_bytes(&png, 3 * 1024);
    let n_chunks = chunks.len();
    let mut out = String::new();
    for (i, chunk) in chunks.into_iter().enumerate() {
        let more = if i + 1 < n_chunks { 1 } else { 0 };
        let b64 = STANDARD.encode(chunk);
        if i == 0 {
            out.push_str(&format!(
                "\u{1b}_Ga=T,t=d,f=100,m={},s={},v={},c={},r={};{}\u{1b}\\",
                more, w, h, width_cells, max_height, b64
            ));
        } else {
            out.push_str(&format!("\u{1b}_Gm={};{}\u{1b}\\", more, b64));
        }
    }

    Ok(out)
}
