# 1.4 文字モデル（Text & Unicode Model）— Reml (Readable & Expressive Meta Language) 言語コア仕様

> 目的：**Unicode 前提**で “書きやすく・読みやすく・誤りにくい” 文字列処理を提供し、**パーサーコンビネーター**が現実のテキスト（絵文字・結合記号・多言語）を安全に扱えるようにする。
> 方針：**ランタイム表現は UTF-8**。**文字＝コードポイント（`Char`）**、**表示単位＝拡張書記素クラスタ（`Grapheme`）** を区別。**等価性は NFC 正規化で定義**しつつ、**バイト等価**や**コードポイント等価**も明示 API で提供する。

---

## A. 基本型と用語

* **`String`**：不変 UTF-8 文字列。コピーオンライト（COW）＋参照カウント（RC）。
* **`Str`**：`String` の**借用スライス**（ビュー）。**バイト境界**に加え**コードポイント境界**でのみ生成可能（不正境界はコンパイル／実行時に拒否）。
* **`Bytes`**：任意バイト列（テキスト前提なし）。
* **`Char`**：Unicode **スカラ値**（U+0000..U+D7FF, U+E000..U+10FFFF）。サロゲートは不可。
* **`Grapheme`**：**拡張書記素クラスタ**。1つ以上の `Char` の連なり（例：`🇯🇵` は 2 コードポイント、`é` は `e` + 合成アクセント）。
* **用語**

  * **コードポイント**（cp）：`Char`。
  * **コードユニット**：UTF-8 のバイト。
  * **クラスタ**：`Grapheme`。
  * **正規化**：NFC/NFD/NFKC/NFKD。

---

## B. 等価性・順序・ハッシュ

* **既定の文字列等価（`==`）**：**NFC 正規化**したコードポイント列の**完全一致**。

  * 生成時に **内部表現は NFC に正規化**（速度重視の構成では “lazy normalize + 先頭で印字” も実装可。ただし API 的には NFC を前提に等価/ハッシュを定義）。
  * **ハッシュ**も NFC 基準（同値性と整合）。
* **代替の等価**（明示 API）

  * `eq_codepoint(str1, str2)`：**正規化せず**コードポイント列の一致。
  * `eq_bytes(str1, str2)`：UTF-8 **バイト列**の一致。
* **順序比較**

  * 既定：コードポイント順（NFC 後）。
  * ロケール依存の\*\*照合（collation）\*\*は `Collator(locale).compare(a,b)` を使用（UTS #10 相当の仕様に準拠）。

---

## C. リテラル・エスケープ・ソース表現

* **ソースエンコーディング**：UTF-8。
* **改行**：`LF` / `CRLF` / `CR` を受理、**字句解析時に `LF` に正規化**。
* **文字列リテラル**

  * 通常：`"text \n \t \" \\ "`
  * 生：`r"^\d+$"`（バックスラッシュ非解釈）
  * 複数行：`""" line1\nline2 """`（内部改行保持）
  * Unicode エスケープ：`\u{1F600}`（1〜6桁 hex、スカラ値のみ）
* **文字リテラル**：`'A'`, `'\u{0301}'`（単一 `Char`）。
* **Bidi 制御文字**：**文字列/コメント内のみ許可**。ソースコード中（識別子・キーワード・演算子）での使用は禁止（セキュリティ対策）。

---

## D. インデクシング・スライス・反復

* **文字列の数値添字アクセスは禁止**（UTF-8 の可変長ゆえ O(1) を保証できず誤用の温床）。
* **明示 API での取得**

  * **バイト**：`bytes(str)` → `Bytes`、`byte_len(str)`, `get_byte(i)`。
  * **コードポイント**：`codepoints(str)` → `Iterator<Char>`、`get_char_at(cp_index)`（線形／オプションでインデックスキャッシュ）。
  * **グラフェム**：`graphemes(str)` → `Iterator<Grapheme>`、`get_grapheme_at(g_index)`。
* **スライス**

  * `slice_bytes(range)`：**任意バイト**範囲（UTF-8 破壊の可能性あり → 戻り値は `Bytes`）。
  * `slice_codepoints(range)`：コードポイント境界のみ（`Str` を返す）。
  * `slice_graphemes(range)`：グラフェム境界（`Str` を返す）。
* **表示幅**

  * `display_width(str)`：東アジア幅・結合マーク・ゼロ幅結合子（ZWJ）に基づく**端末表示幅**を返す。
  * パースエラー出力は **`行:列(グラフェム基準)`** と **バイトオフセット**の両方を併記。

---

## E. 正規化と大小変換

* **既定の内部表現**：**NFC**。
* **API**

  * `normalize_nfc/nfd/nfkc/nfkd(str)`
  * `to_lower(str, locale?)`, `to_upper(str, locale?)`, `to_casefold(str)`（完全/部分大小写、トルコ語例外等はロケールで制御）
* **識別子**

  * 定義：UAX #31 の **XID\_Start / XID\_Continue** に準拠（1.1 参照）。
  * **NFC でなければ字句段階で拒否**。
  * **confusable（紛らわしさ）検査**は警告（UAX #39 の制限スクリプト混在ルールをデフォルト有効）。
  * 混在書記方向は制限（RLO/LRO 等の Bidi 制御は識別子に禁止）。

---

## F. セグメンテーション（境界規則）

* **グラフェム境界**：**UAX #29**（Extended Grapheme Cluster）に準拠。`graphemes()` は絵文字シーケンス（ZWJ）や結合記号を 1 つとして扱う。
* **単語・文境界**：`words(str)` / `sentences(str)` を提供（UAX #29）。
* **行分割**：`line_breaks(str, width, locale?)`（UAX #14）。
* パーサー用途として `Lex` に **クラステスト**を提供：

  * `unicode.category("Lu")`, `unicode.script("Han")`, `unicode.property("White_Space")`
  * `Lex.identifier(...)` は UAX #31 に沿ったプロファイル（先頭/続きの条件、`_` 許容等）。

---

## G. エラーレポートと位置情報

* **`Span`**：`{ byte_start, byte_end, line, column }`

  * `column` は **グラフェム単位**。
  * タブ幅は既定 4（設定で変更可）。
* パーサエラーは **抜粋表示**＋**下線**（グラフェム単位で正確に合致）。
* 文字化けや不正 UTF-8 を検出した場合

  * デコード不可：**字句段階エラー**（位置は**直前の有効バイト**から報告）。
  * 不正スカラ値（サロゲート等）：同様にエラー。

### G.1 Core.Parse との連携規約

* `Core.Parse.State.input` が保持する `line`/`column` は **`GraphemeIndex` キャッシュ**と同期させ、`advance_grapheme()` 等の内部ユーティリティで一貫して更新する。手動で `String.grapheme_at` を逐次呼び出して列計算を行う実装は禁止。
* `DiagState`（2.5 節）の `Span` 生成時は、`Input` が保持する `g_index` / `cp_index` を再利用し、**グラフェム境界で切り出した `Str`** を `Core.Text.display_width` に渡して**表示幅**を求める。これにより、合成絵文字や結合文字を含む行でも下線が正しく揃う。
* `Diagnostic.pretty` 系 API は `display_width` で得た幅をもとに **ハイライトの再描画を行う**。幅計算を独自実装で再現しないこと（Unicode 仕様更新時の揺れを避けるため）。
* これらの規約は 0-1 章の「分かりやすいエラーメッセージ」「Unicode 対応の充実」を満たすための必須要件であり、DSL 実装やツール拡張も同じ契約を引き継ぐ。

---

## H. パフォーマンス・実装指針

* **内部表現**：UTF-8／COW／RC。
* **小文字列最適化（SSO）**：短い `String` はインライン 24〜31bytes を推奨。
* **インデックスキャッシュ**

  * 初回の `codepoints()` / `graphemes()` の際に**境界テーブル**を lazily 構築し、**ビュー単位**で共有。
  * **サブストリング**は親のバッファを **参照共有**（コピー不要）。
* **正規化コスト**

  * 生成時 NFC を基本。大量入力のときは **遅延正規化**モードを選べる実装を許容（等価/ハッシュ計算時に normalize）。
  * グラフェム分割は **Unicode 15.1** に準拠した UAX #29 を基準とし、`ugrapheme` または ICU4X `segmenter::GraphemeClusterBreakSegmenter` のアルゴリズムと照合テストを実施する。
* **境界 API の安全性**：`slice_codepoints` / `slice_graphemes` は**常に整合**、`slice_bytes` は `Bytes` 返却で**型で危険を表明**。

---

## I. 標準 API（抜粋シグネチャ）

```reml
type UnicodeApi = {
  // 生成・変換
  string: fn(Bytes, Bool) -> Result<String, Utf8Error>, // validate_utf8 default true
  to_string: fn(Iterator<Char>) -> String,

  // 長さ
  byte_len: fn(Str) -> usize,
  char_len: fn(Str) -> usize,            // コードポイント数（必要時のみ計算）
  grapheme_len: fn(Str) -> usize,

  // 反復
  bytes: fn(Str) -> Iterator<u8>,
  codepoints: fn(Str) -> Iterator<Char>,
  graphemes: fn(Str) -> Iterator<Grapheme>,

  // スライス
  slice_bytes: fn(Str, Range<usize>) -> Bytes,
  slice_codepoints: fn(Str, Range<usize>) -> Str,
  slice_graphemes: fn(Str, Range<usize>) -> Str,

  // 検索
  find: fn(Str, Str) -> Option<usize>,                 // バイトオフセット
  find_grapheme: fn(Str, fn(Grapheme) -> Bool) -> Option<usize>,

  // 正規化・大小
  normalize_nfc: fn(Str) -> String,
  normalize_nfd: fn(Str) -> String,
  normalize_nfkc: fn(Str) -> String,
  normalize_nfkd: fn(Str) -> String,
  to_lower: fn(Str, Option<Locale>) -> String,
  to_upper: fn(Str, Option<Locale>) -> String,
  to_casefold: fn(Str) -> String,

  // 比較
  eq_bytes: fn(Str, Str) -> Bool,
  eq_codepoint: fn(Str, Str) -> Bool,
}

type Collator
impl Collator {
  fn create(locale: Locale) -> Collator = todo
  fn compare(self, a: Str, b: Str) -> Ordering = todo
}
```

---

## J. パーサー向け `Lex` ヘルパ（文字モデル連携）

* `Lex.grapheme()`：`Parser<Grapheme>`
* `Lex.char_where(pred: Char -> Bool)`：`Parser<Char>`
* `Lex.unicode_category(cat: String)` / `script(name: String)` / `property(name: String)`
* `Lex.identifier(profile = DefaultIdProfile)`：UAX #31 プロファイルに従う識別子パーサ
* `Lex.whitespace()`：Unicode White\_Space に従う
* `Lex.linebreak()`：UAX #14 での行分割候補

> これらは **Core.Parse** の“字句工具”に属するが、**文字モデル**が提供する正確な分類に依存する。

---

## K. セキュリティ・混乱回避

* **Bidi 制御**：識別子・キーワード・演算子・数値リテラルに出現したら**エラー**。文字列/コメントでは**許可＋警告**（設定可）。
* **confusable 検査**：異スクリプト混在や見た目が酷似する文字（例: ラテン `a` とキリル `а`）を**警告**。CI で**禁止リスト**運用も可能。
* **正規化境界**：`String` の生成時 NFC を強制することで**等価性の揺れ**を解消。

---

## L. エラーメッセージの原則（文字周り）

* **位置はグラフェム列で**、括弧内に**バイトオフセット**を併記：`line 12, col 7 (byte 134)`。
* 強調は**グラフェム粒度**で下線：複合絵文字も 1 つとして示す。
* 不正 UTF-8 は**直後のバイトオフセット**を指す診断：「無効な UTF-8 バイト 0x..」。

---

## M. 実装チェックリスト

* [ ] `String` は UTF-8 / RC / COW、SSO を持つ。
* [ ] `Str` はコードポイント境界保証。`slice_*` の整合を型で担保。
* [ ] 生成時 NFC（または遅延正規化＋等価/ハッシュ時に NFC）。
* [ ] 既定の `==`/`hash` は NFC 基準で定義。
* [ ] `graphemes()` は Unicode 15.1 の UAX #29（拡張 GCB）。ZWJ/emoji シーケンス対応。
* [ ] 位置情報はグラフェム列＋バイトオフセットを保持。
* [ ] UAX #31 識別子、Bidi/Confusable のガードを実装。
* [ ] `Lex` の Unicode 分類 API はテーブル生成（ビルド時）の静的データで高速化。
* [ ] 正規化・大小変換は Unicode データファイル（基準: Unicode 15.1 / ICU 74）に基づく（バージョン固定、将来更新可能）。

---

## N. バージョン管理と更新手順

1. **基準バージョン**：Unicode 15.1 と ICU 74 を既定とし、更新時は Unicode コンソーシアムの公表資料と ICU リリースノートをレビュー対象に追加する。
2. **実装評価**：`grapheme_len`・`display_width`・confusable 検査について、`ugrapheme` と ICU4X `segmenter` を比較し CPU/メモリ/バイナリサイズを測定。結果は `../notes/unicode-refresh-<version>.md` に記録する。
3. **テストデータ**：UAX #29/31/39 の公式 conformance データに加え、社内の confusable・表示幅ベンチを `../guides/tests/unicode/` に保守し、誤検知率 < 2% を CI で確認する。
4. **仕様更新**：新バージョンを採用した場合、本節と `2-3-lexer.md` にバージョン番号・導入日・依存ライブラリを追記し、互換性注意点をリリースノートへ掲載する。

---

### まとめ

Reml の文字モデルは **"文字（Char）/表示単位（Grapheme）/バイト" を分離**し、

* **NFC 基準の等価性**でバグと衝突を減らし、
* **安全なスライスと豊富な分割 API**で実運用テキストを扱いやすくし、
* **正確な位置情報と Unicode クラス**により **Core.Parse** の字句・構文設計を強力に支える。

この上で、**パーサーコンビネーター**は "Unicode を正しく意識した" トークナイザを小さな合成で記述できる。

---

## 関連仕様

* [1.1 構文](1-1-syntax.md) - 文字・文字列リテラルと識別子の構文
* [1.3 効果と安全性](1-3-effects-safety.md) - 文字列の安全性と不変性
* [2.1 パーサ型](2-1-parser-type.md) - Input型とSpanの位置情報モデル
* [2.3 字句レイヤ](2-3-lexer.md) - Unicode対応字句解析の完全実装
* [2.5 エラー設計](2-5-error.md) - Unicode位置情報を用いたエラー報告
