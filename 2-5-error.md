# 2.5 エラー設計（Nest.Parse.Err）

> 目的：**説明的で短く、修正指向**の診断を、**最遠位置＋期待集合＋文脈**で一貫して出す。
> 前提：2.1 の `Reply{consumed, committed}`・`Span`、2.2 の `cut/label/attempt/recover`、2.4 の `precedence(operand, …)` と整合。

---

## A. 型（データモデル）

```kestrel
type Severity = Error | Warning | Note

type Expectation =
  | Token(Str)          // 具体トークン（")", "if", "+", …）
  | Keyword(Str)        // 識別子と衝突しない予約語
  | Rule(Str)           // "expression" など人間語ラベル
  | Eof                 // 入力終端
  | Not(Str)            // "直後に英数字が続かないこと" 等の否定
  | Class(Str)          // 文字クラス／種別（"digit", "identifier" など）
  | Custom(Str)         // 任意メッセージ（ライブラリ拡張用）

type FixIt =            // IDE 用 “その場で直せる” 提案
  | Insert{ at: Span, text: Str }
  | Replace{ at: Span, text: Str }
  | Delete{ at: Span }

type Diagnostic = {
  severity: Severity,
  code: Option<Str>,        // "E0001" など（安定ID）
  message: Str,             // 1 行要約
  at: Span,                 // 主位置（1.4: 列=グラフェム）
  notes: List<(Span, Str)>, // 追加メモ（複数可）
  fixits: List<FixIt>
}

type ParseError = {
  at: Span,                        // 失敗の最狭位置（最遠エラーの位置）
  expected: Set<Expectation>,      // 期待集合（重複・包含を縮約）
  context: List<Str>,              // 直近の label / rule 名（外側→内側）
  committed: Bool,                 // cut 後の失敗なら true
  far_consumed: Bool,              // ここまでに一度でも消費したか
  hints: List<Str>,                // "カッコを閉じ忘れ？" 等の簡易ヒント
  secondaries: List<Diagnostic>    // 付随診断（lex/overflow 等）
}
```

* **`ParseError` は集約用の“素の事実”**、**`Diagnostic` は表示用**（`Err.pretty` が `ParseError` から `Diagnostic` を起こす）。
* `Expectation` は**種類別**に持ち、message 生成時に**まとまりで整形**（例：「期待：`)`・`number`・識別子のいずれか」）。

---

## B. 生成と合成（アルゴリズム）

### B-1. 単一パーサの失敗を作る

```kestrel
fn Err.expected(at: Span, xs: Set<Expectation>) -> ParseError
fn Err.custom(at: Span, msg: Str) -> ParseError
```

### B-2. 位置の順序（farthest-first）

1. **より遠い `at`**（`byte_end` が大きい）を採用。
2. 同位置なら：

   * `committed=true` を優先（バックトラック不能な失敗）。
   * それでも同列なら `expected` を **和集合（縮約付き）**。

### B-3. `or` における合成

* 左 `p` が `Err(consumed=true ∨ committed=true)` → **右を試さない**。
* 左が **空失敗** → 右を試す。
* 最終的に**どちらかの最遠**を返す（B-2）。

### B-4. `then / andThen` の合成

* 前段 `p` が成功 → 後段の失敗に **`context` を加える**（`rule/label` 名）。
* 失敗位置が同じなら **後段の `expected` を優先**（「この場で何が要るか」を示す）。

### B-5. `cut` の効果

* `cut(p)` 以降の失敗は **`committed=true`**。`or` は**分岐しない**。
* `expected` は **その地点で“再初期化”**（曖昧な上位の期待は引きずらない）。

### B-6. 期待集合の縮約

* `Token("<=")` と `Token("<")` が同レベルで並ぶ場合は**最長一致規則**を尊重（2.4 起因の内部処理）。
* `Rule("expression")` があり、`Token(")")` 等の**具体トークン**があれば、**具体を優先表示**（抽象は補助に落とす）。
* 多数 (>8) のときは **カテゴリ分け＋上位 N 件**を表示し、残りは「…他 X 件」。

---

## C. 表示（pretty）と多言語

```kestrel
fn Err.pretty(src: Str, e: ParseError, opts: PrettyOptions) -> String

type PrettyOptions = {
  max_expected: usize = 6,           // 一覧上限
  context_depth: usize = 3,          // 文脈表示の深さ
  show_bytes: Bool = true,           // (byte 134) などを併記
  snippet_lines: usize = 2,          // 前後の抜粋行数
  color: Bool = true,                // 終端色付け
  locale: Locale = "ja"              // メッセージ言語
}
```

* **スニペット**：1.4 の **グラフェム列**で正確に下線。
* **主語**：「expected …, found ‘…’」形式だが、ロケールにより語順差し替え。
* **`context`**：「while parsing *expression* → *term* → *factor*」のように**内側 3 段**まで表示。
* **FixIt** は `^` 行に \*\*「ここに ‘)’ を挿入」\*\*のように注記。

**例（括弧閉じ忘れ）**

```
error[E1001]: expected ')' to close '('
  --> main.ks:4:12 (byte 37)
   4 | let x = (1 + 2 * (3 + 4
     |            -----       ^ insert ')'
     |            opened here

help: you may be missing a closing parenthesis
note: while parsing expression → term → factor
```

---

## D. 代表エラーの専用処理（品質を上げる“定形”）

1. **括弧ペアの未完**

* `between(open, p, close)` と 2.4 の二項/前置演算子は、**消費した瞬間に cut**。
* 右が無ければ `FixIt::Insert(")")` など**具体修復**を提示。
* `notes` に「ここで開きました」を**矢印付き**で併記。

2. **非結合演算子の連鎖**（`a < b < c`）

* 2.4 で**専用コード**：`E2001`。
* **提案**：「`(a < b) && (b < c)`」など **置換案**を `Replace` で提示。

3. **キーワード vs 識別子の衝突**

* `keyword()` は**直後が識別子継続ならエラー**（2.3 D）。
* メッセージ：「`ifx` は識別子です。キーワード `if` の後に空白が必要ですか？」。

4. **数値のオーバーフロー**

* 2.3 E の `parseI64/parseF64` で **二次診断**（`secondaries`）を生成。
* 主エラーに「桁列」「最大/最小値」を併記。

5. **空成功の繰返し**

* `many` 系で**検出**し、「この繰返しの本体は空成功します」の専用エラー（`E3001`）。

6. **左再帰サポート無効時の自己呼出**

* `RunConfig.left_recursion=false` かつ検出時に `E4001`。
* 提案：「`precedence` を使う」か「`left_recursion=true` を有効化」。

7. **EOF 必須**

* `run(..., require_eof=true)` で余剰入力があれば：

  * 主エラー：`expected EOF`
  * `notes` に**余剰先頭 32 文字**を抜粋。

---

## E. `recover`（回復）の仕様

```kestrel
fn recover<T>(p: Parser<T>, until: Parser<()>, with: T) -> Parser<T>
```

* `p` が失敗したら、**診断を残しつつ** `until` の位置（例：`";"` または行末）まで**読み捨て**、`with` を返す。
* 返す `with` は AST に **`ErrorNode{span, expected}`** として挿入可能（IDE で赤波線）。
* `RunConfig.merge_warnings` が true の場合、連続する回復を**1 つに集約**（ノイズ低減）。

---

## F. API（作る・混ぜる・見せる）

```kestrel
// 作る
fn expectedToken(at: Span, s: Str) -> ParseError =
  Err.expected(at, {Token(s)})

fn expectedRule(at: Span, name: Str) -> ParseError =
  Err.expected(at, {Rule(name)})

// 混ぜる（farthest-first）
fn merge(a: ParseError, b: ParseError) -> ParseError

// 文脈を積む
fn withContext(e: ParseError, label: Str) -> ParseError

// 表示
fn pretty(src: Str, e: ParseError, o: PrettyOptions = {}) -> String

// IDE 連携
fn toDiagnostics(src: Str, e: ParseError, o: PrettyOptions = {}) -> List<Diagnostic>
```

---

## G. 2.1/2.2/2.4 との“かみ合わせ”規約

* **`label("…", p)`**：`p` の失敗時、`Expectation.Rule("…")` を優先登録。
* **`cut`/`cut_here`**：以降の失敗は `committed=true`（`or` は分岐不可）。
* **`lexeme/symbol/keyword`**：トリビア（空白・コメント）消費後の**実トークン位置**を `Span` にする。
* **`precedence`**：`config.operand_label` があれば、**欠落オペランドの期待をそれに固定**（「`+` の後に *expression* が必要」）。
* **`attempt`**：失敗を**空失敗**に変換（`consumed=false, committed=false`）。
* **`lookahead/notFollowedBy`**：非消費なので `Span` は**現在位置**。

---

## H. セキュリティ/Unicode 診断（1.4 連携）

* **Bidi 制御混入**（識別子/演算子内）→ `E6001`：
  「Bidi 制御文字は識別子に使用できません」＋該当箇所を `Delete`。
* **非 NFC 識別子** → `E6002`：「NFC ではありません。`normalize_nfc` を適用してください」。
* **confusable**（似姿）→ **Warning**：`W6101`。
* いずれも `PrettyOptions.locale` に従いメッセージを切替可能。

---

## I. 実装チェックリスト

* [ ] `Expectation` の**縮約ルール**：具体 > 抽象、最長一致、カテゴリ化。
* [ ] **farthest-first** の**厳密順序**：`byte_end` → `committed` → `expected ∪`。
* [ ] `cut` が **期待の再初期化**を行う。
* [ ] `many` の**空成功検出**と専用コード。
* [ ] `between`/演算子での **FixIt 挿入**。
* [ ] `pretty` は**グラフェム下線**＋**バイト併記**＋**文脈 3 段**。
* [ ] `toDiagnostics` は **LSP 風**に変換（範囲・severity・code・fix）。
* [ ] `recover` は **同期トークン**まで安全に前進し、診断を 1 件に集約。
* [ ] 大入力での **期待集合上限・メモリ制限**（`max_expected`）。

---

## J. ほんの少しの実例

**1) 演算子後の欠落**

```
input: "1 + (2 * 3"
error[E1001]: expected ')'
  --> expr.ks:1:10
   1 | 1 + (2 * 3
     |      ^---- insert ')'
note: while parsing expression → term → factor
```

**2) 予約語の直後**

```
input: "ifx (a) {}"
error[E1203]: expected whitespace after keyword 'if'
  --> stmt.ks:1:1
   1 | ifx (a) {}
     | ^^ 'if' is a keyword; 'ifx' is an identifier
help: put a space: "if x"
```

---

### まとめ

* **最遠位置・期待集合・文脈**の三本柱で、**短く直せる**エラーを一貫生成。
* `cut/label/attempt/recover` と **きれいに連動**し、`precedence` でも**欠落オペランド**や**非結合違反**を高品位に報告。
* **Unicode/安全性**診断も標準化し、**IDE/LSP** へそのまま渡せる **FixIt** を同梱。

---

## 関連仕様

* [1.4 文字モデル](1-4-test-unicode-model.md) - Unicode位置情報とセキュリティ診断の基盤
* [2.1 パーサ型](2-1-parser-type.md) - エラー型とReply構造の定義
* [2.2 コア・コンビネータ](2-2-core-combinator.md) - cut/label/attempt/recoverの動作仕様
* [2.3 字句レイヤ](2-3-lexer.md) - 字句エラーとの統合
* [2.4 演算子優先度ビルダー](2-4-op-builder.md) - 演算子特有のエラー処理
* [2.6 実行戦略](2-6-execution-strategy.md) - エラー集約とトレースの実装
