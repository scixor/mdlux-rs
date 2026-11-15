# C API


Build and link the static archive:

```bash
cargo build --release
gcc app.c -I mdlux-rs/include mdlux-rs/target/release/libmdlux.a -lm -lpthread -ldl
```

From an extracted C release archive, the library is at the archive root:

```bash
gcc app.c -I include libmdlux.a -lm -lpthread -ldl
```

For the `x86_64-unknown-linux-musl` archive, use Zig 0.17 to supply the musl-compatible unwind runtime:

```bash
zig cc -target x86_64-linux-musl app.c -I include libmdlux.a -lm -lpthread -ldl -lunwind
```

A CMake helper is available:

```cmake
set(MDLUX_ROOT /path/to/source-tree-or-extracted-C-archive)
# Either copy/install cmake/mdluxConfig.cmake into your CMAKE_PREFIX_PATH or include it
# after defining MDLUX_ROOT.
include(${MDLUX_ROOT}/cmake/mdluxConfig.cmake)
target_link_libraries(app PRIVATE mdlux::mdlux)
```

See `include/mdlux.h` and `examples/capi.c`. The C entry point returns the rendered UTF-8 buffer and media slots/requests. The host must call `mdlux_output_free`.

## Remote media flow

1. Call `mdlux_render` once with an empty media array.
2. Read `output.requests` for URLs/resources. mdlux returns requests for images and
   media-like links; ordinary links remain links.
3. Fetch, authenticate, cache, and classify those resources in the host application.
4. Call `mdlux_render` again with matching `mdlux_media_resolution` entries.
5. For images, mdlux can insert Kitty graphics (`embed_media=1`) or reserve cells and return
   placement metadata (`embed_media=0`).

mdlux performs no downloads or playback. Hosts own network security, caching, authentication,
thumbnails, video/audio playback, and cursor/screen placement.
