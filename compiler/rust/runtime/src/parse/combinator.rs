use super::cst::{CstBuilder, CstNode, CstOutput, Token as CstToken, Trivia, TriviaKind};
use super::embedded::{shift_position, ContextBridge, EmbeddedDslSpec, EmbeddedNode};
use super::meta::{normalize_doc, ObservedToken, ParseMetaRegistry, ParserMetaKind};
use super::op_builder::FixitySymbol;
use crate::prelude::ensure::{DiagnosticNote, DiagnosticSeverity, GuardDiagnostic};
use crate::run_config::{LeftRecursionStrategy, RunConfig};
use crate::text::{Str, String as TextString};
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::{json, Map, Value};
use std::any::Any;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use unicode_ident::{is_xid_continue, is_xid_start};
use unicode_normalization::{is_nfc_quick, IsNormalized};

#[cfg(feature = "metrics")]
use crate::diagnostics::apply_dsl_metadata;

#[cfg(not(feature = "metrics"))]
fn apply_dsl_metadata(
    diagnostic: &mut GuardDiagnostic,
    dsl_id: &str,
    parent_id: Option<&str>,
    span: Span,
) {
    let mut span_obj = Map::new();
    let mut start_obj = Map::new();
    start_obj.insert("byte".into(), Value::from(span.start.byte as u64));
    start_obj.insert("line".into(), Value::from(span.start.line as u64));
    start_obj.insert("column".into(), Value::from(span.start.column as u64));
    let mut end_obj = Map::new();
    end_obj.insert("byte".into(), Value::from(span.end.byte as u64));
    end_obj.insert("line".into(), Value::from(span.end.line as u64));
    end_obj.insert("column".into(), Value::from(span.end.column as u64));
    span_obj.insert("start".into(), Value::Object(start_obj));
    span_obj.insert("end".into(), Value::Object(end_obj));
    let span_payload = Value::Object(span_obj);

    diagnostic
        .extensions
        .insert("source_dsl".into(), Value::String(dsl_id.to_string()));
    let mut dsl_extension = Map::new();
    dsl_extension.insert("id".into(), Value::String(dsl_id.to_string()));
    dsl_extension.insert(
        "parent_id".into(),
        parent_id
            .map(|value| Value::String(value.to_string()))
            .unwrap_or(Value::Null),
    );
    dsl_extension.insert("embedding_span".into(), span_payload.clone());
    diagnostic
        .extensions
        .insert("dsl".into(), Value::Object(dsl_extension));
    diagnostic
        .audit_metadata
        .insert("dsl.id".into(), Value::String(dsl_id.to_string()));
    if let Some(parent_id) = parent_id {
        diagnostic
            .audit_metadata
            .insert("dsl.parent_id".into(), Value::String(parent_id.to_string()));
    }
    diagnostic
        .audit_metadata
        .insert("dsl.embedding.span".into(), span_payload);
}

/// Packrat メモキー。
pub type MemoKey = (ParserId, usize);

/// 左再帰検出で利用するエラー文言。
pub const LEFT_RECURSION_MESSAGE: &str = "left recursion";

/// Packrat メモ値（型消去して格納する）。
pub type MemoEntry = Box<dyn Any + Send + Sync>;

/// Packrat メモテーブル。
pub type MemoTable = HashMap<MemoKey, MemoEntry>;

#[derive(Clone)]
struct MemoizedReply<T: Clone> {
    reply: Reply<T>,
}

impl<T: Clone> MemoizedReply<T> {
    fn clone_reply(&self) -> Reply<T> {
        self.reply.clone()
    }
}

/// パーサー ID。診断や Packrat キーに利用する。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ParserId(u32);

impl ParserId {
    /// 新しい ID を生成する。`rule` などで固定 ID を与える場合は `from_raw` を利用する。
    pub fn fresh() -> Self {
        static NEXT_ID: AtomicU32 = AtomicU32::new(1);
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }

    /// 外部で決めた ID を固定化する。
    pub fn from_raw(value: u32) -> Self {
        Self(value)
    }

    pub fn as_u32(self) -> u32 {
        self.0
    }
}

/// 現在位置を表す。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputPosition {
    pub byte: usize,
    pub line: usize,
    pub column: usize,
}

/// 入力ビュー。Arc で共有しつつオフセットのみを進める。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Input {
    source: Arc<str>,
    byte_offset: usize,
    line: usize,
    column: usize,
}

impl Input {
    pub fn new(source: impl AsRef<str>) -> Self {
        Self::from_arc_str(Arc::<str>::from(source.as_ref()))
    }

    pub fn from_arc_str(source: Arc<str>) -> Self {
        Self {
            source,
            byte_offset: 0,
            line: 1,
            column: 1,
        }
    }

    pub fn remaining_checked(&self) -> Option<&str> {
        self.source.get(self.byte_offset..)
    }

    pub fn remaining(&self) -> &str {
        self.remaining_checked().expect(
            "Input.byte_offset が UTF-8 境界ではありません（Input::advance の誤用の可能性）",
        )
    }

    pub fn is_empty(&self) -> bool {
        self.byte_offset >= self.source.len()
    }

    pub fn position(&self) -> InputPosition {
        InputPosition {
            byte: self.byte_offset,
            line: self.line,
            column: self.column,
        }
    }

    pub fn byte_offset(&self) -> usize {
        self.byte_offset
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn column(&self) -> usize {
        self.column
    }

    /// 指定バイト数だけ入力を進めた新しいビューを返す。
    pub fn advance(&self, bytes: usize) -> Self {
        let available = self.source.len().saturating_sub(self.byte_offset);
        let step = bytes.min(available);
        let start = self.byte_offset;
        let end = self.byte_offset + step;
        debug_assert!(
            self.source.is_char_boundary(start),
            "Input.byte_offset が UTF-8 境界ではありません: {start}"
        );
        debug_assert!(
            self.source.is_char_boundary(end),
            "Input.advance(bytes={bytes}) により UTF-8 境界でない位置へ進もうとしました: {end}"
        );
        let slice = self.source.get(start..end).expect(
            "Input.advance の範囲が UTF-8 境界ではありません（advance(bytes) の誤用の可能性）",
        );
        let mut line = self.line;
        let mut column = self.column;

        if slice.is_ascii() {
            let mut last_newline = None;
            let mut newline_count = 0usize;
            for (idx, b) in slice.as_bytes().iter().enumerate() {
                if *b == b'\n' {
                    newline_count += 1;
                    last_newline = Some(idx);
                }
            }
            if newline_count > 0 {
                line += newline_count;
                column = 1;
                let tail_len = slice.len().saturating_sub(last_newline.unwrap_or(0) + 1);
                column += tail_len;
            } else {
                column += slice.len();
            }
        } else {
            let mut last_break = 0usize;
            for (idx, ch) in slice.char_indices() {
                if ch == '\n' {
                    line += 1;
                    column = 1;
                    last_break = idx + ch.len_utf8();
                }
            }
            let tail = &slice[last_break..];
            let graphemes = if tail.is_ascii() {
                tail.len()
            } else {
                Str::from(tail).iter_graphemes().count()
            };
            column += graphemes;
        }

        Self {
            source: Arc::clone(&self.source),
            byte_offset: self.byte_offset + step,
            line,
            column,
        }
    }

    pub fn span_to(&self, end: &Input) -> Span {
        Span::new(self.position(), end.position())
    }
}

/// 入力範囲を表す。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: InputPosition,
    pub end: InputPosition,
}

impl Span {
    pub fn new(start: InputPosition, end: InputPosition) -> Self {
        Self { start, end }
    }
}

fn empty_span(input: &Input) -> Span {
    let pos = input.position();
    Span::new(pos, pos)
}

fn span_from_inputs(start: &Input, end: &Input) -> Span {
    Span::new(start.position(), end.position())
}

fn slice_input_text(start: &Input, end: &Input) -> Option<TextString> {
    if start.byte_offset > end.byte_offset {
        return None;
    }
    start
        .source
        .get(start.byte_offset..end.byte_offset)
        .map(TextString::from)
}

fn parser_id_from_name(name: &str) -> ParserId {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    name.hash(&mut hasher);
    let raw = hasher.finish() as u32;
    // 0 を避けるため 1 を足す。
    ParserId::from_raw(raw.wrapping_add(1))
}

static EXTENDED_PICTOGRAPHIC_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\p{Extended_Pictographic}$").expect("emoji regex init failed"));
static EMOJI_COMPONENT_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\p{Emoji_Component}$").expect("emoji regex init failed"));

/// 簡易的な識別子継続文字の判定。
fn is_ident_start(ch: char, profile: IdentifierProfile) -> bool {
    match profile {
        IdentifierProfile::Unicode => ch == '_' || is_xid_start(ch),
        IdentifierProfile::AsciiCompat => ch == '_' || ch.is_ascii_alphabetic(),
    }
}

fn is_ident_continue(ch: char, profile: IdentifierProfile) -> bool {
    match profile {
        IdentifierProfile::Unicode => {
            ch == '_'
                || is_xid_continue(ch)
                || matches!(ch, '\u{200D}' | '\u{FE0F}')
                || is_extended_pictographic(ch)
                || is_emoji_component(ch)
        }
        IdentifierProfile::AsciiCompat => ch == '_' || ch.is_ascii_alphanumeric(),
    }
}

fn is_extended_pictographic(ch: char) -> bool {
    let mut buf = [0u8; 4];
    let s = ch.encode_utf8(&mut buf);
    EXTENDED_PICTOGRAPHIC_RE.is_match(s)
}

fn is_emoji_component(ch: char) -> bool {
    let mut buf = [0u8; 4];
    let s = ch.encode_utf8(&mut buf);
    EMOJI_COMPONENT_RE.is_match(s)
}

fn is_bidi_control(ch: char) -> bool {
    matches!(
        ch,
        '\u{061C}' // ARABIC LETTER MARK
            | '\u{200E}' // LRM
            | '\u{200F}' // RLM
            | '\u{202A}'..='\u{202E}' // LRE/RLE/PDF/LRO/RLO
            | '\u{2066}'..='\u{2069}' // LRI/RLI/FSI/PDI
    )
}

fn is_nfc_char(ch: char) -> bool {
    let mut buf = [0u8; 4];
    let s = ch.encode_utf8(&mut buf);
    matches!(is_nfc_quick(s.chars()), IsNormalized::Yes)
}

/// レイアウト（オフサイド）プロファイル。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutProfile {
    pub indent_token: String,
    pub dedent_token: String,
    pub newline_token: String,
    pub offside: bool,
    pub allow_mixed_tabs: bool,
}

impl Default for LayoutProfile {
    fn default() -> Self {
        Self {
            indent_token: "<indent>".to_string(),
            dedent_token: "<dedent>".to_string(),
            newline_token: "<newline>".to_string(),
            offside: false,
            allow_mixed_tabs: false,
        }
    }
}

/// autoWhitespace 設定。
#[derive(Clone, Debug)]
pub struct AutoWhitespaceConfig {
    /// Lex プロファイル由来の空白パーサ（未指定なら RunConfig/現行の space を利用）。
    pub profile: Option<Parser<()>>,
    /// レイアウト（オフサイド）プロファイル。
    pub layout: Option<LayoutProfile>,
    /// RunConfig 優先/強制/無効化の切替。
    pub strategy: AutoWhitespaceStrategy,
}

impl Default for AutoWhitespaceConfig {
    fn default() -> Self {
        Self {
            profile: None,
            layout: None,
            strategy: AutoWhitespaceStrategy::PreferRunConfig,
        }
    }
}

/// autoWhitespace の戦略。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutoWhitespaceStrategy {
    /// RunConfig.extensions["lex"] を優先し、無ければ profile を利用。
    PreferRunConfig,
    /// RunConfig を無視して profile を強制。
    ForceProfile,
    /// Lex ブリッジを無効化し、現行 space/profile のみ利用。
    NoLexBridge,
}

/// パース時に収集するメトリクス。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ParserProfile {
    pub packrat_hits: u64,
    pub packrat_misses: u64,
    pub backtracks: u64,
    pub recoveries: u64,
    pub left_recursion_guard_hits: u64,
    pub memo_entries: usize,
}

impl ParserProfile {
    pub fn to_json(&self) -> Value {
        json!({
            "packrat_hits": self.packrat_hits,
            "packrat_misses": self.packrat_misses,
            "backtracks": self.backtracks,
            "recoveries": self.recoveries,
            "left_recursion_guard_hits": self.left_recursion_guard_hits,
            "memo_entries": self.memo_entries,
        })
    }
}

#[derive(Clone, Debug)]
struct ParseObserver {
    profile: ParserProfile,
    enabled: bool,
    profile_output: Option<PathBuf>,
}

impl ParseObserver {
    fn new(enabled: bool, profile_output: Option<PathBuf>) -> Self {
        Self {
            profile: ParserProfile::default(),
            enabled,
            profile_output,
        }
    }

    fn record_packrat_hit(&mut self) {
        if self.enabled {
            self.profile.packrat_hits += 1;
        }
    }

    fn record_packrat_miss(&mut self) {
        if self.enabled {
            self.profile.packrat_misses += 1;
        }
    }

    fn record_backtrack(&mut self) {
        if self.enabled {
            self.profile.backtracks += 1;
        }
    }

    fn record_recovery(&mut self) {
        if self.enabled {
            self.profile.recoveries += 1;
        }
    }

    fn record_left_recursion_guard(&mut self) {
        if self.enabled {
            self.profile.left_recursion_guard_hits += 1;
        }
    }

    fn finalize(mut self, memo_entries: usize) -> Option<(ParserProfile, Option<PathBuf>)> {
        if !self.enabled {
            return None;
        }
        self.profile.memo_entries = memo_entries;
        Some((self.profile, self.profile_output))
    }
}

#[derive(Default)]
struct ParseProfileConfig {
    enabled: bool,
    profile_output: Option<PathBuf>,
}

fn decode_profile_config(run_config: &RunConfig) -> ParseProfileConfig {
    let mut config = ParseProfileConfig {
        enabled: run_config.profile,
        profile_output: None,
    };
    if let Some(parse) = run_config.extensions.get("parse") {
        if let Some(enabled) = parse.get("profile").and_then(|v| v.as_bool()) {
            config.enabled |= enabled;
        }
        if let Some(output) = parse.get("profile_output").and_then(|v| v.as_str()) {
            config.enabled = true;
            config.profile_output = Some(PathBuf::from(output));
        }
    }
    config
}

fn decode_cst_mode(run_config: &RunConfig) -> bool {
    run_config
        .extensions
        .get("parse")
        .and_then(|parse| parse.get("cst"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn enable_cst_config(run_config: &RunConfig) -> RunConfig {
    run_config.with_extension("parse", |mut ext| {
        ext.insert("cst".into(), Value::Bool(true));
        ext
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RecoverMode {
    Off,
    Collect,
}

impl Default for RecoverMode {
    fn default() -> Self {
        Self::Off
    }
}

#[derive(Clone, Debug, Default)]
struct RecoverConfig {
    mode: RecoverMode,
    sync_tokens: Vec<String>,
    max_diagnostics: Option<usize>,
    max_resync_bytes: Option<usize>,
    max_recoveries: Option<usize>,
    notes: bool,
}

fn decode_recover_config(run_config: &RunConfig) -> RecoverConfig {
    let mut config = RecoverConfig::default();
    let recover = match run_config.extensions.get("recover") {
        Some(ext) => ext,
        None => return config,
    };

    config.mode = recover
        .get("mode")
        .and_then(Value::as_str)
        .map(|raw| match raw.to_ascii_lowercase().as_str() {
            "collect" => RecoverMode::Collect,
            _ => RecoverMode::Off,
        })
        .unwrap_or_default();

    if let Some(tokens) = recover.get("sync_tokens") {
        match tokens {
            Value::Array(values) => {
                config.sync_tokens = values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(|token| token.to_string())
                    .filter(|token| !token.is_empty())
                    .collect();
            }
            Value::String(token) if !token.is_empty() => {
                config.sync_tokens = vec![token.clone()];
            }
            _ => {}
        }
    }

    config.max_diagnostics = recover
        .get("max_diagnostics")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok());
    config.max_resync_bytes = recover
        .get("max_resync_bytes")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok());
    config.max_recoveries = recover
        .get("max_recoveries")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok());

    config.notes = recover
        .get("notes")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    config
}

/// `RunConfig.extensions["lex"].identifier_profile` から派生した識別子プロファイル。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdentifierProfile {
    Unicode,
    AsciiCompat,
}

impl IdentifierProfile {
    fn from_run_config(cfg: &RunConfig) -> Self {
        cfg.extensions
            .get("lex")
            .and_then(|ext| ext.get("identifier_profile"))
            .and_then(|value| value.as_str())
            .map(|label| match label {
                "ascii-compat" => IdentifierProfile::AsciiCompat,
                _ => IdentifierProfile::Unicode,
            })
            .unwrap_or(IdentifierProfile::Unicode)
    }

    fn validate_char(&self, ch: char) -> Result<(), &'static str> {
        if is_bidi_control(ch) {
            return Err("識別子に Bidi 制御文字は使用できません");
        }
        if !is_nfc_char(ch) {
            return Err("識別子は NFC 正規化されている必要があります");
        }
        Ok(())
    }
}

/// 優先度ビルダーで利用する単項/二項演算子の型。
pub type UnaryOp<T> = fn(T) -> T;
pub type BinaryOp<T> = fn(T, T) -> T;
pub type TernaryBuild<T> = fn(T, T, T) -> T;

#[derive(Clone)]
pub struct TernaryOp<T> {
    pub head: Parser<()>,
    pub mid: Parser<()>,
    pub build: TernaryBuild<T>,
}

/// 演算子レベルの設定。
#[derive(Clone)]
pub struct ExprOpLevel<T> {
    pub prefix: Vec<Parser<UnaryOp<T>>>,
    pub postfix: Vec<Parser<UnaryOp<T>>>,
    pub infixl: Vec<Parser<BinaryOp<T>>>,
    pub infixr: Vec<Parser<BinaryOp<T>>>,
    pub infixn: Vec<Parser<BinaryOp<T>>>,
    pub ternary: Vec<TernaryOp<T>>,
}

impl<T: Clone + Send + Sync + 'static> ExprOpLevel<T> {
    fn with_space(&self, space: &Option<Parser<()>>) -> Self {
        let apply_unary = |ops: &Vec<Parser<UnaryOp<T>>>| {
            if let Some(sp) = space.clone() {
                ops.iter()
                    .cloned()
                    .map(|p| p.with_space(sp.clone()))
                    .collect::<Vec<_>>()
            } else {
                ops.clone()
            }
        };
        let apply_binary = |ops: &Vec<Parser<BinaryOp<T>>>| {
            if let Some(sp) = space.clone() {
                ops.iter()
                    .cloned()
                    .map(|p| p.with_space(sp.clone()))
                    .collect::<Vec<_>>()
            } else {
                ops.clone()
            }
        };
        let apply_ternary = |ops: &Vec<TernaryOp<T>>| {
            if let Some(sp) = space.clone() {
                ops.iter()
                    .cloned()
                    .map(|op| TernaryOp {
                        head: op.head.clone().with_space(sp.clone()),
                        mid: op.mid.clone().with_space(sp.clone()),
                        build: op.build,
                    })
                    .collect::<Vec<_>>()
            } else {
                ops.clone()
            }
        };
        Self {
            prefix: apply_unary(&self.prefix),
            postfix: apply_unary(&self.postfix),
            infixl: apply_binary(&self.infixl),
            infixr: apply_binary(&self.infixr),
            infixn: apply_binary(&self.infixn),
            ternary: apply_ternary(&self.ternary),
        }
    }

    fn split_by_fixity(&self) -> Vec<(FixitySymbol, ExprOpLevel<T>)> {
        let mut parts = Vec::new();
        if !self.prefix.is_empty() {
            parts.push((
                FixitySymbol::Prefix,
                ExprOpLevel {
                    prefix: self.prefix.clone(),
                    postfix: Vec::new(),
                    infixl: Vec::new(),
                    infixr: Vec::new(),
                    infixn: Vec::new(),
                    ternary: Vec::new(),
                },
            ));
        }
        if !self.postfix.is_empty() {
            parts.push((
                FixitySymbol::Postfix,
                ExprOpLevel {
                    prefix: Vec::new(),
                    postfix: self.postfix.clone(),
                    infixl: Vec::new(),
                    infixr: Vec::new(),
                    infixn: Vec::new(),
                    ternary: Vec::new(),
                },
            ));
        }
        if !self.infixl.is_empty() {
            parts.push((
                FixitySymbol::InfixLeft,
                ExprOpLevel {
                    prefix: Vec::new(),
                    postfix: Vec::new(),
                    infixl: self.infixl.clone(),
                    infixr: Vec::new(),
                    infixn: Vec::new(),
                    ternary: Vec::new(),
                },
            ));
        }
        if !self.infixr.is_empty() {
            parts.push((
                FixitySymbol::InfixRight,
                ExprOpLevel {
                    prefix: Vec::new(),
                    postfix: Vec::new(),
                    infixl: Vec::new(),
                    infixr: self.infixr.clone(),
                    infixn: Vec::new(),
                    ternary: Vec::new(),
                },
            ));
        }
        if !self.infixn.is_empty() {
            parts.push((
                FixitySymbol::InfixNonassoc,
                ExprOpLevel {
                    prefix: Vec::new(),
                    postfix: Vec::new(),
                    infixl: Vec::new(),
                    infixr: Vec::new(),
                    infixn: self.infixn.clone(),
                    ternary: Vec::new(),
                },
            ));
        }
        if !self.ternary.is_empty() {
            parts.push((
                FixitySymbol::Ternary,
                ExprOpLevel {
                    prefix: Vec::new(),
                    postfix: Vec::new(),
                    infixl: Vec::new(),
                    infixr: Vec::new(),
                    infixn: Vec::new(),
                    ternary: self.ternary.clone(),
                },
            ));
        }
        parts
    }
}

impl<T> Default for ExprOpLevel<T> {
    fn default() -> Self {
        Self {
            prefix: Vec::new(),
            postfix: Vec::new(),
            infixl: Vec::new(),
            infixr: Vec::new(),
            infixn: Vec::new(),
            ternary: Vec::new(),
        }
    }
}

/// 優先度ビルダーの commit スタイル。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExprCommit {
    Preserve,
    CommitOperators,
}

/// 優先度ビルダーのコンフィグ。
#[derive(Clone)]
pub struct ExprBuilderConfig {
    pub space: Option<Parser<()>>,
    pub operand_label: Option<String>,
    pub commit_style: ExprCommit,
}

impl Default for ExprBuilderConfig {
    fn default() -> Self {
        Self {
            space: None,
            operand_label: None,
            commit_style: ExprCommit::Preserve,
        }
    }
}

#[derive(Clone, Debug)]
struct OperatorTableOverride {
    fixities: Vec<FixitySymbol>,
    commit_operators: Option<bool>,
}

fn decode_operator_table_override(run_config: &RunConfig) -> Option<OperatorTableOverride> {
    let parse = run_config.extensions.get("parse")?;
    let levels = parse.get("operator_table")?.as_array()?;
    let mut fixities = Vec::new();
    for level in levels {
        if let Some(label) = level.get("fixity").and_then(Value::as_str) {
            let symbol = match label {
                ":prefix" | "prefix" => FixitySymbol::Prefix,
                ":postfix" | "postfix" => FixitySymbol::Postfix,
                ":infix_left" | "infixl" | "infix_left" => FixitySymbol::InfixLeft,
                ":infix_right" | "infixr" | "infix_right" => FixitySymbol::InfixRight,
                ":infix_nonassoc" | "infixn" | "infix_nonassoc" => FixitySymbol::InfixNonassoc,
                ":ternary" | "ternary" => FixitySymbol::Ternary,
                _ => continue,
            };
            fixities.push(symbol);
        }
    }
    let commit_operators = parse
        .get("commit_operators")
        .and_then(Value::as_bool)
        .or_else(|| parse.get("commitOperators").and_then(Value::as_bool));

    Some(OperatorTableOverride {
        fixities,
        commit_operators,
    })
}

fn reorder_levels<T: Clone + Send + Sync + 'static>(
    levels: &[ExprOpLevel<T>],
    override_fixities: &[FixitySymbol],
) -> Vec<ExprOpLevel<T>> {
    let mut buckets: HashMap<FixitySymbol, VecDeque<ExprOpLevel<T>>> = HashMap::new();
    for level in levels {
        for (fixity, part) in level.split_by_fixity() {
            buckets.entry(fixity).or_default().push_back(part);
        }
    }
    let mut reordered = Vec::new();
    for fixity in override_fixities {
        if let Some(queue) = buckets.get_mut(fixity) {
            if let Some(level) = queue.pop_front() {
                reordered.push(level);
            }
        }
    }
    for queue in buckets.values_mut() {
        while let Some(level) = queue.pop_front() {
            reordered.push(level);
        }
    }
    reordered
}

fn choice_ops<T: Clone + Send + Sync + 'static>(ops: &[Parser<T>]) -> Option<Parser<T>> {
    match ops.len() {
        0 => None,
        1 => Some(ops[0].clone()),
        _ => Some(choice(ops.to_vec())),
    }
}

fn apply_prefix_postfix<T: Clone + Send + Sync + 'static>(
    term: Parser<T>,
    prefix: Option<Parser<UnaryOp<T>>>,
    postfix: Option<Parser<UnaryOp<T>>>,
) -> Parser<T> {
    let prefix_many = prefix.map(|p| p.many()).unwrap_or_else(|| ok(Vec::new()));
    let postfix_many = postfix.map(|p| p.many()).unwrap_or_else(|| ok(Vec::new()));
    prefix_many.and_then(move |pres| {
        let postfix_many = postfix_many.clone();
        let term = term.clone();
        term.and_then(move |core| {
            let pres_clone = pres.clone();
            postfix_many.clone().map(move |posts| {
                let with_prefix = pres_clone.iter().rev().fold(core.clone(), |acc, f| f(acc));
                posts.into_iter().fold(with_prefix, |acc, f| f(acc))
            })
        })
    })
}

fn infix_nonassoc<T: Clone + Send + Sync + 'static>(
    term: Parser<T>,
    op: Parser<BinaryOp<T>>,
) -> Parser<T> {
    term.clone().and_then(move |lhs| {
        let lhs_for_ok = lhs.clone();
        let op = op.clone();
        let term = term.clone();
        op.and_then(move |f| {
            let term = term.clone();
            let lhs_for_map = lhs.clone();
            term.map(move |rhs| f(lhs_for_map.clone(), rhs))
        })
        .or(ok(lhs_for_ok))
    })
}

fn build_level<T: Clone + Send + Sync + 'static>(
    term: Parser<T>,
    level: &ExprOpLevel<T>,
    commit: ExprCommit,
) -> Parser<T> {
    let mut prefix = level.prefix.clone();
    let mut postfix = level.postfix.clone();
    let mut infixl = level.infixl.clone();
    let mut infixr = level.infixr.clone();
    let mut infixn = level.infixn.clone();
    if commit == ExprCommit::CommitOperators {
        prefix = prefix
            .into_iter()
            .map(|p| p.then(cut_here()).map(|(f, _)| f))
            .collect();
        postfix = postfix
            .into_iter()
            .map(|p| p.then(cut_here()).map(|(f, _)| f))
            .collect();
        infixl = infixl
            .into_iter()
            .map(|p| p.then(cut_here()).map(|(f, _)| f))
            .collect();
        infixr = infixr
            .into_iter()
            .map(|p| p.then(cut_here()).map(|(f, _)| f))
            .collect();
        infixn = infixn
            .into_iter()
            .map(|p| p.then(cut_here()).map(|(f, _)| f))
            .collect();
    }

    let prefix_choice = choice_ops(&prefix);
    let postfix_choice = choice_ops(&postfix);
    let infixl_choice = choice_ops(&infixl);
    let infixr_choice = choice_ops(&infixr);
    let infixn_choice = choice_ops(&infixn);
    let ternary_choice = if level.ternary.is_empty() {
        None
    } else {
        // ternary は複数あっても順序は同レベル扱い。最初にマッチしたものを使用。
        let ops = level.ternary.clone();
        let parser = Parser::new(move |state| {
            for op in ops.iter() {
                let op_parser = Parser::new({
                    let op = op.clone();
                    move |state| Reply::Ok {
                        value: op.clone(),
                        span: empty_span(state.input()),
                        consumed: false,
                        rest: state.input().clone(),
                    }
                });
                match op_parser.parse(state) {
                    ok @ Reply::Ok { .. } => return ok,
                    Reply::Err { .. } => continue,
                }
            }
            Reply::Err {
                error: ParseError::new("ternary operator not matched", state.input().position()),
                consumed: false,
                committed: false,
            }
        });
        Some(parser)
    };

    let term = apply_prefix_postfix(term, prefix_choice, postfix_choice);

    let term = if let Some(ternary) = ternary_choice {
        apply_ternary(term, ternary)
    } else {
        term
    };

    if let Some(op) = infixl_choice {
        chainl1(term, op)
    } else if let Some(op) = infixr_choice {
        chainr1(term, op)
    } else if let Some(op) = infixn_choice {
        infix_nonassoc(term, op)
    } else {
        term
    }
}

fn apply_ternary<T: Clone + Send + Sync + 'static>(
    term: Parser<T>,
    op: Parser<TernaryOp<T>>,
) -> Parser<T> {
    term.clone().and_then(move |cond| {
        let cond_for_ok = cond.clone();
        let cond_shared = Arc::new(cond.clone());
        let term_branch = term.clone();
        let op_parser = op.clone();
        op_parser
            .and_then(move |op| {
                let t_branch = term_branch.clone();
                let f_branch = term_branch.clone();
                op.head
                    .clone()
                    .then(t_branch)
                    .then(op.mid.clone())
                    .then(f_branch)
                    .map({
                        let cond_shared = cond_shared.clone();
                        move |((((), t_val), ()), f_val)| {
                            (op.build)((*cond_shared).clone(), t_val, f_val)
                        }
                    })
            })
            .or(ok(cond_for_ok))
    })
}

/// `makeExprParser` 相当の優先度ビルダー。
pub fn expr_builder<T: Clone + Send + Sync + 'static>(
    atom: Parser<T>,
    levels: Vec<ExprOpLevel<T>>,
    config: ExprBuilderConfig,
) -> Parser<T> {
    Parser::new(move |state| {
        let override_table = decode_operator_table_override(&state.run_config);
        let commit_style = match override_table.as_ref().and_then(|cfg| cfg.commit_operators) {
            Some(true) => ExprCommit::CommitOperators,
            Some(false) => ExprCommit::Preserve,
            None => config.commit_style,
        };

        let space = config.space.clone().or_else(|| state.space());
        let base_atom = match &space {
            Some(sp) => atom.clone().with_space(sp.clone()),
            None => atom.clone(),
        };

        let reordered = if let Some(cfg) = override_table {
            reorder_levels(&levels, &cfg.fixities)
        } else {
            levels.clone()
        };

        let spaced_levels: Vec<ExprOpLevel<T>> =
            reordered.iter().map(|lvl| lvl.with_space(&space)).collect();

        let mut parser = base_atom;
        for level in spaced_levels.iter() {
            parser = build_level(parser, level, commit_style);
        }
        parser.parse(state)
    })
}

fn decode_lex_space(run_config: &RunConfig) -> Option<Parser<()>> {
    let lex = run_config.extensions.get("lex")?;
    let ascii_only = lex
        .get("profile")
        .and_then(Value::as_str)
        .map(|label| label == "ascii-compat")
        .unwrap_or(false);
    let parser_id = lex
        .get("space_id")
        .and_then(Value::as_u64)
        .map(|raw| ParserId::from_raw(raw as u32))
        .unwrap_or_else(ParserId::fresh);
    let space = Parser::with_id(parser_id, move |state| {
        let start = state.input().clone();
        let mut last = None;
        for (idx, ch) in start.remaining().char_indices() {
            let is_ws = if ascii_only {
                ch.is_ascii_whitespace()
            } else {
                ch.is_whitespace()
            };
            if is_ws {
                last = Some(idx + ch.len_utf8());
            } else {
                break;
            }
        }

        if let Some(boundary) = last {
            let rest = start.advance(boundary);
            state.set_input(rest.clone());
            Reply::Ok {
                value: (),
                span: span_from_inputs(&start, &rest),
                consumed: true,
                rest,
            }
        } else {
            Reply::Ok {
                value: (),
                span: empty_span(&start),
                consumed: false,
                rest: start,
            }
        }
    });
    Some(space)
}

fn count_indent(input: &str, allow_mixed_tabs: bool) -> (usize, bool, usize) {
    let mut spaces = 0usize;
    let mut tabs = 0usize;
    let mut consumed_bytes = 0usize;
    for (idx, ch) in input.char_indices() {
        match ch {
            ' ' => spaces += 1,
            '\t' => tabs += 1,
            _ => {
                consumed_bytes = idx;
                break;
            }
        }
    }
    if consumed_bytes == 0 {
        consumed_bytes = input.len();
    }
    let mixed = spaces > 0 && tabs > 0 && !allow_mixed_tabs;
    let width = spaces + tabs;
    (width, mixed, consumed_bytes)
}

fn decode_layout_profile(run_config: &RunConfig) -> Option<LayoutProfile> {
    let lex = run_config.extensions.get("lex")?;
    let layout_value = lex.get("layout_profile")?;
    let mut profile = LayoutProfile::default();
    if let Some(obj) = layout_value.as_object() {
        if let Some(indent) = obj.get("indent_token").and_then(Value::as_str) {
            profile.indent_token = indent.to_string();
        }
        if let Some(dedent) = obj.get("dedent_token").and_then(Value::as_str) {
            profile.dedent_token = dedent.to_string();
        }
        if let Some(newline) = obj.get("newline_token").and_then(Value::as_str) {
            profile.newline_token = newline.to_string();
        }
        if let Some(offside) = obj.get("offside").and_then(Value::as_bool) {
            profile.offside = offside;
        }
        if let Some(mixed) = obj.get("allow_mixed_tabs").and_then(Value::as_bool) {
            profile.allow_mixed_tabs = mixed;
        }
        Some(profile)
    } else {
        None
    }
}

/// パースエラーの骨組み。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub position: InputPosition,
    pub source_dsl: Option<String>,
    pub expected_tokens: Vec<String>,
    pub recover: Option<RecoverMeta>,
    pub fixits: Vec<ParseFixIt>,
    pub notes: Vec<String>,
}

impl ParseError {
    pub fn new(message: impl Into<String>, position: InputPosition) -> Self {
        Self {
            message: message.into(),
            position,
            source_dsl: None,
            expected_tokens: Vec::new(),
            recover: None,
            fixits: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn with_source_dsl(mut self, dsl_id: impl Into<String>) -> Self {
        self.source_dsl = Some(dsl_id.into());
        self
    }

    pub fn with_expected_tokens(
        mut self,
        tokens: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.expected_tokens
            .extend(tokens.into_iter().map(Into::into));
        self
    }

    pub fn with_recover(mut self, meta: RecoverMeta) -> Self {
        self.recover = Some(meta);
        self
    }

    pub fn with_fixit(mut self, fixit: ParseFixIt) -> Self {
        self.fixits.push(fixit);
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn to_guard_diagnostic(&self) -> GuardDiagnostic {
        let mut extensions = Map::new();
        let mut audit_metadata = Map::new();
        extensions.insert(
            "parser.position".into(),
            Value::Object({
                let mut obj = Map::new();
                obj.insert("byte".into(), Value::from(self.position.byte as u64));
                obj.insert("line".into(), Value::from(self.position.line as u64));
                obj.insert("column".into(), Value::from(self.position.column as u64));
                obj
            }),
        );
        if !self.expected_tokens.is_empty() {
            extensions.insert(
                "parser.syntax.expected_tokens".into(),
                Value::Array(
                    self.expected_tokens
                        .iter()
                        .cloned()
                        .map(Value::from)
                        .collect(),
                ),
            );
            audit_metadata.insert(
                "parser.syntax.expected_tokens.count".into(),
                Value::from(self.expected_tokens.len() as u64),
            );
        }
        if let Some(recover) = &self.recover {
            let mut recover_payload = Map::new();
            if let Some(mode) = recover.mode.as_ref() {
                recover_payload.insert("mode".into(), Value::String(mode.clone()));
            }
            if let Some(action) = recover.action.as_ref() {
                recover_payload.insert("action".into(), Value::String(action.as_str().into()));
            }
            if let Some(sync) = recover.sync.as_ref() {
                recover_payload.insert("sync".into(), Value::String(sync.clone()));
            }
            if let Some(inserted) = recover.inserted.as_ref() {
                recover_payload.insert("inserted".into(), Value::String(inserted.clone()));
            }
            if let Some(context) = recover.context.as_ref() {
                recover_payload.insert("context".into(), Value::String(context.clone()));
            }
            if !recover_payload.is_empty() {
                extensions.insert("recover".into(), Value::Object(recover_payload));
            }
        }
        if !self.fixits.is_empty() {
            extensions.insert(
                "fixits".into(),
                Value::Array(self.fixits.iter().map(ParseFixIt::to_json).collect()),
            );
            audit_metadata.insert(
                "parser.fixits.count".into(),
                Value::from(self.fixits.len() as u64),
            );
        }

        let mut diagnostic = GuardDiagnostic {
            code: if self.expected_tokens.is_empty() {
                "parser.syntax.error"
            } else {
                "parser.syntax.expected_tokens"
            },
            domain: "parser",
            severity: DiagnosticSeverity::Error,
            message: self.message.clone(),
            notes: self
                .notes
                .iter()
                .cloned()
                .map(DiagnosticNote::plain)
                .collect(),
            extensions,
            audit_metadata,
        };
        if let Some(source_dsl) = self.source_dsl.as_deref() {
            apply_dsl_metadata(
                &mut diagnostic,
                source_dsl,
                None,
                Span::new(self.position, self.position),
            );
        }
        diagnostic
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseFixIt {
    InsertToken { token: String },
}

impl ParseFixIt {
    fn to_json(&self) -> Value {
        match self {
            Self::InsertToken { token } => json!({
                "kind": "insert_token",
                "token": token,
            }),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecoverAction {
    Default,
    Skip,
    Insert,
    Context,
}

impl RecoverAction {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Skip => "skip",
            Self::Insert => "insert",
            Self::Context => "context",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoverMeta {
    pub mode: Option<String>,
    pub action: Option<RecoverAction>,
    pub sync: Option<String>,
    pub inserted: Option<String>,
    pub context: Option<String>,
}

impl RecoverMeta {
    pub fn collect(action: RecoverAction) -> Self {
        Self {
            mode: Some("collect".into()),
            action: Some(action),
            sync: None,
            inserted: None,
            context: None,
        }
    }

    pub fn with_sync(mut self, sync: Option<String>) -> Self {
        self.sync = sync;
        self
    }

    pub fn with_inserted(mut self, inserted: impl Into<String>) -> Self {
        self.inserted = Some(inserted.into());
        self
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }
}

/// Parser から返される Reply。consumed/committed を明示する。
#[derive(Clone, Debug)]
pub enum Reply<T> {
    Ok {
        value: T,
        span: Span,
        consumed: bool,
        rest: Input,
    },
    Err {
        error: ParseError,
        consumed: bool,
        committed: bool,
    },
}

/// ランナーが外部へ返す結果。
#[derive(Clone, Debug)]
pub struct ParseResult<T> {
    pub value: Option<T>,
    pub span: Option<Span>,
    pub diagnostics: Vec<ParseError>,
    pub recovered: bool,
    pub legacy_error: Option<ParseError>,
    pub profile: Option<ParserProfile>,
}

impl<T> ParseResult<T> {
    pub fn from_value(value: T, span: Span) -> Self {
        Self {
            value: Some(value),
            span: Some(span),
            diagnostics: Vec::new(),
            recovered: false,
            legacy_error: None,
            profile: None,
        }
    }

    pub fn from_error(error: ParseError, legacy_result: bool) -> Self {
        Self {
            value: None,
            span: None,
            diagnostics: vec![error.clone()],
            recovered: false,
            legacy_error: legacy_result.then_some(error),
            profile: None,
        }
    }

    pub fn extend_diagnostics(&mut self, diagnostics: Vec<ParseError>) {
        self.diagnostics.extend(diagnostics);
    }

    pub fn guard_diagnostics(&self) -> Vec<GuardDiagnostic> {
        self.diagnostics
            .iter()
            .map(ParseError::to_guard_diagnostic)
            .collect()
    }
}

/// パーサー本体。`ParserId` と実行クロージャを保持する。
pub struct Parser<T> {
    id: ParserId,
    f: Arc<dyn Fn(&mut ParseState) -> Reply<T> + Send + Sync>,
}

impl<T> Clone for Parser<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            f: Arc::clone(&self.f),
        }
    }
}

impl<T> fmt::Debug for Parser<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Parser").field("id", &self.id).finish()
    }
}

impl<T: Clone + Send + Sync + 'static> Parser<T> {
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(&mut ParseState) -> Reply<T> + Send + Sync + 'static,
    {
        Self {
            id: ParserId::fresh(),
            f: Arc::new(f),
        }
    }

    pub fn with_id<F>(id: ParserId, f: F) -> Self
    where
        F: Fn(&mut ParseState) -> Reply<T> + Send + Sync + 'static,
    {
        Self { id, f: Arc::new(f) }
    }

    pub fn id(&self) -> ParserId {
        self.id
    }

    pub fn parse(&self, state: &mut ParseState) -> Reply<T> {
        let key = (self.id, state.input().byte_offset());
        if matches!(state.run_config.left_recursion, LeftRecursionStrategy::Off)
            && state.left_recursion_active(key)
        {
            return Reply::Err {
                error: ParseError::new(LEFT_RECURSION_MESSAGE, state.input().position()),
                consumed: false,
                committed: true,
            };
        }
        if state.packrat_enabled() {
            if let Some(memo) = state.memo_get::<T>(key) {
                state.record_packrat_hit();
                return memo;
            } else {
                state.record_packrat_miss();
            }
        }
        state.enter_parser(key);
        let reply = (self.f)(state);
        state.exit_parser(key);
        if state.packrat_enabled() {
            state.memo_put(key, &reply);
        }
        reply
    }

    /// 既定の空白パーサーをスコープに設定する。
    pub fn with_space(self, space: Parser<()>) -> Parser<T> {
        Parser::with_id(self.id, move |state| {
            let previous = state.space();
            state.set_space(Some(space.clone()));
            let result = self.parse(state);
            state.set_space(previous);
            result
        })
    }

    /// Doc comment を紐付ける。
    pub fn with_doc(self, doc: impl AsRef<str>) -> Parser<T> {
        let doc = normalize_doc(doc.as_ref());
        Parser::with_id(self.id, move |state| {
            let result = self.parse(state);
            state.update_meta_doc(self.id, doc.clone());
            result
        })
    }

    /// 値を変換する。
    pub fn map<U, F>(self, f: F) -> Parser<U>
    where
        U: Clone + Send + Sync + 'static,
        F: Fn(T) -> U + Send + Sync + 'static,
    {
        Parser::with_id(self.id, move |state| match self.parse(state) {
            Reply::Ok {
                value,
                span,
                consumed,
                rest,
            } => {
                state.set_input(rest.clone());
                Reply::Ok {
                    value: f(value),
                    span,
                    consumed,
                    rest,
                }
            }
            Reply::Err {
                error,
                consumed,
                committed,
            } => Reply::Err {
                error,
                consumed,
                committed,
            },
        })
    }

    /// 直列合成。
    pub fn then<U>(self, other: Parser<U>) -> Parser<(T, U)>
    where
        U: Clone + Send + Sync + 'static,
    {
        Parser::new(move |state| {
            let start_input = state.input().clone();
            match self.parse(state) {
                Reply::Ok {
                    value: left,
                    span: _,
                    consumed: left_consumed,
                    rest: left_rest,
                } => {
                    state.set_input(left_rest.clone());
                    match other.parse(state) {
                        Reply::Ok {
                            value: right,
                            span: _right_span,
                            consumed: right_consumed,
                            rest,
                        } => {
                            let span = span_from_inputs(&start_input, &rest);
                            state.set_input(rest.clone());
                            Reply::Ok {
                                value: (left, right),
                                span,
                                consumed: left_consumed || right_consumed,
                                rest,
                            }
                        }
                        Reply::Err {
                            error,
                            consumed,
                            committed,
                        } => {
                            state.set_input(start_input);
                            Reply::Err {
                                error,
                                consumed: left_consumed || consumed,
                                committed,
                            }
                        }
                    }
                }
                Reply::Err {
                    error,
                    consumed,
                    committed,
                } => {
                    state.set_input(start_input);
                    Reply::Err {
                        error,
                        consumed,
                        committed,
                    }
                }
            }
        })
    }

    /// flatMap 相当。
    pub fn and_then<U, F>(self, f: F) -> Parser<U>
    where
        U: Clone + Send + Sync + 'static,
        F: Fn(T) -> Parser<U> + Send + Sync + 'static,
    {
        Parser::new(move |state| {
            let start_input = state.input().clone();
            match self.parse(state) {
                Reply::Ok {
                    value,
                    consumed,
                    rest,
                    ..
                } => {
                    state.set_input(rest.clone());
                    let next = f(value);
                    match next.parse(state) {
                        Reply::Ok {
                            value: out,
                            span,
                            consumed: next_consumed,
                            rest,
                        } => {
                            state.set_input(rest.clone());
                            Reply::Ok {
                                value: out,
                                span,
                                consumed: consumed || next_consumed,
                                rest,
                            }
                        }
                        Reply::Err {
                            error,
                            consumed: next_consumed,
                            committed,
                        } => {
                            state.set_input(start_input);
                            Reply::Err {
                                error,
                                consumed: consumed || next_consumed,
                                committed,
                            }
                        }
                    }
                }
                Reply::Err {
                    error,
                    consumed,
                    committed,
                } => {
                    state.set_input(start_input);
                    Reply::Err {
                        error,
                        consumed,
                        committed,
                    }
                }
            }
        })
    }

    /// 左側を捨てて右側を返す。
    pub fn skip_l<U>(self, other: Parser<U>) -> Parser<U>
    where
        U: Clone + Send + Sync + 'static,
    {
        self.then(other).map(|(_, r)| r)
    }

    /// 右側を捨てて左側を返す。
    pub fn skip_r<U>(self, other: Parser<U>) -> Parser<T>
    where
        U: Clone + Send + Sync + 'static,
    {
        self.then(other).map(|(l, _)| l)
    }

    /// 左優先の選択。
    pub fn or(self, other: Parser<T>) -> Parser<T> {
        Parser::new(move |state| {
            let start_input = state.input().clone();
            match self.parse(state) {
                ok @ Reply::Ok { .. } => ok,
                Reply::Err {
                    error,
                    consumed,
                    committed,
                } => {
                    if consumed || committed {
                        state.set_input(start_input);
                        Reply::Err {
                            error,
                            consumed,
                            committed,
                        }
                    } else {
                        state.set_input(start_input.clone());
                        other.parse(state)
                    }
                }
            }
        })
    }

    /// 値を変換しつつ committed を付与する。
    pub fn cut(self) -> Parser<T> {
        Parser::with_id(self.id, move |state| match self.parse(state) {
            Reply::Ok {
                value,
                span,
                consumed,
                rest,
            } => {
                state.set_input(rest.clone());
                Reply::Ok {
                    value,
                    span,
                    consumed,
                    rest,
                }
            }
            Reply::Err {
                error,
                consumed,
                committed: _,
            } => Reply::Err {
                error,
                consumed,
                committed: true,
            },
        })
    }

    /// 直近の消費を巻き戻し、空失敗に変換する。
    pub fn attempt(self) -> Parser<T> {
        Parser::with_id(self.id, move |state| {
            let start_input = state.input().clone();
            match self.parse(state) {
                Reply::Ok {
                    value,
                    span,
                    consumed,
                    rest,
                } => {
                    state.set_input(rest.clone());
                    Reply::Ok {
                        value,
                        span,
                        consumed,
                        rest,
                    }
                }
                Reply::Err { error, .. } => {
                    state.record_backtrack();
                    state.set_input(start_input);
                    Reply::Err {
                        error,
                        consumed: false,
                        committed: false,
                    }
                }
            }
        })
    }

    /// 失敗時に until まで読み飛ばし、with の値で継続する。
    pub fn recover(self, until: Parser<()>, with: T) -> Parser<T>
    where
        T: Clone + 'static,
    {
        self.recover_with_payload(until, with, RecoverMeta::collect(RecoverAction::Skip), None)
    }

    fn recover_with_payload(
        self,
        until: Parser<()>,
        with: T,
        meta: RecoverMeta,
        fixit: Option<ParseFixIt>,
    ) -> Parser<T>
    where
        T: Clone + 'static,
    {
        Parser::new(move |state| {
            let start_input = state.input().clone();
            match self.parse(state) {
                ok @ Reply::Ok { .. } => ok,
                Reply::Err {
                    error,
                    consumed,
                    committed,
                } => {
                    if state.recover_config.mode != RecoverMode::Collect {
                        state.set_input(start_input);
                        return Reply::Err {
                            error,
                            consumed,
                            committed,
                        };
                    }

                    state.set_input(start_input.clone());
                    let mut cursor = start_input.clone();
                    loop {
                        state.set_input(cursor.clone());
                        match until.parse(state) {
                            Reply::Ok {
                                rest,
                                consumed: until_consumed,
                                ..
                            } => {
                                if !until_consumed {
                                    let err = ParseError::new(
                                        "recover until が空成功しました",
                                        cursor.position(),
                                    );
                                    state.set_input(start_input);
                                    return Reply::Err {
                                        error: err,
                                        consumed: false,
                                        committed: false,
                                    };
                                }
                                if state.recover_limits_exceeded() {
                                    state.set_input(start_input);
                                    return Reply::Err {
                                        error,
                                        consumed,
                                        committed,
                                    };
                                }
                                if let Some(limit) = state.recover_config.max_diagnostics {
                                    if state.diagnostics.len() >= limit {
                                        state.set_input(start_input);
                                        return Reply::Err {
                                            error,
                                            consumed,
                                            committed,
                                        };
                                    }
                                }

                                let sync = state.match_sync_token(&cursor, &rest);
                                let recover_meta = meta.clone().with_sync(sync);
                                let mut diagnostic =
                                    error.clone().with_recover(recover_meta.clone());
                                if let Some(fixit) = fixit.clone() {
                                    diagnostic = diagnostic.with_fixit(fixit);
                                }
                                if state.recover_config.notes {
                                    if let Some(context) = recover_meta.context.as_ref() {
                                        diagnostic = diagnostic.with_note(context.clone());
                                    }
                                }
                                state.push_diagnostic(diagnostic);
                                state.set_input(rest.clone());
                                state.record_recovery();
                                state.recoveries = state.recoveries.saturating_add(1);
                                state.recovered = true;
                                let span = span_from_inputs(&start_input, &rest);
                                return Reply::Ok {
                                    value: with.clone(),
                                    span,
                                    consumed: true,
                                    rest,
                                };
                            }
                            Reply::Err {
                                consumed: until_consumed,
                                committed: until_committed,
                                error: until_err,
                            } => {
                                if until_consumed || until_committed {
                                    state.set_input(start_input);
                                    return Reply::Err {
                                        error: until_err,
                                        consumed: true,
                                        committed: until_committed,
                                    };
                                }
                            }
                        }

                        if cursor.is_empty() {
                            state.set_input(start_input);
                            return Reply::Err {
                                error,
                                consumed,
                                committed,
                            };
                        }

                        if let Some((idx, ch)) = cursor.remaining().char_indices().next() {
                            let step = ch.len_utf8().max(1);
                            let advance = idx + step;
                            state.recover_resync_bytes =
                                state.recover_resync_bytes.saturating_add(advance);
                            if let Some(limit) = state.recover_config.max_resync_bytes {
                                if state.recover_resync_bytes >= limit {
                                    state.set_input(start_input);
                                    return Reply::Err {
                                        error,
                                        consumed,
                                        committed,
                                    };
                                }
                            }
                            cursor = cursor.advance(advance);
                        } else {
                            state.set_input(start_input);
                            return Reply::Err {
                                error,
                                consumed,
                                committed,
                            };
                        }
                    }
                }
            }
        })
    }

    pub fn recover_with_default(self, until: Parser<()>, with: T) -> Parser<T>
    where
        T: Clone + 'static,
    {
        self.recover_with_payload(
            until,
            with,
            RecoverMeta::collect(RecoverAction::Default),
            None,
        )
    }

    pub fn recover_until(self, until: Parser<()>, with: T) -> Parser<T>
    where
        T: Clone + 'static,
    {
        self.recover_with_payload(until, with, RecoverMeta::collect(RecoverAction::Skip), None)
    }

    pub fn recover_with_insert(
        self,
        until: Parser<()>,
        token: impl Into<String>,
        with: T,
    ) -> Parser<T>
    where
        T: Clone + 'static,
    {
        let token = token.into();
        self.recover_with_payload(
            until,
            with,
            RecoverMeta::collect(RecoverAction::Insert).with_inserted(token.clone()),
            Some(ParseFixIt::InsertToken { token }),
        )
    }

    pub fn recover_with_context(
        self,
        until: Parser<()>,
        message: impl Into<String>,
        with: T,
    ) -> Parser<T>
    where
        T: Clone + 'static,
    {
        self.recover_with_payload(
            until,
            with,
            RecoverMeta::collect(RecoverAction::Context).with_context(message),
            None,
        )
    }

    pub fn recover_missing(self, until: Parser<()>, token: impl Into<String>, with: T) -> Parser<T>
    where
        T: Clone + 'static,
    {
        self.recover_with_insert(until, token, with)
    }

    pub fn panic_until(self, until: Parser<()>, with: T) -> Parser<T>
    where
        T: Clone + 'static,
    {
        self.recover_with_payload(
            until,
            with,
            RecoverMeta::collect(RecoverAction::Skip).with_context("panic"),
            None,
        )
    }

    pub fn panic_block(self, open: Parser<()>, close: Parser<()>, with: T) -> Parser<T>
    where
        T: Clone + 'static,
    {
        let sync = panic_block_sync(open, close);
        self.recover_with_payload(
            sync,
            with,
            RecoverMeta::collect(RecoverAction::Skip).with_context("panic_block"),
            None,
        )
    }

    /// トレースフラグを尊重して内側を実行する（現状は透過）。
    pub fn trace(self) -> Parser<T> {
        Parser::with_id(self.id, move |state| self.parse(state))
    }

    /// 0 回または 1 回。
    pub fn opt(self) -> Parser<Option<T>> {
        Parser::new(move |state| {
            let start_input = state.input().clone();
            match self.parse(state) {
                Reply::Ok {
                    value,
                    span,
                    consumed,
                    rest,
                } => {
                    state.set_input(rest.clone());
                    Reply::Ok {
                        value: Some(value),
                        span,
                        consumed,
                        rest,
                    }
                }
                Reply::Err {
                    consumed,
                    committed,
                    error,
                } => {
                    if consumed || committed {
                        state.set_input(start_input);
                        Reply::Err {
                            error,
                            consumed,
                            committed,
                        }
                    } else {
                        state.set_input(start_input.clone());
                        Reply::Ok {
                            value: None,
                            span: empty_span(&start_input),
                            consumed: false,
                            rest: start_input,
                        }
                    }
                }
            }
        })
    }

    /// 0 回以上。
    pub fn many(self) -> Parser<Vec<T>> {
        Parser::new(move |state| {
            let mut values = Vec::new();
            let start_input = state.input().clone();
            let mut current_input = start_input.clone();
            let mut any_consumed = false;

            loop {
                state.set_input(current_input.clone());
                match self.parse(state) {
                    Reply::Ok {
                        value,
                        consumed,
                        rest,
                        ..
                    } => {
                        if !consumed {
                            let err = ParseError::new(
                                "繰り返し本体が空成功しました",
                                current_input.position(),
                            );
                            state.set_input(start_input);
                            return Reply::Err {
                                error: err,
                                consumed: false,
                                committed: false,
                            };
                        }
                        any_consumed = true;
                        current_input = rest.clone();
                        values.push(value);
                    }
                    Reply::Err {
                        consumed,
                        committed,
                        error,
                    } => {
                        if consumed || committed {
                            state.set_input(start_input);
                            return Reply::Err {
                                error,
                                consumed,
                                committed,
                            };
                        } else {
                            state.set_input(current_input.clone());
                            let span = span_from_inputs(&start_input, &current_input);
                            return Reply::Ok {
                                value: values,
                                span,
                                consumed: any_consumed,
                                rest: current_input,
                            };
                        }
                    }
                }
            }
        })
    }

    /// 1 回以上。
    pub fn many1(self) -> Parser<Vec<T>> {
        Parser::new(move |state| {
            let start_input = state.input().clone();
            match self.clone().many().parse(state) {
                Reply::Ok {
                    value,
                    consumed,
                    rest,
                    span,
                } => {
                    if value.is_empty() {
                        state.set_input(start_input);
                        let err = ParseError::new(
                            "1 回以上の繰り返しが必要です",
                            state.input().position(),
                        );
                        Reply::Err {
                            error: err,
                            consumed: false,
                            committed: false,
                        }
                    } else {
                        state.set_input(rest.clone());
                        Reply::Ok {
                            value,
                            span,
                            consumed,
                            rest,
                        }
                    }
                }
                err @ Reply::Err { .. } => err,
            }
        })
    }

    /// 繰り返し回数を指定する。
    pub fn repeat(self, min: usize, max: Option<usize>) -> Parser<Vec<T>> {
        Parser::new(move |state| {
            let mut values = Vec::new();
            let start_input = state.input().clone();
            let mut current_input = start_input.clone();
            let mut any_consumed = false;

            loop {
                if let Some(limit) = max {
                    if values.len() >= limit {
                        state.set_input(current_input.clone());
                        let span = span_from_inputs(&start_input, &current_input);
                        return Reply::Ok {
                            value: values,
                            span,
                            consumed: any_consumed,
                            rest: current_input,
                        };
                    }
                }

                state.set_input(current_input.clone());
                match self.parse(state) {
                    Reply::Ok {
                        value,
                        consumed,
                        rest,
                        ..
                    } => {
                        if !consumed {
                            let err = ParseError::new(
                                "繰り返し本体が空成功しました",
                                current_input.position(),
                            );
                            state.set_input(start_input);
                            return Reply::Err {
                                error: err,
                                consumed: false,
                                committed: false,
                            };
                        }
                        any_consumed = true;
                        current_input = rest.clone();
                        values.push(value);
                    }
                    Reply::Err {
                        consumed,
                        committed,
                        error,
                    } => {
                        if consumed || committed {
                            state.set_input(start_input);
                            return Reply::Err {
                                error,
                                consumed,
                                committed,
                            };
                        } else if values.len() >= min {
                            state.set_input(current_input.clone());
                            let span = span_from_inputs(&start_input, &current_input);
                            return Reply::Ok {
                                value: values,
                                span,
                                consumed: any_consumed,
                                rest: current_input,
                            };
                        } else {
                            state.set_input(start_input);
                            let err = ParseError::new(
                                "指定回数の繰り返しに失敗しました",
                                current_input.position(),
                            );
                            return Reply::Err {
                                error: err,
                                consumed: false,
                                committed: false,
                            };
                        }
                    }
                }
            }
        })
    }

    /// セパレータ区切り（0 回以上）。
    pub fn sep_by<U>(self, sep: Parser<U>) -> Parser<Vec<T>>
    where
        U: Clone + Send + Sync + 'static,
    {
        Parser::new(move |state| {
            let start_input = state.input().clone();
            let mut values = Vec::new();
            let mut current_input = start_input.clone();

            state.set_input(current_input.clone());
            match self.parse(state) {
                Reply::Ok {
                    value,
                    consumed,
                    rest,
                    ..
                } => {
                    if !consumed {
                        let err = ParseError::new(
                            "繰り返し本体が空成功しました",
                            current_input.position(),
                        );
                        state.set_input(start_input);
                        return Reply::Err {
                            error: err,
                            consumed: false,
                            committed: false,
                        };
                    }
                    values.push(value);
                    current_input = rest.clone();
                }
                Reply::Err {
                    consumed,
                    committed,
                    error,
                } => {
                    if consumed || committed {
                        state.set_input(start_input);
                        return Reply::Err {
                            error,
                            consumed,
                            committed,
                        };
                    } else {
                        state.set_input(start_input.clone());
                        return Reply::Ok {
                            value: values,
                            span: empty_span(&start_input),
                            consumed: false,
                            rest: start_input,
                        };
                    }
                }
            }

            loop {
                state.set_input(current_input.clone());
                match sep.parse(state) {
                    Reply::Ok {
                        consumed: sep_consumed,
                        rest: sep_rest,
                        ..
                    } => {
                        if !sep_consumed {
                            let err = ParseError::new(
                                "セパレータが空成功しました",
                                current_input.position(),
                            );
                            state.set_input(start_input);
                            return Reply::Err {
                                error: err,
                                consumed: false,
                                committed: false,
                            };
                        }
                        current_input = sep_rest.clone();
                    }
                    Reply::Err {
                        consumed: sep_consumed,
                        committed: sep_committed,
                        error: sep_error,
                    } => {
                        if sep_consumed || sep_committed {
                            state.set_input(start_input);
                            return Reply::Err {
                                error: sep_error,
                                consumed: sep_consumed,
                                committed: sep_committed,
                            };
                        } else {
                            state.set_input(current_input.clone());
                            let span = span_from_inputs(&start_input, &current_input);
                            return Reply::Ok {
                                value: values,
                                span,
                                consumed: true,
                                rest: current_input,
                            };
                        }
                    }
                }

                state.set_input(current_input.clone());
                match self.parse(state) {
                    Reply::Ok {
                        value,
                        consumed,
                        rest,
                        ..
                    } => {
                        if !consumed {
                            let err = ParseError::new(
                                "繰り返し本体が空成功しました",
                                current_input.position(),
                            );
                            state.set_input(start_input);
                            return Reply::Err {
                                error: err,
                                consumed: false,
                                committed: false,
                            };
                        }
                        values.push(value);
                        current_input = rest.clone();
                    }
                    Reply::Err {
                        consumed: item_consumed,
                        committed: item_committed,
                        error: item_error,
                    } => {
                        if item_consumed || item_committed {
                            state.set_input(start_input);
                            return Reply::Err {
                                error: item_error,
                                consumed: item_consumed,
                                committed: item_committed,
                            };
                        } else {
                            state.set_input(start_input);
                            let err = ParseError::new(
                                "区切りの後に要素が見つかりません",
                                current_input.position(),
                            );
                            return Reply::Err {
                                error: err,
                                consumed: true,
                                committed: false,
                            };
                        }
                    }
                }
            }
        })
    }

    /// セパレータ区切り（1 回以上）。
    pub fn sep_by1<U>(self, sep: Parser<U>) -> Parser<Vec<T>>
    where
        U: Clone + Send + Sync + 'static,
    {
        Parser::new(
            move |state| match self.clone().sep_by(sep.clone()).parse(state) {
                Reply::Ok {
                    value,
                    consumed,
                    rest,
                    span,
                } => {
                    if value.is_empty() {
                        let err = ParseError::new(
                            "1 回以上の繰り返しが必要です",
                            state.input().position(),
                        );
                        Reply::Err {
                            error: err,
                            consumed: false,
                            committed: false,
                        }
                    } else {
                        Reply::Ok {
                            value,
                            consumed,
                            rest,
                            span,
                        }
                    }
                }
                err @ Reply::Err { .. } => err,
            },
        )
    }

    /// end まで読み続ける。
    pub fn many_till<U>(self, end: Parser<U>) -> Parser<Vec<T>>
    where
        U: Clone + Send + Sync + 'static,
    {
        Parser::new(move |state| {
            let mut values = Vec::new();
            let start_input = state.input().clone();
            let mut current_input = start_input.clone();

            loop {
                state.set_input(current_input.clone());
                match end.parse(state) {
                    Reply::Ok {
                        rest,
                        consumed: end_consumed,
                        ..
                    } => {
                        state.set_input(rest.clone());
                        let span = span_from_inputs(&start_input, &rest);
                        let consumed_flag = !values.is_empty() || end_consumed;
                        return Reply::Ok {
                            value: values,
                            span,
                            consumed: consumed_flag,
                            rest,
                        };
                    }
                    Reply::Err {
                        consumed: end_consumed,
                        committed: end_committed,
                        error: end_error,
                    } => {
                        if end_consumed || end_committed {
                            state.set_input(start_input);
                            return Reply::Err {
                                error: end_error,
                                consumed: end_consumed,
                                committed: end_committed,
                            };
                        }
                    }
                }

                state.set_input(current_input.clone());
                match self.parse(state) {
                    Reply::Ok {
                        value,
                        consumed,
                        rest,
                        ..
                    } => {
                        if !consumed {
                            let err = ParseError::new(
                                "繰り返し本体が空成功しました",
                                current_input.position(),
                            );
                            state.set_input(start_input);
                            return Reply::Err {
                                error: err,
                                consumed: false,
                                committed: false,
                            };
                        }
                        current_input = rest.clone();
                        values.push(value);
                    }
                    Reply::Err {
                        consumed,
                        committed,
                        error,
                    } => {
                        if consumed || committed {
                            state.set_input(start_input);
                            return Reply::Err {
                                error,
                                consumed,
                                committed,
                            };
                        } else {
                            state.set_input(start_input);
                            return Reply::Err {
                                error,
                                consumed: false,
                                committed: false,
                            };
                        }
                    }
                }
            }
        })
    }

    /// 先読み（非消費）。
    pub fn lookahead(self) -> Parser<T>
    where
        T: Clone + 'static,
    {
        Parser::new(move |state| {
            let start_input = state.input().clone();
            match self.parse(state) {
                Reply::Ok { value, span, .. } => {
                    state.set_input(start_input.clone());
                    Reply::Ok {
                        value: value.clone(),
                        span,
                        consumed: false,
                        rest: start_input,
                    }
                }
                Reply::Err {
                    error, committed, ..
                } => {
                    state.set_input(start_input);
                    Reply::Err {
                        error,
                        consumed: false,
                        committed,
                    }
                }
            }
        })
    }

    /// 否定先読み。
    pub fn not_followed_by(self) -> Parser<()> {
        Parser::new(move |state| {
            let start_input = state.input().clone();
            match self.parse(state) {
                Reply::Ok { span, .. } => {
                    state.set_input(start_input.clone());
                    let err = ParseError::new("後続禁止のパターンにマッチしました", span.start);
                    Reply::Err {
                        error: err,
                        consumed: false,
                        committed: false,
                    }
                }
                Reply::Err {
                    consumed,
                    committed,
                    error,
                } => {
                    state.set_input(start_input.clone());
                    if consumed || committed {
                        Reply::Err {
                            error,
                            consumed,
                            committed,
                        }
                    } else {
                        Reply::Ok {
                            value: (),
                            span: empty_span(&start_input),
                            consumed: false,
                            rest: start_input,
                        }
                    }
                }
            }
        })
    }

    /// 値とスパンを返す。
    pub fn spanned(self) -> Parser<(T, Span)> {
        Parser::new(move |state| match self.parse(state) {
            Reply::Ok {
                value,
                span,
                consumed,
                rest,
            } => {
                state.set_input(rest.clone());
                Reply::Ok {
                    value: (value, span.clone()),
                    span,
                    consumed,
                    rest,
                }
            }
            Reply::Err {
                error,
                consumed,
                committed,
            } => Reply::Err {
                error,
                consumed,
                committed,
            },
        })
    }

    /// 左結合のチェーン。
    pub fn chainl1<F>(self, op: Parser<F>) -> Parser<T>
    where
        T: Clone,
        F: Clone + Fn(T, T) -> T + Send + Sync + 'static,
    {
        Parser::new(move |state| {
            let start_input = state.input().clone();
            let mut current_input = start_input.clone();
            state.set_input(current_input.clone());
            let reply = match self.parse(state) {
                Reply::Ok {
                    value,
                    span,
                    consumed,
                    rest,
                } => {
                    current_input = rest.clone();
                    (value, span, consumed)
                }
                err @ Reply::Err { .. } => {
                    state.set_input(start_input);
                    return err;
                }
            };

            let mut acc = reply.0;
            let mut acc_span = reply.1;
            let mut any_consumed = reply.2;

            loop {
                let iter_input = current_input.clone();
                state.set_input(iter_input.clone());
                let step = op.clone().then(self.clone()).attempt();
                match step.parse(state) {
                    Reply::Ok {
                        value: (f, rhs),
                        span: rhs_span,
                        consumed,
                        rest,
                    } => {
                        if !consumed {
                            let err = ParseError::new(
                                "繰り返し本体が空成功しました",
                                iter_input.position(),
                            );
                            state.set_input(start_input);
                            return Reply::Err {
                                error: err,
                                consumed: false,
                                committed: false,
                            };
                        }
                        acc = f(acc, rhs);
                        acc_span = Span::new(acc_span.start, rhs_span.end);
                        any_consumed = true;
                        current_input = rest.clone();
                    }
                    Reply::Err {
                        consumed,
                        committed,
                        error,
                    } => {
                        state.set_input(iter_input);
                        if consumed || committed {
                            state.set_input(start_input);
                            return Reply::Err {
                                error,
                                consumed,
                                committed,
                            };
                        } else {
                            state.set_input(current_input.clone());
                            let span = Span::new(acc_span.start, current_input.position());
                            return Reply::Ok {
                                value: acc,
                                span,
                                consumed: any_consumed,
                                rest: current_input,
                            };
                        }
                    }
                }
            }
        })
    }

    /// 右結合のチェーン。
    pub fn chainr1<F>(self, op: Parser<F>) -> Parser<T>
    where
        T: Clone,
        F: Clone + Fn(T, T) -> T + Send + Sync + 'static,
    {
        Parser::new(move |state| {
            let start_input = state.input().clone();
            state.set_input(start_input.clone());
            let first = match self.parse(state) {
                Reply::Ok {
                    value,
                    consumed,
                    rest,
                    ..
                } => (value, consumed, rest),
                err @ Reply::Err { .. } => return err,
            };

            let mut operands = vec![first.0];
            let mut operators: Vec<F> = Vec::new();
            let mut consumed_any = first.1;
            let mut current_input = first.2.clone();

            loop {
                state.set_input(current_input.clone());
                let step = op.clone().then(self.clone()).attempt();
                match step.parse(state) {
                    Reply::Ok {
                        value: (f, rhs),
                        consumed,
                        rest,
                        ..
                    } => {
                        if !consumed {
                            let err = ParseError::new(
                                "繰り返し本体が空成功しました",
                                current_input.position(),
                            );
                            state.set_input(start_input);
                            return Reply::Err {
                                error: err,
                                consumed: false,
                                committed: false,
                            };
                        }
                        operators.push(f);
                        operands.push(rhs);
                        consumed_any = true;
                        current_input = rest.clone();
                    }
                    Reply::Err {
                        consumed,
                        committed,
                        error,
                    } => {
                        if consumed || committed {
                            state.set_input(start_input);
                            return Reply::Err {
                                error,
                                consumed,
                                committed,
                            };
                        }
                        break;
                    }
                }
            }

            let mut result = operands
                .pop()
                .expect("chainr1 で operands が空になることはありません");
            while let Some(lhs) = operands.pop() {
                if let Some(op_fn) = operators.pop() {
                    result = op_fn(lhs, result);
                }
            }

            let span = span_from_inputs(&start_input, &current_input);
            state.set_input(current_input.clone());
            Reply::Ok {
                value: result,
                span,
                consumed: consumed_any,
                rest: current_input,
            }
        })
    }
}

impl Parser<()> {
    /// 空白パーサーの安定 ID を取得する。
    pub fn space_id(&self) -> ParserId {
        // ID 割り当ての安定性は `rule` 由来に依存する。Lex ブリッジ連携時に共有する。
        self.id
    }
}

/// 非消費で成功するパーサー。
pub fn ok<T>(value: T) -> Parser<T>
where
    T: Clone + Send + Sync + 'static,
{
    Parser::new(move |state| Reply::Ok {
        value: value.clone(),
        span: empty_span(state.input()),
        consumed: false,
        rest: state.input().clone(),
    })
}

/// 非消費で失敗するパーサー。
pub fn fail<T>(message: impl Into<String>) -> Parser<T>
where
    T: Clone + Send + Sync + 'static,
{
    let msg = message.into();
    Parser::new(move |state| Reply::Err {
        error: ParseError::new(msg.clone(), state.input().position()),
        consumed: false,
        committed: false,
    })
}

/// 入力終端のみ成功する。
pub fn eof() -> Parser<()> {
    Parser::new(|state| {
        if state.input().is_empty() {
            Reply::Ok {
                value: (),
                span: empty_span(state.input()),
                consumed: false,
                rest: state.input().clone(),
            }
        } else {
            Reply::Err {
                error: ParseError::new("入力の終端を期待しました", state.input().position())
                    .with_expected_tokens([String::from("<eof>")]),
                consumed: false,
                committed: false,
            }
        }
    })
}

/// 名前付きパーサー（ParserId を固定化する）。
pub fn rule<T>(name: impl AsRef<str>, parser: Parser<T>) -> Parser<T>
where
    T: Clone + Send + Sync + 'static,
{
    let name = name.as_ref().to_string();
    let id = parser_id_from_name(&name);
    Parser::with_id(id, move |state| {
        state.register_meta(id, ParserMetaKind::Rule, name.clone(), None);
        state.enter_rule_meta(id);
        let reply = parser.parse(state);
        state.exit_rule_meta(id);
        reply
    })
}

/// エラー時のラベルを差し替える。
pub fn label<T>(name: impl Into<String>, parser: Parser<T>) -> Parser<T>
where
    T: Clone + Send + Sync + 'static,
{
    let label = name.into();
    Parser::new(move |state| match parser.parse(state) {
        Reply::Ok {
            value,
            span,
            consumed,
            rest,
        } => {
            state.set_input(rest.clone());
            Reply::Ok {
                value,
                span,
                consumed,
                rest,
            }
        }
        Reply::Err {
            mut error,
            consumed,
            committed,
        } => {
            error.message = label.clone();
            if !error.expected_tokens.contains(&label) {
                error.expected_tokens.push(label.clone());
            }
            Reply::Err {
                error,
                consumed,
                committed,
            }
        }
    })
}

/// Doc comment を付与する。
pub fn with_doc<T>(parser: Parser<T>, doc: impl AsRef<str>) -> Parser<T>
where
    T: Clone + Send + Sync + 'static,
{
    parser.with_doc(doc)
}

/// トークン種別を付与する。
pub fn token<T>(kind: impl AsRef<str>, parser: Parser<T>) -> Parser<T>
where
    T: Clone + Send + Sync + 'static,
{
    let kind = kind.as_ref().to_string();
    let id = ParserId::fresh();
    Parser::with_id(id, move |state| {
        state.register_meta(id, ParserMetaKind::Token, kind.clone(), Some(kind.clone()));
        let start_input = state.input().clone();
        match parser.parse(state) {
            Reply::Ok {
                value,
                span,
                consumed,
                rest,
            } => {
                state.set_input(rest.clone());
                if state.cst_enabled() && start_input.byte_offset() < rest.byte_offset() {
                    if let Some(text) = slice_input_text(&start_input, &rest) {
                        state.record_cst_token(TextString::from(kind.clone()), text, span.clone());
                    }
                }
                state.record_semantic_token(kind.clone(), span.clone());
                Reply::Ok {
                    value,
                    span,
                    consumed,
                    rest,
                }
            }
            Reply::Err {
                error,
                consumed,
                committed,
            } => Reply::Err {
                error,
                consumed,
                committed,
            },
        }
    })
}

/// 選択肢の列を左から試す。
pub fn choice<T>(parsers: Vec<Parser<T>>) -> Parser<T>
where
    T: Clone + Send + Sync + 'static,
{
    parsers
        .into_iter()
        .reduce(|acc, p| acc.or(p))
        .unwrap_or_else(|| fail("選択肢がありません"))
}

/// ゼロ幅コミット。
pub fn cut_here() -> Parser<()> {
    Parser::new(|state| Reply::Ok {
        value: (),
        span: empty_span(state.input()),
        consumed: true,
        rest: state.input().clone(),
    })
}

pub fn sync_to(sync: Parser<()>) -> Parser<()> {
    Parser::new(move |state| {
        let start_input = state.input().clone();
        let mut cursor = start_input.clone();
        loop {
            state.set_input(cursor.clone());
            match sync.parse(state) {
                Reply::Ok { rest, consumed, .. } => {
                    let progressed = start_input.byte_offset() != rest.byte_offset();
                    if !progressed {
                        let err = ParseError::new("sync_to が空成功しました", cursor.position());
                        state.set_input(start_input);
                        return Reply::Err {
                            error: err,
                            consumed: false,
                            committed: false,
                        };
                    }
                    let span = span_from_inputs(&start_input, &rest);
                    state.set_input(rest.clone());
                    return Reply::Ok {
                        value: (),
                        span,
                        consumed: true,
                        rest,
                    };
                }
                Reply::Err {
                    error,
                    consumed,
                    committed,
                } => {
                    if consumed || committed {
                        state.set_input(start_input);
                        return Reply::Err {
                            error,
                            consumed,
                            committed,
                        };
                    }
                }
            }

            if cursor.is_empty() {
                let err = ParseError::new(
                    "sync_to が同期点を見つけられませんでした",
                    cursor.position(),
                );
                state.set_input(start_input);
                return Reply::Err {
                    error: err,
                    consumed: false,
                    committed: false,
                };
            }

            if let Some((idx, ch)) = cursor.remaining().char_indices().next() {
                let step = ch.len_utf8().max(1);
                cursor = cursor.advance(idx + step);
            } else {
                let err = ParseError::new(
                    "sync_to が同期点を見つけられませんでした",
                    cursor.position(),
                );
                state.set_input(start_input);
                return Reply::Err {
                    error: err,
                    consumed: false,
                    committed: false,
                };
            }
        }
    })
}

fn panic_block_sync(open: Parser<()>, close: Parser<()>) -> Parser<()> {
    Parser::new(move |state| {
        let start_input = state.input().clone();
        let mut cursor = start_input.clone();
        let mut depth = 0usize;

        loop {
            state.set_input(cursor.clone());
            match open.parse(state) {
                Reply::Ok { rest, consumed, .. } => {
                    if !consumed || cursor.byte_offset() == rest.byte_offset() {
                        let err =
                            ParseError::new("panic_block が空成功しました", cursor.position());
                        state.set_input(start_input);
                        return Reply::Err {
                            error: err,
                            consumed: false,
                            committed: false,
                        };
                    }
                    depth = depth.saturating_add(1);
                    cursor = rest.clone();
                    continue;
                }
                Reply::Err {
                    error,
                    consumed,
                    committed,
                } => {
                    if consumed || committed {
                        state.set_input(start_input);
                        return Reply::Err {
                            error,
                            consumed,
                            committed,
                        };
                    }
                }
            }

            state.set_input(cursor.clone());
            match close.parse(state) {
                Reply::Ok { rest, consumed, .. } => {
                    if !consumed || cursor.byte_offset() == rest.byte_offset() {
                        let err =
                            ParseError::new("panic_block が空成功しました", cursor.position());
                        state.set_input(start_input);
                        return Reply::Err {
                            error: err,
                            consumed: false,
                            committed: false,
                        };
                    }
                    if depth == 0 {
                        let span = span_from_inputs(&start_input, &rest);
                        state.set_input(rest.clone());
                        return Reply::Ok {
                            value: (),
                            span,
                            consumed: true,
                            rest,
                        };
                    }
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        let span = span_from_inputs(&start_input, &rest);
                        state.set_input(rest.clone());
                        return Reply::Ok {
                            value: (),
                            span,
                            consumed: true,
                            rest,
                        };
                    }
                    cursor = rest.clone();
                    continue;
                }
                Reply::Err {
                    error,
                    consumed,
                    committed,
                } => {
                    if consumed || committed {
                        state.set_input(start_input);
                        return Reply::Err {
                            error,
                            consumed,
                            committed,
                        };
                    }
                }
            }

            if cursor.is_empty() {
                let err = ParseError::new(
                    "panic_block が同期点を見つけられませんでした",
                    cursor.position(),
                );
                state.set_input(start_input);
                return Reply::Err {
                    error: err,
                    consumed: false,
                    committed: false,
                };
            }

            if let Some((idx, ch)) = cursor.remaining().char_indices().next() {
                let step = ch.len_utf8().max(1);
                cursor = cursor.advance(idx + step);
            } else {
                let err = ParseError::new(
                    "panic_block が同期点を見つけられませんでした",
                    cursor.position(),
                );
                state.set_input(start_input);
                return Reply::Err {
                    error: err,
                    consumed: false,
                    committed: false,
                };
            }
        }
    })
}

/// 2 つのパーサーの間に挟む。
pub fn between<A>(open: Parser<()>, parser: Parser<A>, close: Parser<()>) -> Parser<A>
where
    A: Clone + Send + Sync + 'static,
{
    open.skip_l(parser).skip_r(close)
}

/// 前置パーサーを読み捨てる。
pub fn preceded<A, B>(pre: Parser<A>, parser: Parser<B>) -> Parser<B>
where
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
{
    pre.skip_l(parser)
}

/// 後置パーサーを読み捨てる。
pub fn terminated<A, B>(parser: Parser<A>, post: Parser<B>) -> Parser<A>
where
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
{
    parser.skip_r(post)
}

/// a b c の中央だけを返す。
pub fn delimited<A, B, C>(a: Parser<A>, b: Parser<B>, c: Parser<C>) -> Parser<B>
where
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
{
    a.skip_l(b).skip_r(c)
}

/// 先読み。
pub fn lookahead<T>(parser: Parser<T>) -> Parser<T>
where
    T: Clone + Send + Sync + 'static,
{
    parser.lookahead()
}

/// 否定先読み。
pub fn not_followed_by<T>(parser: Parser<T>) -> Parser<()>
where
    T: Clone + Send + Sync + 'static,
{
    parser.not_followed_by()
}

/// レイアウトトークン（indent/dedent/newline）を消費する。
pub fn layout_token(text: impl AsRef<str>) -> Parser<()> {
    let expected = text.as_ref().to_string();
    Parser::new(move |state| {
        if !state.layout_active() {
            return Reply::Err {
                error: ParseError::new(
                    format!("レイアウトが無効の状態で {} を要求しました", expected),
                    state.input().position(),
                )
                .with_expected_tokens([expected.clone()]),
                consumed: false,
                committed: false,
            };
        }
        state.produce_layout_tokens();
        if let Some(token) = state.layout_pop_token() {
            if token == expected {
                if state.cst_enabled() {
                    let span = empty_span(state.input());
                    state.record_cst_trivia(
                        TriviaKind::Layout,
                        TextString::from(token),
                        span,
                        true,
                    );
                }
                return Reply::Ok {
                    value: (),
                    span: empty_span(state.input()),
                    consumed: false,
                    rest: state.input().clone(),
                };
            } else {
                return Reply::Err {
                    error: ParseError::new(
                        format!("期待したレイアウトトークン: {}", expected),
                        state.input().position(),
                    )
                    .with_expected_tokens([expected.clone()]),
                    consumed: false,
                    committed: false,
                };
            }
        }
        Reply::Err {
            error: ParseError::new(
                format!("レイアウトトークンが不足しています: {}", expected),
                state.input().position(),
            )
            .with_expected_tokens([expected.clone()]),
            consumed: false,
            committed: false,
        }
    })
}

/// RunConfig/Lex プロファイルと結び付けた空白/レイアウト共有を行う。
pub fn auto_whitespace<T>(parser: Parser<T>, cfg: AutoWhitespaceConfig) -> Parser<T>
where
    T: Clone + Send + Sync + 'static,
{
    Parser::new(move |state| {
        let run_space = match cfg.strategy {
            AutoWhitespaceStrategy::ForceProfile | AutoWhitespaceStrategy::NoLexBridge => None,
            _ => decode_lex_space(&state.run_config),
        };
        let chosen_space = match cfg.strategy {
            AutoWhitespaceStrategy::PreferRunConfig => run_space.or_else(|| cfg.profile.clone()),
            AutoWhitespaceStrategy::ForceProfile => cfg.profile.clone(),
            AutoWhitespaceStrategy::NoLexBridge => None,
        };
        let prev_space = state.space();
        let applied_space = chosen_space.clone().or_else(|| prev_space.clone());
        if let Some(sp) = applied_space {
            state.set_space(Some(sp));
        } else {
            state.set_space(None);
        }

        let run_layout = match cfg.strategy {
            AutoWhitespaceStrategy::ForceProfile | AutoWhitespaceStrategy::NoLexBridge => None,
            _ => decode_layout_profile(&state.run_config),
        };
        let chosen_layout = match cfg.strategy {
            AutoWhitespaceStrategy::PreferRunConfig => {
                run_layout.clone().or_else(|| cfg.layout.clone())
            }
            AutoWhitespaceStrategy::ForceProfile => cfg.layout.clone(),
            AutoWhitespaceStrategy::NoLexBridge => None,
        };
        let prev_layout = state.layout_profile();
        let applied_layout = chosen_layout.clone().or_else(|| prev_layout.clone());
        state.set_layout_profile(applied_layout);

        let reply = parser.parse(state);
        state.set_space(prev_space);
        state.set_layout_profile(prev_layout);
        reply
    })
}

/// 後続の空白をまとめて処理する。
pub fn lexeme<A, S>(space: S, parser: Parser<A>) -> Parser<A>
where
    A: Clone + Send + Sync + 'static,
    S: Into<Option<Parser<()>>>,
{
    let space = space.into();
    Parser::new(move |state| {
        let start_input = state.input().clone();
        match parser.parse(state) {
            Reply::Ok {
                value,
                span,
                consumed,
                rest,
            } => {
                state.set_input(rest.clone());
                let mut tail_input = rest.clone();
                let mut consumed_flag = consumed;
                if !state.layout_active() {
                    if let Some(space_parser) = space.clone().or_else(|| state.space()) {
                        state.set_input(tail_input.clone());
                        let space_start = tail_input.clone();
                        match space_parser.parse(state) {
                            Reply::Ok {
                                rest: space_rest,
                                consumed: space_consumed,
                                ..
                            } => {
                                consumed_flag = consumed_flag || space_consumed;
                                tail_input = space_rest.clone();
                                state.set_input(space_rest);
                                if space_consumed && state.cst_enabled() {
                                    if let Some(text) = slice_input_text(&space_start, &tail_input)
                                    {
                                        let span = span_from_inputs(&space_start, &tail_input);
                                        state.record_cst_trivia(
                                            TriviaKind::Whitespace,
                                            text,
                                            span,
                                            true,
                                        );
                                    }
                                }
                            }
                            Reply::Err {
                                error,
                                consumed: space_consumed,
                                committed,
                            } => {
                                if space_consumed || committed {
                                    state.set_input(start_input);
                                    return Reply::Err {
                                        error,
                                        consumed: true,
                                        committed,
                                    };
                                } else {
                                    state.set_input(tail_input.clone());
                                }
                            }
                        }
                    }
                }
                Reply::Ok {
                    value,
                    span,
                    consumed: consumed_flag,
                    rest: tail_input,
                }
            }
            Reply::Err {
                error,
                consumed,
                committed,
            } => Reply::Err {
                error,
                consumed,
                committed,
            },
        }
    })
}

/// 記号を読み取り、後続の空白もまとめて消費する。
pub fn symbol<S>(space: S, text: impl AsRef<str>) -> Parser<()>
where
    S: Into<Option<Parser<()>>>,
{
    let text = text.as_ref().to_string();
    let space = space.into();
    let id = ParserId::fresh();
    Parser::with_id(id, move |state| {
        state.register_meta(id, ParserMetaKind::Symbol, text.clone(), None);
        if text.is_empty() {
            return Reply::Err {
                error: ParseError::new("空の記号は許可されていません", state.input().position()),
                consumed: false,
                committed: false,
            };
        }
        let start_input = state.input().clone();
        let remaining = start_input.remaining();
        if remaining.starts_with(&text) {
            let rest = start_input.advance(text.len());
            let span = span_from_inputs(&start_input, &rest);
            state.set_input(rest.clone());
            if state.cst_enabled() {
                state.record_cst_token(
                    TextString::from("symbol"),
                    TextString::from(text.clone()),
                    span.clone(),
                );
            }
            let mut tail_input = rest.clone();
            let mut consumed_flag = true;
            if !state.layout_active() {
                if let Some(space_parser) = space.clone().or_else(|| state.space()) {
                    state.set_input(tail_input.clone());
                    let space_start = tail_input.clone();
                    match space_parser.parse(state) {
                        Reply::Ok {
                            rest: space_rest,
                            consumed: space_consumed,
                            ..
                        } => {
                            consumed_flag = consumed_flag || space_consumed;
                            tail_input = space_rest.clone();
                            state.set_input(space_rest);
                            if space_consumed && state.cst_enabled() {
                                if let Some(text) = slice_input_text(&space_start, &tail_input) {
                                    let span = span_from_inputs(&space_start, &tail_input);
                                    state.record_cst_trivia(
                                        TriviaKind::Whitespace,
                                        text,
                                        span,
                                        true,
                                    );
                                }
                            }
                        }
                        Reply::Err {
                            error,
                            consumed: space_consumed,
                            committed,
                        } => {
                            if space_consumed || committed {
                                state.set_input(start_input);
                                return Reply::Err {
                                    error,
                                    consumed: true,
                                    committed,
                                };
                            } else {
                                state.set_input(tail_input.clone());
                            }
                        }
                    }
                }
            }
            state.record_semantic_token("operator", span.clone());
            Reply::Ok {
                value: (),
                span,
                consumed: consumed_flag,
                rest: tail_input,
            }
        } else {
            Reply::Err {
                error: ParseError::new(format!("期待した記号: {}", text), state.input().position())
                    .with_expected_tokens([text.clone()]),
                consumed: false,
                committed: false,
            }
        }
    })
}

/// キーワードを読み取り、識別子境界を検査する。
pub fn keyword<S>(space: S, kw: impl AsRef<str>) -> Parser<()>
where
    S: Into<Option<Parser<()>>>,
{
    let kw = kw.as_ref().to_string();
    let space = space.into();
    let id = ParserId::fresh();
    Parser::with_id(id, move |state| {
        state.register_meta(id, ParserMetaKind::Keyword, kw.clone(), None);
        if kw.is_empty() {
            return Reply::Err {
                error: ParseError::new(
                    "空のキーワードは許可されていません",
                    state.input().position(),
                ),
                consumed: false,
                committed: false,
            };
        }
        let start_input = state.input().clone();
        let remaining = start_input.remaining();
        if remaining.starts_with(&kw) {
            let rest = start_input.advance(kw.len());
            if let Some(ch) = rest.remaining().chars().next() {
                if let Err(msg) = state.identifier_profile().validate_char(ch) {
                    state.set_input(start_input);
                    return Reply::Err {
                        error: ParseError::new(msg, rest.position()),
                        consumed: true,
                        committed: false,
                    };
                }
                if is_ident_continue(ch, state.identifier_profile())
                    || is_ident_start(ch, state.identifier_profile())
                {
                    state.set_input(start_input);
                    return Reply::Err {
                        error: ParseError::new(
                            format!("キーワード '{}' の後ろに識別子が続いています", kw),
                            rest.position(),
                        ),
                        consumed: true,
                        committed: false,
                    };
                }
            }
            let span = span_from_inputs(&start_input, &rest);
            state.set_input(rest.clone());
            if state.cst_enabled() {
                state.record_cst_token(
                    TextString::from("keyword"),
                    TextString::from(kw.clone()),
                    span.clone(),
                );
            }
            let mut tail_input = rest.clone();
            let mut consumed_flag = true;
            if !state.layout_active() {
                if let Some(space_parser) = space.clone().or_else(|| state.space()) {
                    state.set_input(tail_input.clone());
                    let space_start = tail_input.clone();
                    match space_parser.parse(state) {
                        Reply::Ok {
                            rest: space_rest,
                            consumed: space_consumed,
                            ..
                        } => {
                            consumed_flag = consumed_flag || space_consumed;
                            tail_input = space_rest.clone();
                            state.set_input(space_rest);
                            if space_consumed && state.cst_enabled() {
                                if let Some(text) = slice_input_text(&space_start, &tail_input) {
                                    let span = span_from_inputs(&space_start, &tail_input);
                                    state.record_cst_trivia(
                                        TriviaKind::Whitespace,
                                        text,
                                        span,
                                        true,
                                    );
                                }
                            }
                        }
                        Reply::Err {
                            error,
                            consumed: space_consumed,
                            committed,
                        } => {
                            if space_consumed || committed {
                                state.set_input(start_input);
                                return Reply::Err {
                                    error,
                                    consumed: true,
                                    committed,
                                };
                            } else {
                                state.set_input(tail_input.clone());
                            }
                        }
                    }
                }
            }
            state.record_semantic_token("keyword", span.clone());
            Reply::Ok {
                value: (),
                span,
                consumed: consumed_flag,
                rest: tail_input,
            }
        } else {
            Reply::Err {
                error: ParseError::new(
                    format!("期待したキーワード: {}", kw),
                    state.input().position(),
                )
                .with_expected_tokens([kw.clone()]),
                consumed: false,
                committed: false,
            }
        }
    })
}

/// 位置パーサー。
pub fn position() -> Parser<Span> {
    Parser::new(|state| {
        let span = empty_span(state.input());
        Reply::Ok {
            value: span.clone(),
            span,
            consumed: false,
            rest: state.input().clone(),
        }
    })
}

/// 値とスパンを返す。
pub fn spanned<T>(parser: Parser<T>) -> Parser<(T, Span)>
where
    T: Clone + Send + Sync + 'static,
{
    parser.spanned()
}

/// 埋め込み DSL をパースするコンビネータ。
pub fn embedded_dsl<T>(spec: EmbeddedDslSpec<T>) -> Parser<EmbeddedNode<T>>
where
    T: Clone + Send + Sync + 'static,
{
    let spec = Arc::new(spec);
    Parser::new(move |state| {
        let input = state.input().clone();
        let Some(after_start) = spec.boundary.match_start(&input) else {
            return Reply::Err {
                error: ParseError::new("埋め込み DSL の開始境界が見つかりません", input.position()),
                consumed: false,
                committed: false,
            };
        };

        let content_start = after_start.clone();
        let remaining = after_start.remaining();
        let end_index = match remaining.find(spec.boundary.end.as_str()) {
            Some(index) => index,
            None => {
                return Reply::Err {
                    error: ParseError::new(
                        "埋め込み DSL の終了境界が見つかりません",
                        after_start.position(),
                    ),
                    consumed: true,
                    committed: false,
                };
            }
        };
        let content = &remaining[..end_index];
        let after_content = after_start.advance(end_index);
        let after_end = after_content.advance(spec.boundary.end.len());
        let span = input.span_to(&after_end);

        let mut embedded_state = ParseState::new(content, state.run_config.clone());
        embedded_state.enter_dsl(&spec.dsl_id);
        embedded_state.set_context_bridge(Some(spec.context.clone()));
        let embedded_reply = spec.parser.parse(&mut embedded_state);
        embedded_state.exit_dsl();
        let mut diagnostics = embedded_state.take_diagnostics();
        let base_pos = content_start.position();

        for diag in diagnostics.iter_mut() {
            diag.position = shift_position(base_pos, diag.position);
            if diag.source_dsl.is_none() {
                diag.source_dsl = Some(spec.dsl_id.clone());
            }
        }
        for diag in diagnostics.iter().cloned() {
            state.push_diagnostic(diag);
        }

        match embedded_reply {
            Reply::Ok {
                value,
                span: _embedded_span,
                rest,
                ..
            } => {
                if state.run_config.require_eof && !rest.is_empty() {
                    let mut error = ParseError::new("未消費の入力が残っています", rest.position())
                        .with_source_dsl(spec.dsl_id.clone());
                    error.position = shift_position(base_pos, error.position);
                    return Reply::Err {
                        error,
                        consumed: true,
                        committed: false,
                    };
                }
                Reply::Ok {
                    value: EmbeddedNode {
                        dsl_id: spec.dsl_id.clone(),
                        span: span.clone(),
                        ast: value,
                        cst: None,
                        diagnostics,
                    },
                    span,
                    consumed: true,
                    rest: after_end,
                }
            }
            Reply::Err {
                mut error,
                committed,
                ..
            } => {
                if error.source_dsl.is_none() {
                    error.source_dsl = Some(spec.dsl_id.clone());
                }
                error.position = shift_position(base_pos, error.position);
                Reply::Err {
                    error,
                    consumed: true,
                    committed,
                }
            }
        }
    })
}

/// 左結合チェーン。
pub fn chainl1<T, F>(term: Parser<T>, op: Parser<F>) -> Parser<T>
where
    T: Clone + Send + Sync + 'static,
    F: Clone + Fn(T, T) -> T + Send + Sync + 'static,
{
    term.chainl1(op)
}

/// 右結合チェーン。
pub fn chainr1<T, F>(term: Parser<T>, op: Parser<F>) -> Parser<T>
where
    T: Clone + Send + Sync + 'static,
    F: Clone + Fn(T, T) -> T + Send + Sync + 'static,
{
    term.chainr1(op)
}

/// パース実行時の可変状態。
#[derive(Debug)]
pub struct ParseState {
    input: Input,
    pub run_config: RunConfig,
    pub memo: MemoTable,
    observer: Option<ParseObserver>,
    diagnostics: Vec<ParseError>,
    dsl_stack: Vec<String>,
    context_bridge: Option<ContextBridge>,
    pub recovered: bool,
    active_parsers: HashSet<MemoKey>,
    recover_config: RecoverConfig,
    recoveries: usize,
    recover_resync_bytes: usize,
    space: Option<Parser<()>>,
    layout_profile: Option<LayoutProfile>,
    layout_pending: VecDeque<String>,
    layout_stack: Vec<usize>,
    identifier_profile: IdentifierProfile,
    meta_registry: ParseMetaRegistry,
    meta_rule_stack: Vec<ParserId>,
    observed_tokens: Vec<ObservedToken>,
    cst_builder: Option<CstBuilder>,
}

impl ParseState {
    pub fn new(source: &str, run_config: RunConfig) -> Self {
        Self::new_with_input(Input::new(source), run_config)
    }

    pub fn new_shared(source: Arc<str>, run_config: RunConfig) -> Self {
        Self::new_with_input(Input::from_arc_str(source), run_config)
    }

    pub fn new_with_input(input: Input, run_config: RunConfig) -> Self {
        let identifier_profile = IdentifierProfile::from_run_config(&run_config);
        let space = decode_lex_space(&run_config);
        let layout_profile = decode_layout_profile(&run_config);
        let profile_config = decode_profile_config(&run_config);
        let observer = profile_config
            .enabled
            .then(|| ParseObserver::new(true, profile_config.profile_output));
        let cst_enabled = decode_cst_mode(&run_config);
        let cst_builder = cst_enabled.then(|| CstBuilder::new(input.position()));
        let mut layout_stack = Vec::new();
        if matches!(layout_profile, Some(ref lp) if lp.offside) {
            layout_stack.push(0);
        }
        let recover_config = decode_recover_config(&run_config);
        Self {
            input,
            run_config,
            memo: MemoTable::new(),
            observer,
            diagnostics: Vec::new(),
            dsl_stack: Vec::new(),
            context_bridge: None,
            recovered: false,
            active_parsers: HashSet::new(),
            recover_config,
            recoveries: 0,
            recover_resync_bytes: 0,
            space,
            layout_profile,
            layout_pending: VecDeque::new(),
            layout_stack,
            identifier_profile,
            meta_registry: ParseMetaRegistry::default(),
            meta_rule_stack: Vec::new(),
            observed_tokens: Vec::new(),
            cst_builder,
        }
    }

    pub fn input(&self) -> &Input {
        &self.input
    }

    pub fn set_input(&mut self, input: Input) {
        self.input = input;
    }

    pub fn enter_dsl(&mut self, dsl_id: &str) {
        self.dsl_stack.push(dsl_id.to_string());
    }

    pub fn exit_dsl(&mut self) {
        self.dsl_stack.pop();
    }

    pub fn current_dsl_id(&self) -> Option<&str> {
        self.dsl_stack.last().map(String::as_str)
    }

    pub fn set_context_bridge(&mut self, bridge: Option<ContextBridge>) {
        self.context_bridge = bridge;
    }

    pub fn context_bridge(&self) -> Option<&ContextBridge> {
        self.context_bridge.as_ref()
    }

    pub fn space(&self) -> Option<Parser<()>> {
        self.space.clone()
    }

    pub fn set_space(&mut self, space: Option<Parser<()>>) {
        self.space = space;
    }

    pub fn layout_profile(&self) -> Option<LayoutProfile> {
        self.layout_profile.clone()
    }

    pub fn set_layout_profile(&mut self, layout: Option<LayoutProfile>) {
        self.layout_profile = layout;
        self.layout_pending.clear();
        self.layout_stack.clear();
        if matches!(self.layout_profile, Some(ref lp) if lp.offside) {
            self.layout_stack.push(0);
        }
    }

    fn layout_active(&self) -> bool {
        matches!(self.layout_profile, Some(ref lp) if lp.offside)
    }

    #[allow(dead_code)]
    fn layout_peek_token(&self) -> Option<String> {
        self.layout_pending.front().cloned()
    }

    fn layout_pop_token(&mut self) -> Option<String> {
        self.layout_pending.pop_front()
    }

    fn emit_layout_diagnostic(&mut self, message: impl Into<String>) {
        let error = ParseError::new(message, self.input.position());
        self.push_diagnostic(error);
    }

    fn produce_layout_tokens(&mut self) {
        if !self.layout_active() {
            return;
        }
        if !self.layout_pending.is_empty() {
            return;
        }
        let profile = match self.layout_profile.clone() {
            Some(p) => p,
            None => return,
        };

        // EOF 時に未処理の dedent を吐き出す。
        if self.input.is_empty() {
            while self.layout_stack.len() > 1 {
                self.layout_stack.pop();
                self.layout_pending.push_back(profile.dedent_token.clone());
            }
            return;
        }

        // 改行を検出して消費し、newline トークンを生成。
        let remaining = self.input.remaining();
        let mut advanced_newline = false;
        if remaining.starts_with("\r\n") {
            let rest = self.input.advance(2);
            self.set_input(rest);
            advanced_newline = true;
        } else if remaining.starts_with('\n') {
            let rest = self.input.advance(1);
            self.set_input(rest);
            advanced_newline = true;
        }
        if advanced_newline && !(self.input.line() == 1 && self.input.byte_offset() == 0) {
            self.layout_pending.push_back(profile.newline_token.clone());
        }

        // 行頭でインデント幅を評価し、indent/dedent を生成。
        if self.input.column() == 1 {
            let (indent_width, mixed, consumed_bytes) =
                count_indent(self.input.remaining(), profile.allow_mixed_tabs);
            if mixed {
                self.emit_layout_diagnostic("インデントにタブとスペースが混在しています");
            }
            if consumed_bytes > 0 {
                let rest = self.input.advance(consumed_bytes);
                self.set_input(rest);
            }
            let current = *self.layout_stack.last().unwrap_or(&0);
            if indent_width > current {
                self.layout_stack.push(indent_width);
                self.layout_pending.push_back(profile.indent_token.clone());
            } else if indent_width < current {
                while let Some(&top) = self.layout_stack.last() {
                    if top > indent_width {
                        self.layout_stack.pop();
                        self.layout_pending.push_back(profile.dedent_token.clone());
                    } else {
                        break;
                    }
                }
            }
        }
    }

    pub fn identifier_profile(&self) -> IdentifierProfile {
        self.identifier_profile
    }

    pub fn meta_registry(&self) -> &ParseMetaRegistry {
        &self.meta_registry
    }

    pub fn observed_tokens(&self) -> &[ObservedToken] {
        &self.observed_tokens
    }

    pub fn take_meta_registry(&mut self) -> ParseMetaRegistry {
        std::mem::take(&mut self.meta_registry)
    }

    pub fn take_observed_tokens(&mut self) -> Vec<ObservedToken> {
        std::mem::take(&mut self.observed_tokens)
    }

    pub fn cst_enabled(&self) -> bool {
        self.cst_builder.is_some()
    }

    pub fn take_cst(&mut self) -> Option<CstNode> {
        let end = self.input.position();
        self.cst_builder.take().map(|builder| builder.finish(end))
    }

    pub fn packrat_enabled(&self) -> bool {
        self.run_config.packrat
    }

    fn register_meta(
        &mut self,
        id: ParserId,
        kind: ParserMetaKind,
        name: String,
        token_kind: Option<String>,
    ) {
        self.meta_registry.register(id, kind, name, token_kind);
    }

    fn update_meta_doc(&mut self, id: ParserId, doc: String) {
        self.meta_registry.update_doc(id, doc);
    }

    fn enter_rule_meta(&mut self, id: ParserId) {
        if let Some(parent) = self.meta_rule_stack.last().copied() {
            if parent != id {
                self.meta_registry.add_child(parent, id);
            }
        }
        self.meta_rule_stack.push(id);
    }

    fn exit_rule_meta(&mut self, id: ParserId) {
        if let Some(last) = self.meta_rule_stack.pop() {
            if last != id {
                self.meta_rule_stack.push(last);
                if let Some(pos) = self.meta_rule_stack.iter().rposition(|entry| *entry == id) {
                    self.meta_rule_stack.remove(pos);
                }
            }
        }
    }

    fn record_semantic_token(&mut self, kind: impl Into<String>, span: Span) {
        self.observed_tokens.push(ObservedToken {
            kind: kind.into(),
            span,
        });
    }

    fn record_cst_token(&mut self, kind: TextString, text: TextString, span: Span) {
        if let Some(builder) = self.cst_builder.as_mut() {
            builder.push_token(CstToken { kind, text, span });
        }
    }

    fn record_cst_trivia(
        &mut self,
        kind: TriviaKind,
        text: TextString,
        span: Span,
        trailing: bool,
    ) {
        if let Some(builder) = self.cst_builder.as_mut() {
            builder.push_trivia(Trivia { kind, text, span }, trailing);
        }
    }

    fn left_recursion_active(&self, key: MemoKey) -> bool {
        self.active_parsers.contains(&key)
    }

    fn enter_parser(&mut self, key: MemoKey) {
        self.active_parsers.insert(key);
    }

    fn exit_parser(&mut self, key: MemoKey) {
        self.active_parsers.remove(&key);
    }

    pub fn push_diagnostic(&mut self, mut error: ParseError) {
        if error.source_dsl.is_none() {
            if let Some(dsl_id) = self.current_dsl_id() {
                error.source_dsl = Some(dsl_id.to_string());
            }
        }
        self.diagnostics.push(error);
    }

    fn recover_limits_exceeded(&self) -> bool {
        if let Some(limit) = self.recover_config.max_recoveries {
            if self.recoveries >= limit {
                return true;
            }
        }
        if let Some(limit) = self.recover_config.max_resync_bytes {
            if self.recover_resync_bytes >= limit {
                return true;
            }
        }
        false
    }

    fn match_sync_token(&self, start: &Input, end: &Input) -> Option<String> {
        let consumed_bytes = end.byte_offset().saturating_sub(start.byte_offset());
        let consumed = start.remaining().get(..consumed_bytes)?;
        if self.recover_config.sync_tokens.is_empty() {
            return Some(consumed.to_string());
        }
        if self
            .recover_config
            .sync_tokens
            .iter()
            .any(|token| token == consumed)
        {
            return Some(consumed.to_string());
        }
        self.recover_config
            .sync_tokens
            .iter()
            .find(|token| consumed.starts_with(token.as_str()))
            .cloned()
            .or_else(|| Some(consumed.to_string()))
    }

    pub fn take_diagnostics(&mut self) -> Vec<ParseError> {
        std::mem::take(&mut self.diagnostics)
    }

    pub fn memo_get<T: Clone + Send + Sync + 'static>(&self, key: MemoKey) -> Option<Reply<T>> {
        self.memo
            .get(&key)
            .and_then(|entry| entry.downcast_ref::<MemoizedReply<T>>())
            .map(MemoizedReply::clone_reply)
    }

    pub fn memo_put<T: Clone + Send + Sync + 'static>(&mut self, key: MemoKey, reply: &Reply<T>) {
        self.memo.insert(
            key,
            Box::new(MemoizedReply {
                reply: reply.clone(),
            }),
        );
    }

    pub fn record_packrat_hit(&mut self) {
        if let Some(observer) = self.observer.as_mut() {
            observer.record_packrat_hit();
        }
    }

    pub fn record_packrat_miss(&mut self) {
        if let Some(observer) = self.observer.as_mut() {
            observer.record_packrat_miss();
        }
    }

    pub fn record_backtrack(&mut self) {
        if let Some(observer) = self.observer.as_mut() {
            observer.record_backtrack();
        }
    }

    pub fn record_recovery(&mut self) {
        if let Some(observer) = self.observer.as_mut() {
            observer.record_recovery();
        }
    }

    pub fn record_left_recursion_guard(&mut self) {
        if let Some(observer) = self.observer.as_mut() {
            observer.record_left_recursion_guard();
        }
    }

    pub fn take_profile(&mut self) -> Option<(ParserProfile, Option<PathBuf>)> {
        let memo_entries = self.memo.len();
        self.observer
            .take()
            .and_then(|observer| observer.finalize(memo_entries))
    }
}

/// バッチランナー。`require_eof` と Packrat 設定を反映する。
pub fn run<T>(parser: &Parser<T>, input: &str, cfg: &RunConfig) -> ParseResult<T>
where
    T: Clone + Send + Sync + 'static,
{
    let mut state = ParseState::new(input, cfg.clone());
    run_with_state(parser, &mut state, cfg)
}

/// 入力バッファ（共有済み）を受け取り、余計な全体コピーを避けて実行する。
pub fn run_shared<T>(parser: &Parser<T>, input: Arc<str>, cfg: &RunConfig) -> ParseResult<T>
where
    T: Clone + Send + Sync + 'static,
{
    let mut state = ParseState::new_shared(input, cfg.clone());
    run_with_state(parser, &mut state, cfg)
}

/// CST を収集しながら実行する。
pub fn run_with_cst<T>(
    parser: &Parser<T>,
    input: &str,
    cfg: &RunConfig,
) -> ParseResult<CstOutput<T>>
where
    T: Clone + Send + Sync + 'static,
{
    let cst_cfg = enable_cst_config(cfg);
    let mut state = ParseState::new(input, cst_cfg.clone());
    run_with_state_cst(parser, &mut state, &cst_cfg)
}

/// 入力バッファ（共有済み）を受け取り、CST を収集しながら実行する。
pub fn run_with_cst_shared<T>(
    parser: &Parser<T>,
    input: Arc<str>,
    cfg: &RunConfig,
) -> ParseResult<CstOutput<T>>
where
    T: Clone + Send + Sync + 'static,
{
    let cst_cfg = enable_cst_config(cfg);
    let mut state = ParseState::new_shared(input, cst_cfg.clone());
    run_with_state_cst(parser, &mut state, &cst_cfg)
}

fn run_with_state<T>(parser: &Parser<T>, state: &mut ParseState, cfg: &RunConfig) -> ParseResult<T>
where
    T: Clone + Send + Sync + 'static,
{
    let reply = parser.parse(state);
    let mut result = match reply {
        Reply::Ok {
            value, span, rest, ..
        } => {
            state.set_input(rest);
            if cfg.require_eof && !state.input().is_empty() {
                let error = ParseError::new("未消費の入力が残っています", state.input().position());
                ParseResult::from_error(error, cfg.legacy_result)
            } else {
                ParseResult::from_value(value, span)
            }
        }
        Reply::Err { error, .. } => ParseResult::from_error(error, cfg.legacy_result),
    };

    let diagnostics = state.take_diagnostics();
    if !diagnostics.is_empty() {
        result.extend_diagnostics(diagnostics);
    }
    result.recovered |= state.recovered;
    if let Some((profile, output)) = state.take_profile() {
        if let Some(path) = output {
            if let Err(err) = write_profile_report(&profile, &path) {
                eprintln!("parse profile 出力に失敗しました: {err}");
            }
        }
        result.profile = Some(profile);
    }
    result
}

fn run_with_state_cst<T>(
    parser: &Parser<T>,
    state: &mut ParseState,
    cfg: &RunConfig,
) -> ParseResult<CstOutput<T>>
where
    T: Clone + Send + Sync + 'static,
{
    let reply = parser.parse(state);
    let mut result = match reply {
        Reply::Ok {
            value, span, rest, ..
        } => {
            state.set_input(rest);
            if cfg.require_eof && !state.input().is_empty() {
                let error = ParseError::new("未消費の入力が残っています", state.input().position());
                ParseResult::from_error(error, cfg.legacy_result)
            } else {
                let cst = state.take_cst().unwrap_or_else(CstNode::empty);
                ParseResult::from_value(CstOutput { ast: value, cst }, span)
            }
        }
        Reply::Err { error, .. } => ParseResult::from_error(error, cfg.legacy_result),
    };

    let diagnostics = state.take_diagnostics();
    if !diagnostics.is_empty() {
        result.extend_diagnostics(diagnostics);
    }
    result.recovered |= state.recovered;
    if let Some((profile, output)) = state.take_profile() {
        if let Some(path) = output {
            if let Err(err) = write_profile_report(&profile, &path) {
                eprintln!("parse profile 出力に失敗しました: {err}");
            }
        }
        result.profile = Some(profile);
    }
    result
}

/// RunConfig を指定しない場合のエイリアス。
pub fn run_with_default<T>(parser: &Parser<T>, input: &str) -> ParseResult<T>
where
    T: Clone + Send + Sync + 'static,
{
    run(parser, input, &RunConfig::default())
}

/// `RunConfig.extensions["recover"].mode="collect"` を強制しつつ実行するヘルパ。
///
/// * 既定では `sync_tokens=[";"]` を補う（未指定の場合のみ）。
/// * `extensions["recover"].notes=true` が指定されている場合、`recover_with_context` の
///   `context` を `ParseError.notes` にも露出する。
pub fn run_with_recovery<T>(parser: &Parser<T>, input: &str) -> ParseResult<T>
where
    T: Clone + Send + Sync + 'static,
{
    run_with_recovery_config(parser, input, &RunConfig::default())
}

/// 既存の RunConfig をベースに `mode="collect"` を有効化して実行する。
pub fn run_with_recovery_config<T>(
    parser: &Parser<T>,
    input: &str,
    cfg: &RunConfig,
) -> ParseResult<T>
where
    T: Clone + Send + Sync + 'static,
{
    let cfg = cfg.with_extension("recover", |mut ext| {
        ext.insert("mode".into(), Value::String("collect".into()));
        if !ext.contains_key("sync_tokens") {
            ext.insert(
                "sync_tokens".into(),
                Value::Array(vec![Value::String(";".into())]),
            );
        }
        ext
    });
    run(parser, input, &cfg)
}

/// CLI / LSP など外部向け診断形式へ変換する。
pub fn parse_result_to_guard_diagnostics<T>(result: &ParseResult<T>) -> Vec<GuardDiagnostic> {
    result.guard_diagnostics()
}

/// ParseError の列を GuardDiagnostic へ変換する。
pub fn parse_errors_to_guard_diagnostics(errors: &[ParseError]) -> Vec<GuardDiagnostic> {
    errors.iter().map(ParseError::to_guard_diagnostic).collect()
}

fn write_profile_report(profile: &ParserProfile, path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let body = serde_json::to_string_pretty(&profile.to_json())?;
    fs::write(path, body)
}
