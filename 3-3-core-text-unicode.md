# 3.3 Core Text & Unicode

> 目的：`byte/char/grapheme` の三層モデルを標準 API 化し、文字列操作・正規化・セグメンテーション・Lex 連携を統一する。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | 正式仕様 |
| 効果タグ | `@pure`, `effect {mem}`, `effect {unicode}`, `effect {io}`, `effect {regex}` |
| 依存モジュール | `Core.Prelude`, `Core.Iter`, `Core.Collections`, `Core.Diagnostics`, `Core.IO` |
| 相互参照 | [1.4 Unicode 文字モデル](1-4-test-unicode-model.md), [2.3 字句レイヤユーティリティ](2-3-lexer.md), [3.2 Core Collections](3-2-core-collections.md), [3.5 Core IO & Path](3-5-core-io-path.md) |

## 1. モジュール構成と import

- `use Core.Text;` は高レベルな `String`/`Str`/`TextBuilder` を、`use Core.Unicode;` は低レベルな `Grapheme`, `Scalar`, 正規化ユーティリティを提供する。【F:1-4-test-unicode-model.md†L1-L40】
- `Core.Text` は基本的に `@pure` を維持しつつ、メモリ確保やバッファ再配置を行う API に `effect {mem}` を付与する。
- `Core.Unicode` は `effect {unicode}` を導入し、正規化やセグメンテーションが計算コストを伴うことを明示する。
- `use Core;` 経由で Prelude/Iter から `Text` 操作を行う場合、`Iter.flat_map` と `Unicode.segment_graphemes` を組み合わせた例を提供する。【F:3-1-core-prelude-iteration.md†L167-L171】

## 2. 文字列型の層構造

```reml
pub type Bytes
pub type Str
pub type String
pub type Grapheme
pub type GraphemeSeq = List<Grapheme>  // 所有型として採用
pub type TextBuilder

fn as_bytes(str: Str) -> Bytes                // `@pure`
fn to_string(bytes: Bytes) -> Result<String, DecodeError> // `effect {unicode}`
fn str_from_slice(bytes: Bytes) -> Result<Str, DecodeError> // `@pure`
fn string_clone(str: Str) -> String            // `effect {mem}`
fn grapheme_seq(str: Str) -> GraphemeSeq       // `effect {unicode}`
fn builder() -> TextBuilder                    // `effect {mem}`
```

| 型 | 役割 | 主な操作 | 効果 |
| --- | --- | --- | --- |
| `Bytes` | UTF-8 生バイト列。IO や圧縮との境界を表す。 | `Bytes.decode_utf8`, `Bytes.slice` | `effect {io}` or `@pure` |
| `Str` | UTF-8 参照スライス（不変）。 | `Str.len_bytes`, `Str.iter_graphemes` | `@pure` |
| `String` | 所有文字列。内部で `Vec<u8>` を保持。 | `String.push_str`, `String.normalize` | `effect {mem}` / `effect {unicode}` |
| `GraphemeSeq` | `Grapheme` の列。表示単位操作を提供。 | `GraphemeSeq.segment`, `GraphemeSeq.width` | `effect {unicode}` |
| `TextBuilder` | 可変構築器。複数段階で文字列を構成。 | `append`, `push_grapheme`, `finish` | `effect {mem, unicode}` |

- `String`/`Str` は `Core.Collections` の `Vec<u8>` を内部利用し、`Iter.collect_vec` からの構築を効率化する計画である。【F:3-2-core-collections.md†L42-L60】
- `Bytes -> String` 変換は `Result<String, DecodeError>` を返し、0-0 章で強調した「例外なし」の方針を維持する。【F:0-0-overview.md†L27-L40】

## 3. 正規化とケース変換

### 3.1 正規化 API

```reml
enum NormalizationForm = NFC | NFD | NFKC | NFKD

type UnicodeError = {
  kind: UnicodeErrorKind,
  span: Option<Span>,
  message: Str
}

type UnicodeErrorKind = InvalidUtf8 | UnsupportedScalar | UnsupportedLocale

fn normalize(string: String, form: NormalizationForm) -> Result<String, UnicodeError> // `effect {unicode}`
fn is_normalized(str: Str, form: NormalizationForm) -> Bool                            // `@pure`
fn prepare_identifier(str: Str) -> Result<String, UnicodeError>                       // `effect {unicode}`
```

- `prepare_identifier` は `Core.Parse` の字句解析で利用し、識別子の正規化方針を統一する。【F:2-3-lexer.md†L90-L140】
- `UnicodeError` は `Core.Diagnostics` の `Diagnostic` と相互変換するヘルパ（`UnicodeError::to_diagnostic`）を提供し、`audit_id` を保持できるメタデータスロットを確保する。【F:2-5-error.md†L50-L83】

### 3.2 ケース・幅調整

```reml
fn to_upper(string: String, locale: Locale) -> Result<String, UnicodeError> // `effect {unicode}`
fn to_lower(string: String, locale: Locale) -> Result<String, UnicodeError> // `effect {unicode}`
fn width_map(str: Str, mode: WidthMode) -> Result<String, UnicodeError>     // `effect {unicode}`
```

- `Locale` が未対応の場合は `UnicodeErrorKind::UnsupportedLocale` を返す。
- 幅変換は全角/半角を双方向に変換し、CLI 出力や差分表示をロケール無依存に整える。

## 4. セグメンテーションと検索

### 4.1 Grapheme / Word / Sentence 境界

```reml
fn segment_graphemes(str: Str) -> Iter<Grapheme>             // `effect {unicode}`
fn segment_words(str: Str) -> Iter<Str>                      // `effect {unicode}`
fn segment_sentences(str: Str) -> Iter<Str>                  // `effect {unicode}`
fn grapheme_width(gr: Grapheme) -> usize                     // `@pure`
```

- `segment_graphemes` は ICU ベースの規則に準拠し、`Iter` とシームレスに連携する。
- Lex レイヤではコメントや文字列リテラル判定に `segment_graphemes` を利用し、結合文字の誤判定を防ぐ。【F:2-3-lexer.md†L40-L88】

### 4.2 部分一致と検索

```reml
enum TextPattern = Literal(Str) | GraphemeSeq(List<Grapheme>) | Regex(RegexHandle)

type RegexHandle

fn find(str: Str, pattern: TextPattern) -> Option<ByteIndex>         // `effect {regex}`
fn find_grapheme(str: Str, pattern: TextPattern) -> Option<GraphemeIndex> // `effect {regex}`
fn replace(str: Str, pattern: TextPattern, with: Str) -> Result<String, UnicodeError> // `effect {unicode, mem}`
```

- `RegexHandle` の実装はオプション機能（`feature {regex}`）として提供し、`effect {regex}` を追加で要求する。
- `GraphemeIndex` と `ByteIndex` の相互変換ヘルパ（`to_byte_index`, `to_grapheme_index`）を定義し、`Core.Parse` との整合を保つ。

## 5. IO / Diagnostics との接続

| API | シグネチャ | 効果 | 用途 |
| --- | --- | --- | --- |
| `decode_stream` | `fn decode_stream(reader: IO.Reader, options: TextDecodeOptions) -> Result<String, Diagnostic>` | `effect {io, unicode}` | ストリーミング decode。BOM 処理を Options で制御。 |
| `encode_stream` | `fn encode_stream(writer: IO.Writer, text: Str, options: TextEncodeOptions) -> Result<(), Diagnostic>` | `effect {io, unicode}` | 書き出し時のエンコーディング制御。 |
| `log_grapheme_stats` | `fn log_grapheme_stats(text: Str, audit: AuditSink) -> Result<(), Diagnostic>` | `effect {audit, unicode}` | 監査ログへ文字幅や方向性を記録。 |

- `TextDecodeOptions` にはバッファサイズ・BOM 要否・不正バイトハンドリング（`Replace`/`Error`）を定義する。
- `log_grapheme_stats` は `audit_id` と `change_set` を共通語彙として持ち、Chapter 3.6 で定義する監査モデルに合流する想定。

### 5.1 Diagnostic ハイライト統合

- `Core.Diagnostics.from_parse_error` と `Diagnostic.pretty` は、`Span` が示す範囲を `Core.Text.slice_graphemes` で抽出し、**`display_width` または `GraphemeSeq::width`** を利用して列オフセットと下線の長さを計算する。これにより、結合文字や絵文字を含む行でも 0-1 章で掲げる「分かりやすいエラーメッセージ」の条件を満たす。
- `Core.Parse` から受け取る `Input` の `g_index` / `cp_index` キャッシュを再利用し、行頭からの累積幅は `display_width(Str::slice_graphemes(..))` の結果を合計して求める。再スキャンや手動の `grapheme_at` 逐次走査は避ける。
- IDE / CLI での再描画は `GraphemeSeq` を保持したまま行い、均等幅フォント・可変幅フォントの双方で `width_map`・`grapheme_width` と整合することを確認する。幅計算を独自ロジックで複製しない（Unicode 仕様更新時の揺れを防ぐため）。

## 6. テキスト構築とビルダー

```reml
type TextBuilder

fn builder() -> TextBuilder                               // `effect {mem}`
fn append(builder: &mut TextBuilder, value: Str) -> ()     // `effect {mut, unicode}`
fn push_grapheme(builder: &mut TextBuilder, g: Grapheme) -> () // `effect {mut, unicode}`
fn reserve(builder: &mut TextBuilder, additional: usize) -> ()  // `effect {mut, mem}`
fn finish(builder: TextBuilder) -> String                  // `effect {mem}`
```

- `TextBuilder` は内部で `Vec<u8>` を保持し、必要に応じて正規化や NUL チェックを行う。`finish` は `String` を返しつつ構築器を無効化する。
- `builder` と `Iter.collect_vec` を組み合わせることで、`Iter<Grapheme>` から `String` を構築する `collect_string` ヘルパを提供予定。

## 7. 設計決定事項

### 7.1 解決済み設計問題

1. **`GraphemeSeq` の実装**: 所有型 `List<Grapheme>` として採用。不変性と構造共有によりメモリ効率を確保しつつ、安全な操作を提供。

2. **正規化デフォルト**: NFC (Normalization Form Canonical Composition) を標準とし、Web 技術標準との互換性を確保。NFKC は明示指定時のみ使用。

3. **Regex 標準搭載**: オプション機能 `feature {regex}` として提供。監査モードではパターンマッチ結果をログ化。

4. **Unicode バージョン互換性**: Unicode 15.0 をベースラインとし、後方互換性を 3 バージョンまで保証。新しい Unicode 機能は `feature` フラグで提供。

### 7.2 パフォーマンス特性

| 操作 | 計算量 | メモリ使用量 | 実装アルゴリズム |
| --- | --- | --- | --- |
| `segment_graphemes` | O(n) | O(k) where k=clusters | 有限状態オートマトン |
| `normalize` | O(n log m) | O(n) | テーブルルックアップ |
| `find` | O(nm) worst, O(n) average | O(1) | Boyer-Moore + Unicode対応 |
| `width_map` | O(n) | O(n) | 文字種別テーブル |

### 7.3 セキュリティ考慮事項

```reml
// Unicode 攻撃の軽減
fn sanitize_input(text: Str, policy: SanitizePolicy) -> Result<String, UnicodeError> // `effect {unicode}`
fn detect_suspicious_patterns(text: Str) -> List<SuspiciousPattern>                 // `@pure`
fn safe_truncate(text: Str, max_bytes: usize) -> Str                               // `@pure`
```

## 8. テンプレート文字列 API {#template-text}

> 目的：テンプレート DSL のレンダリングに必要な文字列分解・フィルター管理・安全なエスケープ戦略を標準化し、性能と安全性の両立を図る。

### 8.1 セグメント構造と抽象構文

```reml
pub type TemplateSegmentId = u32

pub type TemplateSegment =
  | Literal(str: Str)
  | Interpolate(expr: TemplateExpr, filters: List<TemplateFilterId>)
  | Control(block: TemplateBlock, body: List<TemplateSegment>)
  | Comment(str: Str)

pub type TemplateExpr =
  | Path(IdentPath)
  | Literal(TemplateLiteral)
  | Call(IdentPath, args: List<TemplateExpr>)

pub type TemplateBlock =
  | If(condition: TemplateExpr, else_body: Option<List<TemplateSegment>>)
  | For(bind: Ident, iter: TemplateExpr)
  | Scoped(scope: Ident, params: List<TemplateExpr>)

pub type TemplateLiteral = Str | Bool | Int | Float | Json
```

- `IdentPath` は `Core.Parse` の識別子正規化と整合し、`Path::from_template` が NFC 正規化後に `Result<Path, TemplateError>` を返す。
- `TemplateSegment` は `List` ベースで保持し、`TemplateProgram` 生成時に共有構造を採用することでメモリ効率を確保する。[^template-perf]
- `Control::Scoped` はテンプレート固有のネームスペースを表し、プラグインで拡張できる余地を残す。
- `Value` は Chapter 3.7 の `Core.Config`/`Core.Data` が提供する `Value` を再利用し、テンプレートと設定ファイルのシリアライズ仕様を統一する。

### 8.2 フィルター登録と Capability 連携

```reml
pub type TemplateFilter = fn(Value, args: List<Value>, ctx: &TemplateRenderCtx) -> Result<Value, TemplateError>

pub struct TemplateFilterRegistry

pub type TemplateFilterId = u32

fn register_secure(registry: &mut TemplateFilterRegistry, name: Str, filter: TemplateFilter, requires: CapabilityId) -> Result<(), TemplateFilterError> // `effect {security}`
fn register_pure(registry: &mut TemplateFilterRegistry, name: Str, filter: TemplateFilter) -> Result<(), TemplateFilterError>                        // `@pure`
fn lookup(registry: &TemplateFilterRegistry, id: TemplateFilterId) -> Option<(TemplateFilter, CapabilityId)>
```

- `register_secure` は Capability Registry (3.8 節) の検証フローを呼び出し、権限未付与の場合は `TemplateError::CapabilityMissing` を返す。`TemplateFilterError` は `Diagnostic` に変換し、`template.filter.register_failed` を発火させる。
- `register_pure` は `@pure` なフィルター向けで、`requires` を持たず Capability チェックをスキップする。
- `TemplateFilterId` は登録時に生成され、テンプレート解析段階でシンボル解決を行う。

### 8.3 コンパイルとレンダリング API

```reml
pub type TemplateProgram
pub type TemplateContext = Map<Str, Value>
pub type TemplateRenderCtx
pub type TemplateSink = fn(Chunk) -> Result<(), TemplateError>
pub enum EscapePolicy = HtmlStrict | Text | Custom(fn(Str) -> Result<String, TemplateError>)

fn compile(template: Str, registry: &TemplateFilterRegistry) -> Result<TemplateProgram, TemplateError> // `effect {unicode, mem}`
fn render(program: &TemplateProgram, context: TemplateContext, sink: TemplateSink) -> Result<(), TemplateError> // `effect {runtime, io, security}`
fn render_to_string(program: &TemplateProgram, context: TemplateContext) -> Result<String, TemplateError>        // `effect {runtime, mem, security}`
fn with_escape_policy(program: &TemplateProgram, policy: EscapePolicy) -> TemplateProgram                       // `@pure`
```

- `compile` は `Core.Parse.Template` の構文解析を再利用し、`TemplateSegment` を分析して `TemplateProgram` を生成する。複数回レンダリングする場合でもパース済み構造を再利用できるよう、イミュータブルな共有ノードを採用する。
- `render` はストリーミング出力を前提に `TemplateSink` へ逐次チャンクを渡す。`Chunk` は `Str` と `GraphemeSeq` を組み合わせた所有型であり、`Core.Text` が提供する幅計算を利用できる。
- `with_escape_policy` はデフォルトの `EscapePolicy::HtmlStrict`（制御文字・危険タグの除去）を切り替えるユーティリティであり、Context ごとのエスケープ差異を明示的に扱う。

### 8.4 エラーモデル

```reml
pub enum TemplateError =
  | ParseError(Diagnostic)
  | FilterMissing{name: Str, available: List<Str>}
  | CapabilityMissing{id: CapabilityId, filter: Str}
  | RenderPanic{reason: Str, backtrace: Option<TraceId>}
  | SinkFailed{diagnostic: Diagnostic}

fn to_diagnostic(err: TemplateError, span: Option<Span>) -> Diagnostic
fn from_parse_error(parse: ParseError) -> TemplateError
```

- `FilterMissing` は `available` に候補フィルターを提示し、`Core.Diagnostics` と連携して修正候補を表示する。[^template-safe]
- `RenderPanic` はフィルター側で未捕捉例外が発生した場合に利用し、`Core.Diagnostics.Audit` の `record_dsl_failure` が検証ログを生成する。
- `SinkFailed` は IO/ネットワーク書き込みエラーを保持し、`Diagnostic.domain` を `DiagnosticDomain::Template` に設定する。

[^template-perf]: [0-1-project-purpose.md](0-1-project-purpose.md) §1.1 で示す性能基準を満たすため、`TemplateProgram` は解析済み構造を共有し、レンダリング時の再パースを避ける。
[^template-safe]: [0-1-project-purpose.md](0-1-project-purpose.md) §1.2 の安全性方針に基づき、未定義フィルターや権限逸脱を `Result`/`Diagnostic` で検出する。

## 9. 使用例（Lex 連携と Grapheme 操作）

```reml
use Core;
use Core.Text;
use Core.Unicode;
use Core.Parse.Lex;

type Token =
  | Identifier(name: String)
  | Number(value: Str)
  | Emoji(grapheme: Grapheme)
  | DocComment(text: String)

fn tokenize_identifier() -> Parser<Token> =
  Unicode.segment_words(lexeme(spaces0(), identifier_raw()))
    |> Iter.try_fold(String::empty(), |acc, word|
         let normalized = Unicode.prepare_identifier(word)?;
         Ok(acc + normalized)
       )
    |> Result.map(|name| Token::Identifier(name))

fn tokenize_emoji() -> Parser<Token> =
  Unicode.segment_graphemes(string("")) // 実際は絵文字範囲を判定
    |> Iter.find(|g| g.category().is_emoji())
    |> Option.map(|g|
         Token::Emoji(g)
       )
    |> Option.to_result(|| Diagnostic::expected("emoji"))

fn tokenize_doc_comment() -> Parser<Token> =
  comment_block("/**", "*/", nested=false)
    .map(|text|
      text
        |> GraphemeSeq::new
        |> Iter.from
        |> Iter.map(|g| Unicode.width_map(g.as_str(), WidthMode::Narrow).unwrap_or(g.as_str().to_string()))
        |> Iter.collect_vec()
        |> TextBuilder::builder()
        |> TextBuilder::finish()
        |> Token::DocComment
    )
```

- `tokenize_identifier` は `Unicode.prepare_identifier` を通じて NFC 正規化された識別子を生成し、Chapter 2.3 の `lexeme` と組み合わせて空白処理を統一する。
- `tokenize_emoji` は `segment_graphemes` と `category().is_emoji()` を利用し、絵文字トークンを抽出。`Option.to_result` により `Diagnostic` へ変換する点で 4.2 の失敗制御と整合。
- `tokenize_doc_comment` はブロックコメント本文を `GraphemeSeq`→`TextBuilder` で再構築し、幅変換を適用して CLI / LSP 表示に適した文字列へ変換する例。

### 9.1 エンコーディング変換ヘルパ

```reml
// 主要エンコーディング間の変換
fn to_utf16(text: Str) -> Result<Vec<u16>, UnicodeError>        // `effect {mem, unicode}`
fn from_utf16(data: &[u16]) -> Result<String, UnicodeError>     // `effect {mem, unicode}`
fn to_latin1(text: Str) -> Result<Vec<u8>, UnicodeError>        // `effect {mem, unicode}`
fn from_latin1(data: &[u8]) -> String                          // `effect {mem}`

// ベース64 エンコーディング
fn to_base64(data: &[u8]) -> String                            // `effect {mem}`
fn from_base64(text: Str) -> Result<Vec<u8>, DecodeError>       // `effect {mem}`
```

> 関連: [2.3 字句レイヤユーティリティ](2-3-lexer.md), [3.1 Core Prelude & Iteration](3-1-core-prelude-iteration.md), [3.2 Core Collections](3-2-core-collections.md), [3.5 Core IO & Path](3-5-core-io-path.md)
