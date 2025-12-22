#![forbid(unsafe_code)]
//! Mandate converts Markdown/CommonMark or YAML-with-Markdown into roff manpages.
//!
//! # Example
//!
//! ```no_run
//! let markdown = "# mytool(1) -- Example tool\n\n## SYNOPSIS\n\nExample.";
//! let options = mandate::ManpageOptions::new("mytool", "1", "Mytool Manual", None, None);
//! let roff = mandate::convert_markdown_to_roff(markdown, &options)?;
//! # Ok::<(), mandate::MandateError>(())
//! ```

use jsonschema::validator_for;
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use serde_json::{Map as JsonMap, Number as JsonNumber, Value as JsonValue};
use yaml_rust::{yaml::Hash, Yaml, YamlLoader};
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::Path;

pub const BUILTIN_SCHEMA: &str = include_str!("../data/manual_schema.yml");

#[derive(Debug, Clone)]
pub struct Document {
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone)]
pub enum Block {
    Heading { level: u8, content: Vec<Inline> },
    Paragraph(Vec<Inline>),
    List { kind: ListKind, items: Vec<ListItem> },
    CodeBlock { text: String },
}

#[derive(Debug, Clone)]
pub enum ListKind {
    Unordered,
    Ordered { start: u64 },
}

#[derive(Debug, Clone)]
pub struct ListItem {
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone)]
pub enum Inline {
    Text(String),
    Code(String),
    Emphasis(Vec<Inline>),
    Strong(Vec<Inline>),
    Link {
        url: String,
        title: Option<String>,
        content: Vec<Inline>,
    },
    LineBreak(LineBreak),
}

#[derive(Debug, Clone, Copy)]
pub enum LineBreak {
    Soft,
    Hard,
}

#[derive(Debug, Clone)]
pub struct ManpageOptions {
    pub program: String,
    pub section: String,
    pub title: String,
    pub manual_section: Option<String>,
    pub source: Option<String>,
}

impl ManpageOptions {
    pub fn new(
        program: impl Into<String>,
        section: impl Into<String>,
        title: impl Into<String>,
        manual_section: Option<String>,
        source: Option<String>,
    ) -> Self {
        Self {
            program: program.into(),
            section: section.into(),
            title: title.into(),
            manual_section,
            source,
        }
    }
}

#[derive(Debug)]
pub enum MandateError {
    Unimplemented(&'static str),
    Markdown(String),
    Yaml(String),
    Schema(String),
}

impl fmt::Display for MandateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MandateError::Unimplemented(msg) => write!(f, "{msg}"),
            MandateError::Markdown(msg) => write!(f, "markdown parse error: {msg}"),
            MandateError::Yaml(msg) => write!(f, "yaml parse error: {msg}"),
            MandateError::Schema(msg) => write!(f, "schema validation error: {msg}"),
        }
    }
}

impl Error for MandateError {}

pub type Result<T> = std::result::Result<T, MandateError>;

#[derive(Debug, Default)]
struct BlockContainerFrame {
    blocks: Vec<Block>,
    pending_inlines: Vec<Inline>,
}

impl BlockContainerFrame {
    fn push_inline(&mut self, inline: Inline) {
        self.pending_inlines.push(inline);
    }

    fn flush_pending(&mut self) {
        if !self.pending_inlines.is_empty() {
            let inlines = std::mem::take(&mut self.pending_inlines);
            self.blocks.push(Block::Paragraph(inlines));
        }
    }

    fn push_block(&mut self, block: Block) {
        self.flush_pending();
        self.blocks.push(block);
    }

    fn finish(mut self) -> Vec<Block> {
        self.flush_pending();
        self.blocks
    }
}

#[derive(Debug)]
enum Frame {
    Document(BlockContainerFrame),
    BlockContainer(BlockContainerFrame),
    List { kind: ListKind, items: Vec<ListItem> },
    ListItem(BlockContainerFrame),
    Paragraph { inlines: Vec<Inline> },
    Heading { level: u8, inlines: Vec<Inline> },
    Emphasis { inlines: Vec<Inline> },
    Strong { inlines: Vec<Inline> },
    Link {
        url: String,
        title: Option<String>,
        inlines: Vec<Inline>,
    },
    Image {
        _url: String,
        _title: Option<String>,
        inlines: Vec<Inline>,
    },
    CodeBlock { text: String },
    HtmlBlock { text: String },
}

pub fn parse_markdown(markdown: &str) -> Result<Document> {
    let parser = Parser::new_ext(markdown, Options::empty());
    parse_events(parser)
}

fn parse_events<'a, I>(events: I) -> Result<Document>
where
    I: IntoIterator<Item = Event<'a>>,
{
    let mut stack = vec![Frame::Document(BlockContainerFrame::default())];
    for event in events {
        match event {
            Event::Start(tag) => handle_start(tag, &mut stack)?,
            Event::End(tag_end) => handle_end(tag_end, &mut stack)?,
            Event::Text(text) => {
                if let Some(Frame::CodeBlock { text: buffer }) = stack.last_mut() {
                    buffer.push_str(&text);
                } else if let Some(Frame::HtmlBlock { text: buffer }) = stack.last_mut() {
                    buffer.push_str(&text);
                } else {
                    push_inline(&mut stack, Inline::Text(text.into_string()))?;
                }
            }
            Event::Code(text) => {
                push_inline(&mut stack, Inline::Code(text.into_string()))?;
            }
            Event::Html(html) | Event::InlineHtml(html) => {
                if let Some(Frame::HtmlBlock { text: buffer }) = stack.last_mut() {
                    buffer.push_str(&html);
                } else {
                    push_inline(&mut stack, Inline::Text(html.into_string()))?;
                }
            }
            Event::SoftBreak => {
                push_inline(&mut stack, Inline::LineBreak(LineBreak::Soft))?;
            }
            Event::HardBreak => {
                push_inline(&mut stack, Inline::LineBreak(LineBreak::Hard))?;
            }
            Event::FootnoteReference(label) => {
                push_inline(&mut stack, Inline::Text(label.into_string()))?;
            }
            Event::Rule => {
                // Ignore horizontal rules for now.
            }
            Event::TaskListMarker(_) => {
                // Ignore task list markers for now.
            }
            Event::InlineMath(_) | Event::DisplayMath(_) => {
                // Ignore math for now.
            }
        }
    }
    finish_stack(stack)
}

fn finish_stack(mut stack: Vec<Frame>) -> Result<Document> {
    if stack.len() != 1 {
        return Err(MandateError::Markdown(
            "unbalanced markdown structure".to_string(),
        ));
    }

    match stack.pop() {
        Some(Frame::Document(frame)) => Ok(Document {
            blocks: frame.finish(),
        }),
        _ => Err(MandateError::Markdown(
            "unexpected parser state at end of document".to_string(),
        )),
    }
}

fn handle_start(tag: Tag<'_>, stack: &mut Vec<Frame>) -> Result<()> {
    match tag {
        Tag::Paragraph => {
            flush_pending_block_container(stack);
            stack.push(Frame::Paragraph { inlines: Vec::new() });
        }
        Tag::Heading { level, .. } => {
            flush_pending_block_container(stack);
            stack.push(Frame::Heading {
                level: heading_level_to_u8(level),
                inlines: Vec::new(),
            });
        }
        Tag::List(start) => {
            flush_pending_block_container(stack);
            let kind = start.map_or(ListKind::Unordered, |value| ListKind::Ordered { start: value });
            stack.push(Frame::List {
                kind,
                items: Vec::new(),
            });
        }
        Tag::Item => {
            stack.push(Frame::ListItem(BlockContainerFrame::default()));
        }
        Tag::CodeBlock(_) => {
            flush_pending_block_container(stack);
            stack.push(Frame::CodeBlock { text: String::new() });
        }
        Tag::BlockQuote(_) => {
            flush_pending_block_container(stack);
            stack.push(Frame::BlockContainer(BlockContainerFrame::default()));
        }
        Tag::HtmlBlock => {
            flush_pending_block_container(stack);
            stack.push(Frame::HtmlBlock { text: String::new() });
        }
        Tag::Emphasis => {
            stack.push(Frame::Emphasis { inlines: Vec::new() });
        }
        Tag::Strong => {
            stack.push(Frame::Strong { inlines: Vec::new() });
        }
        Tag::Link { dest_url, title, .. } => {
            let title = if title.is_empty() {
                None
            } else {
                Some(title.into_string())
            };
            stack.push(Frame::Link {
                url: dest_url.into_string(),
                title,
                inlines: Vec::new(),
            });
        }
        Tag::Image { dest_url, title, .. } => {
            let title = if title.is_empty() {
                None
            } else {
                Some(title.into_string())
            };
            stack.push(Frame::Image {
                _url: dest_url.into_string(),
                _title: title,
                inlines: Vec::new(),
            });
        }
        Tag::Table(_) | Tag::TableHead | Tag::TableRow | Tag::TableCell => {
            return Err(MandateError::Markdown(
                "tables are not supported".to_string(),
            ));
        }
        Tag::FootnoteDefinition(_) | Tag::Strikethrough | Tag::MetadataBlock(_) => {
            return Err(MandateError::Markdown(
                "unsupported markdown construct encountered".to_string(),
            ));
        }
    }
    Ok(())
}

fn handle_end(tag_end: TagEnd, stack: &mut Vec<Frame>) -> Result<()> {
    match tag_end {
        TagEnd::Paragraph => {
            let inlines = match stack.pop() {
                Some(Frame::Paragraph { inlines }) => inlines,
                _ => return Err(MandateError::Markdown("paragraph mismatch".to_string())),
            };
            push_block(stack, Block::Paragraph(inlines))?;
        }
        TagEnd::Heading(_) => {
            let (level, inlines) = match stack.pop() {
                Some(Frame::Heading { level, inlines }) => (level, inlines),
                _ => return Err(MandateError::Markdown("heading mismatch".to_string())),
            };
            push_block(stack, Block::Heading { level, content: inlines })?;
        }
        TagEnd::List(_) => {
            let (kind, items) = match stack.pop() {
                Some(Frame::List { kind, items }) => (kind, items),
                _ => return Err(MandateError::Markdown("list mismatch".to_string())),
            };
            push_block(stack, Block::List { kind, items })?;
        }
        TagEnd::Item => {
            let blocks = match stack.pop() {
                Some(Frame::ListItem(frame)) => frame.finish(),
                _ => return Err(MandateError::Markdown("list item mismatch".to_string())),
            };
            let list_item = ListItem { blocks };
            match stack.last_mut() {
                Some(Frame::List { items, .. }) => items.push(list_item),
                _ => return Err(MandateError::Markdown("list item parent mismatch".to_string())),
            }
        }
        TagEnd::Emphasis => {
            let inlines = match stack.pop() {
                Some(Frame::Emphasis { inlines }) => inlines,
                _ => return Err(MandateError::Markdown("emphasis mismatch".to_string())),
            };
            push_inline(stack, Inline::Emphasis(inlines))?;
        }
        TagEnd::Strong => {
            let inlines = match stack.pop() {
                Some(Frame::Strong { inlines }) => inlines,
                _ => return Err(MandateError::Markdown("strong mismatch".to_string())),
            };
            push_inline(stack, Inline::Strong(inlines))?;
        }
        TagEnd::Link => {
            let (url, title, inlines) = match stack.pop() {
                Some(Frame::Link { url, title, inlines }) => (url, title, inlines),
                _ => return Err(MandateError::Markdown("link mismatch".to_string())),
            };
            push_inline(
                stack,
                Inline::Link {
                    url,
                    title,
                    content: inlines,
                },
            )?;
        }
        TagEnd::Image => {
            let inlines = match stack.pop() {
                Some(Frame::Image { inlines, .. }) => inlines,
                _ => return Err(MandateError::Markdown("image mismatch".to_string())),
            };
            let text = inline_text(&inlines);
            push_inline(stack, Inline::Text(text))?;
        }
        TagEnd::CodeBlock => {
            let text = match stack.pop() {
                Some(Frame::CodeBlock { text }) => text,
                _ => return Err(MandateError::Markdown("code block mismatch".to_string())),
            };
            push_block(stack, Block::CodeBlock { text })?;
        }
        TagEnd::HtmlBlock => {
            let text = match stack.pop() {
                Some(Frame::HtmlBlock { text }) => text,
                _ => return Err(MandateError::Markdown("html block mismatch".to_string())),
            };
            push_block(stack, Block::Paragraph(vec![Inline::Text(text)]))?;
        }
        TagEnd::BlockQuote => {
            let blocks = match stack.pop() {
                Some(Frame::BlockContainer(frame)) => frame.finish(),
                _ => return Err(MandateError::Markdown("block container mismatch".to_string())),
            };
            for block in blocks {
                push_block(stack, block)?;
            }
        }
        TagEnd::Table
        | TagEnd::TableHead
        | TagEnd::TableRow
        | TagEnd::TableCell
        | TagEnd::FootnoteDefinition
        | TagEnd::Strikethrough
        | TagEnd::MetadataBlock(_) => {
            return Err(MandateError::Markdown(
                "unsupported markdown construct encountered".to_string(),
            ));
        }
    }
    Ok(())
}

fn push_inline(stack: &mut Vec<Frame>, inline: Inline) -> Result<()> {
    match stack.last_mut() {
        Some(Frame::Paragraph { inlines })
        | Some(Frame::Heading { inlines, .. })
        | Some(Frame::Emphasis { inlines })
        | Some(Frame::Strong { inlines })
        | Some(Frame::Link { inlines, .. })
        | Some(Frame::Image { inlines, .. }) => {
            inlines.push(inline);
        }
        Some(Frame::ListItem(frame))
        | Some(Frame::Document(frame))
        | Some(Frame::BlockContainer(frame)) => {
            frame.push_inline(inline);
        }
        Some(Frame::List { .. }) => {
            return Err(MandateError::Markdown(
                "inline content found directly inside list".to_string(),
            ));
        }
        Some(Frame::CodeBlock { .. }) | Some(Frame::HtmlBlock { .. }) => {
            return Err(MandateError::Markdown(
                "inline content found inside code/html block".to_string(),
            ));
        }
        None => {
            return Err(MandateError::Markdown(
                "inline content found without container".to_string(),
            ));
        }
    }
    Ok(())
}

fn push_block(stack: &mut Vec<Frame>, block: Block) -> Result<()> {
    match stack.last_mut() {
        Some(Frame::Document(frame))
        | Some(Frame::ListItem(frame))
        | Some(Frame::BlockContainer(frame)) => {
            frame.push_block(block);
            Ok(())
        }
        Some(Frame::List { .. }) => Err(MandateError::Markdown(
            "block found directly inside list".to_string(),
        )),
        _ => Err(MandateError::Markdown(
            "block found without container".to_string(),
        )),
    }
}

fn flush_pending_block_container(stack: &mut Vec<Frame>) {
    if let Some(Frame::Document(frame))
    | Some(Frame::ListItem(frame))
    | Some(Frame::BlockContainer(frame)) = stack.last_mut()
    {
        frame.flush_pending();
    }
}

fn heading_level_to_u8(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn inline_text(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(text) | Inline::Code(text) => out.push_str(text),
            Inline::Emphasis(children)
            | Inline::Strong(children)
            | Inline::Link { content: children, .. } => out.push_str(&inline_text(children)),
            Inline::LineBreak(LineBreak::Soft) | Inline::LineBreak(LineBreak::Hard) => {
                out.push('\n');
            }
        }
    }
    out
}

struct RoffWriter {
    output: String,
}

impl RoffWriter {
    fn new() -> Self {
        Self {
            output: String::new(),
        }
    }

    fn finish(self) -> String {
        self.output
            .lines()
            .map(|line| {
                if line.starts_with("\\.") {
                    format!("\\&{}", line)
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn write_header(&mut self, options: &ManpageOptions) {
        let manual = options
            .manual_section
            .as_ref()
            .map(|value| format!("\"{}\"", self.sanitize(value)))
            .unwrap_or_else(|| "\"\"".to_string());
        let source = options
            .source
            .as_ref()
            .map(|value| format!("\"{}\"", self.sanitize(value)))
            .unwrap_or_else(|| "\"\"".to_string());
        let heading = format!(
            ".TH \"{}\" \"{}\" \"{}\" {} {}",
            self.sanitize(&options.program),
            self.sanitize(&options.section),
            self.sanitize(&options.title),
            manual,
            source
        );
        self.write_cmd(&heading);
    }

    fn write_blocks(&mut self, blocks: &[Block], parent: Option<ParentTag>) {
        let mut idx = 0;
        let mut last_heading = false;
        while idx < blocks.len() {
            match &blocks[idx] {
                Block::Heading { level, content } => {
                    self.write_heading(*level, content);
                    last_heading = matches!(level, 2 | 3);
                }
                Block::Paragraph(inlines) => {
                    if !matches!(parent, Some(ParentTag::ListItem)) && !last_heading {
                        self.write_cmd(".P");
                    }
                    self.write_inlines(inlines, false);
                    self.write_raw("\n");
                    last_heading = false;
                }
                Block::List { kind, items } => {
                    let consumed = self.write_list(kind, items, &blocks[idx + 1..]);
                    idx += consumed;
                    last_heading = false;
                }
                Block::CodeBlock { .. } => {
                    let mut combined = String::new();
                    let mut first = true;
                    let mut offset = idx;
                    while let Some(Block::CodeBlock { text }) = blocks.get(offset) {
                        if !first {
                            combined.push('\n');
                        }
                        combined.push_str(text);
                        first = false;
                        offset += 1;
                    }
                    self.write_cmd(".IP \"\" 4");
                    self.write_cmd(".nf\n");
                    self.write_raw(&self.pre_sanitize(&combined));
                    if !combined.ends_with('\n') {
                        self.write_raw("\n");
                    }
                    self.write_cmd(".fi");
                    self.write_cmd(".IP \"\" 0");
                    idx = offset - 1;
                    last_heading = false;
                }
            }
            idx += 1;
        }
    }

    fn write_heading(&mut self, level: u8, content: &[Inline]) {
        let text = self.inline_text(content);
        if level == 1 {
            self.write_cmd(".SH \"NAME\"");
            let (name, desc) = self.split_name_description(&text);
            let name = self.sanitize(&name);
            let desc = self.sanitize(&desc);
            if desc.is_empty() {
                self.write_raw(&format!("\\fB{}\\fR\n", name));
            } else {
                self.write_raw(&format!("\\fB{}\\fR \\- {}\n", name, desc));
            }
        } else if level == 2 {
            self.write_cmd(&format!(".SH \"{}\"", self.sanitize(&text)));
        } else {
            self.write_cmd(&format!(".SS \"{}\"", self.h3_sanitize(&text)));
        }
    }

    fn write_list(&mut self, _kind: &ListKind, items: &[ListItem], following: &[Block]) -> usize {
        if self.is_special_list(items) {
            self.write_cmd(".TP");
            if let Some(item) = items.first() {
                self.write_list_item(item);
            }
            self.ensure_newline();
            let mut consumed = 0;
            while let Some(Block::Paragraph(inlines)) = following.get(consumed) {
                if matches!(following.get(consumed + 1), Some(Block::CodeBlock { .. })) {
                    break;
                }
                self.write_cmd(".IP");
                self.write_inlines(inlines, false);
                self.write_raw("\n");
                consumed += 1;
            }
            consumed
        } else {
            for item in items {
                self.write_cmd(".IP \"\\(bu\" 4");
                self.write_list_item(item);
                self.write_raw("\n");
            }
            if !matches!(following.first(), Some(Block::CodeBlock { .. })) {
                self.write_cmd(".IP \"\" 0");
            }
            0
        }
    }

    fn write_list_item(&mut self, item: &ListItem) {
        if item.blocks.is_empty() {
            return;
        }
        let mut blocks = item.blocks.clone();
        if let Some(Block::Paragraph(inlines)) = blocks.first() {
            self.write_inlines(inlines, true);
            if blocks.len() > 1 {
                self.write_raw("\n");
            }
            blocks.remove(0);
        }
        if !blocks.is_empty() {
            self.write_blocks(&blocks, Some(ParentTag::ListItem));
        }
    }

    fn ensure_newline(&mut self) {
        if !self.output.ends_with('\n') {
            self.output.push('\n');
        }
    }

    fn write_inlines(&mut self, inlines: &[Inline], in_list: bool) {
        for inline in inlines {
            match inline {
                Inline::Text(text) => self.write_raw(&self.sanitize(text)),
                Inline::Code(text) => {
                    let text = self.code_sanitize(text);
                    self.write_raw(&format!("\\fB{}\\fR", text));
                }
                Inline::Emphasis(children) => {
                    let text = self.inline_text(children);
                    self.write_raw(&format!("\\fI{}\\fR", self.sanitize(&text)));
                }
                Inline::Strong(children) => {
                    let text = self.inline_text(children);
                    self.write_raw(&format!("\\fB{}\\fR", self.sanitize(&text)));
                }
                Inline::Link { content, .. } => {
                    let text = self.inline_text(content);
                    self.write_raw(&self.sanitize(&text));
                }
                Inline::LineBreak(LineBreak::Soft) => self.write_raw(" "),
                Inline::LineBreak(LineBreak::Hard) => {
                    if in_list {
                        self.write_raw("\n");
                    } else {
                        self.write_raw(" ");
                    }
                }
            }
        }
    }

    fn is_special_list(&self, items: &[ListItem]) -> bool {
        if items.len() != 1 {
            return false;
        }
        let item = &items[0];
        if item.blocks.len() != 1 {
            return false;
        }
        match item.blocks.first() {
            Some(Block::Paragraph(inlines)) => {
                let text = self.inline_text(inlines).trim().to_string();
                text.ends_with(':')
            }
            _ => false,
        }
    }

    fn split_name_description(&self, text: &str) -> (String, String) {
        let separators = [" -- ", " - ", " — "];
        for sep in separators {
            if let Some((left, right)) = text.split_once(sep) {
                let name = left.trim();
                let desc = right.trim();
                let name = name.split('(').next().unwrap_or(name).trim();
                return (name.to_string(), desc.to_string());
            }
        }
        let name = text.split('(').next().unwrap_or(text).trim();
        (name.to_string(), String::new())
    }

    fn inline_text(&self, inlines: &[Inline]) -> String {
        inline_text(inlines)
    }

    fn sanitize(&self, text: &str) -> String {
        let mut out = String::new();
        let mut last_space = false;
        for ch in text.chars() {
            let chunk = match ch {
                '\\' => "\\e".to_string(),
                '.' => "\\.".to_string(),
                '\'' => "\\'".to_string(),
                '-' => "\\-".to_string(),
                '\n' => " ".to_string(),
                _ => ch.to_string(),
            };
            if ch.is_whitespace() {
                if last_space {
                    continue;
                }
                last_space = true;
            } else {
                last_space = false;
            }
            out.push_str(&chunk);
        }
        self.sanitize_angle_brackets(&out)
    }

    fn sanitize_angle_brackets(&self, text: &str) -> String {
        let mut out = String::new();
        let mut buffer = String::new();
        let mut in_angle = false;
        for ch in text.chars() {
            if ch == '<' {
                if in_angle {
                    out.push('<');
                    out.push_str(&buffer);
                    buffer.clear();
                } else {
                    in_angle = true;
                    buffer.clear();
                }
            } else if ch == '>' && in_angle {
                let inner = buffer.clone();
                out.push_str(&format!("\\fI{}\\fR", inner));
                buffer.clear();
                in_angle = false;
            } else if in_angle {
                buffer.push(ch);
            } else {
                out.push(ch);
            }
        }
        if in_angle {
            out.push('<');
            out.push_str(&buffer);
        }
        out
    }

    fn pre_sanitize(&self, text: &str) -> String {
        self.base_sanitize(text)
    }

    fn code_sanitize(&self, text: &str) -> String {
        let mut out = String::new();
        for ch in text.chars() {
            if ch.is_whitespace() {
                out.push(' ');
            } else {
                out.push_str(&self.base_sanitize(&ch.to_string()));
            }
        }
        out
    }

    fn h3_sanitize(&self, text: &str) -> String {
        let base = self.base_sanitize(text);
        base.split('\n').collect::<Vec<_>>().join(" ")
    }

    fn base_sanitize(&self, text: &str) -> String {
        let mut out = String::new();
        for ch in text.chars() {
            match ch {
                '\\' => out.push_str("\\e"),
                '.' => out.push_str("\\."),
                '\'' => out.push_str("\\'"),
                '-' => out.push_str("\\-"),
                _ => out.push(ch),
            }
        }
        out
    }

    fn write_cmd(&mut self, cmd: &str) {
        self.output.push_str(cmd);
        if !cmd.ends_with('\n') {
            self.output.push('\n');
        }
    }

    fn write_raw(&mut self, text: &str) {
        self.output.push_str(text);
    }
}

#[derive(Debug, Clone, Copy)]
enum ParentTag {
    ListItem,
}

pub fn convert_markdown_to_roff(markdown: &str, options: &ManpageOptions) -> Result<String> {
    let document = parse_markdown(markdown)?;
    render_roff(&document, options)
}

pub fn convert_yaml_to_markdown(_yaml: &str) -> Result<String> {
    let docs = YamlLoader::load_from_str(_yaml)
        .map_err(|err| MandateError::Yaml(err.to_string()))?;
    let manual = docs
        .get(0)
        .ok_or_else(|| MandateError::Yaml("empty yaml document".to_string()))?;
    let manual = ensure_mapping(manual, "manual root")?;

    let mut out = String::new();
    out.push_str(&map_get_string(manual, "manpage_intro")?.unwrap_or_else(|| "\n".to_string()));
    out.push_str(&dedent_body(
        &map_get_string(manual, "body")?.unwrap_or_else(|| "\n".to_string()),
    ));

    if let Some(sections) = map_get_sequence(manual, "sections")? {
        for section in sections {
            let section_map = ensure_mapping(section, "sections item")?;
            let title = map_get_string(section_map, "title")?.unwrap_or_default();
            out.push_str(&format!("## {}\n", title.to_uppercase()));
            out.push_str(&dedent_body(
                &map_get_string(section_map, "body")?.unwrap_or_else(|| "\n".to_string()),
            ));
            out.push('\n');

            if let Some(entries) = map_get_sequence(section_map, "entries")? {
                for entry in entries {
                    let entry_map = ensure_mapping(entry, "entry")?;
                    let title = map_get_string(entry_map, "title")?.unwrap_or_default();
                    out.push_str(&format!("### {}\n", title));
                    out.push_str(&dedent_body(
                        &map_get_string(entry_map, "body")?.unwrap_or_else(|| "\n".to_string()),
                    ));
                    out.push('\n');

                    if let Some(examples) = map_get_sequence(entry_map, "examples")? {
                        out.push_str("~~~~\n");
                        let mut first = true;
                        for example in examples {
                            let example_map = ensure_mapping(example, "example")?;
                            if !first {
                                out.push('\n');
                            }
                            first = false;
                            let program =
                                map_get_string(example_map, "program")?.unwrap_or_default();
                            let input = map_get_string(example_map, "input")?.unwrap_or_default();
                            out.push_str(&format!("jq '{}'\n", program));
                            out.push_str(&format!("   {}\n", input));
                            let outputs = map_get_sequence(example_map, "output")?;
                            let outputs = outputs
                                .unwrap_or(&[])
                                .iter()
                                .map(yaml_value_to_string)
                                .collect::<Vec<_>>();
                            out.push_str(&format!("=> {}\n", outputs.join(", ")));
                        }
                        out.push_str("~~~~\n");
                    }
                }
            }
            out.push('\n');
        }
    }

    out.push_str(&map_get_string(manual, "manpage_epilogue")?.unwrap_or_default());
    Ok(out)
}

pub fn convert_yaml_to_roff(yaml: &str, options: &ManpageOptions) -> Result<String> {
    let markdown = convert_yaml_to_markdown(yaml)?;
    convert_markdown_to_roff(&markdown, options)
}

pub fn parse_yaml_to_document(yaml: &str) -> Result<Document> {
    let markdown = convert_yaml_to_markdown(yaml)?;
    parse_markdown(&markdown)
}

pub fn render_roff(document: &Document, options: &ManpageOptions) -> Result<String> {
    let mut writer = RoffWriter::new();
    writer.write_header(options);
    writer.write_blocks(&document.blocks, None);
    Ok(writer.finish())
}

pub fn validate_yaml_with_schema<P: AsRef<Path>>(yaml: &str, schema_path: P) -> Result<()> {
    let schema_source = fs::read_to_string(schema_path.as_ref())
        .map_err(|err| MandateError::Schema(err.to_string()))?;
    validate_yaml_with_schema_str(yaml, &schema_source)
}

pub fn validate_yaml_with_schema_str(yaml: &str, schema_source: &str) -> Result<()> {
    let docs = YamlLoader::load_from_str(yaml)
        .map_err(|err| MandateError::Yaml(err.to_string()))?;
    let document = docs
        .get(0)
        .ok_or_else(|| MandateError::Yaml("empty yaml document".to_string()))?;
    let schema_docs = YamlLoader::load_from_str(&schema_source)
        .map_err(|err| MandateError::Schema(err.to_string()))?;
    let schema_yaml = schema_docs
        .get(0)
        .ok_or_else(|| MandateError::Schema("empty schema document".to_string()))?;
    let schema_json = yaml_to_json(schema_yaml);
    let instance_json = yaml_to_json(document);
    let validator = validator_for(&schema_json)
        .map_err(|err| MandateError::Schema(err.to_string()))?;
    if let Err(error) = validator.validate(&instance_json) {
        return Err(MandateError::Schema(error.to_string()));
    }
    Ok(())
}

fn dedent_body(body: &str) -> String {
    body.split('\n')
        .map(|line| {
            if line.starts_with("  ") {
                let remainder = &line[2..];
                if remainder
                    .chars()
                    .next()
                    .map(|ch| !ch.is_whitespace())
                    .unwrap_or(false)
                {
                    return remainder.to_string();
                }
            }
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn ensure_mapping<'a>(value: &'a Yaml, context: &str) -> Result<&'a Hash> {
    value
        .as_hash()
        .ok_or_else(|| MandateError::Yaml(format!("expected mapping for {context}")))
}

fn map_get_string(map: &Hash, key: &str) -> Result<Option<String>> {
    match map.get(&Yaml::String(key.to_string())) {
        None | Some(Yaml::Null) => Ok(None),
        Some(Yaml::String(value)) => Ok(Some(value.clone())),
        Some(other) => Err(MandateError::Yaml(format!(
            "expected string for key '{key}', found {}",
            yaml_type_name(other)
        ))),
    }
}

fn map_get_sequence<'a>(map: &'a Hash, key: &str) -> Result<Option<&'a [Yaml]>> {
    match map.get(&Yaml::String(key.to_string())) {
        None | Some(Yaml::Null) => Ok(None),
        Some(Yaml::Array(values)) => Ok(Some(values)),
        Some(other) => Err(MandateError::Yaml(format!(
            "expected sequence for key '{key}', found {}",
            yaml_type_name(other)
        ))),
    }
}

fn yaml_value_to_string(value: &Yaml) -> String {
    match value {
        Yaml::Null => "null".to_string(),
        Yaml::Boolean(value) => value.to_string(),
        Yaml::Integer(value) => value.to_string(),
        Yaml::Real(value) => value.clone(),
        Yaml::String(value) => value.clone(),
        Yaml::Array(values) => {
            let items = values
                .iter()
                .map(yaml_value_to_string)
                .collect::<Vec<_>>();
            format!("[{}]", items.join(", "))
        }
        Yaml::Hash(map) => {
            let mut pairs = Vec::new();
            for (key, value) in map.iter() {
                pairs.push(format!(
                    "{}: {}",
                    yaml_value_to_string(key),
                    yaml_value_to_string(value)
                ));
            }
            format!("{{{}}}", pairs.join(", "))
        }
        Yaml::Alias(alias) => format!("*{alias}"),
        Yaml::BadValue => "!!badvalue".to_string(),
    }
}

fn yaml_type_name(value: &Yaml) -> &'static str {
    match value {
        Yaml::Null => "null",
        Yaml::Boolean(_) => "bool",
        Yaml::Integer(_) => "int",
        Yaml::Real(_) => "float",
        Yaml::String(_) => "string",
        Yaml::Array(_) => "sequence",
        Yaml::Hash(_) => "mapping",
        Yaml::Alias(_) => "alias",
        Yaml::BadValue => "bad",
    }
}

fn yaml_to_json(value: &Yaml) -> JsonValue {
    match value {
        Yaml::Null => JsonValue::Null,
        Yaml::Boolean(value) => JsonValue::Bool(*value),
        Yaml::Integer(value) => JsonValue::Number(JsonNumber::from(*value)),
        Yaml::Real(value) => value
            .parse::<f64>()
            .ok()
            .and_then(JsonNumber::from_f64)
            .map(JsonValue::Number)
            .unwrap_or_else(|| JsonValue::String(value.clone())),
        Yaml::String(value) => JsonValue::String(value.clone()),
        Yaml::Array(values) => {
            JsonValue::Array(values.iter().map(yaml_to_json).collect::<Vec<_>>())
        }
        Yaml::Hash(map) => {
            let mut out = JsonMap::new();
            for (key, value) in map.iter() {
                let key = match key {
                    Yaml::String(value) => value.clone(),
                    _ => yaml_value_to_string(key),
                };
                out.insert(key, yaml_to_json(value));
            }
            JsonValue::Object(out)
        }
        Yaml::Alias(alias) => JsonValue::String(format!("*{alias}")),
        Yaml::BadValue => JsonValue::String("!!badvalue".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options() -> ManpageOptions {
        ManpageOptions::new("mandate", "1", "Test", None, None)
    }

    #[test]
    fn split_name_description_variants() {
        let writer = RoffWriter::new();
        let (name, desc) = writer.split_name_description("mandate(1) -- Example Tool");
        assert_eq!(name, "mandate");
        assert_eq!(desc, "Example Tool");

        let (name, desc) = writer.split_name_description("mandate - Another Tool");
        assert_eq!(name, "mandate");
        assert_eq!(desc, "Another Tool");

        let (name, desc) = writer.split_name_description("mandate — Dash Tool");
        assert_eq!(name, "mandate");
        assert_eq!(desc, "Dash Tool");

        let (name, desc) = writer.split_name_description("mandate");
        assert_eq!(name, "mandate");
        assert!(desc.is_empty());
    }

    #[test]
    fn sanitize_angle_brackets_emits_italic() {
        let writer = RoffWriter::new();
        let sanitized = writer.sanitize("Use <arg> and <file>");
        assert!(sanitized.contains("\\fIarg\\fR"));
        assert!(sanitized.contains("\\fIfile\\fR"));
    }

    #[test]
    fn render_bulleted_list_with_code_and_linebreaks() {
        let markdown = r#"
## LIST

- Item one
- Item two with  \
  hard break

```
code
```
"#;
        let roff = convert_markdown_to_roff(markdown, &options()).expect("render roff");
        assert!(roff.contains(".IP \"\\(bu\" 4"));
        assert!(roff.contains("Item one"));
        assert!(roff.contains("Item two with"));
        assert!(roff.contains(".nf"));
        assert!(roff.contains("code"));
        assert!(roff.contains(".fi"));
    }

    #[test]
    fn render_image_uses_alt_text() {
        let markdown = r#"
## IMG

![Alt Text](https://example.com/image.png)
"#;
        let roff = convert_markdown_to_roff(markdown, &options()).expect("render roff");
        assert!(roff.contains("Alt Text"));
    }

    #[test]
    fn soft_breaks_collapse_to_space() {
        let markdown = r#"
## TEXT

line one
line two
"#;
        let roff = convert_markdown_to_roff(markdown, &options()).expect("render roff");
        assert!(roff.contains("line one line two"));
    }

    #[test]
    fn special_list_stops_before_code_block() {
        let markdown = r#"
## OPTIONS

- Foo:

Paragraph before code.

```
code
```
"#;
        let roff = convert_markdown_to_roff(markdown, &options()).expect("render roff");
        assert!(!roff.contains(".IP\nParagraph before code"));
        assert!(roff.contains(".nf"));
        assert!(roff.contains("code"));
    }

    #[test]
    fn consecutive_code_blocks_are_combined() {
        let markdown = r#"
## CODE

```
first
```

```
second
```
"#;
        let roff = convert_markdown_to_roff(markdown, &options()).expect("render roff");
        assert!(roff.contains("first"));
        assert!(roff.contains("second"));
        assert!(roff.contains(".nf"));
        assert!(roff.contains(".fi"));
    }

    #[test]
    fn link_text_is_rendered() {
        let markdown = r#"
## LINKS

See [example](https://example.com).
"#;
        let roff = convert_markdown_to_roff(markdown, &options()).expect("render roff");
        assert!(roff.contains("See example"));
    }

    #[test]
    fn leading_dot_line_is_escaped() {
        let markdown = r#"
## TEXT

.leading dot
"#;
        let roff = convert_markdown_to_roff(markdown, &options()).expect("render roff");
        assert!(roff.contains("\\&\\.leading"));
    }

    #[test]
    fn yaml_value_to_string_covers_collections() {
        let mut map = Hash::new();
        map.insert(Yaml::String("k".to_string()), Yaml::Integer(1));
        let value = Yaml::Hash(map);
        assert_eq!(yaml_value_to_string(&value), "{k: 1}");

        let array = Yaml::Array(vec![Yaml::Integer(1), Yaml::Integer(2)]);
        assert_eq!(yaml_value_to_string(&array), "[1, 2]");

        let alias = Yaml::Alias(3);
        assert_eq!(yaml_value_to_string(&alias), "*3");

        let bad = Yaml::BadValue;
        assert_eq!(yaml_value_to_string(&bad), "!!badvalue");
    }

    #[test]
    fn convert_yaml_to_markdown_errors_on_wrong_type() {
        let yaml = "[]";
        let err = convert_yaml_to_markdown(yaml).expect_err("expected error");
        match err {
            MandateError::Yaml(msg) => assert!(msg.contains("mapping")),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn map_get_string_errors_on_non_string() {
        let mut map = Hash::new();
        map.insert(Yaml::String("title".to_string()), Yaml::Integer(5));
        let err = map_get_string(&map, "title").expect_err("expected error");
        match err {
            MandateError::Yaml(msg) => assert!(msg.contains("expected string")),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn map_get_sequence_errors_on_non_sequence() {
        let mut map = Hash::new();
        map.insert(Yaml::String("entries".to_string()), Yaml::String("nope".to_string()));
        let err = map_get_sequence(&map, "entries").expect_err("expected error");
        match err {
            MandateError::Yaml(msg) => assert!(msg.contains("expected sequence")),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn yaml_type_name_variants() {
        assert_eq!(yaml_type_name(&Yaml::Null), "null");
        assert_eq!(yaml_type_name(&Yaml::Boolean(true)), "bool");
        assert_eq!(yaml_type_name(&Yaml::Integer(1)), "int");
        assert_eq!(yaml_type_name(&Yaml::Real("1.2".to_string())), "float");
        assert_eq!(yaml_type_name(&Yaml::String("x".to_string())), "string");
        assert_eq!(yaml_type_name(&Yaml::Array(vec![])), "sequence");
        assert_eq!(yaml_type_name(&Yaml::Hash(Hash::new())), "mapping");
        assert_eq!(yaml_type_name(&Yaml::Alias(1)), "alias");
        assert_eq!(yaml_type_name(&Yaml::BadValue), "bad");
    }
}
