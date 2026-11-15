const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    const cargo_target = cargoTargetFromZig(b, target.result);
    const archive = if (target.result.os.tag == .windows)
        b.fmt("target/{s}/release/mdlux.lib", .{cargo_target})
    else
        b.fmt("target/{s}/release/libmdlux.a", .{cargo_target});

    const cargo = b.addSystemCommand(&.{
        "cargo",    "build",      "--locked", "--release",
        "--target", cargo_target,
    });
    cargo.setCwd(b.path("."));
    cargo.setName("cargo-build-mdlux");

    const cargo_stub = b.addLibrary(.{
        .name = "mdlux",
        .root_module = b.createModule(.{
            .target = target,
            .optimize = optimize,
            .link_libc = true,
        }),
        .linkage = .static,
    });

    const module = b.addModule("mdlux", .{
        .root_source_file = b.path("src/mdlux.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    module.addObjectFile(b.path(archive));
    module.linkLibrary(cargo_stub);
    linkSystemLibraries(module, target.result);

    const test_mod = b.createModule(.{
        .root_source_file = b.path("src/mdlux.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    test_mod.addObjectFile(b.path(archive));
    linkSystemLibraries(test_mod, target.result);

    const tests = b.addTest(.{ .root_module = test_mod });
    const run_tests = b.addRunArtifact(tests);
    const test_step = b.step("test", "Run package tests");
    test_step.dependOn(&run_tests.step);
    test_step.dependOn(&cargo.step);
}

fn linkSystemLibraries(module: *std.Build.Module, target: std.Target) void {
    switch (target.os.tag) {
        .linux => {
            for ([_][]const u8{ "m", "pthread", "dl", "unwind" }) |name| {
                module.linkSystemLibrary(name, .{});
            }
        },
        .macos => {
            module.linkFramework("Security", .{});
            module.linkFramework("CoreFoundation", .{});
        },
        else => {},
    }
}

fn cargoTargetFromZig(b: *std.Build, target: std.Target) []const u8 {
    const arch = switch (target.cpu.arch) {
        .x86_64 => "x86_64",
        .aarch64 => "aarch64",
        .x86 => "x86",
        .arm => "arm",
        else => @panic("unsupported architecture for Zig-based Rust build"),
    };
    switch (target.os.tag) {
        .linux => return switch (target.abi) {
            .musl, .muslf32, .musln32 => b.fmt("{s}-unknown-linux-musl", .{arch}),
            .gnu, .gnuf32, .gnun32, .gnusf, .gnux32, .none => b.fmt("{s}-unknown-linux-gnu", .{arch}),
            else => @panic("unsupported Linux ABI"),
        },
        .macos => return b.fmt("{s}-apple-darwin", .{arch}),
        .windows => return b.fmt("{s}-pc-windows-msvc", .{arch}),
        else => @panic("unsupported target OS"),
    }
}
