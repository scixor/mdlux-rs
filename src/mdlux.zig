//! Minimal Zig wrapper around the C ABI.
//!
//! `Options` and `Output` mirror `mdlux_options` and `mdlux_output`.
//! `renderMarkdown` renders UTF-8 markdown. `outputFree` releases output memory.

const std = @import("std");

pub const Opaque = opaque {
    pub const _ = @This();
};

pub const Color = extern struct {
    kind: u32,
    index: u8,
    rgb: [3]u8,
};

pub const Style = extern struct {
    foreground: Color,
    background: Color,
    bold: u8,
    italic: u8,
    underline: u8,
    dim: u8,
};

pub const Theme = extern struct {
    name: ?[*:0]const u8,
    heading1: Style,
    heading2: Style,
    heading3: Style,
    heading4: Style,
    text: Style,
    emphasis: Style,
    strong: Style,
    inline_code: Style,
    code: Style,
    quote_marker: Style,
    quote_text: Style,
    link_label: Style,
    link_url: Style,
    rule: Style,
    table_header: Style,
    table_border: Style,
    list_marker: Style,
};

pub const Options = extern struct {
    width_cells: u32,
    theme_name: ?[*:0]const u8,
    custom_theme: ?*const Theme,
    code_theme_name: ?[*:0]const u8,
    source_dir: ?[*:0]const u8,
    ansi: u8,
    highlight: u8,
    kitty_text_size: u8,
    kitty_graphics: u8,
    kitty_hyperlinks: u8,
    embed_media: u8,
    cell_width_px: u32,
    cell_height_px: u32,
    max_media_width_cells: u32,
    max_media_height_cells: u32,
};

pub const MediaResolution = extern struct {
    source: ?[*:0]const u8,
    source_len: usize,
    path: ?[*:0]const u8,
    bytes: ?[*]const u8,
    bytes_len: usize,
    width_cells: u32,
    height_cells: u32,
    display_mode: u8,
};

pub const MediaRequest = extern struct {
    source: ?[*:0]u8,
    alt: ?[*:0]u8,
    title: ?[*:0]u8,
    kind: i32,
};

pub const MediaSlot = extern struct {
    source: ?[*:0]u8,
    alt: ?[*:0]u8,
    kind: i32,
    line: u32,
    column: u32,
    width_cells: u32,
    height_cells: u32,
};

pub const Output = extern struct {
    bytes: ?[*]u8,
    bytes_len: usize,
    line_count: u32,
    slots: ?[*]MediaSlot,
    slot_count: usize,
    requests: ?[*]MediaRequest,
    request_count: usize,
};

pub const MDLUX_OK: i32 = 0;

pub extern "C" fn mdlux_render(
    source: [*]const u8,
    source_len: usize,
    options: *const Options,
    media: ?*const MediaResolution,
    media_count: usize,
    output: *Output,
) callconv(.c) i32;

pub extern "C" fn mdlux_output_free(output: *Output) callconv(.c) void;

pub extern "C" fn mdlux_theme_count() callconv(.c) i32;

pub extern "C" fn mdlux_theme_name(index: i32) callconv(.c) ?[*:0]const u8;

pub extern "C" fn mdlux_theme_builtin(
    name: [*:0]const u8,
    output: *Theme,
) callconv(.c) i32;

pub extern "C" fn mdlux_version() callconv(.c) u32;

pub fn renderMarkdown(source: []const u8, options: *const Options) !Output {
    return renderMarkdownWithMedia(source, options, &[_]MediaResolution{});
}

pub fn renderMarkdownWithMedia(
    source: []const u8,
    options: *const Options,
    media: []const MediaResolution,
) !Output {
    var output = std.mem.zeroes(Output);
    const media_ptr = if (media.len == 0) null else &media[0];
    const status = mdlux_render(source.ptr, source.len, options, media_ptr, media.len, &output);
    return if (status == MDLUX_OK) output else error.MdluxRender;
}

pub fn outputFree(output: *Output) void {
    mdlux_output_free(output);
}

test "renders markdown through the C ABI" {
    const source = "# Hello\\n\\nSome *text*.\\n";
    var options = std.mem.zeroes(Options);
    options.width_cells = 80;
    options.theme_name = "dark";

    var output = try renderMarkdown(source, &options);
    defer outputFree(&output);
    try std.testing.expect(output.bytes_len > 0);
}
