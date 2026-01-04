# 2.3 字句レイヤ（Core.Parse.Lex）

> 目的：**Unicode 前提**で安全・高性能・書きやすい“字句レイヤ”を、**ごく少数の基礎プリミティブ**と**実用的ユーティリティ**で提供する。
> 方針：
>
> * 既定は **UTF-8 / NFC 等価**（1.4）に整合。
> * **ゼロコピー**で `Str`/`String` を返す。
> * **小さな核 + 合成**で、必要十分な API 面に留める。
> * エラーは **期待集合 + 最遠位置 + ラベル**で説明的（2.5 と整合）。

---

## A. 設計の核（プリミティブ 6）

> この 6 つに、2.2 のコア・コンビネータを合わせれば大半の字句処理が書ける。

```reml
// 1) 一文字（コードポイント）判定
fn satisfy(pred: Char -> Bool) -> Parser<Char> = todo

// 2) 固定文字列（最長一致、NFC 前提）
fn string(s: Str) -> Parser<Str> = todo

// 3) 1 文字でも良い “集合”
fn oneOf(chars: Str) -> Parser<Char> = todo      // 任意の 1 文字が含まれていれば
fn noneOf(chars: Str) -> Parser<Char> = todo     // いずれでもない

// 4) 走査（※空成功を返さない版も提供）
fn takeWhile(pred: Char -> Bool)  -> Parser<Str> = todo     // 0 文字以上
fn takeWhile1(pred: Char -> Bool) -> Parser<Str> = todo     // 1 文字以上（空なら失敗）

// 5) 直後を覗くだけ（非消費）
fn peek() -> Parser<Option<Char>> = todo

// 6) 行区切り（CR/LF/CRLF を LF へ正規化）
fn lineEnding() -> Parser<()> = todo    // 行末を 1 回読む（非返却）
```

**注意**

* `string` は **バイト比較**の高速経路を持つ（ASCII/短文字列最適化）。
* `takeWhile` は **空成功**を返すため、`many` と組み合わせるときは **`takeWhile1` を推奨**。

---

## B. 空白・改行・コメント（スキップ系）

```reml
// Unicode White_Space（UAX #44）、タブ/全角空白を含む
fn whitespace() -> Parser<()> = todo                   // 1 つ以上
fn spaces0() -> Parser<()> = todo                      // 0 以上
fn hspace0() -> Parser<()> = todo                      // 水平のみ
fn vspace0() -> Parser<()> = todo                      // 垂直のみ（改行系）

// コメント
fn commentLine(prefix: Str) -> Parser<()> = todo       // 例: "//"
fn commentBlock(start: Str, end: Str, nested: Bool = true) -> Parser<()> = todo

// スキップ合成：空白・コメントを任意数
fn skipMany(p: Parser<()>) -> Parser<()> = todo
```

**推奨定義（例）**

```reml
let sc =
  (whitespace()
   .or(commentLine("//"))
   .or(commentBlock("/*","*/", nested=true)))
  |> skipMany
```

---

## C. トークン化の基本ユーティリティ

```reml
// 後ろの空白・コメントを食う “lexeme”
fn lexeme<A>(space: Parser<()>, p: Parser<A>) -> Parser<A> = todo

// 固定記号（; , ( ) など）。成功時は () を返す
fn symbol(space: Parser<()>, s: Str) -> Parser<()> = todo   // = lexeme(space, string(s)).skipR(space)

// 先頭側だけをスキップする糖衣
fn leading<A>(space: Parser<()>, p: Parser<A>) -> Parser<A> = todo

// 前後をまとめて処理する（`leading` + `lexeme`）
fn trim<A>(space: Parser<()>, p: Parser<A>) -> Parser<A> = todo

// 前後を食う
fn padded<A>(p: Parser<A>, space: Parser<()>) -> Parser<A> = todo

// 位置付きトークン（値 + Span）
fn token<A>(p: Parser<A>, space: Parser<()>) -> Parser<(A, Span)> = todo
```

* `symbol(sc, "(")` は `"("` を読んで**後続の `sc` を必ず消費**。
* `leading(sc, expr)` は `skipL(sc, expr)` と等価で、構文側の空白処理を 1 行にまとめる。
* `trim(sc, expr)` は `leading` と `lexeme` を組み合わせた糖衣。JSON や PL/0 サンプルのように両端の空白を許容する際に有効。【F:../examples/language-impl-samples/reml/pl0_combinator.reml†L95-L107】
* `token` は AST へ**位置**を付与したいときの定番（`spanned` の字句版）。
* `..=` は **`..` と `=` に分割して**トークン化する（`..=` を単一トークンとしては扱わない）。


---

## D. 識別子・キーワード（UAX #31 準拠）

### D-1. プロファイル

```reml
type IdentifierProfile = {
  allow_underscore: Bool = true,
  start_pred: Char -> Bool,   // 既定: XID_Start or '_'
  cont_pred:  Char -> Bool,   // 既定: XID_Continue or '_'
  nfc_required: Bool = true,  // NFC でない識別子は拒否
  forbid_bidi_ctrl: Bool = true,   // Bidi 制御文字禁止
  confusable_warn: Bool = true     // 見かけ紛らわし警告
}

fn identifier(profile: IdentifierProfile = DefaultId) -> Parser<Str> = todo
```

* 既定プロファイル `DefaultId` は **XID\_Start/XID\_Continue + '\_'**。`RunConfig.extensions["lex"].identifier_profile` で `ascii-compat` を指定すると Phase 1 互換の ASCII 限定挙動に切り替えられる。
* 文字モデル（1.4）に従い、**NFC でない**／**Bidi 制御含む**識別子は**エラー**。
* \*\*紛らわし（UAX #39）\*\*は **警告**として `ParseError.notes` に蓄積。

### D-2. キーワードと境界

```reml
// ident と同一文字列ならキーワードとして成功し、直後が ident-continue なら失敗
fn keyword(space: Parser<()>, kw: Str) -> Parser<()> = todo     // lexeme + 境界確認

// 予約語集合をまとめて拒否（identifier と組み合わせ）
fn reserved(profile: IdentifierProfile, set: Set<Str>) -> Parser<Never> = todo
```

* `keyword(sc, "if")` は `ifx` を**誤認しない**（`notFollowedBy(ident-continue)` を内部使用）。

---

## E. 数値リテラル（区切り `_` / 基数 / 範囲チェック）

> 単項マイナスは\*\*構文側（単項演算子）\*\*で扱う。ここでは **符号なし**を原則。

```reml
// 10 進（"1_234" 許容）
fn int10() -> Parser<Str> = todo                     // “文字列” として取得（桁を維持）
fn int(radix: 2|8|10|16, allow_prefix: Bool = false) -> Parser<Str> = todo
// 0b / 0o / 0x プレフィックス対応
fn intAuto() -> Parser<(radix: u8, digits: Str)> = todo // 0x..., 0o..., 0b..., それ以外は 10

// 浮動小数（10 進、"1.23", ".5", "1e-9", "1_000.0" など）
fn float() -> Parser<Str> = todo

// 文字列から数値へ（範囲チェック付き）
fn parseI64(digits: Str, radix: u8 = 10)  -> Result<i64, Overflow> = todo
fn parseU64(digits: Str, radix: u8 = 10)  -> Result<u64, Overflow> = todo
fn parseF64(repr: Str)                    -> Result<f64, ParseFloatError> = todo
```

**指針**

* 字句段階は **文字列で保持**（桁情報・原文再現）。
* 値化は構文側で `map` して `parseI64/parseF64`。**オーバーフローは説明的エラー**に。

### E-1. 数値エラーの診断変換 {#numeric-diagnostic}

> 0-1 §1.1 の性能基準と 0-1 §1.2 の安全原則を守りつつ、数値リテラルの失敗を一貫した `Diagnostic` に落とし込むための共通ヘルパ。

```reml
type NumericOverflow = {
  min: Str,
  max: Str,
  target: Str,
}

fn numeric_overflow_error(span: Span, digits: Str, info: NumericOverflow, radix: u8) -> ParseError = todo
fn numeric_parse_error(span: Span, repr: Str, cause: ParseFloatError) -> ParseError = todo
```

* `numeric_overflow_error` は `parseI64`/`parseU64` が返した `Overflow` 情報から `NumericOverflow` を構築し、`DiagnosticDomain::Parser`・`message_key = "parser.number.overflow"`・`expected = {Range(info.target)}` を設定する。`notes` には `min`/`max` を記録し、`extensions["numeric"].radix = radix` を付与することで IDE が基数を明示できる。
* `numeric_parse_error` は `parseF64`（および今後追加される浮動小数系）で発生した `ParseFloatError` を `message_key = "parser.number.invalid"` に変換し、`cause` を `extensions["numeric"].cause` として保持する。指数部や桁区切り `_` の不正もここで扱う。
* いずれのヘルパも `Parse.fail(numeric_overflow_error(...))` のように使用し、2.5 §B-11 のフローで `Diagnostic` へ変換される。`RunConfig.locale` は `PrettyOptions` に伝播し、0-1 §2.2 が求める多言語化を満たす。
* CLI/IDE は `Diagnostic.code = Some("E7101")`（整数オーバーフロー）または `Some("E7102")`（浮動小数の不正値）を割り当てる運用を推奨し、3.6 §2.3 のカタログ登録でメッセージテンプレートを共有する。

---

## F. 文字列・文字リテラル（エスケープ/生/複数行）

```reml
// 通常文字列（C 風エスケープ、\u{...} は Unicode スカラ値）
fn stringLit() -> Parser<String> = todo

// 原文を保持したい場合（アンエスケープしない）
fn stringRaw() -> Parser<Str> = todo                // r" ... "
fn stringRawHash(level: usize) -> Parser<Str> = todo// r#" ... "#, r##" ... "## など

// 複数行（トリプルクォート）。インデント除去オプション
fn stringMultiline(dedent: Bool = true) -> Parser<String> = todo

// 1 文字リテラル（Unicode スカラ値 1 個）
fn charLit() -> Parser<Char> = todo
```

**エスケープ仕様（抜粋）**

* `\n \r \t \\ \" \'`、`\/`（JSON 互換）、`\u{1F600}`（1〜6桁 hex、スカラ値必須）
* 不正なエスケープ／サロゲート：**位置付きエラー**。
* `stringMultiline(dedent=true)` は **最小共通インデント**を除去（ドキュメント文字列向け）。

---

## G. 汎用“取り込み”ユーティリティ

```reml
fn till<A>(end: Parser<A>) -> Parser<Str> = todo            // end が来るまで（非貪欲）
fn take(n_bytes: usize) -> Parser<Bytes> = todo             // バイト数で取得（テキスト前提なし）
fn takeCodepoints(n: usize) -> Parser<Str> = todo           // コードポイント数で取得
fn grapheme() -> Parser<Grapheme> = todo                    // 拡張書記素 1 つ
```

* `till` は **ゼロ幅 end** に注意（実装で guard）。
* `take` は **Bytes** を返し、**UTF-8 破壊の可能性を型で表す**（1.4 の方針に従う）。

### G-1. 設定ファイル互換プロファイル

> 目的：JSON/TOML などの設定 DSL が要求する「コメント許可」「トレーリングカンマ許容」等の互換性要件を、再利用可能なプロファイルと診断一貫性のあるユーティリティに集約する。0-1 §1.1 の性能と 0-1 §1.2 の安全性を守りつつ、開発現場で一般的な拡張仕様へ素早く適応できるようにする。

```reml
type ConfigTriviaProfile = {
  line: List<Str> = ["//"],
  block: List<CommentPair> = [CommentPair("/*", "*/", nested=false)],
  shebang: Bool = false,
  hash_inline: Bool = false,
  doc_comment: Option<Str> = None,
}

type CommentPair = {
  start: Str,
  end: Str,
  nested: Bool = true,
}

const ConfigTriviaProfile::strict_json: ConfigTriviaProfile = todo
const ConfigTriviaProfile::json_relaxed: ConfigTriviaProfile = todo
const ConfigTriviaProfile::toml_relaxed: ConfigTriviaProfile = todo

fn config_trivia(profile: ConfigTriviaProfile) -> Parser<()> = todo
fn config_lexeme<A>(profile: ConfigTriviaProfile, p: Parser<A>) -> Parser<A> = todo
fn config_symbol(profile: ConfigTriviaProfile, s: Str) -> Parser<()> = todo
```

* `ConfigTriviaProfile::strict_json` はコメント禁止・`shebang=false`・`hash_inline=false`。JSON 仕様準拠の入力に使用する。
* `ConfigTriviaProfile::json_relaxed` は `line=["//"]`・`block=[CommentPair("/*","*/", nested=false)]`・`shebang=true` を既定とし、`../examples/language-impl-samples/reml/json_extended.reml` が手書きしていた設定を置き換えられる。
* `ConfigTriviaProfile::toml_relaxed` は `line=["#","//"]`・`block=[]`・`hash_inline=true` を既定とし、`Cargo.toml` 互換のコメント挙動を提供する。

**診断と RunConfig 連携**

* `config_trivia` は `whitespace()`/`commentLine()`/`commentBlock()` を内部で合成し、`shebang` が有効な場合は入力先頭に限り `#!` を読み飛ばす。複数回呼び出しても二行目以降の `#!` はコメント扱いしない。
* 失敗時には `ParseError.label = "lex.config.trivia"` を付与し、3-6 §2.2 の `from_parse_error` で `Diagnostic.code = "config.trivia.invalid"` が生成される。
* `doc_comment=Some(prefix)` の場合、そのコメントを `Diagnostic.notes` に `comment.doc` ラベルで追記し、LSP/CLI が設定項目の説明を提示できるようにする。
* `RunConfig.extensions["config"].trivia` に `ConfigTriviaProfile` を格納すると、`config_trivia`/`config_lexeme`/`config_symbol` が同じプロファイルを共有し、CLI・LSP・テストが互換モードを自動で揃えられる。未指定時は `ConfigTriviaProfile::strict_json`。
* トレーリングカンマや未定義フィールドなど、構文側で処理すべき互換機能は 3-7 §1.5 の `ConfigCompatibility` と連携する。字句段階はカンマの存在を正確に報告し、許可されていない場合は構文が `Diagnostic.severity = Error` として拒否する。

> **実装メモ**: `ConfigTriviaProfile` は `RunConfig.extensions["config"].features` による feature guard と併用し、互換機能を本番環境で段階的に有効化できるようにする（3-10 §2）。

---

## H. 行頭・行末・インデント（任意）

Reml 本体はオフサイド規則を採用しないが、DSL 用に提供。

```reml
fn bol() -> Parser<()> = todo                     // 行頭（BOL）
fn eol() -> Parser<()> = todo                     // 行末（EOL）
fn indentEq(n: usize) -> Parser<()> = todo        // その行の列 == n
fn indentGt(n: usize) -> Parser<()> = todo        // 列 > n
fn column() -> Parser<usize> = todo               // 現在列（グラフェム数）
```

### H-2. LayoutProfile（Phase 9 ドラフト）

```reml
type LayoutProfile = {
  indent_token: Str = "<indent>",
  dedent_token: Str = "<dedent>",
  newline_token: Str = "<newline>",
  offside: Bool = false,          // true のときオフサイド規則を有効化
  allow_mixed_tabs: Bool = false, // タブとスペース混在を許容するか
}

fn layout(profile: LayoutProfile) -> Parser<()> = todo           // レイアウトトークンを生成
fn layout_token(profile: LayoutProfile, s: Str) -> Parser<()> = todo // indent/dedent/newline 判定
```

* `layout` は Lex 側でインデント幅を追跡し、オフサイド規則に従って仮想トークン（`indent_token`/`dedent_token`/`newline_token`）を発行する。`Core.Parse` からは通常の `symbol`/`keyword` と同様に扱え、`autoWhitespace`（2-2 §B-2）経由で空白スキップと併用できる。
* `allow_mixed_tabs=false` が既定で、タブ・スペース混在行は `lex.layout.mixed_indent` 診断として報告する。`offside=false` の場合は仮想トークンを発行せず、単なる改行スキップとして振る舞う。
* `RunConfig.extensions["lex"].layout_profile` に `LayoutProfile` を格納すると `autoWhitespace` が検出して Parser 側へ伝搬する。未指定時は Layout を無効化し、2-2 §B-2 のフォールバック空白を用いる。
* 期待集合生成では `indent_token` 等を `expected` に含め、エラー箇所の列情報は `column()` が返すグラフェム数に基づいて計算する（0-1 §3.1 の Unicode 要件と整合）。

---

## I. セキュリティ・正規化（安全モード）

```reml
// 入力ストリームから Bidi 制御（U+2066…U+2069, RLO/LRO/RLE/LRE/PDF など）を拒否/警告
fn forbidBidiControls() -> Parser<()> = todo      // 文字列/コメント以外の出現でエラー

// NFC でない連なりを検出（識別子などで使用）
fn requireNfc(s: Str) -> Result<Str, NfcError> = todo

// 見かけ紛らわし（UAX #39）を検出して notes に追加
fn warnConfusable(s: Str) -> () = todo
```

---

## J. エラー品質のための流儀

* **`label` と `expect`（= label+cut）**

  ```reml
  let sym(s) = expect("symbol '" + s + "'", symbol(sc, s))
  let kw(s)  = expect("keyword " + s, keyword(sc, s))
  ```
* **“最長一致”の曖昧は `lookahead`/`notFollowedBy`** で解消
  例：`ident` と `keyword("if")` の競合。
* **繰返し本体の空成功を禁止**（ライブラリ側で検知し説明）。
* **数値のオーバーフロー**は **字句 → 値化**の境界で検出し、**桁列**を含む診断に。

---

## K. 性能規約（実装者向け）

* **ASCII 高速経路**：`string`, `oneOf/noneOf`, `takeWhile` に ASCII 専用分岐。
* **テーブル駆動**：Unicode カテゴリ/プロパティは **生成済みテーブル**（ビルド時）を使用。
* **NFA/Goto 最適化**：`identifier` などホットパスは **手書き状態機械**にコンパイル。
* **ゼロコピー**：`Str` は親 `String` を参照、SSO と RC/COW による共有。
* **Packrat**：字句パーサは一般に **左再帰なし**。`rule()` による ParserId 固定は忘れない。
* **境界キャッシュ**：コードポイント/グラフェム境界は **lazy 構築**しビュー間で共有（1.4）。

---

## L. 代表的なレシピ

### L-1. Reml 識別子/キーワード

```reml
let sc = (whitespace().or(commentLine("//")).or(commentBlock("/*","*/", true))) |> skipMany
let sym(s) = symbol(sc, s)
let kw(s)  = expect("keyword " + s, keyword(sc, s))

let ident = lexeme(sc, identifier(DefaultId))
  |> label("identifier")

let reservedSet = {"fn","let","var","type","match","with","if","then","else","use","pub","return","true","false"}
let nonReservedIdent =
  ident.andThen(|name| if reservedSet.contains(name) then fail("reserved") else ok(name))
```

> **NOTE**: `Set<T>` の実行時表現は Runtime の `reml_set_*` ABI に委譲され、Backend は不透明ポインタとして扱う。詳細は [3.2 Core Collections §2.2.1](3-2-core-collections.md#set-runtime-abi) を参照。

### L-2. 数値（整数 or 浮動小数）

```reml
let number: Parser<Either<i64,f64>> =
  lexeme(sc,
    float().map(|s| Right(parseF64(s)?))
    .or(intAuto().andThen(|(r,d)| ok(Left(parseI64(d, r)?))))
  ).label("number")
```

### L-3. 文字列（通常/生/複数行）

```reml
let strLit: Parser<String> =
  lexeme(sc,
    stringLit()
    .or(stringRaw())
    .or(stringMultiline(dedent=true))
  ).label("string")
```

### L-4. 既定ランナー統合

```reml
use Core.Parse
use Core.Parse.Lex

struct LexPack {
  space: Any,
  symbol: Any,
  ident: Any,
}

fn lex_pack(profile: ConfigTriviaProfile = ConfigTriviaProfile::strict_json) -> LexPack =
  LexPack { space: todo, symbol: todo, ident: todo }
fn parse_entry<T>(p: Parser<T>, src: String, cfg: RunConfig = {}) -> ParseResult<T> = todo
```

* `lex_pack` はコードベースで 1 箇所だけ定義し、空白・コメント・識別子スキーマを共有する。`ConfigTriviaProfile` を受け取ることで JSON/TOML 対応などの互換モードを即座に選べる。
* `parse_entry` のようなエントリポイントで `LexPack` を初期化し、`RunConfig.extensions["lex"].space` に格納する。これにより CLI/LSP が字句スキップ設定を把握でき、`config_trivia` の変更が検査ポリシーや監査ログと同期する。【参照: 3-7-core-config-data.md §1.5】
* `cfg.with_extension` は `RunConfig` に対するイミュータブル更新ヘルパで、`Map<Str, Any>` を受け取り新しい `RunConfig` を返す。これにより 0-1 §1.1 が求める共有メモリ戦略を崩さずに設定を差し替えられる。
* `with_space` は構文パーサ内で `lexeme` と同一の空白処理を共有するヘルパ（`Parser<T>` にメソッド追加）。`space_id()` は字句スキップパーサの安定 ID (`ParserId`) を返し、ストリーミング実行やパーサ差分検証で空白設定が一致しているかチェックする。
* この構成により、0-1 §1.1 が求める「線形時間・ゼロコピー」を保ったまま、LSP や CLI で `RunConfig` から既定 lex 設定を再構成できる。`cfg.extensions` を通じて IDE が同じプロフィールを再利用すると、字句/構文の診断が 0-1 §2.2 の一貫性要件を満たす。

---

## M. API 一覧（サマリ）

**プリミティブ**：`satisfy` / `string` / `oneOf` / `noneOf` / `takeWhile` / `takeWhile1` / `peek` / `lineEnding`
**空白・コメント**：`whitespace` / `spaces0` / `hspace0` / `vspace0` / `commentLine` / `commentBlock` / `skipMany`
**トークン**：`lexeme` / `symbol` / `padded` / `token`
**識別子**：`identifier(profile)` / `keyword(space, kw)` / `reserved`
**数値**：`int10` / `int(radix, allow_prefix)` / `intAuto` / `float` / `parseI64` / `parseU64` / `parseF64`
**文字列**：`stringLit` / `stringRaw` / `stringRawHash` / `stringMultiline` / `charLit`
**走査**：`till` / `take(n_bytes)` / `takeCodepoints(n)` / `grapheme`
**行頭・列**：`bol` / `eol` / `indentEq` / `indentGt` / `column`
**安全**：`forbidBidiControls` / `requireNfc` / `warnConfusable`

---

## N. チェックリスト

* [ ] 6 プリミティブを核に、**字句ユースケースの 95%** を網羅。
* [ ] **UAX #31/29/14** に整合（識別子・グラフェム・行分割）。
* [ ] **lexeme/symbol** が **エラー品質**（期待名/コミット）と噛み合う。
* [ ] 数値/文字列は **原文保持 → 値化**の二段階、**範囲/不正**の診断が明快。
* [ ] **Bidi/NFC/Confusable** の安全策を標準で同梱。
* [ ] **ASCII 高速経路 + テーブル駆動 + ゼロコピー**で実用性能。

---

## O. 文字モデル統合（1.4 連携）

文字モデル（[1.4 文字モデル](1-4-test-unicode-model.md)）で定義された三層構造（Byte/Char/Grapheme）を活用するパーサヘルパ群：

```reml
// 文字レベルでの解析
fn grapheme() -> Parser<Grapheme> = todo                    // 拡張書記素クラスタ単位
fn char_where(pred: Char -> Bool) -> Parser<Char> = todo    // Unicode スカラ値での条件解析

// Unicode プロパティベース解析
fn unicode_category(cat: String) -> Parser<Char> = todo     // "Lu", "Nd" など
fn unicode_script(name: String) -> Parser<Char> = todo      // "Han", "Hiragana" など
fn unicode_property(name: String) -> Parser<Char> = todo    // "White_Space" など

// UAX #31 識別子解析（安全性統合）
fn identifier(profile: IdentifierProfile) -> Parser<Str> = todo  // NFC・Bidi・Confusable検査含む
```

**文字モデル統合のポイント：**

* **等価性判定**：[1.4](1-4-test-unicode-model.md) で定義されたNFC正規化ベースの等価性を使用
* **位置情報**：グラフェム単位での列位置とバイトオフセットを併用
* **セキュリティ**：Bidi制御文字の検出とConfusable文字の警告機能
* **パフォーマンス**：境界キャッシュとテーブル駆動による高速化

これらのAPIにより、**Unicode前提**での安全で高性能な字句解析が実現されます。

---

### まとめ

Core.Parse.Lex は **最小の核**（6 プリミティブ）に、

* Unicode 正しい **空白/コメント/識別子/数値/文字列**の**実務ユーティリティ**を重ね、
* `lexeme`/`symbol`/`keyword` と **`cut/label` の流儀**で、
  **書きやすさ・読みやすさ・エラー品質**を最大化する。
