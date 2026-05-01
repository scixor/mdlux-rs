use pulldown_cmark::{
    Alignment as MdAlignment, CodeBlockKind, CowStr, Event, HeadingLevel, Options, Parser, Tag,
    TagEnd,
};

use crate::markdown::ast::{Alignment, Block, Inline};

pub fn parse_markdown(input: &str) -> Vec<Block> {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_FOOTNOTES;

    let parser = Parser::new_ext(input, options);
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
                    let inlines = self.parse_tight_inlines(end);
                    if !inlines.is_empty() {
                        if let Some(image) = image_only_paragraph(&inlines) {
                            blocks.push(image);
                        } else {
                            blocks.push(Block::Paragraph(inlines));
                        }
                    }
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
                        if let Some(image) = image_only_paragraph(&inlines) {
                            blocks.push(image);
                        } else {
                            blocks.push(Block::Paragraph(inlines));
                        }
                    }
                    Tag::CodeBlock(kind) => {
                        let lang = extract_lang(kind);
                        self.idx += 1;
                        let code = self.collect_code_text();
                        blocks.push(Block::CodeBlock { lang, code });
                    }
                    Tag::BlockQuote(_) => {
                        self.idx += 1;
                        let inner = self.parse_blocks_until(BlockEnd::BlockQuote);
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
                        rows.push(current_row.clone());
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
    if input.is_empty() {
        None
    } else {
        Some(input.to_string())
    }
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
