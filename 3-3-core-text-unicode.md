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

## 8. 使用例（Lex 連携と Grapheme 操作）

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

### 8.1 エンコーディング変換ヘルパ

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
