# 2.1 パーサ型

> 目標：**小さく強いコア**で、**高品質エラー**と**実用性能（ゼロコピー・Packrat/左再帰対応）**を両立。
> 原則：**純粋（副作用なし）**・**Unicode前提**・**デフォルト安全**。
> スコープ：パーサの**型**・**入出力モデル**・**実行時状態**・**コミット/消費の意味論**を確定します（詳細なエラー統合は *2.5* で掘り下げ）。

---

## A. 主要型

```kestrel
// コア：パーサは Input を読み、成功/失敗と残り入力を返す純関数
type Parser<T> = fn(&mut State) -> Reply<T>

// 実行結果（consumed/committed の2ビットを明示）
type Reply<T> =
  | Ok(value: T, rest: Input, span: Span, consumed: Bool)
  | Err(error: ParseError, consumed: Bool, committed: Bool)

// 実行状態（不変入力 + 可変の解析状態）
type State = {
  input: Input,                // 現在の入力ビュー（不変データの参照＋オフセット）
  config: RunConfig,           // 実行設定（Packrat 等）
  memo: MemoTable,             // Packrat/左再帰用メモテーブル
  diag: DiagState,             // 最遠エラー等の集約
  trace: TraceState            // 追跡（オフ既定）
}
```

**ポイント**

* `Reply` は **4状態**を表現可能：
  `Ok(consumed=false/true)` / `Err(consumed=false/true, committed=false/true)`
  → `or` の分岐可否や `cut` の挙動を**分岐なし**で実装できる（Parsec 流の *empty/consumed* + *commit*）。
* `span` は **そのパーサが消費した範囲**（`Ok` のみ）。ノード単位の位置取りに使う。

---

## B. 入力モデル `Input`

```kestrel
type Input = {
  source: SourceId,        // ファイル/文字列単位の識別子
  bytes: Bytes,            // UTF-8 本体（共有参照/COW）
  byte_off: usize,         // 現在の先頭（バイト）
  line: usize,             // 現在の行番号（1-origin）
  column: usize,           // 現在の列（拡張書記素基準、1-origin）
  // 境界キャッシュ（必要時だけ構築、ビュー間で共有）
  cp_index: Option<CpIndex>,    // コードポイント境界表
  g_index: Option<GraphemeIndex>// グラフェム境界表
}
```

* **不変ビュー**：`Input` は参照共有の **ゼロコピー**スライス。`rest` は **オフセットを進めた写像**のみ。
* **位置**は 1.4 の文字モデルに準拠（行=LF 正規化、列=グラフェム）。
* `mark()/rewind()` は `Input` の**スナップショット**で安価に取れる（バックトラックに使用）。

---

## C. スパンとトレース

```kestrel
type Span = {
  source: SourceId,
  byte_start: usize, byte_end: usize,
  line_start: usize, col_start: usize,
  line_end: usize,   col_end: usize
}

// 成功断片の履歴（IDE/可視化目的）。既定は OFF。
type SpanTrace = List<(name: String, span: Span)>
```

* 既定では **成功スパンのみ**保持（軽量）。
* `.spanned()` コンビネータで **「値 + Span」** を得る（AST への位置付与に使う）。
* `SpanTrace` は実行時 `RunConfig.trace = On` のときのみ収集（オーバーヘッド遮断）。

---

## D. 実行設定 `RunConfig` とメモ

```kestrel
type RunConfig = {
  exec_mode: "normal" | "packrat" | "hybrid" | "streaming" = "normal",
  require_eof: Bool = false,            // 全消費を要求（parse_all 相当）
  packrat: Bool = false,                // Packrat メモ化を明示的に有効化
  left_recursion: "off" | "on" | "auto" = "auto",
  fuel_max_steps: Option<usize> = None, // 評価ステップ上限（DoS/ループ防止）
  fuel_on_empty_loop: "error" | "warn" = "error",
  packrat_window_bytes: Option<usize> = Some(1 << 20),
  memo_max_entries: Option<usize> = Some(1 << 20),
  trace: Bool = false,
  merge_warnings: Bool = true,
  stream_buffer_bytes: Option<usize> = Some(64 * 1024)
}

type ParserId = u32  // ルール毎に安定ID（rule()/label() が付与）
type MemoKey  = (ParserId, usize /*byte_off*/)
type MemoVal<T> = Reply<T>  // Ok/Err ごと丸ごとキャッシュ
type MemoTable = Map<MemoKey, Any>  // 実装上は型消去（内部用）
```

* **RunConfig の主な項目**
  - `exec_mode` で実行戦略を切替え（詳細は [2.6 実行戦略](2-6-execution-strategy.md)）。
  - `packrat` と `left_recursion` は Packrat メモ化と seed-growing 左再帰を制御。
  - `fuel_max_steps` / `fuel_on_empty_loop` は停止性の安全弁。
  - `packrat_window_bytes` / `memo_max_entries` はキャッシュのメモリ上限。
  - `stream_buffer_bytes` はストリーム入力のリングバッファ既定サイズ。
  - それ以外は 1.1 で説明したエラー報告やトレースの挙動を調整する。
* `rule(name, p)` が **ParserId とラベル**を付与し、Packrat と診断に使う。

---

## E. コミットと消費の意味論

* `consumed`：**入力を1バイト以上前進**したか。
* `committed`：`cut` 境界を**越えた**とマーク（消費の有無に関わらず）。

**合成の基本規則（抜粋）**

* `p.or(q)`：

  * `p` が `Err(consumed=true, _ )` または `Err(_, committed=true)` → **q を試さない**。
  * `p` が `Err(consumed=false, committed=false)` → **q を試す**。
* `p.then(q)`：

  * `p` が `Ok(consumed=*)` → `q` へ続行（`consumed` は合成：`p||q`）
  * `p` が `Err` → そのまま `Err`。
* `cut`：以降で失敗したら **`committed=true`** を返す（期待集合は 2.5 参照）。
* `label("x", p)`：`p` の期待名を `"x"` に差し替え（エラー統合で優先）。

> この規則で **`try` 相当**は不要：`cut` を使わず書けば *empty エラー* として `or` に落ちる。必要なら `recover` を使う。

---

## F. 失敗表現（最小要素：2.5 と両立）

```kestrel
type ParseError = {
  at: Span,                           // 失敗位置（最狭）
  expected: Set<Expectation>,         // 期待集合（トークン/ラベル/EOF 等）
  context: List<Label>,               // 直近の label からの文脈
  committed: Bool,                    // cut を越えた失敗か
  notes: List<String>                 // 補助（回復やヒント）
}
```

* **Ok/Err に `consumed/committed` を分離**したことで、エラー統合（最遠位置の採用・期待セットの和/差）を**一意に定義**できる（詳細は *2.5*）。

---

## G. ランナー API（外部からの呼び出し）

```kestrel
fn run<T>(p: Parser<T>, src: String, cfg: RunConfig = {}) -> Result<(T, Span), ParseError>
// 成功時は値と**全体のスパン**（開始〜終了）を返す。
// cfg.require_eof=true なら残余があれば EOF 期待エラーを返す。

fn run_partial<T>(p: Parser<T>, src: String, cfg: RunConfig = {}) -> Result<(T, Input, Span), ParseError>
// 部分パース：残り Input も返す（REPL/トークナイザ向け）。

fn run_stream<T>(p: Parser<T>, feeder: Feeder, cfg: RunConfig = {}) -> Result<StreamOutcome<T>, ParseError>
// ストリーム入力。Pending が返った場合は続きが必要。

fn resume<T>(cont: Continuation<T>, more: Bytes) -> Result<StreamOutcome<T>, ParseError>
// 追加バイトを供給してストリームを再開。

type StreamOutcome<T> =
  | Completed(value: T, span: Span)
  | Pending(Continuation<T>)

type Feeder = {
  pull: fn(max_bytes: usize) -> Result<Bytes, FeederSignal>
}

type FeederSignal =
  | Ready
  | Await
  | Closed
  | Error(StreamError)

type StreamError = { kind: String, detail: Option<String> }

type Continuation<T> = {
  state: Opaque,
  commit_watermark: usize,
  buffered: Input
}
```

* `StreamOutcome::Pending` が返った場合は `resume` に `Continuation` と追加バイトを渡す。
* `Feeder.pull` は `FeederSignal::Ready` でチャンクを返し、`::Await` でバックプレッシャ、`::Closed` で終端、`::Error` でストリームエラーを通知。
* **ゼロコピー**：`src` は `Input.bytes` へ **参照共有**。
* 文字モデル（1.4）により、列は**グラフェム**、`Span` は**バイトと行列の両方**を保持。

---

## H. 代数則（使用者向けの直観）

* **純度**：`Parser<T>` は参照透過（同じ `State` → 同じ `Reply`）。
* **Functor**：`map` は恒等・合成を保つ。
* **Applicative/Monadic**：`then/andThen` は結合律を満たす（エラー統合規則の範囲で）。
* **`or` の単位**：`fail("x")` は空失敗（`consumed=false, committed=false`）。
* **`cut`**：`label("x", cut(p))` で「ここから先は x を期待」を強制。

---

## I. プラグイン登録と Capability（Draft）

> DSL プラグインを登録し、Parser capability を管理するための暫定 API 案。

```kestrel
type CapabilitySet = Set<String>

type PluginCapability = {
  name: String,
  version: SemVer,
  traits: Set<String>,
  since: Option<SemVer>,
  deprecated: Option<SemVer>
}

type PluginRegistrar = {
  register_schema: fn(name: String, schema: Any) -> (),
  register_parser: fn(name: String, factory: fn() -> Parser<Any>) -> (),
  register_capability: fn(CapabilitySet) -> ()
}

type ParserPlugin = {
  name: String,
  version: SemVer,
  capabilities: List<PluginCapability>,
  register: fn(PluginRegistrar) -> ()
}

fn register_plugin(plugin: ParserPlugin) -> Result<(), PluginError>
fn with_capabilities<T>(cap: CapabilitySet, p: Parser<T>) -> Parser<T>
```

* `register_plugin` はプラグインが提供する DSL/コンビネータを登録し、`PluginRegistrar` 経由で `ParserId` を割り当てる。
* `CapabilitySet` は `parser.requires({"template"})` のような照会・制約に利用。
* `with_capabilities` はプラグインが要求する capability を宣言し、満たされない場合 `PluginError::MissingCapability` を返す。

### I-1. 互換性とバージョン

* `SemVer` 準拠で互換性チェックを行い、競合時は `PluginError::Conflict { plugin, existing }` を返す。
* `PluginCapability` の `since` / `deprecated` により、利用側が警告やフェーズアウトを制御できる。

### I-2. サンプル（Draft）

```kestrel
let templating = ParserPlugin {
  name = "Kestrel.Web.Templating",
  version = SemVer(1, 2, 0),
  capabilities = [
    { name = "template", version = SemVer(1,0,0), traits = {"render"}, since = Some(SemVer(1,0,0)), deprecated = None }
  ],
  register = |reg| {
    reg.register_schema("TemplateConfig", templateSchema);
    reg.register_parser("render", || renderParser);
  }
}

register_plugin(templating)?

let render = with_capabilities({"template"}, renderParser)
```

## I. メモリと性能（実装規約）

* **Input**：COW/RC・SSO（短文字列インライン）・部分文字列は親バッファ参照。
* **Span**：必要最小を保持。`SpanTrace` は OFF 既定。
* **Packrat**：

  * キーは `(ParserId, byte_off)`、値は `Reply<T>`。
  * LRU/リングで上限を設け、巨大入力でのメモリ爆発を回避。
* **左再帰**：`left_recursion=true` のとき、既知の **種別変換法**（seed-growing）を使用（ルールに `ParserId` が必須）。
* **ステップ上限**：`fuel_max_steps` で無限ループ検出（診断に直近のルール列を含める）。

---

## J. プラグイン登録と Capability（Draft）

> DSL プラグインを登録し、Parser capability を管理するための暫定 API 案。

### J-1. API 定義

```kestrel
type CapabilitySet = Set<String>

type PluginCapability = {
  name: String,
  version: SemVer,
  traits: Set<String>,
  since: Option<SemVer>,
  deprecated: Option<SemVer>
}

type PluginRegistrar = {
  register_schema: fn(name: String, schema: Any) -> (),
  register_parser: fn(name: String, factory: fn() -> Parser<Any>) -> (),
  register_capability: fn(CapabilitySet) -> ()
}

type ParserPlugin = {
  name: String,
  version: SemVer,
  capabilities: List<PluginCapability>,
  register: fn(PluginRegistrar) -> ()
}

fn register_plugin(plugin: ParserPlugin) -> Result<(), PluginError>
fn with_capabilities<T>(cap: CapabilitySet, p: Parser<T>) -> Parser<T>

```

type PluginError =
  | MissingCapability { name: String }
  | Conflict { plugin: String, existing: SemVer, incoming: SemVer }
  | RegistrationFailed { reason: String }

type PluginWarning =
  | DeprecatedCapability { name: String, deprecated: SemVer }
```

| 構造体 | フィールド | 意味 |
| --- | --- | --- |
| `PluginCapability` | `name` | DSL 機能名。例: `"template"`, `"config"` |
|  | `version` | Capability のバージョン (SemVer) |
|  | `traits` | 提供する機能タグ（例: `render`, `diff`） |
|  | `since` / `deprecated` | 利用可能開始バージョン、廃止予定バージョン |
| `ParserPlugin` | `name` | プラグイン識別子 (却下時参照) |
|  | `capabilities` | 提供する Capability の一覧 |
|  | `register` | コンビネータやスキーマを登録する関数 |
| `PluginRegistrar` | `register_schema` | スキーマ DSL を登録 |
|  | `register_parser` | パーサ・コンビネータを登録 |
|  | `register_capability` | 追加 Capability を宣言 |

* `register_plugin` はプラグインが提供する DSL/コンビネータを登録し、`PluginRegistrar` 経由で `ParserId` を割り当てる。成功時は `Ok(())`、失敗時は `PluginError` を返す。
* `CapabilitySet` は `parser.requires({"template"})` のような照会・制約に利用。
* `with_capabilities` はプラグインが要求する capability を宣言し、実行時に満たされない場合 `PluginError::MissingCapability` を返す。

### J-2. 互換性とバージョン

| ケース | ルール | エラー例 |
| --- | --- | --- |
| 同名プラグイン重複 | SemVer 互換なら最新版へ更新、非互換なら拒否 | `PluginError::Conflict { plugin, existing }` |
| Capability 未満 | `with_capabilities` で指定した名前が未登録 | `PluginError::MissingCapability { name }` |
| Deprecated | `deprecated` <= 現行バージョンで警告、将来削除 | `PluginWarning::DeprecatedCapability` |

### J-3. サンプル（Draft）

```kestrel
let templating = ParserPlugin {
  name = "Kestrel.Web.Templating",
  version = SemVer(1, 2, 0),
  capabilities = [
    { name = "template", version = SemVer(1,0,0), traits = {"render"}, since = Some(SemVer(1,0,0)), deprecated = None }
  ],
  register = |reg| {
    reg.register_schema("TemplateConfig", templateSchema);
    reg.register_parser("render", || renderParser);
  }
}

register_plugin(templating)?

let render = with_capabilities({"template"}, renderParser)
```

---

## K. ミニ例（意味論の確認）

```kestrel
// トークン
let sym = |s: Str| rule("sym(" + s + ")", Lex.symbol(sc, s))

// 式: atom ('*' atom)*
let atom: Parser<i64> =
  rule("atom",
    (Lex.int(10).map(|n| n)                      // Ok(..., consumed=true)
     .or(sym("(").then(expr).then(sym(")")).map(|(_,v,_)| v))  // 括弧
     .or(label("number or '('", fail())))        // 空失敗 → or が次を試す
  )

let term: Parser<i64> =
  rule("term",
    atom.andThen( many( sym("*").cut().then(atom) ) )
        .map(|(h, tail)| tail.fold(h, |a, (_,b)| a * b))
  )
// '*' の直後に cut → 以降の err は committed=true になり、
// `atom or (...)` に戻らず “ここは '*' の右項が必要” と報告される。
```

---

## K. 仕様チェックリスト

* [ ] `Reply` は **Ok/Err × consumed/committed** を表現（4状態）。
* [ ] `Input` は UTF-8/COW、行=LF正規化、列=グラフェム、**ゼロコピー**。
* [ ] `Span` は**開始/終了の行列＋バイト**を保持。
* [ ] `run / run_partial / run_stream / resume` の外部 API を定義（`require_eof` やストリーム継続など）。
* [ ] `RunConfig` で **Packrat/左再帰/トレース**を切替。
* [ ] `rule(name, p)` で **ParserId/ラベル**を付与（Packrat & 診断）。
* [ ] `or/then/cut/label` の**合成規則**を確定。
* [ ] メモ上限・ステップ上限の**安全弁**を持つ。
* [ ] 文字モデル（1.4）と**列=グラフェム**で位置整合。
* [ ] すべて**純関数**（1.3 効果）— 外界作用は `Parser` の外で扱う。

---

### まとめ

* `Parser<T> = fn(&mut State) -> Reply<T>` による**最小核**に、

  * **consumed/committed** の2ビット、
  * **ゼロコピー入力と正確な Span**、
  * **Packrat/左再帰/トレース**の *ON/OFF* を備え、
* **書きやすさ**（`cut/label` が直観的）、**読みやすさ**（`rule` 命名・位置情報）、**エラー品質**（期待集合×最遠位置）、**性能**（メモ化・ゼロコピー）を同時に満たします。

この 2.1 を土台に、次は **2.2 コア・コンビネータ**で API の最小公理系を詰めましょう。
