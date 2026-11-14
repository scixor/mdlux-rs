#ifndef MDLUX_H
#define MDLUX_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

enum mdlux_color_kind {
  MDLUX_COLOR_NONE = 0,
  MDLUX_COLOR_INDEXED = 1,
  MDLUX_COLOR_RGB = 2
};

enum mdlux_media_kind {
  MDLUX_MEDIA_IMAGE = 0,
  MDLUX_MEDIA_VIDEO = 1,
  MDLUX_MEDIA_AUDIO = 2,
  MDLUX_MEDIA_UNKNOWN = 3
};

enum mdlux_status {
  MDLUX_OK = 0,
  MDLUX_ERROR_ARGUMENT = 1,
  MDLUX_ERROR_INVALID_UTF8 = 2,
  MDLUX_ERROR_OPTIONS = 3,
  MDLUX_ERROR_MEDIA = 4,
  MDLUX_ERROR_RENDER = 5,
  MDLUX_ERROR_ALLOCATION = 6,
  MDLUX_ERROR_PANIC = 7
};

typedef struct mdlux_color {
  uint32_t kind;
  uint8_t index;
  uint8_t rgb[3];
} mdlux_color;

typedef struct mdlux_style {
  mdlux_color foreground;
  mdlux_color background;
  uint8_t bold;
  uint8_t italic;
  uint8_t underline;
  uint8_t dim;
} mdlux_style;

typedef struct mdlux_theme {
  const char *name;
  mdlux_style heading1;
  mdlux_style heading2;
  mdlux_style heading3;
  mdlux_style heading4;
  mdlux_style text;
  mdlux_style emphasis;
  mdlux_style strong;
  mdlux_style inline_code;
  mdlux_style code;
  mdlux_style quote_marker;
  mdlux_style quote_text;
  mdlux_style link_label;
  mdlux_style link_url;
  mdlux_style rule;
  mdlux_style table_header;
  mdlux_style table_border;
  mdlux_style list_marker;
} mdlux_theme;

typedef struct mdlux_options {
  uint32_t width_cells;
  const char *theme_name;
  const mdlux_theme *custom_theme;
  const char *code_theme_name;
  const char *source_dir;
  uint8_t ansi;
  uint8_t highlight;
  uint8_t kitty_text_size;
  uint8_t kitty_graphics;
  uint8_t kitty_hyperlinks;
  uint8_t embed_media;
  uint32_t cell_width_px;
  uint32_t cell_height_px;
  uint32_t max_media_width_cells;
  uint32_t max_media_height_cells;
} mdlux_options;

typedef struct mdlux_media_resolution {
  const char *source;
  size_t source_len;
  const char *path;
  const uint8_t *bytes;
  size_t bytes_len;
  uint32_t width_cells;
  uint32_t height_cells;
} mdlux_media_resolution;

typedef struct mdlux_media_request {
  char *source;
  char *alt;
  char *title;
  int kind;
} mdlux_media_request;

typedef struct mdlux_media_slot {
  char *source;
  char *alt;
  int kind;
  uint32_t line;
  uint32_t column;
  uint32_t width_cells;
  uint32_t height_cells;
} mdlux_media_slot;

typedef struct mdlux_output {
  uint8_t *bytes;
  size_t bytes_len;
  uint32_t line_count;
  mdlux_media_slot *slots;
  size_t slot_count;
  mdlux_media_request *requests;
  size_t request_count;
} mdlux_output;

int mdlux_render(const uint8_t *source,
                 size_t source_len,
                 const mdlux_options *options,
                 const mdlux_media_resolution *media,
                 size_t media_count,
                 mdlux_output *output);

void mdlux_output_free(mdlux_output *output);

int mdlux_theme_count(void);
const char *mdlux_theme_name(int index);
int mdlux_theme_builtin(const char *name, mdlux_theme *output);
uint32_t mdlux_version(void);

#ifdef __cplusplus
}
#endif

#endif
