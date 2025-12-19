use unicode_width::UnicodeWidthStr;

use super::Str;

/// プリティプリント用のドキュメント構造。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Doc {
    Text(String),
    Line,
    Softline,
    Concat(Box<Doc>, Box<Doc>),
    Nest(usize, Box<Doc>),
    Group(Box<Doc>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Flat,
    Break,
}

#[derive(Clone, Debug)]
struct DocFrame {
    indent: usize,
    mode: Mode,
    doc: Doc,
}

/// 文字列ドキュメントを生成する。
pub fn text(value: impl Into<String>) -> Doc {
    Doc::Text(value.into())
}

/// 常に改行するラインドキュメントを生成する。
pub fn line() -> Doc {
    Doc::Line
}

/// グループ内でのみ改行候補になるラインドキュメントを生成する。
pub fn softline() -> Doc {
    Doc::Softline
}

/// ドキュメントをグループ化する。
pub fn group(doc: Doc) -> Doc {
    Doc::Group(Box::new(doc))
}

/// ドキュメントの改行インデントを増やす。
pub fn nest(indent: i64, doc: Doc) -> Doc {
    let indent = indent.max(0) as usize;
    Doc::Nest(indent, Box::new(doc))
}

/// ドキュメントを連結する。
pub fn concat(left: Doc, right: Doc) -> Doc {
    Doc::Concat(Box::new(left), Box::new(right))
}

/// 指定幅でドキュメントをレンダリングする。
pub fn render(doc: Doc, width: i64) -> String {
    let width = width.max(0) as usize;
    let mut output = String::new();
    let mut stack = vec![DocFrame {
        indent: 0,
        mode: Mode::Break,
        doc,
    }];
    let mut column = 0usize;

    while let Some(frame) = stack.pop() {
        match frame.doc {
            Doc::Text(value) => {
                column += text_width(&value);
                output.push_str(&value);
            }
            Doc::Line => {
                output.push('\n');
                push_indent(&mut output, frame.indent);
                column = frame.indent;
            }
            Doc::Softline => match frame.mode {
                Mode::Flat => {
                    output.push(' ');
                    column += 1;
                }
                Mode::Break => {
                    output.push('\n');
                    push_indent(&mut output, frame.indent);
                    column = frame.indent;
                }
            },
            Doc::Concat(left, right) => {
                stack.push(DocFrame {
                    indent: frame.indent,
                    mode: frame.mode,
                    doc: *right,
                });
                stack.push(DocFrame {
                    indent: frame.indent,
                    mode: frame.mode,
                    doc: *left,
                });
            }
            Doc::Nest(indent, doc) => {
                let indent = frame.indent.saturating_add(indent);
                stack.push(DocFrame {
                    indent,
                    mode: frame.mode,
                    doc: *doc,
                });
            }
            Doc::Group(doc) => {
                let doc = *doc;
                let mut trial = stack.clone();
                trial.push(DocFrame {
                    indent: frame.indent,
                    mode: Mode::Flat,
                    doc: doc.clone(),
                });
                let remaining = width as isize - column as isize;
                let flat = fits(remaining, &trial);
                stack.push(DocFrame {
                    indent: frame.indent,
                    mode: if flat { Mode::Flat } else { Mode::Break },
                    doc,
                });
            }
        }
    }

    output
}

fn fits(remaining: isize, stack: &[DocFrame]) -> bool {
    let mut remaining = remaining;
    let mut stack = stack.to_vec();

    while let Some(frame) = stack.pop() {
        if remaining < 0 {
            return false;
        }
        match frame.doc {
            Doc::Text(value) => {
                remaining -= text_width(&value) as isize;
            }
            Doc::Line => return true,
            Doc::Softline => match frame.mode {
                Mode::Flat => remaining -= 1,
                Mode::Break => return true,
            },
            Doc::Concat(left, right) => {
                stack.push(DocFrame {
                    indent: frame.indent,
                    mode: frame.mode,
                    doc: *right,
                });
                stack.push(DocFrame {
                    indent: frame.indent,
                    mode: frame.mode,
                    doc: *left,
                });
            }
            Doc::Nest(indent, doc) => {
                let indent = frame.indent.saturating_add(indent);
                stack.push(DocFrame {
                    indent,
                    mode: frame.mode,
                    doc: *doc,
                });
            }
            Doc::Group(doc) => {
                stack.push(DocFrame {
                    indent: frame.indent,
                    mode: frame.mode,
                    doc: *doc,
                });
            }
        }
    }

    true
}

fn text_width(value: &str) -> usize {
    let str_ref = Str::from(value);
    str_ref
        .iter_graphemes()
        .map(|grapheme| UnicodeWidthStr::width(grapheme).max(1))
        .sum()
}

fn push_indent(output: &mut String, indent: usize) {
    if indent == 0 {
        return;
    }
    output.push_str(&" ".repeat(indent));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_group_uses_flat_layout_when_it_fits() {
        let doc = group(concat(
            text("let"),
            nest(2, concat(softline(), text("x = 1"))),
        ));
        assert_eq!(render(doc.clone(), 10), "let x = 1");
        assert_eq!(render(doc, 5), "let\n  x = 1");
    }
}
