use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::kitty::graphics::{KittyImageOptions, render_image_png};
use crate::markdown::ast::{Block, Inline};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    Image,
    Video,
    Audio,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaRequest {
    pub source: String,
    pub alt: String,
    pub title: Option<String>,
    pub kind: MediaKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MediaDisplayMode {
    #[default]
    Auto,
    Inline,
    Block,
}

#[derive(Debug, Clone, Default)]
pub struct MediaResolution {
    pub source: String,
    pub path: Option<PathBuf>,
    pub bytes: Option<Vec<u8>>,
    pub width_cells: Option<u32>,
    pub height_cells: Option<u32>,
    pub display_mode: MediaDisplayMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaSlot {
    pub source: String,
    pub alt: String,
    pub kind: MediaKind,
    pub line: u32,
    pub column: u32,
    pub width_cells: u32,
    pub height_cells: u32,
}

#[derive(Debug, Clone)]
pub struct PreparedImage {
    pub sequence: String,
    pub width_cells: u32,
    pub height_cells: u32,
}

#[derive(Debug, Clone)]
pub enum MediaImageInput<'a> {
    Path(std::borrow::Cow<'a, Path>),
    Bytes(&'a [u8]),
}

pub fn extract_media_requests(blocks: &[Block]) -> Vec<MediaRequest> {
    let mut requests = Vec::new();
    let mut seen = HashSet::new();
    collect_block_requests(blocks, &mut requests, &mut seen);
    requests
}

fn collect_block_requests(
    blocks: &[Block],
    requests: &mut Vec<MediaRequest>,
    seen: &mut HashSet<String>,
) {
    for block in blocks {
        match block {
            Block::Heading { content, .. } | Block::Paragraph(content) => {
                collect_inline_requests(content, requests, seen);
            }
            Block::BlockQuote(blocks) => collect_block_requests(blocks, requests, seen),
            Block::List { items, .. } => {
                for item in items {
                    collect_block_requests(item, requests, seen);
                }
            }
            Block::Table { headers, rows, .. } => {
                for cell in headers.iter().chain(rows.iter().flat_map(|row| row.iter())) {
                    collect_inline_requests(cell, requests, seen);
                }
            }
            Block::Image { alt, path, title } => {
                push_request(
                    requests,
                    seen,
                    path.clone(),
                    alt.clone(),
                    title.clone(),
                    MediaKind::Image,
                );
            }
            Block::FootnoteDefinition { content, .. } => {
                collect_block_requests(content, requests, seen);
            }
            Block::CodeBlock { .. } | Block::Rule => {}
        }
    }
}

fn collect_inline_requests(
    inlines: &[Inline],
    requests: &mut Vec<MediaRequest>,
    seen: &mut HashSet<String>,
) {
    for inline in inlines {
        match inline {
            Inline::Image { alt, path, title } => {
                push_request(
                    requests,
                    seen,
                    path.clone(),
                    alt.clone(),
                    title.clone(),
                    MediaKind::Image,
                );
            }
            Inline::Link { text, dest, title } => {
                collect_inline_requests(text, requests, seen);
                if let Some(kind) = media_kind_from_link(dest) {
                    push_request(
                        requests,
                        seen,
                        dest.clone(),
                        String::new(),
                        title.clone(),
                        kind,
                    );
                }
            }
            Inline::Emph(inner) | Inline::Strong(inner) | Inline::Strike(inner) => {
                collect_inline_requests(inner, requests, seen);
            }
            _ => {}
        }
    }
}

fn push_request(
    requests: &mut Vec<MediaRequest>,
    seen: &mut HashSet<String>,
    source: String,
    alt: String,
    title: Option<String>,
    kind: MediaKind,
) {
    if !seen.insert(source.clone()) {
        return;
    }
    requests.push(MediaRequest {
        source,
        alt,
        title,
        kind,
    });
}

pub fn media_kind_from_link(source: &str) -> Option<MediaKind> {
    if let Some(kind) = media_kind_from_data_uri(source) {
        return Some(kind);
    }
    let path = source.split(['?', '#']).next().unwrap_or(source);
    let extension = path.rsplit_once('.')?.1.to_ascii_lowercase();
    const IMAGE: &[&str] = &["png", "jpg", "jpeg", "webp", "gif", "svg", "bmp", "ico"];
    const VIDEO: &[&str] = &["mp4", "webm", "mov", "m4v", "avi"];
    const AUDIO: &[&str] = &["mp3", "wav", "ogg", "flac", "m4a"];
    if IMAGE.contains(&extension.as_str()) {
        Some(MediaKind::Image)
    } else if VIDEO.contains(&extension.as_str()) {
        Some(MediaKind::Video)
    } else if AUDIO.contains(&extension.as_str()) {
        Some(MediaKind::Audio)
    } else {
        None
    }
}

fn media_kind_from_data_uri(source: &str) -> Option<MediaKind> {
    let lower = source.to_ascii_lowercase();
    let kind = if lower.starts_with("data:image/") {
        MediaKind::Image
    } else if lower.starts_with("data:video/") {
        MediaKind::Video
    } else if lower.starts_with("data:audio/") {
        MediaKind::Audio
    } else {
        return None;
    };
    Some(kind)
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_image(
    image: &image::DynamicImage,
    width: usize,
    max_width_cells: Option<u32>,
    max_height_cells: Option<u32>,
    cell_width_px: Option<u32>,
    cell_height_px: Option<u32>,
    override_width: Option<u32>,
    override_height: Option<u32>,
) -> Result<PreparedImage> {
    let (width_cells, height_cells) = measure_image(
        image.width(),
        image.height(),
        width,
        max_width_cells,
        max_height_cells,
        cell_width_px,
        cell_height_px,
        override_width,
        override_height,
    )?;
    let sequence = encode_image(image, width_cells, Some(height_cells))?;
    Ok(PreparedImage {
        sequence,
        width_cells,
        height_cells,
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn measure_image(
    image_width: u32,
    image_height: u32,
    width: usize,
    max_width_cells: Option<u32>,
    max_height_cells: Option<u32>,
    cell_width_px: Option<u32>,
    cell_height_px: Option<u32>,
    override_width: Option<u32>,
    override_height: Option<u32>,
) -> Result<(u32, u32)> {
    size_cells(
        image_width,
        image_height,
        width.max(1),
        max_width_cells,
        max_height_cells,
        cell_width_px,
        cell_height_px,
        override_width,
        override_height,
    )
    .ok_or_else(|| anyhow::anyhow!("image cell metrics required"))
}

pub(crate) fn encode_image(
    image: &image::DynamicImage,
    width_cells: u32,
    height_cells: Option<u32>,
) -> Result<String> {
    let mut png = Vec::new();
    image.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)?;
    let sequence = render_image_png(
        image,
        &png,
        KittyImageOptions {
            max_width_cells: width_cells.min(u16::MAX as u32) as u16,
            max_height_cells: height_cells.map(|height| height.min(u16::MAX as u32) as u16),
        },
    )?;
    Ok(sequence)
}

#[allow(clippy::too_many_arguments)]
fn size_cells(
    image_width: u32,
    image_height: u32,
    width: usize,
    max_width_cells: Option<u32>,
    max_height_cells: Option<u32>,
    cell_width_px: Option<u32>,
    cell_height_px: Option<u32>,
    override_width: Option<u32>,
    override_height: Option<u32>,
) -> Option<(u32, u32)> {
    if let (Some(width), Some(height)) = (override_width, override_height) {
        return Some((width.max(1), height.max(1)));
    }
    let (Some(cell_width_px), Some(cell_height_px)) = (cell_width_px, cell_height_px) else {
        return None;
    };
    if image_width == 0 || image_height == 0 || cell_width_px == 0 || cell_height_px == 0 {
        return None;
    }

    let natural_width = image_width.div_ceil(cell_width_px);
    let natural_height = image_height.div_ceil(cell_height_px);
    let max_width = max_width_cells
        .unwrap_or(width as u32)
        .min(width as u32)
        .max(1);
    let max_height = max_height_cells.unwrap_or(u32::MAX).max(1);
    let scale_w = max_width as f32 / natural_width as f32;
    let scale_h = if max_height >= natural_height {
        f32::INFINITY
    } else {
        max_height as f32 / natural_height as f32
    };
    let scale = scale_w.min(scale_h).min(1.0);
    let result_width = ((natural_width as f32 * scale).ceil() as u32).max(1);
    let result_height = ((natural_height as f32 * scale).ceil() as u32).max(1);
    Some((result_width, result_height))
}

pub fn find_resolution<'a>(
    resolutions: &'a [MediaResolution],
    source: &str,
) -> Option<&'a MediaResolution> {
    resolutions
        .iter()
        .find(|resolution| resolution.source == source)
}

pub fn image_input<'a>(
    source: &str,
    source_dir: &Path,
    resolutions: &'a [MediaResolution],
) -> Option<MediaImageInput<'a>> {
    let resolution = find_resolution(resolutions, source);
    if let Some(path) = resolution.and_then(|item| item.path.as_deref()) {
        return Some(MediaImageInput::Path(std::borrow::Cow::Borrowed(path)));
    }
    if let Some(bytes) = resolution.and_then(|item| item.bytes.as_deref()) {
        return Some(MediaImageInput::Bytes(bytes));
    }

    let lower = source.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("ftp://")
    {
        return None;
    }
    let path = Path::new(source);
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        source_dir.join(path)
    };
    Some(MediaImageInput::Path(std::borrow::Cow::Owned(path)))
}

pub fn load_image(input: MediaImageInput<'_>) -> Result<image::DynamicImage> {
    match input {
        MediaImageInput::Path(path) => image::ImageReader::open(path.as_ref())?
            .with_guessed_format()?
            .decode(),
        MediaImageInput::Bytes(bytes) => image::ImageReader::new(std::io::Cursor::new(bytes))
            .with_guessed_format()?
            .decode(),
    }
    .map_err(|error| anyhow::anyhow!("failed to load media: {error}"))
}

pub fn requests_without_resolutions(
    blocks: &[Block],
    resolutions: &[MediaResolution],
) -> Vec<MediaRequest> {
    let resolved: HashSet<&str> = resolutions
        .iter()
        .map(|item| item.source.as_str())
        .collect();
    extract_media_requests(blocks)
        .into_iter()
        .filter(|request| !resolved.contains(request.source.as_str()))
        .collect()
}

pub fn has_duplicates(resolutions: &[MediaResolution]) -> bool {
    let mut seen = HashSet::new();
    resolutions
        .iter()
        .any(|item| !seen.insert(item.source.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::ast::{Block, Inline};

    #[test]
    fn extracts_unique_media_requests() {
        let blocks = vec![Block::Paragraph(vec![
            Inline::Image {
                alt: "logo".to_string(),
                path: "https://example.com/logo.png".to_string(),
                title: Some("Logo".to_string()),
            },
            Inline::Text(" ".to_string()),
            Inline::Image {
                alt: "logo again".to_string(),
                path: "https://example.com/logo.png".to_string(),
                title: None,
            },
            Inline::Link {
                text: vec![Inline::Text("video".to_string())],
                dest: "https://example.com/demo.mp4".to_string(),
                title: None,
            },
            Inline::Link {
                text: vec![Inline::Text("ordinary".to_string())],
                dest: "https://example.com/docs".to_string(),
                title: None,
            },
        ])];
        let requests = extract_media_requests(&blocks);
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].kind, MediaKind::Image);
        assert_eq!(requests[0].title.as_deref(), Some("Logo"));
        assert_eq!(requests[1].kind, MediaKind::Video);
    }

    #[test]
    fn unresolved_requests_exclude_resolutions() {
        let blocks = vec![Block::Image {
            alt: "logo".to_string(),
            path: "https://example.com/logo.png".to_string(),
            title: None,
        }];
        let resolutions = vec![MediaResolution {
            source: "https://example.com/logo.png".to_string(),
            bytes: Some(vec![1, 2, 3]),
            ..MediaResolution::default()
        }];
        assert!(requests_without_resolutions(&blocks, &resolutions).is_empty());
    }

    #[test]
    fn computes_image_cells_with_terminal_metrics() {
        let image = image::DynamicImage::new_luma8(20, 40);
        let prepared = prepare_image(
            &image,
            80,
            Some(10),
            Some(10),
            Some(10),
            Some(20),
            None,
            None,
        )
        .expect("metrics supplied");
        assert_eq!(prepared.width_cells, 2);
        assert_eq!(prepared.height_cells, 2);
        assert!(prepared.sequence.starts_with("\u{1b}_G"));
    }
}
