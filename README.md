# mdlux

Terminal Markdown renderer which I made for myself because I didn't find others adequate.	ᕕ(⌐■_■)ᕗ ♪♬

Some basic features:
- plain text (just convert it to simple text without styling, quite useful sometimes)
- ANSI styling (for anything other than kitty)
- Kitty enhancements (text sizing and images)
- Common inline/block HTML conversion (not full browser HTML/CSS)
- Kitty/table image rendering with media metadata
- Some themes (built-ins can be cloned and restyled through the library)

What it doesn't support atm:
- callouts (i do plan to support this)
- tons of other things which I don't use a lot

## Install

### Cargo (crates.io)

```bash
cargo install mdlux
```

### eget (GitHub releases)

```bash
# usually works directly by picking the matching release asset
eget scixor/mdlux-rs

# if needed, force the asset pattern explicitly
# Linux (x86_64 musl):
eget scixor/mdlux-rs --asset x86_64-unknown-linux-musl
# macOS (Apple Silicon):
eget scixor/mdlux-rs --asset aarch64-apple-darwin
```

## Usage

```bash
mdlux README.md
cat README.md | mdlux
# Override width as mdlux tries to fit widh by default
mdlux --theme nord --width 100 doc.md
# Turn off text re-sizing
mdlux --text-size never --images auto README.md
```

## Flags

```text
--width <cols>                                   // if you want to enforce
--theme <ansi|dark|light|nord|gruvbox|kanagawa>  // I like nord/kanagawa a lot
--text-size <auto|always|never>                  // resize text (basically kitty)
--images <auto|always|never>                     // render out images if supportd
--no-highlight
--plain
--list-themes
```

### Basic styling
> Theme: **ANSI**

```md
# mdlux showcase

Welcome to a markdown stress test for `mdlux`.

> This file is intentionally varied so you can quickly eyeball formatting, wrapping, links, code highlighting, and tables.

---

## Inline styles

Normal text, *italic text*, **bold text**, and `inline code` all in one line.

You can also combine things like **bold with `inline code`** and links like [Rust](https://www.rust-lang.org/).

## Lists and nesting

- Unordered item one
- Unordered item two with a long sentence to test line wrapping behavior when terminal width gets narrower than expected.
- Unordered item three

1. Ordered item one
2. Ordered item two
3. Ordered item three

## Blockquote

> Markdown in terminals is nicest when structure survives: headings, quotes, bullets, and code should all remain readable.

```

![Basic styling - theme ansi](./img/ansi.png)

### Code

> Theme: Nord

````md
## Code: Rust

```rust
use std::collections::HashMap;

fn word_count(input: &str) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for word in input.split_whitespace() {
        let key = word.to_lowercase();
        *counts.entry(key).or_insert(0) += 1;
    }
    counts
}

fn main() {
    let sample = "Rust rust RUST and markdown";
    let counts = word_count(sample);
    println!("{counts:#?}");
}
```

## Code: Zig

```zig
const std = @import("std");

pub fn main() !void {
    const stdout = std.io.getStdOut().writer();
    const nums = [_]i32{ 1, 2, 3, 4, 5 };

    var sum: i32 = 0;
    for (nums) |n| sum += n;

    try stdout.print("sum = {d}\n", .{sum});
}
```
````

![Code example - theme nord](./img/nord.png)

**Bonus**:

> Theme: Kanagawa
````md
## Code: Bash

```bash
#!/usr/bin/env bash
set -euo pipefail

for file in *.md; do
  printf "Checking %s\n" "$file"
done

echo "Done"
```

## Table

| Feature | Plain | ANSI | Kitty |
|---|---:|---:|---:|
| Headings | yes | yes | yes |
| Syntax highlighting | no | yes | yes |
| Image rendering | no | no | yes |
| Text sizing | no | no | yes |

## Links and references

- Project repo style docs: [README](./README.md)
- Theme test target: `--theme kanagawa`
- Another theme test target: `--theme nord`

````


![Bonus - theme kanagawa](./img/kanagawa.png)

## Library use and customization

The Rust library exposes `render::render_markdown` through:
```rust
use mdlux::render::{Capabilities, RenderOptions, render_markdown};

let options = RenderOptions { ... };
let output = mdlux::render::render_markdown(markdown_or_html, &options)?;
println!("{}", output.text);
```

Themes are values. Start from a built-in theme, clone it, alter any style, and pass it in
`RenderOptions.theme`. `code_theme_name` is optional and chooses the syntax-highlighting palette
separately from document furniture styles.

### C library

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

### Remote media flow

1. Call `mdlux_render` once with an empty media array.
2. Read `output.requests` for URLs/resources. mdlux returns requests for images and
   media-like links; ordinary links remain links.
3. Fetch, authenticate, cache, and classify those resources in the host application.
4. Call `mdlux_render` again with matching `mdlux_media_resolution` entries.
5. For images, mdlux can insert Kitty graphics (`embed_media=1`) or reserve cells and return
   placement metadata (`embed_media=0`).

mdlux performs no downloads or playback. Hosts own network security, caching, authentication,
thumbnails, video/audio playback, and cursor/screen placement.
