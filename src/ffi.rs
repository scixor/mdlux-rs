#![allow(clippy::missing_safety_doc, unsafe_op_in_unsafe_fn)]

use std::alloc::{Layout, dealloc};
use std::borrow::Cow;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_uint};
use std::path::PathBuf;
use std::ptr;

use crate::render::{Capabilities, MediaKind, MediaResolution, RenderOptions, RenderOutput};
use crate::theme::{Color, Style, Theme};

pub const MDLUX_OK: c_int = 0;
pub const MDLUX_ERROR_ARGUMENT: c_int = 1;
pub const MDLUX_ERROR_INVALID_UTF8: c_int = 2;
pub const MDLUX_ERROR_OPTIONS: c_int = 3;
pub const MDLUX_ERROR_MEDIA: c_int = 4;
pub const MDLUX_ERROR_RENDER: c_int = 5;
pub const MDLUX_ERROR_ALLOCATION: c_int = 6;
pub const MDLUX_ERROR_PANIC: c_int = 7;

pub const MDLUX_COLOR_NONE: c_uint = 0;
pub const MDLUX_COLOR_INDEXED: c_uint = 1;
pub const MDLUX_COLOR_RGB: c_uint = 2;

pub const MDLUX_MEDIA_IMAGE: c_int = 0;
pub const MDLUX_MEDIA_VIDEO: c_int = 1;
pub const MDLUX_MEDIA_AUDIO: c_int = 2;
pub const MDLUX_MEDIA_UNKNOWN: c_int = 3;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct mdlux_color {
    pub kind: c_uint,
    pub index: u8,
    pub rgb: [u8; 3],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct mdlux_style {
    pub foreground: mdlux_color,
    pub background: mdlux_color,
    pub bold: u8,
    pub italic: u8,
    pub underline: u8,
    pub dim: u8,
}

#[repr(C)]
pub struct mdlux_theme {
    pub name: *const c_char,
    pub heading1: mdlux_style,
    pub heading2: mdlux_style,
    pub heading3: mdlux_style,
    pub heading4: mdlux_style,
    pub text: mdlux_style,
    pub emphasis: mdlux_style,
    pub strong: mdlux_style,
    pub inline_code: mdlux_style,
    pub code: mdlux_style,
    pub quote_marker: mdlux_style,
    pub quote_text: mdlux_style,
    pub link_label: mdlux_style,
    pub link_url: mdlux_style,
    pub rule: mdlux_style,
    pub table_header: mdlux_style,
    pub table_border: mdlux_style,
    pub list_marker: mdlux_style,
}

#[repr(C)]
pub struct mdlux_options {
    pub width_cells: c_uint,
    pub theme_name: *const c_char,
    pub custom_theme: *const mdlux_theme,
    pub code_theme_name: *const c_char,
    pub source_dir: *const c_char,
    pub ansi: u8,
    pub highlight: u8,
    pub kitty_text_size: u8,
    pub kitty_graphics: u8,
    pub kitty_hyperlinks: u8,
    pub embed_media: u8,
    pub cell_width_px: c_uint,
    pub cell_height_px: c_uint,
    pub max_media_width_cells: c_uint,
    pub max_media_height_cells: c_uint,
}

#[repr(C)]
pub struct mdlux_media_resolution {
    pub source: *const c_char,
    pub source_len: usize,
    pub path: *const c_char,
    pub bytes: *const u8,
    pub bytes_len: usize,
    pub width_cells: c_uint,
    pub height_cells: c_uint,
    pub display_mode: u8,
}

#[repr(C)]
pub struct mdlux_media_request {
    pub source: *mut c_char,
    pub alt: *mut c_char,
    pub title: *mut c_char,
    pub kind: c_int,
}

#[repr(C)]
pub struct mdlux_media_slot {
    pub source: *mut c_char,
    pub alt: *mut c_char,
    pub kind: c_int,
    pub line: c_uint,
    pub column: c_uint,
    pub width_cells: c_uint,
    pub height_cells: c_uint,
}

#[repr(C)]
pub struct mdlux_output {
    pub bytes: *mut u8,
    pub bytes_len: usize,
    pub line_count: c_uint,
    pub slots: *mut mdlux_media_slot,
    pub slot_count: usize,
    pub requests: *mut mdlux_media_request,
    pub request_count: usize,
}

static THEME_NAMES: [&CStr; 6] = [c"ansi", c"dark", c"light", c"nord", c"gruvbox", c"kanagawa"];

fn optional_cstr(pointer: *const c_char) -> Option<String> {
    if pointer.is_null() {
        return None;
    }
    // SAFETY: C ABI callers must pass valid NUL-terminated UTF-8 strings for this field.
    Some(
        unsafe { CStr::from_ptr(pointer) }
            .to_string_lossy()
            .into_owned(),
    )
}

fn required_cstr(pointer: *const c_char) -> Result<String, c_int> {
    optional_cstr(pointer).ok_or(MDLUX_ERROR_OPTIONS)
}

fn style_from_c(style: &mdlux_style) -> Result<Style, c_int> {
    Ok(Style {
        fg: color_from_c(&style.foreground)?,
        bg: color_from_c(&style.background)?,
        bold: style.bold != 0,
        italic: style.italic != 0,
        underline: style.underline != 0,
        dim: style.dim != 0,
    })
}

fn color_from_c(color: &mdlux_color) -> Result<Option<Color>, c_int> {
    match color.kind {
        MDLUX_COLOR_NONE => Ok(None),
        MDLUX_COLOR_INDEXED => Ok(Some(Color::Ansi(color.index))),
        MDLUX_COLOR_RGB => Ok(Some(Color::Rgb(color.rgb[0], color.rgb[1], color.rgb[2]))),
        _ => Err(MDLUX_ERROR_OPTIONS),
    }
}

fn style_to_c(style: &Style) -> mdlux_style {
    fn color(color: Option<Color>) -> mdlux_color {
        match color {
            Some(Color::Ansi(index)) => mdlux_color {
                kind: MDLUX_COLOR_INDEXED,
                index,
                rgb: [0, 0, 0],
            },
            Some(Color::Rgb(red, green, blue)) => mdlux_color {
                kind: MDLUX_COLOR_RGB,
                index: 0,
                rgb: [red, green, blue],
            },
            None => mdlux_color {
                kind: MDLUX_COLOR_NONE,
                index: 0,
                rgb: [0, 0, 0],
            },
        }
    }

    mdlux_style {
        foreground: color(style.fg),
        background: color(style.bg),
        bold: style.bold as u8,
        italic: style.italic as u8,
        underline: style.underline as u8,
        dim: style.dim as u8,
    }
}

fn theme_to_c(theme: &Theme, name: &str) -> mdlux_theme {
    let style = style_to_c;
    mdlux_theme {
        name: THEME_NAMES
            .iter()
            .find(|candidate| candidate.to_string_lossy() == name)
            .map(|candidate| candidate.as_ptr())
            .unwrap_or(c"custom".as_ptr()),
        heading1: style(&theme.heading1),
        heading2: style(&theme.heading2),
        heading3: style(&theme.heading3),
        heading4: style(&theme.heading4),
        text: style(&theme.text),
        emphasis: style(&theme.emphasis),
        strong: style(&theme.strong),
        inline_code: style(&theme.inline_code),
        code: style(&theme.code),
        quote_marker: style(&theme.quote_marker),
        quote_text: style(&theme.quote_text),
        link_label: style(&theme.link_label),
        link_url: style(&theme.link_url),
        rule: style(&theme.rule),
        table_header: style(&theme.table_header),
        table_border: style(&theme.table_border),
        list_marker: style(&theme.list_marker),
    }
}

fn theme_from_c(theme: &mdlux_theme) -> Result<Theme, c_int> {
    let name = optional_cstr(theme.name).unwrap_or_else(|| "custom".to_string());
    Ok(Theme {
        name: Cow::Owned(name),
        heading1: style_from_c(&theme.heading1)?,
        heading2: style_from_c(&theme.heading2)?,
        heading3: style_from_c(&theme.heading3)?,
        heading4: style_from_c(&theme.heading4)?,
        text: style_from_c(&theme.text)?,
        emphasis: style_from_c(&theme.emphasis)?,
        strong: style_from_c(&theme.strong)?,
        inline_code: style_from_c(&theme.inline_code)?,
        code: style_from_c(&theme.code)?,
        quote_marker: style_from_c(&theme.quote_marker)?,
        quote_text: style_from_c(&theme.quote_text)?,
        link_label: style_from_c(&theme.link_label)?,
        link_url: style_from_c(&theme.link_url)?,
        rule: style_from_c(&theme.rule)?,
        table_header: style_from_c(&theme.table_header)?,
        table_border: style_from_c(&theme.table_border)?,
        list_marker: style_from_c(&theme.list_marker)?,
    })
}

unsafe fn media_from_c(
    media: *const mdlux_media_resolution,
    count: usize,
) -> Result<Vec<MediaResolution>, c_int> {
    if count == 0 {
        return Ok(Vec::new());
    }
    if media.is_null() {
        return Err(MDLUX_ERROR_ARGUMENT);
    }
    let mut result = Vec::with_capacity(count);
    for index in 0..count {
        let item = &*media.add(index);
        if item.source.is_null() || item.source_len == 0 {
            return Err(MDLUX_ERROR_ARGUMENT);
        }
        let source_bytes = std::slice::from_raw_parts(item.source.cast::<u8>(), item.source_len);
        let source = std::str::from_utf8(source_bytes)
            .map_err(|_| MDLUX_ERROR_INVALID_UTF8)?
            .to_string();
        let display_mode = match item.display_mode {
            1 => crate::render::media::MediaDisplayMode::Inline,
            2 => crate::render::media::MediaDisplayMode::Block,
            _ => crate::render::media::MediaDisplayMode::Auto,
        };
        result.push(MediaResolution {
            source,
            path: optional_cstr(item.path).map(PathBuf::from),
            bytes: if item.bytes.is_null() || item.bytes_len == 0 {
                None
            } else {
                Some(std::slice::from_raw_parts(item.bytes, item.bytes_len).to_vec())
            },
            width_cells: (item.width_cells != 0).then_some(item.width_cells),
            height_cells: (item.height_cells != 0).then_some(item.height_cells),
            display_mode,
        });
    }
    Ok(result)
}

fn kind_to_c(kind: MediaKind) -> c_int {
    match kind {
        MediaKind::Image => MDLUX_MEDIA_IMAGE,
        MediaKind::Video => MDLUX_MEDIA_VIDEO,
        MediaKind::Audio => MDLUX_MEDIA_AUDIO,
        MediaKind::Unknown => MDLUX_MEDIA_UNKNOWN,
    }
}

fn string_to_c(value: &str) -> Result<*mut c_char, c_int> {
    CString::new(value.replace('\0', ""))
        .map(|value| value.into_raw())
        .map_err(|_| MDLUX_ERROR_ALLOCATION)
}

fn allocate_bytes(bytes: String) -> *mut u8 {
    if bytes.is_empty() {
        ptr::null_mut()
    } else {
        Box::<[u8]>::into_raw(bytes.into_bytes().into_boxed_slice()).cast()
    }
}

unsafe fn deallocate_bytes(pointer: *mut u8, len: usize) {
    if pointer.is_null() || len == 0 {
        return;
    }
    let layout = Layout::from_size_align(len, 1).expect("valid byte allocation");
    dealloc(pointer.cast(), layout);
}

fn render_inner(
    source: &[u8],
    options: *const mdlux_options,
    media: *const mdlux_media_resolution,
    media_count: usize,
) -> Result<RenderOutput, c_int> {
    let source = std::str::from_utf8(source).map_err(|_| MDLUX_ERROR_INVALID_UTF8)?;
    let options = unsafe { options.as_ref() }.ok_or(MDLUX_ERROR_ARGUMENT)?;
    let media = unsafe { media_from_c(media, media_count) }?;

    let custom_theme = if options.custom_theme.is_null() {
        None
    } else {
        Some(unsafe { theme_from_c(&*options.custom_theme) }?)
    };
    let theme = match custom_theme {
        Some(theme) => theme,
        None => {
            let name = required_cstr(options.theme_name)?;
            crate::theme::builtin::find_theme(&name)
                .cloned()
                .ok_or(MDLUX_ERROR_OPTIONS)?
        }
    };
    let source_dir =
        PathBuf::from(optional_cstr(options.source_dir).unwrap_or_else(|| ".".to_string()));

    let render_options = RenderOptions {
        width: options.width_cells.max(20) as usize,
        theme,
        capabilities: Capabilities {
            ansi: options.ansi != 0,
            kitty_text_size: options.kitty_text_size != 0,
            kitty_graphics: options.kitty_graphics != 0,
            kitty_hyperlinks: options.kitty_hyperlinks != 0,
        },
        source_dir,
        no_highlight: options.highlight == 0,
        code_theme_name: optional_cstr(options.code_theme_name),
        embed_media: options.embed_media != 0,
        cell_width_px: (options.cell_width_px != 0).then_some(options.cell_width_px),
        cell_height_px: (options.cell_height_px != 0).then_some(options.cell_height_px),
        max_media_width_cells: (options.max_media_width_cells != 0)
            .then_some(options.max_media_width_cells),
        max_media_height_cells: (options.max_media_height_cells != 0)
            .then_some(options.max_media_height_cells),
        media,
    };
    crate::render::render_markdown(source, &render_options).map_err(|_| MDLUX_ERROR_MEDIA)
}

fn output_to_c(output: RenderOutput) -> Result<mdlux_output, c_int> {
    let text_len = output.text.len();
    let bytes = allocate_bytes(output.text);
    if text_len > 0 && bytes.is_null() {
        return Err(MDLUX_ERROR_ALLOCATION);
    }

    let mut slots = Vec::new();
    let mut requests = Vec::new();
    for slot in output.media_slots {
        let source = string_to_c(&slot.source)?;
        let alt = string_to_c(&slot.alt)?;
        slots.push(mdlux_media_slot {
            source,
            alt,
            kind: kind_to_c(slot.kind),
            line: slot.line,
            column: slot.column,
            width_cells: slot.width_cells,
            height_cells: slot.height_cells,
        });
    }
    for request in output.media_requests {
        let source = string_to_c(&request.source)?;
        let alt = string_to_c(&request.alt)?;
        let title = string_to_c(request.title.as_deref().unwrap_or(""))?;
        requests.push(mdlux_media_request {
            source,
            alt,
            title,
            kind: kind_to_c(request.kind),
        });
    }

    if bytes.is_null() && text_len > 0 {
        // Unreachable because allocate_bytes above guards it, but keeps cleanup simple.
        return Err(MDLUX_ERROR_ALLOCATION);
    }

    let slot_count = slots.len();
    let request_count = requests.len();
    Ok(mdlux_output {
        bytes,
        bytes_len: text_len,
        line_count: output.line_count as c_uint,
        slots: if slots.is_empty() {
            ptr::null_mut()
        } else {
            Box::<[mdlux_media_slot]>::into_raw(slots.into_boxed_slice()).cast()
        },
        slot_count,
        requests: if requests.is_empty() {
            ptr::null_mut()
        } else {
            Box::<[mdlux_media_request]>::into_raw(requests.into_boxed_slice()).cast()
        },
        request_count,
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mdlux_render(
    source: *const u8,
    source_len: usize,
    options: *const mdlux_options,
    media: *const mdlux_media_resolution,
    media_count: usize,
    output: *mut mdlux_output,
) -> c_int {
    if source.is_null() || options.is_null() || output.is_null() {
        return MDLUX_ERROR_ARGUMENT;
    }
    let source = std::slice::from_raw_parts(source, source_len);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        render_inner(source, options, media, media_count)
    }));
    let rendered = match result {
        Ok(Ok(value)) => value,
        Ok(Err(status)) => return status,
        Err(_) => return MDLUX_ERROR_PANIC,
    };
    match output_to_c(rendered) {
        Ok(value) => {
            *output = value;
            MDLUX_OK
        }
        Err(status) => status,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mdlux_output_free(output: *mut mdlux_output) {
    if output.is_null() {
        return;
    }
    let output = ptr::read(output);
    deallocate_bytes(output.bytes, output.bytes_len);
    if !output.slots.is_null() {
        let slots = Vec::from_raw_parts(output.slots, output.slot_count, output.slot_count);
        for slot in slots {
            if !slot.source.is_null() {
                drop(CString::from_raw(slot.source));
            }
            if !slot.alt.is_null() {
                drop(CString::from_raw(slot.alt));
            }
        }
    }
    if !output.requests.is_null() {
        let requests =
            Vec::from_raw_parts(output.requests, output.request_count, output.request_count);
        for request in requests {
            if !request.source.is_null() {
                drop(CString::from_raw(request.source));
            }
            if !request.alt.is_null() {
                drop(CString::from_raw(request.alt));
            }
            if !request.title.is_null() {
                drop(CString::from_raw(request.title));
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn mdlux_theme_count() -> c_int {
    THEME_NAMES.len() as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn mdlux_theme_name(index: c_int) -> *const c_char {
    THEME_NAMES
        .get(index as usize)
        .map(|name| name.as_ptr())
        .unwrap_or(ptr::null())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mdlux_theme_builtin(
    name: *const c_char,
    output: *mut mdlux_theme,
) -> c_int {
    if name.is_null() || output.is_null() {
        return MDLUX_ERROR_ARGUMENT;
    }
    let theme_name = match CStr::from_ptr(name).to_str() {
        Ok(value) => value.to_string(),
        Err(_) => return MDLUX_ERROR_INVALID_UTF8,
    };
    let Some(theme) = crate::theme::builtin::find_theme(&theme_name) else {
        return MDLUX_ERROR_OPTIONS;
    };
    *output = theme_to_c(theme, &theme_name);
    MDLUX_OK
}

#[unsafe(no_mangle)]
pub extern "C" fn mdlux_version() -> c_uint {
    let major: u32 = env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap_or(0);
    let minor: u32 = env!("CARGO_PKG_VERSION_MINOR").parse().unwrap_or(0);
    let patch: u32 = env!("CARGO_PKG_VERSION_PATCH").parse().unwrap_or(0);
    (major << 16) | (minor << 8) | patch
}

pub mod c_status {
    use super::{
        MDLUX_ERROR_ALLOCATION, MDLUX_ERROR_ARGUMENT, MDLUX_ERROR_INVALID_UTF8, MDLUX_ERROR_MEDIA,
        MDLUX_ERROR_OPTIONS, MDLUX_ERROR_PANIC, MDLUX_ERROR_RENDER, MDLUX_OK,
    };
    use std::os::raw::c_int;
    pub const OK: c_int = MDLUX_OK;
    pub const ARGUMENT: c_int = MDLUX_ERROR_ARGUMENT;
    pub const INVALID_UTF8: c_int = MDLUX_ERROR_INVALID_UTF8;
    pub const OPTIONS: c_int = MDLUX_ERROR_OPTIONS;
    pub const MEDIA: c_int = MDLUX_ERROR_MEDIA;
    pub const RENDER: c_int = MDLUX_ERROR_RENDER;
    pub const ALLOCATION: c_int = MDLUX_ERROR_ALLOCATION;
    pub const PANIC: c_int = MDLUX_ERROR_PANIC;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_via_c_abi_with_custom_theme() {
        let mut theme: mdlux_theme = unsafe { std::mem::zeroed() };
        let theme_name = c"nord";
        let status = unsafe { mdlux_theme_builtin(theme_name.as_ptr(), &mut theme) };
        assert_eq!(status, MDLUX_OK);
        assert_eq!(theme.heading1.foreground.kind, MDLUX_COLOR_RGB);
        theme.heading1.foreground.rgb = [0, 255, 0];
        theme.heading1.bold = 1;

        let source = b"# Custom\n\nhello\n";
        let source_dir = c".";
        let options = mdlux_options {
            width_cells: 80,
            theme_name: theme_name.as_ptr(),
            custom_theme: &theme,
            code_theme_name: ptr::null(),
            source_dir: source_dir.as_ptr(),
            ansi: 1,
            highlight: 0,
            kitty_text_size: 0,
            kitty_graphics: 0,
            kitty_hyperlinks: 0,
            embed_media: 0,
            cell_width_px: 0,
            cell_height_px: 0,
            max_media_width_cells: 0,
            max_media_height_cells: 0,
        };
        let mut output: mdlux_output = unsafe { std::mem::zeroed() };
        let status = unsafe {
            mdlux_render(
                source.as_ptr(),
                source.len(),
                &options,
                ptr::null(),
                0,
                &mut output,
            )
        };
        assert_eq!(status, MDLUX_OK);
        let rendered = String::from_utf8(unsafe {
            std::slice::from_raw_parts(output.bytes, output.bytes_len).to_vec()
        })
        .expect("UTF-8 output");
        assert!(rendered.contains("Custom"));
        unsafe { mdlux_output_free(&mut output) };
    }

    #[test]
    fn animated_gif_flows_through_c_abi() {
        let source = b"![Neko](x.gif)";
        let media_source = c"x.gif";
        let gif = include_bytes!("../examples/assets/neko.gif");
        let media = mdlux_media_resolution {
            source: media_source.as_ptr(),
            source_len: 5,
            path: ptr::null(),
            bytes: gif.as_ptr(),
            bytes_len: gif.len(),
            width_cells: 0,
            height_cells: 0,
            display_mode: 0,
        };
        let mut options: mdlux_options = unsafe { std::mem::zeroed() };
        options.width_cells = 80;
        options.theme_name = c"nord".as_ptr();
        options.source_dir = c".".as_ptr();
        options.kitty_graphics = 1;
        options.embed_media = 1;
        options.cell_width_px = 8;
        options.cell_height_px = 16;

        let mut output: mdlux_output = unsafe { std::mem::zeroed() };
        let status = unsafe {
            mdlux_render(
                source.as_ptr(),
                source.len(),
                &options,
                &media,
                1,
                &mut output,
            )
        };
        assert_eq!(status, MDLUX_OK);
        let rendered = String::from_utf8_lossy(unsafe {
            std::slice::from_raw_parts(output.bytes, output.bytes_len)
        });
        assert!(rendered.contains("a=f"));
        assert!(rendered.contains("X=1"));
        assert!(rendered.contains("s=3,v=1"));
        unsafe { mdlux_output_free(&mut output) };
    }

    #[test]
    fn rejects_unknown_builtin_theme_name() {
        let mut theme: mdlux_theme = unsafe { std::mem::zeroed() };
        let status = unsafe { mdlux_theme_builtin(c"missing".as_ptr(), &mut theme) };
        assert_eq!(status, MDLUX_ERROR_OPTIONS);
    }
}
