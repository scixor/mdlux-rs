use std::path::PathBuf;

use mdlux::kitty::graphics::{KittyImageOptions, render_image};
use mdlux::markdown::parser::parse_markdown;
use mdlux::render::{Capabilities, RenderContext, render_document};
use mdlux::theme::builtin::find_theme;
use mdlux::util::width::visible_width;

fn ctx(
    ansi: bool,
    kitty_text_size: bool,
    kitty_graphics: bool,
    no_highlight: bool,
) -> RenderContext {
    RenderContext {
        width: 72,
        theme: find_theme("ansi").expect("theme must exist").clone(),
        capabilities: Capabilities {
            ansi,
            kitty_text_size,
            kitty_graphics,
            kitty_hyperlinks: kitty_text_size,
        },
        source_dir: PathBuf::from("."),
        no_highlight,
    }
}

fn render_fixture(name: &str, ctx: &RenderContext) -> String {
    let input =
        std::fs::read_to_string(format!("tests/fixtures/{name}.md")).expect("fixture should exist");
    let blocks = parse_markdown(&input);
    render_document(&blocks, ctx).expect("render should succeed")
}

#[test]
fn snapshot_basic_plain() {
    let out = render_fixture("basic", &ctx(false, false, false, true));
    insta::assert_snapshot!("basic_plain", out);
}

#[test]
fn snapshot_basic_ansi() {
    let out = render_fixture("basic", &ctx(true, false, false, true));
    insta::assert_snapshot!("basic_ansi", out);
}

#[test]
fn snapshot_code_no_highlight() {
    let out = render_fixture("code", &ctx(true, false, false, true));
    insta::assert_snapshot!("code_no_highlight", out);
}

#[test]
fn snapshot_table_plain() {
    let out = render_fixture("table", &ctx(false, false, false, true));
    insta::assert_snapshot!("table_plain", out);
}

#[test]
fn snapshot_image_fallback() {
    let out = render_fixture("image", &ctx(false, false, false, true));
    insta::assert_snapshot!("image_fallback", out);
}

#[test]
fn width_is_respected_for_plain_text_lines() {
    let mut c = ctx(false, false, false, true);
    c.width = 32;
    let out = render_fixture("basic", &c);
    for line in out.lines() {
        if line.trim().is_empty() {
            continue;
        }
        assert!(
            visible_width(line) <= 32,
            "line too wide: {} > 32, line={line:?}",
            visible_width(line)
        );
    }
}

#[test]
fn kitty_text_size_sequence_emitted() {
    let out = render_fixture("basic", &ctx(true, true, false, true));
    assert!(out.contains("\u{1b}]66;"), "expected OSC 66 sequence");
    assert!(
        !out.contains("[1;38;"),
        "should not emit broken ANSI payload inside OSC 66"
    );
}

#[test]
fn kitty_image_encoder_envelope() {
    let path = std::env::temp_dir().join("mdlux_test_image.png");
    let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 0, 255]));
    img.save(&path).expect("save png");

    let seq = render_image(
        &path,
        KittyImageOptions {
            max_width_cells: 20,
            max_height_cells: None,
        },
    )
    .expect("kitty image render");

    assert!(seq.starts_with("\u{1b}_G"), "must start with kitty APC");
    assert!(seq.contains("f=100"), "must include png format tag");
    assert!(seq.ends_with("\u{1b}\\"), "must end with ST");
}

#[test]
fn footnotes_and_strikethrough_render() {
    let out = render_fixture("footnotes", &ctx(false, false, false, true));
    assert!(out.contains("[^dep]"));
    assert!(out.contains("[^dep]: Remove this flag in v2."));
}

#[test]
fn snapshot_lists_nested_plain() {
    let out = render_fixture("lists_nested", &ctx(false, false, false, true));
    insta::assert_snapshot!("lists_nested_plain", out);
}
