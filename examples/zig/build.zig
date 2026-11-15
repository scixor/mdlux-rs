const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    const mdlux = b.dependency("mdlux", .{
        .target = target,
        .optimize = optimize,
    });

    const exe = b.addExecutable(.{
        .name = "mdlux_zig_smoke",
        .root_module = b.createModule(.{
            .root_source_file = b.path("main.zig"),
            .target = target,
            .optimize = optimize,
        }),
    });
    exe.root_module.addImport("mdlux", mdlux.module("mdlux"));
    exe.linkLibC();
    b.installArtifact(exe);

    const run_exe = b.addRunArtifact(exe);
    const test_step = b.step("test", "Run the Zig smoke app");
    test_step.dependOn(&run_exe.step);
}
