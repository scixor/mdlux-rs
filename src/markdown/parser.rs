use pulldown_cmark::{
    Alignment as MdAlignment, CodeBlockKind, CowStr, Event, HeadingLevel, Options, Parser, Tag,
    TagEnd,
};

use crate::markdown::ast::{Alignment, Block, Inline};

use html_to_markdown_rs::convert as convert_html;

pub fn parse_markdown(input: &str) -> Vec<Block> {
    let input = preprocess_html(input);
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_GFM;

    let parser = Parser::new_ext(&input, options);
    let events = parser.collect::<Vec<_>>();
    let mut p = EventParser { events, idx: 0 };
    p.parse_blocks_until(BlockEnd::Never)
}

struct EventParser<'a> {
    events: Vec<Event<'a>>,
    idx: usize,
}

#[derive(Clone, Copy)]
enum BlockEnd {
    Never,
    BlockQuote,
    Item,
    Table,
    FootnoteDefinition,
}

#[derive(Clone, Copy)]
enum InlineEnd {
    Paragraph,
    Heading,
    Emphasis,
    Strong,
    Strikethrough,
    Link,
    Image,
    TableCell,
}

fn push_paragraph_or_image(inlines: Vec<Inline>, blocks: &mut Vec<Block>) {
    if inlines.is_empty() {
        return;
    }
    match image_only_paragraph(&inlines) {
        Some(image) => blocks.push(image),
        None => blocks.push(Block::Paragraph(inlines)),
    }
}

impl<'a> EventParser<'a> {
    fn parse_blocks_until(&mut self, end: BlockEnd) -> Vec<Block> {
        let mut blocks = Vec::new();
        while let Some(ev) = self.events.get(self.idx).cloned() {
            if is_block_end(&ev, end) {
                self.idx += 1;
                break;
            }
            match &ev {
                Event::Rule => {
                    blocks.push(Block::Rule);
                    self.idx += 1;
                }
                Event::Text(_)
                | Event::Code(_)
                | Event::SoftBreak
                | Event::HardBreak
                | Event::TaskListMarker(_)
                | Event::FootnoteReference(_)
                | Event::Start(Tag::Emphasis)
                | Event::Start(Tag::Strong)
                | Event::Start(Tag::Strikethrough)
                | Event::Start(Tag::Link { .. })
                | Event::Start(Tag::Image { .. }) => {
                    push_paragraph_or_image(self.parse_tight_inlines(end), &mut blocks);
                }
                Event::Start(tag) => match tag {
                    Tag::Heading { level, .. } => {
                        self.idx += 1;
                        let inlines = self.parse_inlines_until(InlineEnd::Heading);
                        blocks.push(Block::Heading {
                            level: heading_to_u8(*level),
                            content: inlines,
                        });
                    }
                    Tag::Paragraph => {
                        self.idx += 1;
                        let inlines = self.parse_inlines_until(InlineEnd::Paragraph);
                        push_paragraph_or_image(inlines, &mut blocks);
                    }
                    Tag::CodeBlock(kind) => {
                        let lang = extract_lang(kind);
                        self.idx += 1;
                        let code = self.collect_code_text();
                        blocks.push(Block::CodeBlock { lang, code });
                    }
                    Tag::BlockQuote(kind) => {
                        self.idx += 1;
                        let mut inner = self.parse_blocks_until(BlockEnd::BlockQuote);
                        if let Some(label) = blockquote_kind_label(kind) {
                            inner.insert(
                                0,
                                Block::Paragraph(vec![Inline::Strong(vec![Inline::Text(
                                    label.to_string(),
                                )])]),
                            );
                        }
                        blocks.push(Block::BlockQuote(inner));
                    }
                    Tag::List(start) => {
                        self.idx += 1;
                        blocks.push(self.parse_list(*start));
                    }
                    Tag::Table(aligns) => {
                        self.idx += 1;
                        blocks.push(self.parse_table(aligns.clone(), BlockEnd::Table));
                    }
                    Tag::FootnoteDefinition(name) => {
                        self.idx += 1;
                        let content = self.parse_blocks_until(BlockEnd::FootnoteDefinition);
                        blocks.push(Block::FootnoteDefinition {
                            name: name.to_string(),
                            content,
                        });
                    }
                    _ => {
                        self.idx += 1;
                    }
                },
                _ => {
                    self.idx += 1;
                }
            }
        }
        blocks
    }

    fn parse_list(&mut self, start: Option<u64>) -> Block {
        let mut items = Vec::new();
        while let Some(ev) = self.events.get(self.idx).cloned() {
            match &ev {
                Event::End(TagEnd::List(_)) => {
                    self.idx += 1;
                    break;
                }
                Event::Start(Tag::Item) => {
                    self.idx += 1;
                    let item_blocks = self.parse_blocks_until(BlockEnd::Item);
                    items.push(item_blocks);
                }
                _ => {
                    self.idx += 1;
                }
            }
        }
        Block::List {
            ordered: start.is_some(),
            start,
            items,
        }
    }

    fn parse_table(&mut self, aligns: Vec<MdAlignment>, end: BlockEnd) -> Block {
        let mut headers: Vec<Vec<Inline>> = Vec::new();
        let mut rows: Vec<Vec<Vec<Inline>>> = Vec::new();
        let mut current_row: Vec<Vec<Inline>> = Vec::new();
        let mut in_head = false;
        while let Some(ev) = self.events.get(self.idx).cloned() {
            if is_block_end(&ev, end) {
                self.idx += 1;
                break;
            }
            match &ev {
                Event::Start(Tag::TableHead) => {
                    in_head = true;
                    headers.clear();
                    current_row.clear();
                    self.idx += 1;
                }
                Event::End(TagEnd::TableHead) => {
                    in_head = false;
                    self.idx += 1;
                }
                Event::Start(Tag::TableRow) => {
                    current_row.clear();
                    self.idx += 1;
                }
                Event::End(TagEnd::TableRow) => {
                    self.idx += 1;
                    if !current_row.is_empty() {
                        rows.push(std::mem::take(&mut current_row));
                    }
                }
                Event::Start(Tag::TableCell) => {
                    self.idx += 1;
                    let inlines = self.parse_inlines_until(InlineEnd::TableCell);
                    if in_head {
                        headers.push(inlines);
                    } else {
                        current_row.push(inlines);
                    }
                }
                _ => {
                    self.idx += 1;
                }
            }
        }

        if headers.is_empty() && !rows.is_empty() {
            headers = rows.remove(0);
        }

        Block::Table {
            headers,
            rows,
            aligns: aligns.into_iter().map(map_align).collect(),
        }
    }

    fn parse_inlines_until(&mut self, end: InlineEnd) -> Vec<Inline> {
        let mut out = Vec::new();
        while let Some(ev) = self.events.get(self.idx).cloned() {
            if is_inline_end(&ev, end) {
                self.idx += 1;
                break;
            }
            if !self.consume_inline_event(&ev, &mut out) {
                self.idx += 1;
            }
        }
        out
    }

    fn parse_tight_inlines(&mut self, block_end: BlockEnd) -> Vec<Inline> {
        let mut out = Vec::new();
        while let Some(ev) = self.events.get(self.idx).cloned() {
            if is_block_end(&ev, block_end) || is_block_boundary(&ev) {
                break;
            }
            if !self.consume_inline_event(&ev, &mut out) {
                self.idx += 1;
            }
        }
        out
    }

    fn consume_inline_event(&mut self, ev: &Event<'a>, out: &mut Vec<Inline>) -> bool {
        match ev {
            Event::Text(text) => {
                out.push(Inline::Text(text.to_string()));
                self.idx += 1;
                true
            }
            Event::Code(text) => {
                out.push(Inline::Code(text.to_string()));
                self.idx += 1;
                true
            }
            Event::SoftBreak => {
                out.push(Inline::SoftBreak);
                self.idx += 1;
                true
            }
            Event::HardBreak => {
                out.push(Inline::HardBreak);
                self.idx += 1;
                true
            }
            Event::TaskListMarker(done) => {
                let marker = if *done { "[x] " } else { "[ ] " };
                out.push(Inline::Text(marker.to_string()));
                self.idx += 1;
                true
            }
            Event::FootnoteReference(name) => {
                out.push(Inline::FootnoteRef(name.to_string()));
                self.idx += 1;
                true
            }
            Event::Start(Tag::Emphasis) => {
                self.idx += 1;
                out.push(Inline::Emph(self.parse_inlines_until(InlineEnd::Emphasis)));
                true
            }
            Event::Start(Tag::Strong) => {
                self.idx += 1;
                out.push(Inline::Strong(self.parse_inlines_until(InlineEnd::Strong)));
                true
            }
            Event::Start(Tag::Strikethrough) => {
                self.idx += 1;
                out.push(Inline::Strike(
                    self.parse_inlines_until(InlineEnd::Strikethrough),
                ));
                true
            }
            Event::Start(Tag::Link {
                dest_url, title, ..
            }) => {
                let dest = dest_url.to_string();
                let title = to_opt(title);
                self.idx += 1;
                let text = self.parse_inlines_until(InlineEnd::Link);
                out.push(Inline::Link { text, dest, title });
                true
            }
            Event::Start(Tag::Image {
                dest_url, title, ..
            }) => {
                let path = dest_url.to_string();
                let title = to_opt(title);
                self.idx += 1;
                let text = self.parse_inlines_until(InlineEnd::Image);
                let alt = Inline::plain_text(&text);
                out.push(Inline::Image { alt, path, title });
                true
            }
            _ => false,
        }
    }

    fn collect_code_text(&mut self) -> String {
        let mut out = String::new();
        while let Some(ev) = self.events.get(self.idx).cloned() {
            if matches!(ev, Event::End(TagEnd::CodeBlock)) {
                self.idx += 1;
                break;
            }
            match &ev {
                Event::Text(text) | Event::Code(text) => out.push_str(text),
                Event::SoftBreak | Event::HardBreak => out.push('\n'),
                _ => {}
            }
            self.idx += 1;
        }
        out
    }
}

fn blockquote_kind_label(kind: &Option<pulldown_cmark::BlockQuoteKind>) -> Option<&'static str> {
    match kind {
        Some(pulldown_cmark::BlockQuoteKind::Note) => Some("Note"),
        Some(pulldown_cmark::BlockQuoteKind::Tip) => Some("Tip"),
        Some(pulldown_cmark::BlockQuoteKind::Important) => Some("Important"),
        Some(pulldown_cmark::BlockQuoteKind::Warning) => Some("Warning"),
        Some(pulldown_cmark::BlockQuoteKind::Caution) => Some("Caution"),
        _ => None,
    }
}

fn is_block_end(ev: &Event<'_>, end: BlockEnd) -> bool {
    match end {
        BlockEnd::Never => false,
        BlockEnd::BlockQuote => matches!(ev, Event::End(TagEnd::BlockQuote(_))),
        BlockEnd::Item => matches!(ev, Event::End(TagEnd::Item)),
        BlockEnd::Table => matches!(ev, Event::End(TagEnd::Table)),
        BlockEnd::FootnoteDefinition => matches!(ev, Event::End(TagEnd::FootnoteDefinition)),
    }
}

fn is_block_boundary(ev: &Event<'_>) -> bool {
    matches!(
        ev,
        Event::Rule
            | Event::Start(Tag::Heading { .. })
            | Event::Start(Tag::Paragraph)
            | Event::Start(Tag::CodeBlock(_))
            | Event::Start(Tag::BlockQuote(_))
            | Event::Start(Tag::List(_))
            | Event::Start(Tag::Table(_))
            | Event::Start(Tag::TableHead)
            | Event::Start(Tag::TableRow)
            | Event::Start(Tag::TableCell)
            | Event::Start(Tag::Item)
            | Event::Start(Tag::FootnoteDefinition(_))
            | Event::End(_)
    )
}

fn is_inline_end(ev: &Event<'_>, end: InlineEnd) -> bool {
    match end {
        InlineEnd::Paragraph => matches!(ev, Event::End(TagEnd::Paragraph)),
        InlineEnd::Heading => matches!(ev, Event::End(TagEnd::Heading(_))),
        InlineEnd::Emphasis => matches!(ev, Event::End(TagEnd::Emphasis)),
        InlineEnd::Strong => matches!(ev, Event::End(TagEnd::Strong)),
        InlineEnd::Strikethrough => matches!(ev, Event::End(TagEnd::Strikethrough)),
        InlineEnd::Link => matches!(ev, Event::End(TagEnd::Link)),
        InlineEnd::Image => matches!(ev, Event::End(TagEnd::Image)),
        InlineEnd::TableCell => matches!(ev, Event::End(TagEnd::TableCell)),
    }
}

fn map_align(input: MdAlignment) -> Alignment {
    match input {
        MdAlignment::Left => Alignment::Left,
        MdAlignment::Center => Alignment::Center,
        MdAlignment::Right => Alignment::Right,
        MdAlignment::None => Alignment::None,
    }
}

fn extract_lang(kind: &CodeBlockKind<'_>) -> Option<String> {
    match kind {
        CodeBlockKind::Fenced(info) => info.split_whitespace().next().map(ToString::to_string),
        CodeBlockKind::Indented => None,
    }
}

fn heading_to_u8(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn to_opt(input: &CowStr<'_>) -> Option<String> {
    (!input.is_empty()).then(|| input.to_string())
}

fn image_only_paragraph(inlines: &[Inline]) -> Option<Block> {
    if inlines.len() != 1 {
        return None;
    }
    let Inline::Image { alt, path, title } = &inlines[0] else {
        return None;
    };
    Some(Block::Image {
        alt: alt.clone(),
        path: path.clone(),
        title: title.clone(),
    })
}

// Tests
fn preprocess_html(input: &str) -> String {
    if input.is_empty() {
        return String::new();
    }
    if !input.contains('<') {
        return input.to_string();
    }

    let lines: Vec<&str> = input.split('\n').collect();
    let mut out = String::new();
    let mut i = 0usize;
    let mut in_code_fence = false;
    let mut fence_char = ' ';
    let mut fence_len = 0usize;

    while i < lines.len() {
        if let Some((ch, len)) = fence_marker(lines[i]) {
            if !in_code_fence {
                in_code_fence = true;
                fence_char = ch;
                fence_len = len;
                out.push_str(lines[i]);
                if i + 1 < lines.len() {
                    out.push('\n');
                }
                i += 1;
                continue;
            } else if ch == fence_char && len >= fence_len {
                let rest = lines[i].trim_start()[len..].trim();
                if rest.is_empty() {
                    in_code_fence = false;
                    out.push_str(lines[i]);
                    if i + 1 < lines.len() {
                        out.push('\n');
                    }
                    i += 1;
                    continue;
                }
            }
        }

        if in_code_fence {
            out.push_str(lines[i]);
            if i + 1 < lines.len() {
                out.push('\n');
            }
            i += 1;
            continue;
        }

        let trimmed = lines[i].trim();
        if looks_like_html_block_start(trimmed) {
            let mut buf = String::new();
            buf.push_str(lines[i]);
            let mut j = i + 1;
            while j < lines.len() {
                if lines[j].trim().is_empty() {
                    break;
                }
                if fence_marker(lines[j]).is_some() {
                    break;
                }
                buf.push('\n');
                buf.push_str(lines[j]);
                j += 1;
            }

            if let Ok(result) = convert_html(&buf, None) {
                let content = result.content.unwrap_or_default();
                out.push_str(content.trim_end_matches('\n'));
                out.push_str("\n\n");
            } else {
                out.push_str(&buf);
                if i + 1 < lines.len() {
                    out.push('\n');
                }
            }
            i = j;
            continue;
        }

        out.push_str(&transform_inline_html(lines[i]));
        if i + 1 < lines.len() {
            out.push('\n');
        }
        i += 1;
    }
    out
}

fn looks_like_html_block_start(s: &str) -> bool {
    let Some(second) = s.chars().nth(1) else {
        return false;
    };
    if !s.starts_with('<') {
        return false;
    }
    if !(second.is_alphabetic() || second == '/' || second == '!' || second == '?') {
        return false;
    }
    find_tag_end(s).is_some()
}

fn fence_marker(line: &str) -> Option<(char, usize)> {
    let trimmed = line.trim_start();
    let marker = trimmed.chars().next()?;
    if marker != '`' && marker != '~' {
        return None;
    }
    let count = trimmed.chars().take_while(|c| *c == marker).count();
    if count >= 3 {
        Some((marker, count))
    } else {
        None
    }
}

fn find_tag_end(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut in_single = false;
    let mut in_double = false;
    for i in 0..s.len() {
        let c = bytes[i] as char;
        if c == '\'' && !in_double {
            in_single = !in_single;
        } else if c == '"' && !in_single {
            in_double = !in_double;
        } else if c == '>' && !in_single && !in_double {
            return Some(i);
        }
    }
    None
}

fn is_html_tag(tag: &str) -> bool {
    if !tag.starts_with('<') || !tag.ends_with('>') {
        return false;
    }
    let body = &tag[1..tag.len() - 1];
    let body = body.trim_start_matches('/');
    let first = body.chars().next().unwrap_or(' ');
    first.is_alphabetic() || first == '!' || first == '?'
}

fn transform_inline_html(line: &str) -> String {
    if !line.contains('<') {
        return line.to_string();
    }
    let mut out = String::new();
    let mut pos = 0usize;
    let bytes = line.as_bytes();
    while pos < line.len() {
        let ch = line[pos..].chars().next().unwrap();
        let ch_len = ch.len_utf8();
        if ch == '`' {
            // preserve code spans verbatim
            let mut run = 0usize;
            while pos + run < line.len() && bytes[pos + run] == b'`' {
                run += 1;
            }
            let mut j = pos + run;
            let mut found = None;
            while j < line.len() {
                if bytes[j] == b'`' {
                    let mut r = 0usize;
                    while j + r < line.len() && bytes[j + r] == b'`' {
                        r += 1;
                    }
                    if r >= run {
                        found = Some((j, r));
                        break;
                    }
                    j += r.max(1);
                } else {
                    j += 1;
                }
            }
            if let Some((end, r)) = found {
                let end_pos = end + r;
                out.push_str(&line[pos..end_pos]);
                pos = end_pos;
            } else {
                out.push(ch);
                pos += ch_len;
            }
        } else if ch == '<' {
            let rest = &line[pos..];
            if let Some(end_rel) = find_tag_end(rest) {
                let tag = &line[pos..pos + end_rel + 1];
                if is_html_tag(tag) {
                    let mut converted = convert_html(tag, None)
                        .ok()
                        .and_then(|r| r.content)
                        .unwrap_or_else(|| tag.to_string());
                    if converted.ends_with('\n') {
                        converted = converted.trim_end_matches('\n').to_string();
                    }
                    if converted.is_empty() {
                        if tag.to_ascii_lowercase().contains("<br") {
                            converted = "  \n".to_string();
                        }
                    }
                    out.push_str(&converted);
                    pos += tag.len();
                } else {
                    out.push(ch);
                    pos += ch_len;
                }
            } else {
                out.push(ch);
                pos += ch_len;
            }
        } else {
            out.push(ch);
            pos += ch_len;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::parse_markdown;
    use crate::markdown::ast::{Alignment, Block, Inline};

    #[test]
    fn parses_heading() {
        let blocks = parse_markdown("# Hello");
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Heading { level, content } => {
                assert_eq!(*level, 1);
                assert_eq!(content, &vec![Inline::Text("Hello".to_string())]);
            }
            _ => panic!("expected heading"),
        }
    }

    #[test]
    fn parses_fenced_code() {
        let input = "```rust\nfn main() {}\n```\n";
        let blocks = parse_markdown(input);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::CodeBlock { lang, code } => {
                assert_eq!(lang.as_deref(), Some("rust"));
                assert!(code.contains("fn main()"));
            }
            _ => panic!("expected code block"),
        }
    }

    #[test]
    fn parses_four_tick_fence_with_inner_triple_ticks() {
        let input = "````md\n```ts\nconst x: number = 1\n```\n````\n";
        let blocks = parse_markdown(input);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::CodeBlock { lang, code } => {
                assert_eq!(lang.as_deref(), Some("md"));
                assert!(code.contains("```ts"));
                assert!(code.contains("const x: number = 1"));
                assert!(code.contains("```"));
            }
            _ => panic!("expected code block"),
        }
    }

    #[test]
    fn parses_table() {
        let input = "| A | B |\n| - | -: |\n| x | 1 |\n";
        let blocks = parse_markdown(input);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Table {
                headers,
                rows,
                aligns,
            } => {
                assert_eq!(headers.len(), 2);
                assert_eq!(rows.len(), 1);
                assert_eq!(aligns.len(), 2);
                assert_eq!(aligns[0], Alignment::None);
                assert_eq!(aligns[1], Alignment::Right);
            }
            _ => panic!("expected table"),
        }
    }

    #[test]
    fn parses_nested_list() {
        let input = "- a\n  - b\n";
        let blocks = parse_markdown(input);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::List { items, .. } => {
                assert_eq!(items.len(), 1);
                assert!(!items[0].is_empty());
                let item_text = collect_block_text(&items[0]);
                assert!(item_text.contains("a"));
                assert!(item_text.contains("b"));
            }
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn parses_tight_ordered_list_text() {
        let input = "1. first item\n2. second item\n";
        let blocks = parse_markdown(input);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::List {
                ordered,
                items,
                start,
            } => {
                assert!(*ordered);
                assert_eq!(*start, Some(1));
                assert_eq!(items.len(), 2);
                let text1 = collect_block_text(&items[0]);
                let text2 = collect_block_text(&items[1]);
                assert!(text1.contains("first item"));
                assert!(text2.contains("second item"));
            }
            _ => panic!("expected ordered list"),
        }
    }

    #[test]
    fn parses_task_list_markers() {
        let input = "- [x] done\n- [ ] todo\n";
        let blocks = parse_markdown(input);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::List { items, .. } => {
                assert_eq!(items.len(), 2);
                let text1 = collect_block_text(&items[0]);
                let text2 = collect_block_text(&items[1]);
                assert!(text1.contains("[x]"));
                assert!(text1.contains("done"));
                assert!(text2.contains("[ ]"));
                assert!(text2.contains("todo"));
            }
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn parses_strikethrough_inline() {
        let blocks = parse_markdown("A ~~deprecated~~ flag");
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Paragraph(inlines) => {
                let text = collect_inline_text(inlines);
                assert!(text.contains("deprecated"));
            }
            _ => panic!("expected paragraph"),
        }
    }

    #[test]
    fn parses_footnotes() {
        let input = "Uses note[^a].\n\n[^a]: footnote body\n";
        let blocks = parse_markdown(input);
        assert_eq!(blocks.len(), 2);
        match &blocks[0] {
            Block::Paragraph(inlines) => {
                let text = collect_inline_text(inlines);
                assert!(text.contains("[^a]"));
            }
            _ => panic!("expected paragraph"),
        }
        match &blocks[1] {
            Block::FootnoteDefinition { name, content } => {
                assert_eq!(name, "a");
                let text = collect_block_text(content);
                assert!(text.contains("footnote body"));
            }
            _ => panic!("expected footnote definition"),
        }
    }

    #[test]
    fn parses_image_only_paragraph_as_block_image() {
        let input = "![Logo](assets/logo.png)\n";
        let blocks = parse_markdown(input);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Image { alt, path, .. } => {
                assert_eq!(alt, "Logo");
                assert_eq!(path, "assets/logo.png");
            }
            _ => panic!("expected image block"),
        }
    }

    fn collect_block_text(blocks: &[Block]) -> String {
        let mut out = String::new();
        for block in blocks {
            match block {
                Block::Paragraph(inlines)
                | Block::Heading {
                    content: inlines, ..
                } => {
                    out.push_str(&collect_inline_text(inlines));
                    out.push('\n');
                }
                Block::CodeBlock { code, .. } => {
                    out.push_str(code);
                    out.push('\n');
                }
                Block::BlockQuote(inner) => out.push_str(&collect_block_text(inner)),
                Block::List { items, .. } => {
                    for item in items {
                        out.push_str(&collect_block_text(item));
                    }
                }
                Block::Table { headers, rows, .. } => {
                    for h in headers {
                        out.push_str(&collect_inline_text(h));
                    }
                    for row in rows {
                        for cell in row {
                            out.push_str(&collect_inline_text(cell));
                        }
                    }
                }
                Block::Image { alt, path, .. } => {
                    out.push_str(alt);
                    out.push_str(path);
                }
                Block::FootnoteDefinition { name, content } => {
                    out.push_str(name);
                    out.push_str(&collect_block_text(content));
                }
                Block::Rule => {}
            }
        }
        out
    }

    fn collect_inline_text(inlines: &[Inline]) -> String {
        let mut out = String::new();
        for inline in inlines {
            match inline {
                Inline::Text(t) | Inline::Code(t) => out.push_str(t),
                Inline::Emph(i) | Inline::Strong(i) | Inline::Strike(i) => {
                    out.push_str(&collect_inline_text(i))
                }
                Inline::Link { text, .. } => out.push_str(&collect_inline_text(text)),
                Inline::Image { alt, path, .. } => {
                    out.push_str(alt);
                    out.push_str(path);
                }
                Inline::FootnoteRef(name) => {
                    out.push_str("[^");
                    out.push_str(name);
                    out.push(']');
                }
                Inline::SoftBreak | Inline::HardBreak => out.push(' '),
            }
        }
        out
    }
}
