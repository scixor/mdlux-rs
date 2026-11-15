//! Two-pass mdlux tour with static images and an animated GIF: render for media requests, then resolve and render again.

const std = @import("std");

var stdout_buffer: [4096]u8 = undefined;

pub fn main(init: std.process.Init) !void {
    const io = init.io;
    const gpa = init.gpa;
    const mdlux = @import("mdlux");

    const sources = [_][:0]const u8{
        "examples/assets/cat-1.webp",
        "examples/assets/cat-2.webp",
        "examples/assets/cat-3.webp",
        "examples/assets/neko.gif",
    };
    const source = "# Hello\n\nSome *text* and ![fat cat 1](examples/assets/cat-1.webp) ![fat cat 2](examples/assets/cat-2.webp) ![fat cat 3](examples/assets/cat-3.webp).\n\n![Neko](examples/assets/neko.gif)\n";

    // Pass 1: render without media, so the host can inspect unresolved requests.
    var first_options = std.mem.zeroes(mdlux.Options);
    first_options.width_cells = 80;
    first_options.theme_name = "dark";
    first_options.source_dir = ".";

    var first = mdlux.renderMarkdown(source, &first_options) catch return error.RenderFailed;
    defer mdlux.outputFree(&first);
    std.debug.print("requests: {d}\n", .{first.request_count});

    // Pass 2: resolve each request with bytes already downloaded from stickers.cloud.
    var media: [sources.len]mdlux.MediaResolution = undefined;
    var bytes: [sources.len][]u8 = undefined;
    for (sources, 0..) |source_path, i| {
        bytes[i] = try std.Io.Dir.cwd().readFileAlloc(io, source_path, gpa, .limited(8 << 20));
        media[i] = std.mem.zeroes(mdlux.MediaResolution);
        media[i].source = source_path.ptr;
        media[i].source_len = source_path.len;
        media[i].bytes = bytes[i].ptr;
        media[i].bytes_len = bytes[i].len;
        media[i].width_cells = 4;
        media[i].height_cells = 0;
        media[i].display_mode = 0;
    }
    defer for (bytes) |buffer| gpa.free(buffer);

    var second_options = std.mem.zeroes(mdlux.Options);
    second_options.width_cells = 80;
    second_options.theme_name = "dark";
    second_options.source_dir = ".";
    second_options.kitty_graphics = 1;
    second_options.embed_media = 1;
    second_options.cell_width_px = 8;
    second_options.cell_height_px = 16;
    second_options.max_media_width_cells = 4;
    second_options.max_media_height_cells = 4;

    var second = mdlux.renderMarkdownWithMedia(source, &second_options, &media) catch return error.RenderFailed;
    defer mdlux.outputFree(&second);
    var stdout_writer = std.Io.File.stdout().writer(io, &stdout_buffer);
    const stdout = &stdout_writer.interface;
    const bytes_ = second.bytes orelse return error.NoBytes;
    try stdout.writeAll(bytes_[0..second.bytes_len]);
    try stdout.writeAll("\n");
    try stdout.flush();
}
