//! A selection belongs to the article, including blocks outside the viewport.
use crate::document::{Block, BlockKind, Document, Mark};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position {
    pub block: usize,
    pub byte: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BodySelection {
    pub anchor: Position,
    pub cursor: Position,
}

impl BodySelection {
    pub fn all(document: &Document) -> Option<Self> {
        let last = document.blocks.len().checked_sub(1)?;
        Some(Self {
            anchor: Position::default(),
            cursor: Position {
                block: last,
                byte: document.blocks[last].text.len(),
            },
        })
    }

    pub fn ordered(self) -> (Position, Position) {
        (self.anchor.min(self.cursor), self.anchor.max(self.cursor))
    }

    pub fn normalized(self, document: &Document) -> Option<Self> {
        let anchor_text = &document.blocks.get(self.anchor.block)?.text;
        let cursor_text = &document.blocks.get(self.cursor.block)?.text;
        Some(Self {
            anchor: Position { byte: boundary(anchor_text, self.anchor.byte), ..self.anchor },
            cursor: Position { byte: boundary(cursor_text, self.cursor.byte), ..self.cursor },
        })
    }

    pub fn range(self, block: usize, len: usize) -> Option<std::ops::Range<usize>> {
        let (start, end) = self.ordered();
        if block < start.block || block > end.block {
            return None;
        }
        Some(
            (if block == start.block {
                start.byte.min(len)
            } else {
                0
            })..(if block == end.block {
                end.byte.min(len)
            } else {
                len
            }),
        )
    }

    pub fn text(self, document: &Document) -> String {
        let Some(selection) = self.normalized(document) else { return String::new(); };
        document
            .blocks
            .iter()
            .enumerate()
            .filter_map(|(index, block)| {
                selection.range(index, block.text.len())
                    .map(|range| block.text[range].to_owned())
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Replace the selected content, preserving the untouched prefix/suffix and
    /// their marks. Objects between the endpoints are selected with the text.
    pub fn replace(self, document: &mut Document, input: &str) -> Option<Position> {
        let (start, end) = self.normalized(document)?.ordered();
        let first = &document.blocks[start.block];
        let last = &document.blocks[end.block];
        let mut replacement = if matches!(first.kind, BlockKind::Image | BlockKind::Divider) {
            Block::new(BlockKind::Paragraph, "")
        } else {
            first.clone()
        };
        replacement.text = format!(
            "{}{}{}",
            &first.text[..start.byte],
            input,
            &last.text[end.byte..]
        );
        replacement.marks.clear();
        for mark in &first.marks {
            let mut mark = mark.clone();
            mark.end = mark.end.min(start.byte);
            if mark.start < mark.end {
                replacement.marks.push(mark);
            }
        }
        let suffix_start = start.byte + input.len();
        for mark in &last.marks {
            let begin = mark.start.max(end.byte);
            if begin < mark.end {
                replacement.marks.push(Mark {
                    start: suffix_start + begin - end.byte,
                    end: suffix_start + mark.end - end.byte,
                    ..mark.clone()
                });
            }
        }
        document
            .blocks
            .splice(start.block..=end.block, [replacement]);
        Some(Position {
            block: start.block,
            byte: suffix_start,
        })
    }
}

fn boundary(text: &str, byte: usize) -> usize {
    if byte >= text.len() {
        return text.len();
    }
    text.grapheme_indices(true)
        .map(|(i, _)| i)
        .take_while(|i| *i <= byte)
        .last()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_all_includes_offscreen_blocks_and_empty_lines() {
        let mut doc = Document::default();
        doc.blocks = (0..100)
            .map(|i| Block::new(BlockKind::Paragraph, format!("段落 {i}")))
            .collect();
        let all = BodySelection::all(&doc).unwrap();
        assert!(all.text(&doc).ends_with("段落 99"));
        assert_eq!(all.text(&doc).split("\n\n").count(), 100);
        let position = all.replace(&mut doc, "新正文").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert_eq!(doc.blocks[0].text, "新正文");
        assert_eq!(position.byte, "新正文".len());
    }

    #[test]
    fn reversed_cross_block_replacement_preserves_marks_and_removes_objects() {
        let mut doc = Document::from_markdown("", "**前文**开始\n\n结束*后文*").unwrap();
        doc.blocks.insert(1, Block::new(BlockKind::Image, ""));
        let selected = BodySelection {
            anchor: Position {
                block: 2,
                byte: "结束".len(),
            },
            cursor: Position {
                block: 0,
                byte: "前文".len(),
            },
        };
        assert_eq!(selected.text(&doc), "开始\n\n\n\n结束");
        selected.replace(&mut doc, "替换").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert_eq!(doc.blocks[0].text, "前文替换后文");
        assert!(doc.blocks[0].flags_at(0).0);
        assert!(doc.blocks[0].flags_at("前文替换".len()).1);
    }

    #[test]
    fn replacement_never_splits_an_emoji_grapheme() {
        let mut doc = Document::from_markdown("", "a👩‍💻z").unwrap();
        let selected = BodySelection {
            anchor: Position { block: 0, byte: 3 },
            cursor: Position {
                block: 0,
                byte: "a👩‍💻".len(),
            },
        };
        assert_eq!(selected.text(&doc), "👩‍💻");
        selected.replace(&mut doc, "中").unwrap();
        assert_eq!(doc.blocks[0].text, "a中z");
    }

    #[test]
    fn selection_is_normalized_after_paragraph_content_changes() {
        let mut doc = Document::from_markdown("", "abcd\n\nefgh").unwrap();
        let selection = BodySelection { anchor: Position { block: 0, byte: 1 },
            cursor: Position { block: 1, byte: 3 } };
        doc.blocks[0].text = "中文".into();
        doc.blocks[1].text = "👩‍💻".into();
        assert_eq!(selection.text(&doc), "中文\n\n");
        doc.blocks.pop();
        assert!(selection.normalized(&doc).is_none());
        assert_eq!(selection.text(&doc), "");
    }
}
