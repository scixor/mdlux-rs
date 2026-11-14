use std::process::Command;

use mdlux::markdown::ast::{Block, Inline};
use mdlux::markdown::parser::parse_markdown;
use mdlux::render::{
    Capabilities, MediaKind, MediaResolution, RenderOptions, prepare_image, render_markdown,
};

fn png_bytes() -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(20, 40, image::Rgba([255, 0, 0, 255]));
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut bytes, image::ImageFormat::Png)
        .expect("encode test image");
    bytes.into_inner()
}

fn media_options() -> RenderOptions {
    RenderOptions {
        capabilities: Capabilities {
            kitty_graphics: true,
            ..Capabilities::default()
        },
        cell_width_px: Some(10),
        cell_height_px: Some(20),
        media: vec![MediaResolution {
            source: "x.png".to_string(),
            bytes: Some(png_bytes()),
            ..MediaResolution::default()
        }],
        ..RenderOptions::default()
    }
}

#[test]
fn html_br_becomes_a_hard_break() {
    for source in ["a<br>b", "a<br/>b", "a<br />b", "a<BR>b"] {
        let blocks = parse_markdown(source);
        let [Block::Paragraph(inlines)] = blocks.as_slice() else {
            panic!("expected one paragraph: {blocks:?}");
        };
        assert!(inlines.contains(&Inline::HardBreak), "{inlines:?}");
    }
}

#[test]
fn html_does_not_duplicate_code_spans() {
    let output = render_markdown("x <span>`code`</span> y", &RenderOptions::default()).unwrap();
    assert_eq!(output.text, "x code y\n\n");
}

#[test]
fn cli_images_work_without_host_supplied_cell_metrics() {
    let root = std::env::temp_dir().join(format!("mdlux-cli-image-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    image::RgbaImage::from_pixel(20, 40, image::Rgba([255, 0, 0, 255]))
        .save(root.join("image.png"))
        .unwrap();
    let markdown = root.join("input.md");
    std::fs::write(&markdown, "![alt](image.png)\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_mdlux"))
        .arg("--images")
        .arg("always")
        .arg("--text-size")
        .arg("never")
        .arg(markdown)
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("\u{1b}_G"), "expected Kitty image: {text:?}");
}

#[test]
fn requests_include_images_nested_in_links() {
    let output = render_markdown(
        "[![alt](x.png)](https://example.com)",
        &RenderOptions::default(),
    )
    .unwrap();
    assert!(
        output
            .media_requests
            .iter()
            .any(|item| item.source == "x.png")
    );
}

#[test]
fn media_link_extensions_are_case_insensitive() {
    let output = render_markdown("[watch](movie.MP4)", &RenderOptions::default()).unwrap();
    assert_eq!(output.media_requests.len(), 1);
    assert_eq!(output.media_requests[0].kind, MediaKind::Video);
}

#[test]
fn resolved_inline_images_produce_slots() {
    for source in [
        "before ![alt](x.png) after",
        "[![alt](x.png)](https://example.com)",
        "**![alt](x.png)**",
    ] {
        let output = render_markdown(source, &media_options()).unwrap();
        assert_eq!(output.media_slots.len(), 1, "{:?}", output.text);
    }
}

#[test]
fn host_supplied_media_dimensions_are_used() {
    let mut options = media_options();
    options.media[0].width_cells = Some(7);
    options.media[0].height_cells = Some(3);
    let output = render_markdown("![alt](x.png)", &options).unwrap();
    assert_eq!(output.media_slots.len(), 1);
    assert_eq!(output.media_slots[0].width_cells, 7);
    assert_eq!(output.media_slots[0].height_cells, 3);
}

#[test]
fn nested_image_slots_include_parent_offsets() {
    for (source, column) in [
        ("before\n\n> ![alt](x.png)", 2),
        ("before\n\n- ![alt](x.png)", 2),
        ("before\n\n[^a]: ![alt](x.png)", 6),
    ] {
        let output = render_markdown(source, &media_options()).unwrap();
        assert_eq!(output.media_slots.len(), 1);
        assert_eq!(output.media_slots[0].line, 2, "{source}");
        assert_eq!(output.media_slots[0].column, column, "{source}");
    }
}

#[test]
fn table_image_slot_matches_its_cell_column() {
    let output = render_markdown(
        "| A | B |\n|---|---|\n| text | ![alt](x.png) |",
        &media_options(),
    )
    .unwrap();
    let image_line = output
        .text
        .lines()
        .find(|line| line.contains("alt"))
        .unwrap();
    let expected_column = image_line.find("alt").unwrap() as u32;
    assert_eq!(output.media_slots[0].column, expected_column);
}

#[test]
fn large_cell_metrics_do_not_overflow() {
    let image = image::DynamicImage::new_rgba8(20, 40);
    let prepared =
        prepare_image(&image, 80, None, None, Some(u32::MAX), Some(20), None, None).unwrap();
    assert_eq!((prepared.width_cells, prepared.height_cells), (1, 2));
}

#[test]
fn invalid_host_supplied_media_is_an_error() {
    let options = RenderOptions {
        capabilities: Capabilities {
            kitty_graphics: true,
            ..Capabilities::default()
        },
        cell_width_px: Some(10),
        cell_height_px: Some(20),
        media: vec![MediaResolution {
            source: "x.png".to_string(),
            bytes: Some(vec![1, 2, 3]),
            ..MediaResolution::default()
        }],
        ..RenderOptions::default()
    };
    assert!(render_markdown("![alt](x.png)", &options).is_err());
}

#[test]
fn embedded_table_images_are_not_emitted_in_fallback() {
    let mut options = media_options();
    options.embed_media = true;
    let source = "| A | B |\n|---|---|\n| text | ![alt](x.png) |";

    let output = render_markdown(source, &options).unwrap();
    assert!(output.text.contains("\u{1b}_G"));
    assert_eq!(output.media_slots.len(), 1);
    assert!(
        output
            .text
            .lines()
            .all(|line| !line.contains("\u{1b}_G") || !line.contains("alt"))
    );

    options.width = 8;
    let fallback = render_markdown(source, &options).unwrap();
    assert!(!fallback.text.contains("\u{1b}_G"));
    assert!(fallback.media_slots.is_empty());
}
