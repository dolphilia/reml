use crate::text::{self, Str};

use super::{validate_input, PathError, PathErrorKind, PathResult, PathStyle};

/// 入力文字列を指定スタイルで正規化する。
pub fn normalize_path_str(input: Str<'_>, style: PathStyle) -> PathResult<Str<'static>> {
    let effective = EffectiveStyle::from(style);
    let normalized = normalize_internal(input.as_str(), effective)?;
    let rendered = normalized.render();
    text::record_text_mem_copy(rendered.len());
    Ok(Str::owned(rendered))
}

/// 文字列リストを結合して正規化する。
pub fn join_paths_str(parts: &[Str<'_>], style: PathStyle) -> PathResult<Str<'static>> {
    if parts.is_empty() {
        return Err(PathError::new(
            PathErrorKind::Empty,
            "join_paths requires at least one segment",
        ));
    }
    let effective = EffectiveStyle::from(style);
    let mut buffer = String::new();
    let mut has_segment = false;
    for part in parts {
        let raw = part.as_str();
        if raw.is_empty() {
            continue;
        }
        validate_input(raw)?;
        if is_absolute_component(raw, effective) || !has_segment {
            buffer.clear();
            buffer.push_str(raw);
            has_segment = true;
            continue;
        }
        if !buffer.is_empty() && !ends_with_separator(&buffer, effective) {
            buffer.push(effective.separator_char());
        }
        let trimmed = trim_leading_separators(raw, effective);
        buffer.push_str(trimmed);
        has_segment = true;
    }
    if !has_segment {
        return Err(PathError::new(
            PathErrorKind::Empty,
            "join_paths requires at least one non-empty segment",
        ));
    }
    normalize_path_str(Str::from(buffer.as_str()), style)
}

/// 文字列ベースで絶対パスかどうかを判定する。
pub fn is_absolute_str(text: Str<'_>, style: PathStyle) -> bool {
    let effective = EffectiveStyle::from(style);
    is_absolute_component(text.as_str(), effective)
}

/// base から target への相対パスを計算する。
pub fn relative_to(base: Str<'_>, target: Str<'_>, style: PathStyle) -> PathResult<Str<'static>> {
    let effective = EffectiveStyle::from(style);
    let base_path = normalize_internal(base.as_str(), effective)?;
    let target_path = normalize_internal(target.as_str(), effective)?;
    if base_path.root != target_path.root {
        let rendered = target_path.render();
        text::record_text_mem_copy(rendered.len());
        return Ok(Str::owned(rendered));
    }
    let shared = shared_prefix_len(&base_path, &target_path);
    let mut components: Vec<String> = Vec::new();
    let remaining_base = base_path.components.len().saturating_sub(shared);
    for _ in 0..remaining_base {
        components.push("..".to_string());
    }
    for comp in target_path.components.iter().skip(shared) {
        components.push(comp.clone());
    }
    if components.is_empty() {
        components.push(".".to_string());
    }
    let relative_path = NormalizedPath {
        style: effective,
        root: PathRoot::None,
        components,
        is_absolute: false,
    };
    let rendered = relative_path.render();
    text::record_text_mem_copy(rendered.len());
    Ok(Str::owned(rendered))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EffectiveStyle {
    Posix,
    Windows,
}

impl EffectiveStyle {
    fn separator_char(self) -> char {
        match self {
            EffectiveStyle::Posix => '/',
            EffectiveStyle::Windows => '\\',
        }
    }

    fn alt_separator_char(self) -> Option<char> {
        match self {
            EffectiveStyle::Posix => None,
            EffectiveStyle::Windows => Some('/'),
        }
    }
}

impl From<PathStyle> for EffectiveStyle {
    fn from(value: PathStyle) -> Self {
        match value {
            PathStyle::Native => {
                if cfg!(windows) {
                    EffectiveStyle::Windows
                } else {
                    EffectiveStyle::Posix
                }
            }
            PathStyle::Posix => EffectiveStyle::Posix,
            PathStyle::Windows => EffectiveStyle::Windows,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct NormalizedPath {
    style: EffectiveStyle,
    root: PathRoot,
    components: Vec<String>,
    is_absolute: bool,
}

impl NormalizedPath {
    fn render(&self) -> String {
        match self.style {
            EffectiveStyle::Posix => self.render_posix(),
            EffectiveStyle::Windows => self.render_windows(),
        }
    }

    fn render_posix(&self) -> String {
        if self.is_absolute {
            if self.components.is_empty() {
                "/".to_string()
            } else {
                let mut out = String::from("/");
                out.push_str(&self.components.join("/"));
                out
            }
        } else if self.components.is_empty() {
            ".".to_string()
        } else {
            self.components.join("/")
        }
    }

    fn render_windows(&self) -> String {
        let mut out = String::new();
        match &self.root {
            PathRoot::WindowsDrive { drive, absolute } => {
                out.push(drive.to_ascii_uppercase());
                out.push(':');
                if *absolute {
                    out.push('\\');
                }
            }
            PathRoot::WindowsUnc { server, share } => {
                out.push_str(r"\\");
                out.push_str(server);
                out.push('\\');
                out.push_str(share);
            }
            PathRoot::WindowsRootRelative => {
                out.push('\\');
            }
            PathRoot::Posix | PathRoot::None => {}
        }
        if !self.components.is_empty() {
            if !out.is_empty() && !out.ends_with('\\') {
                out.push('\\');
            }
            for (idx, comp) in self.components.iter().enumerate() {
                if idx > 0 {
                    out.push('\\');
                }
                out.push_str(comp);
            }
        } else if out.is_empty() {
            return ".".to_string();
        }
        out
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum PathRoot {
    None,
    Posix,
    WindowsDrive { drive: char, absolute: bool },
    WindowsUnc { server: String, share: String },
    WindowsRootRelative,
}

fn normalize_internal(value: &str, style: EffectiveStyle) -> PathResult<NormalizedPath> {
    match style {
        EffectiveStyle::Posix => normalize_posix(value),
        EffectiveStyle::Windows => normalize_windows(value),
    }
}

fn normalize_posix(value: &str) -> PathResult<NormalizedPath> {
    validate_input(value)?;
    let is_absolute = value.starts_with('/');
    let mut components = Vec::new();
    for component in value.split('/') {
        apply_component(&mut components, component, is_absolute);
    }
    Ok(NormalizedPath {
        style: EffectiveStyle::Posix,
        root: if is_absolute {
            PathRoot::Posix
        } else {
            PathRoot::None
        },
        components,
        is_absolute,
    })
}

fn normalize_windows(value: &str) -> PathResult<NormalizedPath> {
    validate_input(value)?;
    let mut cursor = value;
    let mut root = PathRoot::None;
    let mut is_absolute = false;

    if cursor.starts_with(r"\\") || cursor.starts_with("//") {
        let after = &cursor[2..];
        let (server, rest) = take_component(after);
        let (share, rest_after_share) = take_component(rest);
        if server.is_empty() || share.is_empty() {
            return Err(PathError::new(
                PathErrorKind::InvalidEncoding,
                "UNC path is missing server/share",
            )
            .with_input(value));
        }
        root = PathRoot::WindowsUnc {
            server: server.to_string(),
            share: share.to_string(),
        };
        cursor = rest_after_share;
        is_absolute = true;
    } else if is_drive_prefix(cursor) {
        let drive = cursor.chars().next().unwrap().to_ascii_uppercase();
        cursor = &cursor[2..];
        if cursor.starts_with('\\') || cursor.starts_with('/') {
            cursor = trim_leading_separators(cursor, EffectiveStyle::Windows);
            is_absolute = true;
            root = PathRoot::WindowsDrive {
                drive,
                absolute: true,
            };
        } else {
            root = PathRoot::WindowsDrive {
                drive,
                absolute: false,
            };
        }
    } else if cursor.starts_with('\\') || cursor.starts_with('/') {
        cursor = trim_leading_separators(cursor, EffectiveStyle::Windows);
        root = PathRoot::WindowsRootRelative;
        is_absolute = true;
    }

    let mut components = Vec::new();
    if !cursor.is_empty() {
        for component in split_components(cursor, EffectiveStyle::Windows) {
            apply_component(&mut components, component, is_absolute);
        }
    }

    Ok(NormalizedPath {
        style: EffectiveStyle::Windows,
        root,
        components,
        is_absolute,
    })
}

fn split_components<'a>(mut input: &'a str, style: EffectiveStyle) -> Vec<&'a str> {
    let mut segments = Vec::new();
    while !input.is_empty() {
        input = trim_leading_separators(input, style);
        if input.is_empty() {
            break;
        }
        let (next, rest) = take_until_separator(input, style);
        segments.push(next);
        input = rest;
    }
    segments
}

fn apply_component(components: &mut Vec<String>, segment: &str, is_absolute: bool) {
    if segment.is_empty() || segment == "." {
        return;
    }
    if segment == ".." {
        if let Some(last) = components.last() {
            if last != ".." {
                components.pop();
                return;
            }
        }
        if !is_absolute {
            components.push("..".to_string());
        }
        return;
    }
    components.push(segment.to_string());
}

fn is_absolute_component(value: &str, style: EffectiveStyle) -> bool {
    match style {
        EffectiveStyle::Posix => value.starts_with('/'),
        EffectiveStyle::Windows => {
            value.starts_with(r"\\")
                || value.starts_with("//")
                || value.starts_with('\\')
                || value.starts_with('/')
                || (is_drive_prefix(value)
                    && value
                        .chars()
                        .nth(2)
                        .map(|c| is_separator(c, style))
                        .unwrap_or(false))
        }
    }
}

fn ends_with_separator(value: &str, style: EffectiveStyle) -> bool {
    value
        .chars()
        .rev()
        .next()
        .map(|ch| is_separator(ch, style))
        .unwrap_or(false)
}

fn trim_leading_separators<'a>(mut value: &'a str, style: EffectiveStyle) -> &'a str {
    while let Some(ch) = value.chars().next() {
        if is_separator(ch, style) {
            value = &value[ch.len_utf8()..];
        } else {
            break;
        }
    }
    value
}

fn take_until_separator<'a>(value: &'a str, style: EffectiveStyle) -> (&'a str, &'a str) {
    for (idx, ch) in value.char_indices() {
        if is_separator(ch, style) {
            return (&value[..idx], &value[idx..]);
        }
    }
    (value, "")
}

fn take_component<'a>(value: &'a str) -> (&'a str, &'a str) {
    let trimmed = trim_leading_separators(value, EffectiveStyle::Windows);
    if trimmed.is_empty() {
        ("", "")
    } else {
        let (segment, rest) = take_until_separator(trimmed, EffectiveStyle::Windows);
        if rest.is_empty() {
            (segment, "")
        } else {
            (segment, rest)
        }
    }
}

fn is_drive_prefix(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() < 2 {
        return false;
    }
    bytes[1] == b':' && bytes[0].is_ascii_alphabetic()
}

fn is_separator(ch: char, style: EffectiveStyle) -> bool {
    ch == style.separator_char()
        || style
            .alt_separator_char()
            .map(|alt| ch == alt)
            .unwrap_or(false)
}

fn shared_prefix_len(base: &NormalizedPath, target: &NormalizedPath) -> usize {
    let max_idx = base.components.len().min(target.components.len());
    let mut shared = 0;
    while shared < max_idx {
        let left = &base.components[shared];
        let right = &target.components[shared];
        let matches = match base.style {
            EffectiveStyle::Posix => left == right,
            EffectiveStyle::Windows => left.eq_ignore_ascii_case(right),
        };
        if matches {
            shared += 1;
        } else {
            break;
        }
    }
    shared
}
