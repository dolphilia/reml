# 2.5 エラー設計（Core.Parse.Err）

> 目的：**説明的で短く、修正指向**の診断を、**最遠位置＋期待集合＋文脈**で一貫して出す。
> 前提：2.1 の `Reply{consumed, committed}`・`Span`、2.2 の `cut/label/attempt/recover`、2.4 の `precedence(operand, …)` と整合。
> **移行ガイド**: 診断モデルと監査連携の全体像は Chapter 3 の [3.6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md) に再整理されています。本章は Core.Parse 観点からの土台を提供します。

---

## A. 型（データモデル）

```reml
type Severity = Error | Warning | Note

type SeverityHint = Rollback | Retry | Ignore | Escalate

type ErrorDomain =
  | Parser
  | Config
  | Runtime
  | Network
  | Data
  | Audit
  | Security
  | CLI

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
  severity_hint: Option<SeverityHint>,
  domain: Option<ErrorDomain>,
  code: Option<Str>,        // "E0001" など（安定ID）
  message: Str,             // 1 行要約
  at: Span,                 // 主位置（1.4: 列=グラフェム）
  expected_summary: Option<ExpectationSummary>,
  notes: List<(Span, Str)>, // 追加メモ（複数可）
  span_trace: Option<SpanTrace>, // RunConfig.trace=true のときに付与される成功履歴
  fixits: List<FixIt>,
  audit_id: Option<Uuid>,   // Config/Audit 系機能と共有する識別子
  change_set: Option<Json>, // 設定差分などを JSON で保持
  stream_meta: Option<Json>,// ストリーミング実行時の補助情報
  quality_report_id: Option<Uuid>, // データ品質レポートとの関連
  extensions: Map<Str, Any> // プラグインが追加情報を格納するための自由領域
}

// 期待集合を人間語へ整形するためのサマリ
type ExpectationSummary = {
  message_key: Option<Str>,          // LSP/翻訳用キー（例: "expected.token")
  locale_args: List<Str>,            // メッセージテンプレートに渡す引数
  humanized: Option<Str>,            // テンプレート未設定時の自然言語
  context_note: Option<Str>,         // 文脈説明（例: "+ の後に式"）
  alternatives: List<Expectation>    // 優先順に並べ替えた候補一覧
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
* `expected_summary` はテンプレート ID と文脈を保持し、IDE/LSP がローカライズ済みメッセージを生成できるようにする。
* `domain` は必要に応じて責務領域を付与する分類タグであり、省略した場合は純粋にパーサからの診断として扱われます。`severity_hint` は運用側への推奨アクション（ロールバック・再試行・即時エスカレーションなど）を表します。
* `audit_id` / `change_set` / `stream_meta` / `quality_report_id` は、それぞれ Config ツール・差分レビュー・ストリーミング実行・データ品質検証から渡される共通メタデータであり、存在しない場合は `None`。`change_set` は [3-7](3-7-core-config-data.md) で定義される `Change` の配列（JSON 化）を保持する。
* `extensions` はプラグインやツールが任意の追加メタデータ（上記以外の設定差分、監査情報、テレメトリなど）を格納する自由領域で、コア仕様はその内容に関与しません。
* `span_trace` は `RunConfig.trace=true` のときにのみ設定され、最外層→失敗地点の順に成功スパンを格納する（[2-1 C](2-1-parser-type.md#c-スパンとトレース)）。IDE はこれを利用して「どのルールを通って失敗したか」を可視化できる。

---

## B. 生成と合成（アルゴリズム）

### B-1. 単一パーサの失敗を作る

```reml
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

### B-7. 期待集合のサマリ生成

1. **分類**：`Expectation` を `Token` / `Keyword` / `Class` / `Rule` / `Custom` に分け、`alternatives` を優先順位で整列（具体トークン → 文字クラス → ルール順）。
2. **テンプレート照合**：`PrettyOptions.expectation_templates` または CLI/LSP の登録テンプレートから `message_key` に一致する文を取得し、`locale_args` を埋め込む。
3. **文脈付与**：`ParseError.context` と `ExpectationSummary.context_note` を結合し、「`+` の後に式が必要」のような自然文を生成。
4. **フォールバック**：テンプレートが無い場合は `humanized` を生成（例：「ここで `)` または 数値 が必要です」）。
5. **LSP 連携**：`toDiagnostics` は `expected_summary.message_key` と `locale_args` を `data.expected` に埋め込み、クライアント側でのローカライズと候補提示を可能にする。

### B-8. SpanTrace の付与

* `RunConfig.trace=true`（[2-1 D](2-1-parser-type.md#d-実行設定-runconfig-とメモ)）のとき、ランタイムは成功区間の履歴 `SpanTrace` を収集する。
* `ParseError` と併せて得られたトレースは、診断生成時に `Diagnostic.span_trace` へそのまま転写する。既定では `Error`/`Warning` で常に保持し、`Note` のみ省略してノイズを抑える。
* CLI 表示では末尾から `PrettyOptions.context_depth` 件を `note: trace: rule @ span` の形式で追加し、LSP 連携では `data.spanTrace = Diagnostic.span_trace` として JSON 配列で共有する。これにより IDE は「どのルールが最終的に失敗へ至ったか」を視覚化できる。

### B-9. 条件付きコンパイル関連診断

| message_key | severity | domain | 説明 |
| --- | --- | --- | --- |
| `target.config.unknown_key` | Error | Config | `@cfg` が参照したキーが `RunConfig.extensions["target"]` で宣言されていない場合に発行。`notes` に既知キー一覧を提示し、FixIt で最も近いキー候補（編集距離ベース）を提案する。 |
| `target.config.unsupported_value` | Error | Config | キーは既知だが値がサポートされない場合に使用。`expected_summary` へ許可値を `Expectation::Custom` で列挙する。 |
| `unresolved.symbol.cfg` | Error | Parser | `@cfg` で無効化された宣言のみが存在し、参照解決できない場合。`notes` に「このシンボルは有効なターゲットが存在しません」と明示する。 |
| `effects.cfg.contract_violation` | Error | Parser | `@cfg` による条件分岐で `@pure` 等の契約を満たさない分岐が残る場合。`notes` へ効果集合の差分を表示する。 |
| `effects.cfg.unreachable` | Warning | Config | `@cfg` 論理式が恒偽であり、宣言が到達不能な場合。後続解析を省くために削除候補として FixIt を提示する。 |

* 上記メッセージは `Diagnostic.extensions["cfg"]` に評価ログを添付できる。`RunConfig` は `extensions["target"].diagnostics = Bool` で詳細ログ出力を切り替える。
* LSP での `@cfg` 可視化を支援するため、`Diagnostic.extensions["cfg"]` に `{ keys: List<Str>, evaluated: Map<Str, Str>, active: Bool }` を載せ、IDE が分岐条件を表示できるようにする。

### B-10. 効果宣言・ハンドラ関連診断（実験段階）

> `-Zalgebraic-effects` フラグが有効な場合に出力される。`stage` 情報は 3.6 §1 で定義する拡張メタデータに格納される。

| message_key | severity | domain | 説明 |
| --- | --- | --- | --- |
| `effects.contract.mismatch` | Error | Effect | ハンドラ適用後の残余効果 `Σ_after` が `@pure` や `@handles`、`@dsl_export(allows_effects=...)` の契約を超過している場合に発行。`notes` に `expected` / `actual` の差集合を表示し、`extensions["effects"].residual` に JSON で残余タグを格納する。 |
| `effects.stage.missing_opt_in` | Error | Effect | `stage = Experimental` の効果を利用しているにも関わらず `@requires_capability(stage="experimental")` が付与されていない場合に発行。`extensions["effects"].stage` に要求された stage と現在の Capability 設定を記録。 |
| `effects.handler.unhandled_operation` | Error | Effect | ハンドラが宣言した `operation` を実装していない場合、または `resume` を呼ばずに終了し残余効果が消えない場合に発行。`notes` で未捕捉操作を列挙し、`extensions["effects"].unhandled` に操作シグネチャを記録。 |
| `effects.handler.invalid_resume` | Error | Effect | `resume` を複数回呼び出した／`@reentrant` が無い状態で再入を試みた場合に発行。Capability Registry が拒否した場合は `notes` に `CapabilityError` の内容を併記。 |
| `effects.stage.promote_without_checks` | Warning | Effect | `stage` を `Beta`/`Stable` に更新した効果宣言に対し、対応する `@dsl_export` やマニフェストが旧ステージのままの場合に発行し、整合チェックの再実行を促す。 |

* `Diagnostic.extensions["effects"]` には `{ stage: Str, before: Set<Str>, handled: Set<Str>, residual: Set<Str>, handler: Option<Str>, unhandled: List<Str> }` を格納する。`before` はハンドラ適用前の潜在効果集合、`handled` は捕捉成功した集合。
* CLI は `--effects-debug` オプションが有効な場合、`extensions["effects"]` を整形して追加表示し、LSP は `data.effects` を参照して UI に残余タグを提示できる。

---

## C. 表示（pretty）と多言語

```reml
fn Err.pretty(src: Str, e: ParseError, opts: PrettyOptions) -> String

type PrettyOptions = {
  max_expected: usize = 6,           // 一覧上限
  context_depth: usize = 3,          // 文脈表示の深さ
  show_bytes: Bool = true,           // (byte 134) などを併記
  snippet_lines: usize = 2,          // 前後の抜粋行数
  color: Bool = true,                // 終端色付け
  locale: Locale = "ja",             // メッセージ言語
  expectation_locale: Option<Locale> = None,   // 期待メッセージのロケール（未指定なら locale を使用）
  expectation_templates: Map<Str, Str> = {}    // message_key -> テンプレート（"{0}", "{1}" 形式）
}
```

* **スニペット**：1.4 の **グラフェム列**で正確に下線。
* **主語**：「expected …, found ‘…’」形式だが、ロケールにより語順差し替え。
* **`context`**：「while parsing *expression* → *term* → *factor*」のように**内側 3 段**まで表示。
* **FixIt** は `^` 行に \*\*「ここに ‘)’ を挿入」\*\*のように注記。
* **期待テンプレート**：`expectation_templates` に登録された `message_key` を優先使用し、未登録時は `humanized` フォールバックを採用。

### C-1. 診断メッセージの国際化ポリシー

#### メッセージキーとテンプレート

* `message_key` は `領域.機能.イベント`（例: `parser.expectation.missing_token`）の **小文字ドット区切り** を原則とし、`RunConfig` やプラグインは自分の名前空間（`plugin.<id>.…`）内で衝突を避ける。
* `locale_args` は 0 オリジンの位置引数 `{0}`, `{1}`, … で参照し、**すべて文字列化済み**とする（構造値は JSON 化して渡す）。
* テンプレート解決順序は以下の通りで固定する。
  1. **診断個別のオーバーライド**：`Diagnostic.extensions["templates"][locale][message_key]` 等、エミッタが明示的に添付したテンプレート。
  2. **`PrettyOptions.expectation_templates`**：実行時オプションで注入された共有テンプレート。
  3. **既定言語辞書**：ビルトインの `default_locale`（通常は `"en"`）のテンプレートまたは `ExpectationSummary.humanized`。

#### 翻訳対象とロケール解決手順

診断の翻訳対象は以下のフィールド全体に及ぶ。

* `Diagnostic.message`（一次見出し）
* `Diagnostic.notes` の各文、および `Span` を伴う注釈
* `FixIt` の `text`（`Insert`/`Replace`/`Delete`）
* `Diagnostic.severity_hint` を人間語に変換した説明文
* `ExpectationSummary` の `message_key` と `context_note`

CLI／IDE／LSP に渡す際のロケール解決は次の段階で行う。

1. **基準ロケールの決定**：`PrettyOptions.locale` を最優先し、LSP ではクライアントからの `initializeParams.locale` 要求があればそれを `PrettyOptions.locale` へ反映する。
2. **期待メッセージの分岐**：`PrettyOptions.expectation_locale` が `Some` の場合は期待テンプレートのみそのロケールを使用し、`None` なら `locale` を共有する。
3. **辞書検索**：上記のテンプレート解決順序で `message_key` を探索し、未対応ロケールで見つからなければクライアント要求ロケールから **既定言語へフォールバック**。
4. **フォールバック適用**：テンプレートが得られない場合は、`ExpectationSummary.humanized` や `Diagnostic.message` の既定文をそのまま表示し、`locale_args` の整形も既定ルールに従う。

#### 翻訳辞書のロードと検証

| ステップ | 内容 | 例 |
| --- | --- | --- |
| 読み込み | `Locale` ごとに JSON/YAML 辞書をロードし、`message_key -> template` を構築する | `load_dictionary("ja")` |
| キャッシュ | `Arc<LruCache<Locale, TemplateMap>>` でロケール単位の辞書をキャッシュし、ホットパスでの I/O を回避する | `TEMPLATE_CACHE.get_or_try_insert(locale, load)` |
| 検証 | 各テンプレートの `{n}` が `locale_args.len()` と整合するか検査し、欠落や過剰があれば警告ログとともに既定言語へフォールバック | `validate_args(message_key, template, locale_args)` |

テンプレート解決の擬似コード例：

```pseudo
fn render(key, locale, args, opts, diag_override): String {
  let dict = resolve_dictionary(locale, diag_override, opts);
  let template = dict.get(key)
    .or_else(|| resolve_dictionary(DEFAULT_LOCALE, None, opts).get(key));
  if !validate_arity(template, args.len()) {
    log.warn("template arity mismatch", key, locale);
    return render_default(key, args);
  }
  return interpolate(template, args);
}
```

`DEFAULT_LOCALE` は CLI 設定またはサーバ既定で定義されたフォールバック言語コード（例: `"en"`）。

`RunConfig` やプラグイン拡張が独自メッセージを追加する場合、以下の手続きを踏む。

1. **キー空間の予約**：`RunConfig` は `config.<feature>.…`、プラグインは `plugin.<id>.…` の接頭辞を採用し、重複した `message_key` を禁止する。
2. **検証フック**：`RunConfig::register_locale_templates(locale, map)` またはプラグイン API で辞書登録時に `validate_args` を通過させ、キー重複や引数不一致を検査する。
3. **失敗時の例外ポリシー**：重大な不整合（欠落テンプレートや重複キー）は `Err::InvalidTemplate` を送出してロードを失敗させ、ランタイムは既定言語で継続しつつ警告ログを残す。実行中に見つかった場合は当該メッセージのみフォールバックし、IDE/LSP へは `data.localeFallback = true` を通知する。

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

8. **数値変換失敗（`as` キャスト／リテラル解決）**

* 共通方針：`Severity = Error`、`domain = Some(Parser)`、`code = Some("E710x")` を割り当て、`message` に**元の値と対象型**を必ず含める。
* **`E7101`（整数→整数）**：
  * `message`: `value {value} does not fit into {target}`。
  * `notes`: `allowed range is {min}..={max}; rounding: none` を添付し、`RunConfig.extensions["type"].numeric_defaults.integer` で選ばれた既定整数型を `notes` に明示する（例：「default integer type: i64」）。
  * `secondaries`: 可能であれば **元のリテラル位置**へ `FixIt::Replace` の候補（例: `value.clamp(min, max)` を示唆）を追加。
* **`E7102`（浮動小数→整数）**：
  * `message`: `cannot convert {value} ({classification}) to {target}`。`classification` には `NaN` / `+∞` / `-∞` / `out of range` のいずれかを入れる。
  * `notes`: `rounding mode: toward zero; valid interval: {min}..={max}` を付記。
* **`E7103`（コードポイント外）**：
  * `message`: `value {value} is not a valid Unicode scalar value`。
  * `notes`: `Unicode scalar range: 0x0000..=0x10FFFF except surrogates` を定型で入れる。
* いずれも `RunConfig.extensions["type"].numeric_defaults` を参照し、曖昧な数値リテラルが **どの型へ既定解決されたか**を `notes` に残す。未設定時は `{ integer: "i64", float: "f64" }` を既定とし、この既定値は CLI/IDE に露出する。
* `RunConfig.extensions["type"].numeric_defaults = { integer: Ident, float: Ident }` をオプションとして予約し、プロジェクト単位でリテラル既定型（例：`integer="i32"`）を差し替えた場合も診断が同じテンプレートを利用できるようにする。

---

## E. `recover`（回復）の仕様

```reml
fn recover<T>(p: Parser<T>, until: Parser<()>, with: T) -> Parser<T>
```

* `p` が失敗したら、**診断を残しつつ** `until` の位置（例：`";"` または行末）まで**読み捨て**、`with` を返す。
* 返す `with` は AST に **`ErrorNode{span, expected}`** として挿入可能（IDE で赤波線）。
* `RunConfig.merge_warnings` が true の場合、連続する回復を**1 つに集約**（ノイズ低減）。

---

## F. API（作る・混ぜる・見せる）

```reml
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

### F-1. 拡張診断メタデータ

`Diagnostic.domain` と `Diagnostic.extensions` を活用することで、プロジェクト固有の情報（例: 設定差分、監査 ID、テレメトリ）を診断へ付加できます。Reml コアはキー名や値の構造を規定せず、拡張側で運用に合わせたスキーマを定義します。

- `domain` によって CLI や IDE でのフィルタリングが行いやすくなります。未指定の場合は `Parser` 相当の扱いとなります。
- `extensions["config.diff"]` のように名前空間付きキーを用いると、複数ツールが衝突せずメタデータを共有できます。
- `severity_hint` は運用オペレーション（ロールバック推奨・再試行推奨など）を伝える簡易フラグとして利用します。

#### F-1-1. エラーコード命名の推奨

ドメイン別のコード規約は実装側で自由に定義できます。参考として `E{domain-prefix}{4桁}`（例: `E1001`）という既存フォーマットを継続利用すると、CLI や IDE の統合が容易です。

* FixIt テンプレート例:
  * `FixIt::AddMissing(field, suggestion)` – 必須項目が欠落した際の補完。
  * `FixIt::InsertToken(token)` – 括弧や記号の補完に利用。
  * `FixIt::ReplaceRange(range, text)` – 誤った構文を置換する提案。

### F-2. IDE/LSP・ログ連携

* `to_lsp_diagnostics` は `domain`・`severity_hint`・`expected_summary` を LSP データへ変換し、`span_trace` があれば `data.spanTrace` に転写する。`extensions` は `data.extensions` にそのまま反映される。
* パターン網羅性や期待集合など機械可読な情報は、`extensions["coverage.missing"]`（残余バリアントの列挙）や `data.coverage = { "missing": [...], "lintLevel": "warning" | "error" }` のように公開することで、IDE が自動補完候補やクイックフィックスを提示できる。
* 構造化ログを出力する場合は、`span_trace`・`extensions` を JSON にそのまま埋め込むことで外部ツールが追加情報を解釈できます。
* 監査や差分管理など高度な連携は、専用プラグインが `extensions` に必要なフィールドを定義し、利用側で合意したスキーマに従って処理してください。

### F-3. サンプル

```reml
let mut diag = Err.pretty(src, parse_error, {});
diag.extensions.insert("config.diff", diff.toJson());
diag.extensions.insert("run_id", currentRunId());
```

```reml
fn toStructuredLog(diag: Diagnostic) -> Json = json!({
  "event": "reml.error",
  "domain": diag.domain,
  "code": diag.code,
  "severity": diag.severity,
  "severity_hint": diag.severity_hint,
  "message": diag.message,
  "extensions": diag.extensions,
  "notes": diag.notes,
  "fixits": diag.fixits
})
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

**3) 非網羅な `match`（警告→属性でエラー）**

```
input:
  match color with
  | Red   -> "warm"
  | Blue  -> "cool"

warning[W4101]: non-exhaustive match; missing variants: Green
  --> palette.ks:2:3
   2 |   match color with
     |   ^^^^^^^^^^^^^^^ missing cases fall back to `panic`
note: this warning becomes error under @no_panic / lint.non_exhaustive_match = "error"
help: add an arm such as `| Green -> ...`
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
