#include <mdlux.h>
#include <cstdio>
#include <cstring>


/**
 * `mdlux_options` holds width, theme, highlighting, and output settings.
 * `mdlux_output` owns text and media metadata; call `mdlux_output_free` when done.
 */
/* Small end-to-end smoke app for the C ABI.
 * Build with: c++ -std=c++20 examples/cpp/main.cpp -I include \
 *     target/release/libmdlux.a -lm -lpthread -ldl -lunwind \
 *     -o /tmp/mdlux-cpp-smoke
 */
int main() {
  mdlux_theme theme{};
  mdlux_options options{};
  if (mdlux_theme_builtin("dark", &theme) != MDLUX_OK) return MDLUX_ERROR_OPTIONS;
  const char *source = "# Hello\n\nSome *text*.\n";

  // Relative media/link paths resolve from source_dir.
  options.width_cells = 80;
  options.theme_name = "dark";
  options.custom_theme = &theme;
  options.source_dir = ".";
  options.ansi = 1;
  options.highlight = 1;
  // `output.bytes` is UTF-8; free it after printing.
  mdlux_output output{};
  if (mdlux_render((const uint8_t *)source, std::strlen(source), &options, nullptr, 0, &output) != MDLUX_OK) {
    std::fputs("mdlux_render failed\n", stderr);
    return MDLUX_ERROR_RENDER;
  }
  std::fwrite(output.bytes, 1, output.bytes_len, stdout);
  std::putchar('\n');
  mdlux_output_free(&output);
  return 0;
}
