#include <mdlux.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(void) {
    mdlux_theme theme = {0};
    if (mdlux_theme_builtin("nord", &theme) != MDLUX_OK) {
        return MDLUX_ERROR_OPTIONS;
    }
    theme.heading1.foreground.kind = MDLUX_COLOR_RGB;
    theme.heading1.foreground.rgb[0] = 0;
    theme.heading1.foreground.rgb[1] = 255;
    theme.heading1.foreground.rgb[2] = 0;
    theme.heading1.bold = 1;

    const char *source =
        "# Hello\n\nA [link](https://example.com) and ![image](missing.png).\n";
    mdlux_options options = {0};
    options.width_cells = 80;
    options.custom_theme = &theme;
    options.theme_name = "nord";
    options.source_dir = ".";
    options.ansi = 1;
    options.highlight = 1;

    mdlux_output output = {0};
    int status = mdlux_render((const uint8_t *)source, strlen(source), &options, NULL, 0, &output);
    if (status != MDLUX_OK) {
        fprintf(stderr, "mdlux_render failed: %d\n", status);
        return status;
    }

    fwrite(output.bytes, 1, output.bytes_len, stdout);
    printf("\nrequests=%zu slots=%zu lines=%u\n", output.request_count, output.slot_count, output.line_count);
    mdlux_output_free(&output);
    return 0;
}
