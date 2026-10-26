use crate::{
    Result,
    text::{Document, orp::Strategy},
};
use pulldown_cmark::{Event, Options, Parser, TagEnd};
use std::{
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

/// Extension point for EPUB, HTML and PDF importers. Output is readable UTF-8.
pub trait Importer {
    fn convert(&self, source: String) -> Result<String>;
}
pub struct PlainText;
pub struct Markdown;
impl Importer for PlainText {
    fn convert(&self, source: String) -> Result<String> {
        Ok(source)
    }
}
impl Importer for Markdown {
    fn convert(&self, source: String) -> Result<String> {
        let mut output = String::with_capacity(source.len());
        for event in Parser::new_ext(
            &source,
            Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS,
        ) {
            match event {
                Event::Text(t) | Event::Code(t) | Event::InlineMath(t) | Event::DisplayMath(t) => {
                    output.push_str(&t)
                }
                Event::SoftBreak => output.push(' '),
                Event::HardBreak => output.push('\n'),
                Event::Rule => output.push_str("\n\n"),
                Event::End(
                    TagEnd::Paragraph
                    | TagEnd::Heading(_)
                    | TagEnd::Item
                    | TagEnd::CodeBlock
                    | TagEnd::TableRow,
                ) => output.push_str("\n\n"),
                Event::End(TagEnd::TableCell) => output.push(' '),
                // HTML tags and destinations are intentionally omitted. Text
                // between inline tags and image alt text remain readable.
                _ => {}
            }
        }
        Ok(output)
    }
}

pub fn load(path: &Path, strategy: Strategy, cancel: &AtomicBool) -> Result<Document> {
    let extension = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !matches!(extension.as_str(), "txt" | "md" | "markdown" | "") {
        return Err("Supported formats: .txt, .md, .markdown".into());
    }
    let size = std::fs::metadata(path)
        .map_err(|e| format!("Cannot inspect document: {e}"))?
        .len();
    if size > 512 * 1024 * 1024 {
        return Err("Document exceeds the 512 MiB limit".into());
    }
    let bytes = std::fs::read(path).map_err(|e| format!("Cannot read document: {e}"))?;
    if cancel.load(Ordering::Relaxed) {
        return Err("Loading cancelled".into());
    }
    let text = String::from_utf8(bytes).map_err(|e| {
        format!(
            "Document must be valid UTF-8 (invalid byte at {})",
            e.utf8_error().valid_up_to()
        )
    })?;
    let text = text
        .strip_prefix('\u{feff}')
        .map(str::to_owned)
        .unwrap_or(text);
    let readable = if matches!(extension.as_str(), "md" | "markdown") {
        Markdown.convert(text)?
    } else {
        PlainText.convert(text)?
    };
    Document::parse_cancellable(readable, &strategy, cancel)
}
