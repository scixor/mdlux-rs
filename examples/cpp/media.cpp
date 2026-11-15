#include <mdlux.h>
#include <cstdio>
#include <cstring>
#include <fstream>
#include <vector>

int main() {
    const char *source = "# Hello\n\nSome *text* and ![fat cat 1](examples/assets/cat-1.webp) ![fat cat 2](examples/assets/cat-2.webp) ![fat cat 3](examples/assets/cat-3.webp).\n";
    mdlux_options options{};
    options.width_cells = 80;
    options.theme_name = "dark";
    options.source_dir = "./examples/assets";
    options.ansi = 1;
    options.highlight = 1;

    mdlux_output first{};
    if (mdlux_render((const uint8_t *)source, std::strlen(source), &options, nullptr, 0, &first) != MDLUX_OK) {
        std::fputs("mdlux_render failed\n", stderr);
        return MDLUX_ERROR_RENDER;
    }
    std::fprintf(stdout, "requests: %zu\n", first.request_count);
    mdlux_output_free(&first);

    std::vector<std::string> sources{
        "examples/assets/cat-1.webp",
        "examples/assets/cat-2.webp",
        "examples/assets/cat-3.webp",
    };

    std::vector<std::vector<uint8_t>> bytes;
    bytes.reserve(sources.size());
    for (const auto &source : sources) {
        std::ifstream stream(source, std::ios::binary);
        bytes.emplace_back((std::istreambuf_iterator<char>(stream)), std::istreambuf_iterator<char>());
    }
    std::vector<mdlux_media_resolution> media(sources.size());
    for (std::size_t index = 0; index < sources.size(); ++index) {
        auto &entry = media[index];
        entry.source = sources[index].c_str();
        entry.source_len = sources[index].size();
        entry.bytes = bytes[index].data();
        entry.bytes_len = bytes[index].size();
        entry.width_cells = 4;
        entry.height_cells = 0;
        entry.display_mode = 0;
    }
    options.kitty_graphics = 1;
    options.embed_media = 1;
    options.cell_width_px = 8;
    options.cell_height_px = 16;
    options.max_media_width_cells = 4;
    options.max_media_height_cells = 4;

    mdlux_output second{};
    if (mdlux_render((const uint8_t *)source, std::strlen(source), &options, media.data(), media.size(), &second) != MDLUX_OK) {
        std::fputs("mdlux_render failed\n", stderr);
        return MDLUX_ERROR_RENDER;
    }
    std::fprintf(stderr, "slots: %zu / requests: %zu\n", second.slot_count, second.request_count);
    std::fwrite(second.bytes, 1, second.bytes_len, stdout);
    mdlux_output_free(&second);
    return 0;
}
