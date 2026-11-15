//! Smoke app for the Zig wrapper.
//!
//! Run from the repo root with:

//! `mdlux.Options` mirrors `mdlux_options` in `include/mdlux.h`.
//! `width_cells` is the target column count.
//! Set `ansi = 1` and `highlight = 1` for a styled TUI.
//! `renderMarkdown` returns text plus media metadata; free it with `outputFree`.
//!   cargo build --release
//!   zig build-exe examples/zig/main.zig -Mmdlux=src/mdlux.zig \\
//!     target/release/libmdlux.a -lm -lpthread -ldl -lunwind -lc

const std = @import("std");

var stdout_buffer: [4096]u8 = undefined;

pub fn main(init: std.process.Init) !void {
    const io = init.io;
    const mdlux = @import("mdlux");

    const source = "# Hello\n\nSome *text*.\n";
    // `Options` controls width/theme/highlighting/media; see mdlux.h.
    var options = std.mem.zeroes(mdlux.Options);
    options.width_cells = 80;
    options.theme_name = "dark";
    options.ansi = 1;
    options.highlight = 1;

    // The output owns UTF-8 bytes plus media metadata.
    var output = try mdlux.renderMarkdown(source, &options);
    defer mdlux.outputFree(&output);

    var stdout_writer = std.Io.File.stdout().writer(io, &stdout_buffer);
    const stdout = &stdout_writer.interface;
    var bytes = output.bytes orelse return error.NoBytes;
    try stdout.writeAll(bytes[0..output.bytes_len]);
    try stdout.writeAll("\n");
    try stdout.flush();
}
