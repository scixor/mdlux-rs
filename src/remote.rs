use std::time::Duration;

use anyhow::{Context, Result};
use ureq::Agent;

use crate::markdown::ast::Block;
use crate::render::{MediaKind, MediaResolution, extract_media_requests};

const MAX_REMOTE_IMAGE_BYTES: u64 = 20 * 1024 * 1024;

fn fetch(agent: &Agent, source: &str) -> Result<Vec<u8>> {
    let mut response = agent
        .get(source)
        .header("Accept", "image/png,image/jpeg,image/gif,image/webp")
        .call()
        .with_context(|| format!("requesting {source:?}"))?;
    if response
        .body()
        .mime_type()
        .is_some_and(|mime| !mime.starts_with("image/"))
    {
        anyhow::bail!("expected an image response from {source:?}");
    }
    let bytes = response
        .body_mut()
        .with_config()
        .limit(MAX_REMOTE_IMAGE_BYTES)
        .read_to_vec()
        .with_context(|| format!("reading {source:?}"))?;
    let format = image::guess_format(&bytes).context("unrecognized image data")?;
    if !matches!(
        format,
        image::ImageFormat::Png
            | image::ImageFormat::Jpeg
            | image::ImageFormat::Gif
            | image::ImageFormat::WebP
    ) {
        anyhow::bail!("unsupported image format: {format:?}");
    }
    Ok(bytes)
}

pub fn fetch_blocks(blocks: &[Block]) -> Vec<MediaResolution> {
    let requests: Vec<_> = extract_media_requests(blocks)
        .into_iter()
        .filter(|request| {
            request.kind == MediaKind::Image
                && (request.source.starts_with("https://") || request.source.starts_with("http://"))
        })
        .collect();
    if requests.is_empty() {
        return Vec::new();
    }

    let config = Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .max_redirects(3)
        .user_agent(concat!("mdlux/", env!("CARGO_PKG_VERSION")))
        .build();
    let agent = Agent::from(config);
    let mut media = Vec::new();
    for batch in requests.chunks(4) {
        std::thread::scope(|scope| {
            let handles: Vec<_> = batch
                .iter()
                .map(|request| {
                    let agent = &agent;
                    let source = &request.source;
                    scope.spawn(move || fetch(agent, source))
                })
                .collect();
            for (request, handle) in batch.iter().zip(handles) {
                match handle.join() {
                    Ok(Ok(bytes)) => media.push(MediaResolution {
                        source: request.source.clone(),
                        bytes: Some(bytes),
                        ..MediaResolution::default()
                    }),
                    Ok(Err(error)) => eprintln!("mdlux: remote image skipped: {error:#}"),
                    Err(_) => eprintln!(
                        "mdlux: remote image skipped: downloader panicked for {:?}",
                        request.source
                    ),
                }
            }
        });
    }
    media
}
