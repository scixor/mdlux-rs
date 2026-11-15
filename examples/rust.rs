use mdlux::render::{MediaResolution, RenderOptions};
// Full tour: render text, inspect media requests, then resolve three locally downloaded fat cat stickers and an animated GIF.
// Run with: cargo run --example rust

const SOURCES: [&str; 4] = [
    "examples/assets/cat-1.webp",
    "examples/assets/cat-2.webp",
    "examples/assets/cat-3.webp",
    "examples/assets/neko.gif",
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = format!(
        "# Hello\n\nSome *text* and ![fat cat 1]({}) ![fat cat 2]({}) ![fat cat 3]({}).\n\n![Neko]({})\n",
        SOURCES[0], SOURCES[1], SOURCES[2], SOURCES[3]
    );

    // Pass 1: render without media, so the host sees unresolved media requests.
    let options = RenderOptions::default();
    let first = mdlux::render::render_markdown(&source, &options)?;
    println!("requests: {}", first.media_requests.len());
    for request in &first.media_requests {
        println!("  {} {:?}", request.source, request.kind);
    }

    let media = SOURCES
        .iter()
        .map(|source| -> Result<MediaResolution, std::io::Error> {
            Ok(MediaResolution {
                source: (*source).to_string(),
                path: None,
                bytes: Some(std::fs::read(source)?),
                width_cells: Some(4),
                height_cells: None,
                display_mode: mdlux::render::MediaDisplayMode::Auto,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let options = mdlux::render::RenderOptions {
        capabilities: mdlux::render::Capabilities {
            ansi: true,
            kitty_text_size: false,
            kitty_graphics: true,
            kitty_hyperlinks: true,
        },
        embed_media: true,
        cell_width_px: Some(8),
        cell_height_px: Some(16),
        max_media_width_cells: Some(4),
        max_media_height_cells: None,
        media,
        ..RenderOptions::default()
    };

    let second = mdlux::render::render_markdown(&source, &options)?;
    println!(
        "slots: {} / requests: {}",
        second.media_slots.len(),
        second.media_requests.len()
    );
    print!("{}", second.text);
    Ok(())
}
