#include <mdlux.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>


/**
 * The C ABI is UTF-8-in, UTF-8-out.
 * `mdlux_options` controls width, theme, highlighting, and the output format.
 * `mdlux_output` owns bytes plus media metadata; free it with `mdlux_output_free`.
 */
/* Small end-to-end smoke app for the C ABI.
 * Build with: cc -std=c17 examples/capi.c -I include target/release/libmdlux.a \
 *     -lm -lpthread -ldl -lunwind -o /tmp/mdlux-c-smoke
 */
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
    /* Use an existing theme or set null custom_theme and pass a theme_name. */
    mdlux_options options = {0};
    /* Relative media/link paths resolve from source_dir. */
    options.width_cells = 80;
    options.custom_theme = &theme;
    options.theme_name = "nord";
    options.source_dir = ".";
    options.ansi = 1;
    options.highlight = 1;

    /* Render and keep ownership of text/slots/requests until freed. */
    mdlux_output output = {0};
    int status = mdlux_render((const uint8_t *)source, strlen(source), &options, NULL, 0, &output);
    if (status != MDLUX_OK) {
        fprintf(stderr, "mdlux_render failed: 0\n", status);
        return status;
    }

    /* `bytes` is UTF-8 text; print as one block. */
    fwrite(output.bytes, 1, output.bytes_len, stdout);
    printf("\nrequests=0 slots=0 lines=0\n", output.request_count, output.slot_count, output.line_count);
    mdlux_output_free(&output);
    return 0;
}
