# 3.6 Core Diagnostics & Audit

> 目的：Reml 全体で統一された診断 (`Diagnostic`) と監査 (`audit_id`, `change_set`) モデルを提供し、パーサ・標準ライブラリ・ツールが同一の情報粒度でログ・レポートを生成できるようにする。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | 正式仕様 |
| 効果タグ | `@pure`, `effect {diagnostic}`, `effect {audit}`, `effect {debug}`, `effect {trace}`, `effect {privacy}` |
| 依存モジュール | `Core.Prelude`, `Core.Text`, `Core.Numeric & Time`, `Core.Config`, `Core.Data`, `Core.IO` |
| 相互参照 | [2.5 エラー設計](2-5-error.md), [3.4 Core Numeric & Time](3-4-core-numeric-time.md), [3.5 Core IO & Path](3-5-core-io-path.md), [3.7 Core Config & Data](3-7-core-config-data.md) |

> **段階的導入ポリシー**: 新しい効果カテゴリや Capability と連携する診断は、`Diagnostic.extensions["effects"].stage`に `Experimental` / `Beta` / `Stable` を記録し、実験フラグで有効化した機能を明示する。CLI と LSP は `stage` が `Experimental` の診断をデフォルトで `Warning` に落とし、`--ack-experimental-diagnostics` を指定した場合のみ `Error` へ昇格させる運用を推奨する。

## 1. `Diagnostic` 構造体

既存の Chapter 2.5 で提示した構造を標準ライブラリ側で正式定義する。

```reml
pub type Diagnostic = {
  id: Option<Uuid>,
  message: Str,
  severity: Severity,
  domain: Option<DiagnosticDomain>,
  code: Option<Str>,
  primary: Span,
  secondary: List<SpanLabel>,
  hints: List<Hint>,
  expected: Option<ExpectationSummary>,
  audit: AuditEnvelope,
  timestamp: Timestamp,
}

pub type SpanLabel = {
  span: Span,
  message: Option<Str>,
}

pub enum Severity = Error | Warning | Info | Hint
```

- `domain` は診断が属する責務領域（構文、型、ターゲット等）を表す。`None` の場合はコンポーネント既定値を利用する。
- `timestamp` は [3.4](3-4-core-numeric-time.md) の `Timestamp` を利用し、診断生成時に `Core.Numeric.now()` を呼び出す。
- `AuditEnvelope` は監査情報を同梱する構造（後述）。
- `ExpectedSummary` は LSP/CLI でメッセージを国際化するための鍵と引数を保持する。

### 1.1 `AuditEnvelope`

```reml
pub type AuditEnvelope = {
  audit_id: Option<Uuid>,
  change_set: Option<Json>,
  capability: Option<CapabilityId>,
  metadata: Map<Str, Json>,
}
```

- `audit_id` は監査トレースの主キー。`change_set` は Config/Data の差分 JSON を保持する。
- `capability` はランタイム機能（Core.Runtime）との整合に利用。
- `metadata` は拡張用の自由領域で、プラグインが追加情報を埋め込む。

#### 1.1.1 監査イベント `AuditEvent`

```reml
pub enum AuditEvent = {
  PipelineStarted,
  PipelineCompleted,
  PipelineFailed,
  CapabilityMismatch,
  AsyncSupervisorRestarted,
  AsyncSupervisorExhausted,
  ConfigCompatChanged,
  EnvMutation,
  Custom(Str),
}
```

- 既定バリアントは Reml ランタイムと CLI が共有する監査カテゴリであり、文字列表現は `snake_case` に変換して `AuditEnvelope.metadata["event.kind"]` に記録する。例: `AuditEvent::PipelineStarted` → `"pipeline_started"`。
- `Custom(Str)` はプラグインや将来拡張向けの逃げ道で、アプリケーション固有のイベント名を格納する。組織内ガイドラインを遵守する場合も `snake_case` を推奨し、`AuditPolicy.include_patterns` と組み合わせてフィルタリング可能にする。
- `AuditEnvelope.metadata` にはイベントごとの必須キーを付与し、診断・CI・監査ツールが同じ情報粒度で追跡できるようにする。`metadata["event.id"]` に UUID を保存すると、同一イベント系列を多チャネルで相関させやすい。

| バリアント | 主な発火条件 | 必須メタデータ (`AuditEnvelope.metadata`) | 0-1 参照 | 関連章 |
| --- | --- | --- | --- | --- |
| `PipelineStarted` | Conductor が DSL 実行を開始したとき | `"pipeline.id"`, `"pipeline.dsl_id"`, `"pipeline.node"`, `"timestamp"` | §1.1 性能、§2.2 診断可視化 | 3-9 §1.4 |
| `PipelineCompleted` | 実行が正常終了したとき | 上記 + `"pipeline.outcome" = "success"`, 処理件数 (`"pipeline.count"`) | §1.1 | 3-9 §1.4.4 |
| `PipelineFailed` | 実行中に致命エラーで停止したとき | 上記 + `"error.code"`, `"error.message"`, `"error.severity"` | §1.2 安全性 | 3-9 §1.8 |
| `CapabilityMismatch` | Stage/Capability の整合検証で不一致を検出 | `"capability.id"`, `"capability.expected_stage"`, `"capability.actual_stage"`, `"dsl.node"` | §1.2 安全性 | 3-8 §1.2, 3-9 §1.4.3 |
| `AsyncSupervisorRestarted` | Supervisor が子役者を再起動 | `"async.supervisor.id"`, `"async.supervisor.actor"`, `"async.supervisor.restart_count"` | §1.2 安全性 | 3-9 §1.9.5 |
| `AsyncSupervisorExhausted` | 再起動予算を消費し尽くした | 直前列 + `"async.supervisor.budget"`, `"async.supervisor.outcome"` | §1.2 安全性 | 3-9 §1.9.5 |
| `ConfigCompatChanged` | 互換モードを CLI/Env/Manifest 等が変更 | `"config.source"`, `"config.format"`, `"config.profile"`, `"config.compatibility"` | §1.2 安全性, §2.2 可視性 | 3-7 §1.5, 3-10 §2 |
| `EnvMutation` | `set_env`/`remove_env` など環境を書き換え | `"env.operation"`, `"env.key"`, `"env.scope"`, `"requested_by"` | §1.2 安全性 | 3-10 §1 |

- 追加メタデータ（例: `"pipeline.latency_ms"`, `"async.supervisor.strategy"`）は自由に拡張してよいが、必須キーが欠落した場合は `AuditEvent::Custom` を使用しない限り仕様違反として扱う。CI は `AuditPolicy.level = AuditLevel::Warning` 以上のときに必須キーの欠落を検出し、0-1 §1.2 の安全性保証を担保する。
- イベントを `Diagnostic` と結合する場合は `AuditReference.events`（§6.1.2）へ列挙し、CLI/LSP で相関ビューを提供する。`AuditEvent::PipelineFailed` は `Severity::Error`、`AuditEvent::ConfigCompatChanged` は少なくとも `Severity::Warning` を推奨する。

### 1.2 診断ドメイン `DiagnosticDomain`

### 1.3 効果診断拡張 `effects`

効果宣言やハンドラに由来する診断では `Diagnostic.extensions["effects"]` を使用し、次の構造を格納する。

```reml
type EffectsExtension = {
  stage: Stage,                    // Experimental | Beta | Stable
  before: Set<EffectTag>,          // ハンドラ適用前の潜在効果集合
  handled: Set<EffectTag>,         // 捕捉に成功した効果集合
  residual: Set<EffectTag>,        // ハンドラ適用後に残った効果集合
  handler: Option<Str>,            // ハンドラ名（存在する場合）
  unhandled_operations: List<Str>, // 未実装 operation の一覧
  capability: Option<CapabilityId>,                    // 必要とされる Capability（任意）
  required_stage: Option<Runtime.StageRequirement>,    // Stage 要件（3-8 §1.2）
  actual_stage: Option<Stage>,                         // Capability Registry に登録された Stage
  capability_metadata: Option<Runtime.CapabilityDescriptor>, // `describe` から得たメタデータ
}

enum Stage = Experimental | Beta | Stable
```

`Stage` は Capability Registry（3.8 §1）と共有される列挙で、CLI/LSP は `Stage` に基づき表示レベルを調整する。`before` / `handled` / `residual` は 1.3 §I の効果計算結果に対応し、`residual = ∅` の場合は純粋化可能であることを意味する。`unhandled_operations` は `effects.handler.unhandled_operation` 診断（2.5 §B-10）で IDE へ提示する一覧として使用する。`required_stage` と `actual_stage` は Capability 要件と実際の Stage の差分を記録し、0-1 §1.2 の安全性指針に基づく是正アクションを促す基礎データとなる。`capability_metadata` には `Runtime.CapabilityDescriptor` を保持し、提供主体・効果タグ・最終検証時刻を監査ログへ転写する。

### 1.4 型クラス診断拡張 `typeclass`

型クラス制約の解決に関する診断では `Diagnostic.extensions["typeclass"]` を利用し、辞書渡し方式およびモノモルフィゼーション評価で必要となるメタデータを共有する。最低限、次の構造体を格納する。

```reml
type TypeclassExtension = {
  constraint: TraitConstraintSummary,                 // 診断対象の制約
  resolution_state: ResolutionState,                  // 成功/失敗/保留などの状態
  candidates: List<TraitCandidateSummary>,            // 解決候補の一覧
  selected: Option<TraitCandidateSummary>,            // 採用された候補（存在する場合）
  pending: List<TraitConstraintSummary>,              // 連鎖して発生した未解決制約
  generalized_typevars: List<TypeVarSummary>,         // 一般化された型変数情報
  dictionary: Option<DictLayoutSummary>,              // 生成・利用した辞書の概要
  graph: Option<ConstraintGraphSummary>,              // 制約グラフのサマリ（Graphviz 等）
}

type TraitConstraintSummary = {
  trait: Str,                     // 例: "Eq"
  parameters: List<TypeRepr>,     // 1-2 §A/B の表示規約で整形した型
  span: Span,                     // 制約が導入された位置
  origin: ConstraintOrigin,       // 推論器が付与する導入理由
}

enum ResolutionState = Pending | Satisfied | Failed

type TraitCandidateSummary = {
  impl_id: Str,                   // 実装 ID（モジュール修飾名）
  score: Option<Float>,           // 評価関数による採点（辞書渡し計測用）
  requires_where: Bool,           // 追加 where 制約を持つか
  source: CandidateSource,        // 派生/ユーザ定義などの由来
}

enum CandidateSource = Builtin | User | Derived | Auto

type DictLayoutSummary = {
  dict_type: TypeRepr,            // Core IR 上の `DictType`
  slots: List<DictSlotSummary>,   // vtable/フィールド構成
  metadata: Map<Str, Json>,       // 追加情報（任意）
}

type DictSlotSummary = {
  name: Str,                      // メソッドまたはフィールド名
  ty: TypeRepr,                   // スロットの型
  index: u32,                     // vtable 上のインデックス
  inlined: Bool,                  // インライン化されたか
}

type ConstraintGraphSummary = {
  nodes: List<TraitConstraintSummary>,
  edges: List<(usize, usize)>,    // `nodes` のインデックスで表現
  export_dot: Option<Str>,        // `Graphviz DOT` 文字列（任意）
}

type TypeVarSummary = {
  name: Str,                      // 例: "a"
  kind: Str,                      // 例: "*" / "* -> *"
  rigidity: Str,                  // "flex" | "rigid"
}

enum ConstraintOrigin = Parameter | WhereClause | AssociatedType | Implicit | Plugin
```

`TypeRepr` は 1-2 §A/B の表示規約に従う文字列表現とし、CLI/LSP はこの構造体を用いて候補比較ビューや辞書可視化を実装する。`candidates` は優先度順（解決アルゴリズムが算出したスコア降順）で並べる。`graph.export_dot` が存在する場合、開発者向けに `--emit-dict-graph` などのデバッグフラグで Graphviz を出力できるようにする。`AuditEnvelope.metadata["typeclass"]` には `TypeclassExtension` を JSON 化したものを格納し、Phase 2 診断タスクで要求される辞書メタデータ監査（2-1 §5.2）と整合させる。辞書が存在しない（モノモルフィゼーション経路）場合は `dictionary = None` を設定し、`candidates` と `resolution_state` のみで差分を説明する。

```reml
pub enum DiagnosticDomain = {
  Syntax,
  Parser,
  Type,
  Effect,
  Runtime,
  Config,
  Manifest,
  Target,
  Security,
  Plugin,
  Cli,
  Lsp,
  Other(Str),
}
```

- ドメインは診断を機能領域ごとに分類し、CLI/LSP/監査ログでのフィルタリングや集計に利用する。
- `Target` はクロスコンパイルやターゲットプロファイル整合性に関する診断を表し、本節 §7 でメッセージ定義を示す。
- `Other(Str)` は将来の拡張やユーザープロジェクト固有の分類に使用し、名前は `snake_case` 推奨とする。

## 2. 診断生成ヘルパ

```reml
fn diagnostic(message: Str) -> DiagnosticBuilder                     // `@pure`

struct DiagnosticBuilder {
  diag: Diagnostic,
}

impl DiagnosticBuilder {
  fn with_span(self, span: Span) -> Self;                             // `@pure`
  fn with_severity(self, severity: Severity) -> Self;                 // `@pure`
  fn with_domain(self, domain: DiagnosticDomain) -> Self;             // `@pure`
  fn with_code(self, code: Str) -> Self;                              // `@pure`
  fn add_hint(self, hint: Hint) -> Self;                              // `@pure`
  fn attach_audit(self, audit: AuditEnvelope) -> Self;                // `@pure`
  fn finish(self) -> Diagnostic;                                      // `@pure`
}
```

- `DiagnosticBuilder` は不可変操作で `Diagnostic` を組み立てる。
- 監査情報を伴う場合は `attach_audit` を利用し、`AuditEnvelope` を再利用できるようにする。

#### ピュア構築と発行責務

- `diagnostic`、`DiagnosticBuilder`、`from_parse_error` など `@pure` 指定の API は診断レコードをデータとして構築するだけであり、乱数・時刻・I/O を伴う処理を内部で呼び出してはならない。`Diag.new_uuid()` や `Core.Numeric.now()` などの効果が必要な値は呼び出し側で生成し、`with_code` や `attach_audit` 等の純粋操作で埋め込む。
- 診断を利用者へ公開するのは `tap_diag` や `emit` など `effect {diagnostic}` を要求する層の責務とする。これらの API は `id` や `timestamp` が未設定の診断に対して、自動的に `Uuid` や `Timestamp` を割り当てた上で監査ポリシーへ転送する。
- すべての `@pure` な検証関数は診断値を返すだけに留め、呼び出し元（CLI、LSP、ランタイム）が `effect {diagnostic, audit}` の文脈でまとめて送出する。これにより 0-1 §1.2（安全性）と §2.2（分かりやすいエラーメッセージ）に沿った決定的な再現性が確保される。
- 推奨パターン：`fn build_diag(...) -> Diagnostic // @pure` と `fn report_diag(...) -> Result<(), AuditError> // effect {diagnostic, audit}` を分離し、後者で `emit` や `AuditSink` へ委譲する。

### 2.1 `Result`/`Option` との連携

```reml
fn expect_ok<T, E: IntoDiagnostic>(result: Result<T, E>) -> Result<T, Diagnostic> // `@pure`
fn tap_diag<T>(result: Result<T, Diagnostic>, inspect: (Diagnostic) -> ()) -> Result<T, Diagnostic> // `effect {diagnostic, audit}`
```

- `IntoDiagnostic` トレイトにより任意のエラー型を `Diagnostic` へ変換。
- `tap_diag` は監査ログ出力や統計集計に利用でき、`effect {diagnostic, audit}` により診断発行と監査送出を同時に扱う。

### 2.2 Core.Parse 連携（`Parse.fail` / `Parse.recover`）

```reml
type ParseDiagnosticOptions = {
  severity: Severity = Severity::Error,
  domain: DiagnosticDomain = DiagnosticDomain::Parser,
  code: Option<Str> = None,
  locale: Option<Locale> = None,
  audit: Option<AuditEnvelope> = None,
  input_name: Option<Str> = None,
  attach_span_trace: Bool = true,
}

fn parse_error_defaults(input_name: Str) -> ParseDiagnosticOptions // `@pure`
fn from_parse_error(src: Str, err: ParseError, opts: ParseDiagnosticOptions) -> Diagnostic      // `@pure`
fn from_parse_errors(src: Str, errs: List<ParseError>, opts: ParseDiagnosticOptions) -> List<Diagnostic> // `@pure`
```

- `locale` は 2.5 §B-11 の手順で `RunConfig.locale` を渡し、未指定時は CLI/LSP 側の既定値を利用する。
- `audit` へ値を渡すと `Diagnostic.audit` が事前に設定され、監査ライン（§3）でそのまま利用できる。`RunConfig.extensions["audit"].envelope` を `Some(AuditEnvelope)` にしておくと、`Core.Parse` は `Parse.fail` 実行時にこの値を引き継ぐ。
- `input_name` は CLI/LSP で表示する入力名や DSL 名を保持する。指定しなかった場合は `"<unknown>"` が暗黙に使われ、監査メタデータでは `parse.input_name` に記録される。
- `parse_error_defaults(input_name)` は 0-1 §2.2 の「分かりやすいエラーメッセージ」を満たす初期値を組み立てるヘルパであり、`severity = Error`・`domain = Parser`・`input_name = Some(input_name)`・`attach_span_trace = true` を固定し、`audit = Some(AuditEnvelope { audit_id: None, change_set: None, capability: None, metadata: Map::empty() })` として `parse.input_name` を事前登録する。戻り値は通常のレコード更新で `code` や `locale` を補強して利用する。
- `attach_span_trace=false` とすると、`ParseError.span_trace` があっても `Diagnostic.span_trace` へコピーしない。ストリーミング実行などで診断サイズを抑えたい場合に使用する。
- `Parse.recover` は `from_parse_error` で得られた `Diagnostic` を `secondary` として保持しつつ、復旧位置に FixIt を追加する。復旧成功後でも診断の `severity` は原則変更しない（CLI 側で `merge_warnings` を有効化すると Warning へ落とす運用が可能）。

`Err.toDiagnostics`（2.5 §F）と CLI/LSP 実装は上記 API を共有し、1 回の失敗につき 1 件以上の `Diagnostic` を生成する。`ParseError.secondaries` に複数の補助診断がある場合、`from_parse_errors` は順序を保持したまま結合し、`Diagnostic.secondary` へ変換する。

### 2.3 エラーコードカタログ

```reml
type DiagnosticCatalog = Map<Str, DiagnosticTemplate>

type DiagnosticTemplate = {
  default_message: Str,
  default_severity: Severity,
  default_domain: DiagnosticDomain,
  docs_url: Option<Str>,
}

fn register_catalog(namespace: Str, catalog: DiagnosticCatalog)
fn resolve_catalog(namespace: Str) -> Option<DiagnosticCatalog>
```

- `namespace` は `parser`, `config`, `runtime` など責務単位で区切る。プラグインは `plugin.<id>` を使用する。
- `register_catalog` は起動時に一度だけ呼び、重複キーがある場合は `diagnostic("catalog.duplicate_key")` を返す。
- `Parse.fail` から個別コードを利用する場合、`from_parse_error` に `code` を渡す前に `DiagnosticCatalog` に登録済みであることを確認し、未登録コードは拒否する。これにより 0-1 §2.2 が求める「修正候補と期待値」の事前審査が可能になる。
- CLI/LSP はカタログの `docs_url` を `Diagnostic` の `extensions["docs"]` に反映し、開発者が即座にトラブルシューティング手順へアクセスできるようにする。

### 2.4 ドメイン別診断プリセット {#diagnostic-presets}

> 0-1 §1.2 と §2.2 が求める「安全性」と「分かりやすい診断」を守るため、ドメインごとに最低限のメタデータとエラー構造を共通化する。`Diag.*` のプリセットは CLI/LSP/監査で同じ情報を再利用することを前提とする。

#### 2.4.1 Core.Parse プリセット `parse_error_defaults` {#diagnostic-parse}

`Diag.parse_error_defaults(input_name)` は `ParseDiagnosticOptions` を初期化し、`Diag.from_parse_error` が同一の監査コンテキストを再構築できるようにする。戻り値は `audit = Some(AuditEnvelope { audit_id: None, change_set: None, capability: None, metadata: Map::empty() })` を基準とし、以下の監査キーを保証する（既に設定されている値は上書きしない）。

| 監査キー | 値の型 | 生成規則 |
| --- | --- | --- |
| `parse.input_name` | `Json.String` | `options.input_name` が `Some` であればその値、未指定時は `"<unknown>"`。 |
| `parse.context_path` | `Json.Array(Json.String)` | `ParseError.context` を外側→内側の順で格納。空配列は許容。 |
| `parse.expected_overview` | `Json.Object` | `ExpectationSummary` を `{ "message_key": Str?, "humanized": Str?, "alternatives": Json.Array }` として埋め込む。 |
| `parse.committed` | `Json.Bool` | `ParseError.committed` の値。 |
| `parse.far_consumed` | `Json.Bool` | `ParseError.far_consumed` の値。 |
| `parse.hint_count` | `Json.Number` | `ParseError.hints` の要素数。 |
| `parse.locale` | `Json.String` | `options.locale` が `Some(locale)` の場合に限り、`Locale::to_string()` を格納。 |
| `parse.secondaries` | `Json.Number` | `ParseError.secondaries` の件数。 |

同じ情報は `Diagnostic.extensions["parse"]` にもコピーし、CLI/LSP が監査ログと同じ粒度で可視化できるようにする。`parse_error_defaults` が返す `AuditEnvelope` は空の `metadata` を持つが、呼び出し側が追加で `Map.merge` した値も `from_parse_error` 実行後に保持される。

```reml
let diagnostic = Diag.from_parse_error(
  source,
  error,
  Diag.parse_error_defaults("GraphQL schema"),
);
```

このプリセットにより、外部 DSL や設定ファイルのブリッジでも `parse.input_name`・`parse.expected_overview` などの必須情報が欠落しなくなり、0-1 §2.2 の指標（行列表示・期待値提示・修正候補）を満たした診断を安定的に生成できる。監査ポリシーはこれらキーを用いて失敗傾向やロケールの逸脱を集計する。

#### 2.4.2 効果診断メッセージ (Effect Domain) {#diagnostic-effect}

> 1-3-effects-safety.md §I.5 と 3-8-core-runtime-capability.md §1.2 で定義した効果行整列・Stage/Capability 検査を `Diagnostic` と監査ログに落とし込むための共通仕様。

| `message_key` | 既定 Severity | 発生条件 | 監査メタデータ | 推奨対応 |
| --- | --- | --- | --- | --- |
| `effects.contract.stage_mismatch` | Error | `@handles` や `@requires_capability` による Stage 宣言と、Capability Registry が認証した Stage（`EffectsExtension.stage`）が一致しない。 | `AuditEnvelope.metadata` に `effect.stage.required`, `effect.stage.actual`, `effect.capability` を格納し、`effects` 拡張の `residual` を JSON として添付する。 | ハンドラ／呼び出し元に正しい `@requires_capability(stage=...)` を付与し、Stage 昇格フローと整合させる。CI では `--deny experimental` を併用して検出を強制。 |
| `effects.contract.reordered` | Warning（`Σ_after` が変化する場合は Error に昇格） | 効果ハンドラの並び替えによって `EffectsExtension.residual` が変化、捕捉対象が曖昧になる、または 1-3-effects-safety.md §I.5 の整列規約から逸脱。 | `AuditEnvelope.metadata` に `effect.order.before`, `effect.order.after`, `effect.residual.diff` を格納し、必要なら `recommendation` に最小修正案を記録する。 | 関連テストとリスク評価を添えたうえで整列規約へ戻すか、差分許容時は仕様書へ根拠を追記。CI では `--fail-on-warning` でブロックを推奨。 |

両診断は `DiagnosticDomain::Effect` を既定とし、`Diagnostic.extensions["effects"]` に Stage・効果集合・未処理 operation を記録する。`AuditCapability` はこれらのメタデータを利用して Stage 昇格レビューを自動起案し、`RunConfig.extensions["effects"]` のポリシーで拒否された場合は `effects.contract.stage_mismatch` をエミットする。

監査ログに出力する最低限のキーは次の通り。

- `effect.stage.required` / `effect.stage.actual`: Stage 不一致の根拠。
- `effect.residual.diff`: ハンドラ順序変更による残余効果の差分。空集合であれば情報系ログとして扱い、Severity を Warning に留める。
- `effect.capability`: Stage チェックと紐づく Capability ID。`CapabilityRegistry::register` の記録と突き合わせて整合性を検証する。

`Iterator` トレイト辞書に起因する `effects.contract.stage_mismatch` では、上記に加えて次のキーを必須とする。これらは `IteratorDictInfo` が提供する Stage 設定と Capability Registry の実測値を突合するためのものだが、一般化された Stage 診断とも矛盾しないよう `effect.stage.iterator.*` 名前空間を用いる。

| 監査キー | 値の型 | 生成規則 |
| --- | --- | --- |
| `effect.stage.iterator.required` | `Json.String` | `IteratorDictInfo.stage_requirement` をシリアライズした値（例: `"beta"` または `"at_least:beta"`）。 |
| `effect.stage.iterator.actual` | `Json.String` / `Json.Null` | Capability Registry が返した Stage。検証レコードが存在しない場合は `null`。 |
| `effect.stage.iterator.kind` | `Json.String` | `IteratorDictInfo.kind` を `snake_case` で記録（`array_like`, `slice_like`, `custom_runtime`, など）。 |
| `effect.stage.iterator.capability` | `Json.String` | `IteratorDictInfo.capability_id`。汎用キー `effect.capability` と同一値だが、監査集計時に Iterator 系メトリクスを抽出しやすくするために重複保持する。 |
| `effect.stage.iterator.source` | `Json.String` / `Json.Null` | DSL マニフェストや FFI ブリッジを指す識別子。`manifest_path` が存在する場合は相対パス、未登録時は `null`。 |

```json
"extensions": {
  "effects": {
    "stage": { "required": "beta", "actual": "experimental" },
    "capability": "core.iterator.collect",
    "iterator": {
      "required": "beta",
      "actual": "experimental",
      "kind": "array_like",
      "capability": "core.iterator.collect",
      "source": "dsl/core.iter.toml"
    }
  }
}
```

CLI/LSP の `Diagnostic.extensions["effects"]["iterator"]` へも同じキー集合を転写し、人間向け出力と監査ログが同じ語彙で比較できるようにする。CI メトリクス `iterator.stage.audit_pass_rate` はこれらのキーが揃っていることを前提に算出され、欠落時は `AuditPolicy` が `Warning` を昇格させる。

これらのキーは `AuditPolicy.exclude_patterns` で除外しない限り永続化され、`CapabilityAudit` レポートや LSP の効果ビューで差分分析に利用できる。

#### 2.4.3 Stage 差分プリセット `EffectDiagnostic` {#effect-diagnostic-stage}

```reml
pub struct EffectDiagnostic;

impl EffectDiagnostic {
  fn stage_violation(span: Span, capability: CapabilityId, err: Runtime.CapabilityError)
    -> Diagnostic // `@pure`
}
```

- `stage_violation` は `Runtime.verify_capability_stage` などから返された `CapabilityError::StageViolation` を受け取り、`Diagnostic.extensions["effects"]` に `required_stage`・`actual_stage`・`capability_metadata` を埋め込んだ `Diagnostic` を生成するためのユーティリティである。
- 戻り値の `Diagnostic` は `Diagnostic.domain = Some(DiagnosticDomain::Effect)`、`code = Some("effects.contract.stage_mismatch")` を既定とし、0-1 §2.2 が求める分かりやすいエラー提示を満たす。`message` は Stage 差分を自然言語で説明し、`capability`・`provider`・`last_verified_at` をヒントとして自動付与する。
- `AuditEnvelope.metadata` には `effect.stage.required` / `effect.stage.actual` / `effect.capability` に加えて `effect.provider`・`effect.manifest_path` を転写する。これにより監査ログから直接 Capability 設定の履歴を追跡でき、0-1 §1.2 の安全性レビューに必要な証跡を確保する。
- `err.actual_stage` が `None` の場合は `Diagnostic.severity = Error` のまま `expected` 情報のみを提示し、Capability 未登録（`CapabilityErrorKind::NotFound`）との区別を明示する。`StageViolation` では `None` を許容しないため、これが発生した場合はランタイム実装側のバグとして別途報告する。
- `Iterator` トレイト辞書由来の StageMismatch では、前節で定義した `effect.stage.iterator.*` を `AuditEnvelope.metadata` と `Diagnostic.extensions["effects"]["iterator"]` 双方に転写し、診断と監査レポートが一致した `IteratorDictInfo` のメタデータを共有する。欠落時は `iterator.stage.audit_pass_rate` メトリクスが `0` と見なされる。

### 2.5 AsyncError と診断統合 {#diagnostic-async}

`Core.Async` が返す `AsyncError`（3.9 §1.8）は `IntoDiagnostic` を実装し、以下のルールで `Diagnostic` と統合する。

```reml
type AsyncDiagnosticExtension = {
  kind: AsyncErrorKind,
  origin: AsyncErrorOrigin,
  metadata: Map<Str, Json>,
  cause_chain: List<AsyncErrorLink>,
}
```

- `Diagnostic.domain` が未設定の場合は `DiagnosticDomain::Runtime` を適用する。`code` は `async.error.<kind>`（`kind` は `AsyncErrorKind` を `snake_case` に変換）を既定値とし、CLI/LSP のハイライトに利用する。
- `Core.Async.timeout` 由来の `AsyncErrorKind::Timeout` は常に `code = Some("async.timeout")` とし、`extensions["async"]["timeout"]` に `waited`・`limit`・`origin` を構造化して保存する。既存の `async.error.timeout` 表記は後方互換のエイリアスとして扱い、UI はどちらも同一イベントにマップする。
- `Diagnostic.primary` は `AsyncError.span` を利用し、値が無い場合は 1-1 §B の合成 Span 規約で生成した位置を割り当てる。
- `AsyncError.cause` の各要素は順序を保持したまま `Diagnostic.secondary` の `SpanLabel` に変換する。`SpanLabel.message` には `AsyncErrorLink.message` を格納し、`origin` と `metadata` から抽出したキー（例: `retry_attempt`, `channel`）を括弧書きで併記する。
- `Diagnostic.extensions["async"]` には上記構造を格納し、`metadata` フィールドに `AsyncError.metadata` をマージする。`diagnostic_id` キーが存在する場合は `AuditEnvelope.metadata["async.diagnostic_id"]` にも反映し、重複報告を避ける。
- `AuditEnvelope.metadata["async.cause_chain"]` へ `AsyncError.cause` を JSON 化して保存し、監査ポリシーが `Trace` 未満でも最初の要素を残す。
- 実行計画エラーは `code = Some("async.plan.invalid")` または `code = Some("async.plan.unsupported")` を利用し、`extensions["async.plan"]` に `{ "plan_hash": Str, "strategy": Str, "backpressure": Str, "missing_capability": Option<RuntimeCapabilityId> }` を格納する。`plan_hash` は 128bit の `Blake3` を 32 文字の 16 進表現で記録し、性能監査（0-1 §1.1）と安全監査（0-1 §1.2）を両立させる。`missing_capability` は `async.plan.unsupported` の場合のみ必須。

これらの手順は 0-1 §1.2 と §2.2 に沿って、原因追跡と再現性を改善する。CLI/LSP は `AsyncDiagnosticExtension` を持つ診断をツリー表示する UI を提供することが推奨される。

#### 2.5.1 Supervisor 診断拡張 {#diagnostic-async-supervisor}

```reml
pub type SupervisorDiagnosticExtension = {
  supervisor_id: Uuid,
  supervisor_name: Str,
  actor_id: ActorId,
  outcome: SupervisorOutcome,
  restart_count: u16,
  strategy: RestartStrategyDigest,
  budget: Option<RestartBudgetDigest>,
  exhausted: Bool,
  stage_required: Option<Stage>,
  stage_actual: Option<Stage>,
}

pub type RestartStrategyDigest = {
  kind: RestartStrategyKind,
  budget: Option<RestartBudgetDigest>,
}

pub enum RestartStrategyKind = OneForOne | OneForAll | Temporary

pub type RestartBudgetDigest = {
  max_restarts: NonZeroU16,
  within: Duration,
  cooldown: Duration,
  restarts_in_window: u16,
  window_started_at: Timestamp,
}
```

- `SupervisorDiagnosticExtension` は 3.9 §1.9.5 の `SupervisorEvent` から組み立てられ、`Diagnostic.extensions["async.supervisor"]` に格納する。`outcome` は `SupervisorOutcome`（3.9 §1.9.5）を再利用し、CLI/LSP は `restart_count` と `restarts_in_window` を可視化することで 0-1 §1.2 の安全基準（過剰再起動の抑制）を監視できる。
- `stage_required` と `stage_actual` は Capability Registry が返した Stage 情報を記録し、`async.supervisor.capability_missing` 発生時に不足ステージを提示する。再起動系診断では `None` のままとし、Stage を UI 表示から省略して差分強調に集中させる。
- `exhausted = true` の診断は `Severity::Error` を既定とし、`RestartBudgetDigest` を必須化する。`Temporary` 戦略の子役者が停止した場合は `budget = None` のまま `SupervisorOutcome::Stopped` を記録し、再起動を行わない。
- `AuditEnvelope.metadata` には `async.supervisor.id`, `async.supervisor.name`, `async.supervisor.actor`, `async.supervisor.restart_count`、`async.supervisor.strategy`（`kind` を文字列化）を保存し、`AuditEvent::AsyncSupervisorRestarted` や `AsyncSupervisorExhausted` と連動させる。監査レポートはこれらのキーを用いて Stage 評価と Capability レビューを自動照合する。

推奨診断コード：

| `Diagnostic.code` | 既定 Severity | 発生条件 | 主な対応 |
| --- | --- | --- | --- |
| `async.supervisor.capability_missing` | Error | `RuntimeCapability::AsyncSupervisor` が未登録、または `stage_actual < stage_required` | `stage_required` / `stage_actual` を比較表示し、`CapabilityRegistry::register` の Stage 更新や `@requires_capability` の調整を案内する。 |
| `async.supervisor.restart` | Info | `SupervisorOutcome::Restarted` が観測された | `restart_count` と `RestartBudgetDigest` を表示し、利用者に再起動頻度を通知する。閾値が近づいた場合は CLI が Warning を提案できるよう `restarts_in_window` を添付する。 |
| `async.supervisor.escalation` | Warning | `SupervisorOutcome::Escalated`、もしくは `SupervisorHandle.restart` が `AsyncErrorKind::InvalidConfiguration` を返した | エスカレーション先の Capability と `SupervisorDiagnosticExtension` を確認し、`RestartStrategy` または子役者の `tags` を見直す。必要に応じて `CapabilityRegistry` で Stage 昇格を申請する。 |
| `async.supervisor.exhausted` | Error | `SupervisorOutcome::Exhausted` が記録され、`SupervisorDiagnosticExtension.exhausted = true` | DSL 側で `ExecutionPlan` を隔離し、`SupervisorSpec.strategy` や `ChildRestartPolicy` を再評価する。監査ログには `AuditEvent::AsyncSupervisorExhausted` を必須で添付する。 |

- LSP/CLI は `SupervisorDiagnosticExtension.exhausted` を閾値判定の入力に利用し、`async.supervisor.restart` が一定回数以上発生した場合に自動で Quick Fix（再起動予算の見直し）を提示することが推奨される。
- `async.supervisor.escalation` は 0-1 §1.2 の安全性を損なう潜在リスクとして扱い、`RunConfig.extensions["supervisor"].escalation_policy` が存在しない環境では `Severity::Error` に昇格させる。監査レポートでは `async.supervisor.strategy` と `CapabilityRegistry::stage_of` を突き合わせ、未承認の Stage へ再起動が波及していないかを確認する。

## 3. 監査ログ出力

```reml
pub type AuditSink = fn(Diagnostic) -> Result<(), AuditError>          // `effect {audit}`

fn emit(diag: Diagnostic, sink: AuditSink) -> Result<(), AuditError>    // `effect {diagnostic, audit}`
fn with_context(diag: Diagnostic, ctx: Json) -> Diagnostic              // `@pure`
fn redact(diag: Diagnostic, policy: RedactPolicy) -> Diagnostic         // `@pure`
```

- `AuditSink` は CLI/LSP/Runtime の橋渡しを抽象化した関数型。
- `emit` は診断の `id`・`timestamp`・`audit.audit_id` が未設定の場合に `effect {diagnostic}` で補完し、`tap_diag` などのユーティリティからも同じ振る舞いで発行される。

```reml
// 具体的な AuditSink 実装例
fn console_audit_sink(diag: Diagnostic) -> Result<(), AuditError>     // CLI 出力
fn json_audit_sink(diag: Diagnostic) -> Result<(), AuditError>       // JSON ログファイル
fn lsp_audit_sink(diag: Diagnostic) -> Result<(), AuditError>        // LSP プロトコル
fn remote_audit_sink(endpoint: Url) -> impl Fn(Diagnostic) -> Result<(), AuditError> // リモートログサーバ

// 複数シンクの組み合わせ
fn multi_audit_sink(sinks: List<AuditSink>) -> AuditSink             // 並列出力
fn filtered_audit_sink(sink: AuditSink, filter: (Diagnostic) -> Bool) -> AuditSink // フィルタリング
```
- `with_context` で監査特有の文脈（リクエスト ID 等）を付与。
- `redact` はポリシーに基づき個人情報などをマスクする。
- `RunConfig.extensions["audit"]` は `{ envelope: Option<AuditEnvelope>, policy: Option<AuditPolicy> }` を格納することを推奨し、`from_parse_error` は `envelope` を自動的に引き継ぐ。`policy` が設定されている場合は CLI 側で §3.2 の `apply_policy` を既定で呼び出す。

### 3.1 `AuditError`

```reml
pub type AuditError = {
  kind: AuditErrorKind,
  message: Str,
}

pub enum AuditErrorKind = Transport | Encoding | PolicyViolation
```

- 監査出力に失敗した場合でもアプリケーションが継続できるよう `AuditError` は非致命エラーとして扱う。

### 3.2 監査ポリシー管理

```reml
pub type AuditPolicy = {
  level: AuditLevel,
  include_patterns: List<Str>,
  exclude_patterns: List<Str>,
  retention_days: Option<u32>,
  anonymize_pii: Bool,
}

pub enum AuditLevel = Off | Error | Warning | Info | Debug | Trace

fn apply_policy(diag: Diagnostic, policy: AuditPolicy) -> Option<Diagnostic>  // `effect {privacy}`
fn audit_with_policy(diag: Diagnostic, sink: AuditSink, policy: AuditPolicy) -> Result<(), AuditError>
```

### 3.3 監査コンテキストとシステム呼び出し連携

`Core.Runtime` の `SyscallCapability` など、長時間に渡る監査対象処理では `AuditContext` を利用してログの一貫性を確保する。

```reml
pub struct AuditContext {
  domain: Str,
  subject: Str,
  sink: AuditSink,
  metadata: Map<Str, Json>,
}

impl AuditContext {
  fn new(domain: Str, subject: Str) -> Result<AuditContext, AuditError>     // `effect {audit}`
  fn with_metadata(self, metadata: Map<Str, Json>) -> Self;                // `@pure`
  fn log(self, event: Str, payload: Json) -> Result<(), AuditError>;       // `effect {audit}`
  fn log_with_span(self, event: Str, span: Span, payload: Json) -> Result<(), AuditError>; // `effect {audit}`
}

fn audited_syscall<T>(
  syscall_name: Str,
  operation: () -> Result<T, SyscallError>,
  sink: AuditSink,
) -> Result<T, SyscallError> // `effect {audit, syscall}`
```

- `AuditContext::new` はドメイン（例: "syscall"）と対象（例: システムコール名）を起点に監査セッションを構築する。\
- `audited_syscall` は [3-8 Core Runtime & Capability Registry](3-8-core-runtime-capability.md) で定義された `SyscallCapability.audited_syscall` の呼び出しモデルと一致し、`effect {audit}` を要求する。\
- `AuditContext::log` で記録した JSON ペイロードは `AuditEnvelope.metadata["event"]` として `Diagnostic` に格納され、`security_audit`（§4.2）へ転送可能。\
- メタデータに `policy_digest` や `security_policy` を埋め込むことで、`SecurityCapability.enforce_security_policy` の実行結果と相互参照できる。

```reml
fn example_syscall(fd: i32, sink: AuditSink) -> Result<usize, SyscallError> = {
  let ctx = AuditContext::new("syscall", "read")?.with_metadata({ "fd": fd.into() });
  audited_syscall("read", || {
    SyscallCapability::raw_syscall(SYS_READ, [fd as i64, 0, 0, 0, 0, 0])
      .map(|ret| ret as usize)
  }, multi_audit_sink([sink, console_audit_sink]))?
  ctx.log("syscall.completed", json!({ "result": "ok" }))?;
  Ok(ret)
}
```

このパターンにより、`effect {audit}` を付与した API は常に監査ログを伴い、`CapabilitySecurity.effect_scope` で宣言した `audit` タグと整合が保たれる。

## 4. プライバシー保護とセキュリティ

### 4.1 個人情報の除去

```reml
pub enum PiiType = Email | PhoneNumber | CreditCard | SocialSecurityNumber | Custom(Str)

pub type RedactPolicy = {
  pii_types: List<PiiType>,
  replacement: Str,
  preserve_structure: Bool,
}

fn detect_pii(text: Str) -> List<(PiiType, Span)>                    // `@pure`
fn redact_pii(diag: Diagnostic, policy: RedactPolicy) -> Diagnostic  // `effect {privacy}`
```

### 4.2 セキュリティ監査

```reml
fn security_audit(event: SecurityEvent, sink: AuditSink) -> Result<(), AuditError> // `effect {audit, security}`

pub type SecurityEvent = {
  event_type: SecurityEventType,
  user_id: Option<Str>,
  resource: Option<Str>,
  outcome: SecurityOutcome,
  timestamp: Timestamp,
}

pub enum SecurityEventType = Login | Logout | DataAccess | PermissionChange | ConfigModification
pub enum SecurityOutcome = Success | Failure(Str) | Suspicious(Str)
```

## 5. 差分・監査テンプレート

`Core.Config` / `Core.Data` と連携して差分情報を埋め込むユーティリティを提供する。

```reml
fn from_change(change: Change) -> AuditEnvelope         // `@pure`
fn merge_envelope(base: AuditEnvelope, extra: AuditEnvelope) -> AuditEnvelope // `@pure`
fn record_change_set<T>(value: T, diff: ChangeSet) -> Result<T, Diagnostic>   // `effect {audit}`
```

- `Change`/`ChangeSet` は Chapter 4.8 で定義。
- `record_change_set` は差分を監査ログに記録し、`effect {audit}` を要求する。

### 5.1 FFI 呼び出し監査テンプレート {#ffi-呼び出し監査テンプレート}

> 目的：`Core.Ffi`（3-9 §2.7）が要求する監査証跡を統一フォーマットで残し、Capability Registry および CI が呼び出し履歴を検証できるようにする。

`AuditEnvelope.metadata["ffi"]` に次の JSON オブジェクトを格納する。

| フィールド | 型 | 必須 | 説明 | 参照 |
| --- | --- | --- | --- | --- |
| `event` | Str | Required | 常に `"ffi.call"` を設定し、他の監査イベントと区別する | 3-9 §2.7 |
| `library` | Str | Required | 解決したライブラリパスまたは Capability が返す識別子 | `LibraryMetadata.path` |
| `symbol` | Str | Required | 呼び出したシンボル名 | `FfiBinding.name` |
| `call_site` | Str | Optional | 呼び出し元ソース位置 (`module:line`) | `Diagnostic.primary` |
| `effect_flags` | List<Str> | Required | 実際に付与した効果タグ。辞書順へ正規化する | 3-9 §2.2 |
| `latency_ns` | u64 | Optional | 呼び出し所要時間（ナノ秒）。測定不能な場合は省略 | 3-9 §2.7 |
| `status` | Str | Required | `success` / `failed` / `stubbed` / `leak` など実行結果を列挙する | `FfiErrorKind`, 3-9 §2.6 |
| `capability` | Str | Optional | `call_with_capability` を利用した場合の Capability ID | 3-8 §5.2 |
| `capability_stage` | Str | Optional | 当該 Capability の Stage（`Experimental` / `Beta` / `Stable`） | 3-8 §5.2.1 |

```json
{
  "event": "ffi.call",
  "library": "libcrypto.so",
  "symbol": "EVP_DigestInit_ex",
  "call_site": "core/crypto.reml:218",
  "effect_flags": ["ffi", "unsafe", "io.blocking"],
  "latency_ns": 32050,
  "status": "success",
  "capability": "runtime.ffi.default",
  "capability_stage": "stable"
}
```

- `status = "failed"` の場合は `Diagnostic.code = Some("ffi.call.failed")` を必須とし、`FfiErrorKind` と `message` を `AuditEnvelope.change_set` へ複写する。
- `status = "stubbed"` は 3-9 §2.3 の効果ハンドラ経由であることを示す。Stage 情報を `capability_stage` に記録し、CI は `Beta` 以上でない場合に警告を発行する。
- `capability` が設定されていない場合でも、`effect_flags` を `CapabilitySecurity.effect_scope` と突き合わせ、`audit_required = true` の Capability では監査漏れを CI が検出できるようにする。

## 6. DSLオーケストレーション向け可観測性

### 6.1 メトリクスプリセット

```reml
pub struct DslMetricsHandle = {
  latency: LatencyHistogram,
  throughput: CounterMetric,
  error_rate: RatioGauge,
  in_flight: GaugeMetric,
}

fn register_dsl_metrics(scope: &ExecutionMetricsScope, dsl_id: DslId) -> Result<DslMetricsHandle, Diagnostic> // `effect {trace}`
fn record_dsl_success(handle: DslMetricsHandle, duration: Duration) -> ()                                  // `effect {trace}`
fn record_dsl_failure(handle: DslMetricsHandle, error: Diagnostic, duration: Duration) -> ()               // `effect {trace, audit}`
fn observe_backpressure(handle: DslMetricsHandle, depth: usize) -> ()                                      // `effect {trace}`
```

- デフォルトメトリクス名（指標 → 型）:
  - `dsl.latency` → `LatencyHistogram`（p50/p95/p99 を追跡）
  - `dsl.throughput` → `CounterMetric`（1秒あたりの完了数）
  - `dsl.error_rate` → `RatioGauge`（成功/失敗比率）
  - `dsl.in_flight` → `GaugeMetric`（実行中タスク数）
- `register_dsl_metrics` は 3-8 §4 の `ExecutionMetricsScope` を入力に取り、`scope.registry()` を利用して DSL メトリクスを初期化する。これにより `Core.Async` の `channel_metrics` と同一スコープで収集され、パイプライン単位の集約が容易になる。
- `scope.resolved_limits()` に含まれる `ResourceLimitDigest` は `DslMetricsHandle` の診断拡張に転写され、`record_dsl_failure` 時に自動で `resource_limits` を添付する。`ExecutionPlanDigest` が存在する場合は `ExecutionMetricsScope.execution_plan` も同時に複写し、性能・安全性レビューの追跡可能性を高める（0-1 §1.1, §1.2）。
- 成功/失敗の記録は `ExecutionPlan` のエラーポリシーと連動し、監査ログ `AuditEnvelope` を自動的に添付できる。

#### 6.1.1 Conductor 監視ベースライン

| メトリクス | 取得手段 | 説明 | 0-1 章との対応 |
| --- | --- | --- | --- |
| `dsl.latency` | `register_dsl_metrics` が返す `latency` | DSL 実行 1 リクエストの所要時間をヒストグラム (p50/p95/p99) で計測し、遅延増大を検知する | §1.1 実用性能 |
| `dsl.throughput` | 上記ハンドルの `throughput` | 1 秒あたりの完了件数をカウントし、スケーラビリティ低下を早期把握する | §1.1 スケーラビリティ |
| `dsl.error_rate` | 上記ハンドルの `error_rate` | 成功/失敗比率を追跡し、エラー発生を迅速に診断へ接続する | §1.2 安全性 |
| `dsl.in_flight` | 上記ハンドルの `in_flight` | 実行中タスク数を測定し、バックプレッシャ兆候を把握する | §1.1 性能 & §1.2 安全性 |
| `channel.queue_depth` | 3.9 §1.4.5 `ChannelMetricsHandle.queue_depth` | チャネル待機数を計測し、閾値超過時に警告を発火する | §1.2 安全性 |
| `channel.dropped_messages` | 同 `dropped_messages` | ドロップ件数を累積し、オーケストレーション損失を追跡する | §1.2 安全性 |
| `channel.producer_latency` / `channel.consumer_latency` | 同 `producer_latency` / `consumer_latency` | 生産者・消費者の各遅延を記録し、責務境界を特定する | §2.2 分かりやすい診断 |
| `channel.throughput` | 同 `throughput` | チャネル単位の処理件数を記録し、DSL 全体の throughput と連携する | §1.1 スケーラビリティ |

- `conductor` の `monitoring` ブロックで `metrics` セクションを省略した場合、ランタイムは上記 8 項目を自動登録し、`ExecutionMetricsScope` を暗黙に生成したうえで `register_dsl_metrics` と 3.9 §1.4.5 `channel_metrics` を内部で呼び出す。利用者が追加メトリクスを指定しても、既定項目は必ず保持する。生成される診断は §6.1.2 の `MonitoringDigest` を介してメトリクス内容を共有する。
- `channel.*` メトリクスは `ChannelMetricOptions` の既定値 (`collect_* = true`) を利用し、`manifest.conductor.channels[].id` をプレフィックスとして系列を生成する。`ChannelLinkDigest` の `channel` にはこのプレフィックスを含めること。
- CLI/監査ログは `dsl.error_rate` が `0.05` を超えた時点で `Severity::Warning` の診断 `conductor.metrics.error_rate_high` を発行し、`channel.queue_depth` が `ExecutionPlan.backpressure.high_watermark` を連続 3 サイクル超過した場合は `Severity::Error` へ昇格させる。診断には `AuditEnvelope.metadata` にサンプリング時刻と計測値を添付し、`conductor.monitoring.metrics` キーに `MonitoringDigest.metrics` を保存する。
- メトリクス転送時は §4 のプライバシー保護手順を適用し、個人情報を含む値は `redact_pii` による匿名化後に CLI/LSP へ渡す。これにより 0-1 §1.2 の安全性と §2.2 の分かりやすい診断を両立する。

#### 6.1.2 Conductor 診断拡張 `conductor`

```reml
pub type ConductorDiagnosticExtension = {
  conductor_id: Str,
  node_id: Str,
  dsl_id: Option<DslId>,
  depends_on: List<Str>,
  capabilities: List<CapabilityId>,
  execution: Option<ExecutionPlanDigest>,
  resource_limits: List<ResourceLimitDigest>,
  monitoring: MonitoringDigest,
  channels: List<ChannelLinkDigest>,
  issue: ConductorIssueKind,
  audit_reference: Option<AuditReference>,
  snapshot: Option<Json>,
}

pub type ExecutionPlanDigest = {
  strategy: ExecutionStrategy,
  backpressure: Option<BackpressureWindow>,
  scheduling: Option<SchedulingPolicy>,
  error: ErrorPropagationPolicy,
}

pub type BackpressureWindow = {
  high_watermark: Option<usize>,
  low_watermark: Option<usize>,
}

pub type ResourceLimitDigest = {
  memory: Option<MemoryLimitSnapshot>,
  cpu: Option<CpuQuotaSnapshot>,
  custom: Map<Str, Json>,
}

pub type MemoryLimitSnapshot = {
  declaration: MemoryLimit,
  hard_bytes: NonZeroU64,
  soft_bytes: Option<NonZeroU64>,
}

pub type CpuQuotaSnapshot = {
  declaration: CpuQuota,
  scheduler_slots: NonZeroU16,
  share: Float,
}

pub type MonitoringDigest = {
  metrics: List<Str>,
  health_check: Option<HealthCheckDigest>,
  tracing: Option<TracingDigest>,
}

pub type HealthCheckDigest = {
  interval: Duration,
  probe: Option<Str>,
}

pub type TracingDigest = {
  mode: TracingMode,
  trigger: Option<Str>,
}

pub enum TracingMode = Disabled | Conditional | Always

pub type ChannelLinkDigest = {
  from: Str,
  to: Str,
  channel: Str,
  buffer: Option<usize>,
  codec: Option<Str>,
}

pub type AuditReference = {
  audit_id: Option<Uuid>,
  events: List<Str>,
}

pub enum ConductorIssueKind = CapabilityMismatch | ResourceLimit | ExecutionPlan | Channel | Monitoring | Custom(Str)
```

- `conductor_id` と `node_id` は診断対象の DSL ノードを一意に示し、LSP/CLI は `depends_on` と `channels` を利用して依存グラフをハイライトする。
- `ExecutionPlanDigest` は 3-9 §1.4 の構成要素を縮約し、`BackpressureWindow` が `None` の場合でも `ExecutionPlan.strategy` を表示する。値は `ExecutionMetricsScope.execution_plan` から自動取得され、閾値不正（例: `high_watermark <= low_watermark`）は `ConductorIssueKind::ExecutionPlan` を設定する。
- `ResourceLimitDigest.memory` と `ResourceLimitDigest.cpu` は 3.5 §9 の `MemoryLimitResolved` / `CpuQuotaNormalized` を縮約して格納し、値は `ExecutionMetricsScope.resolved_limits()` から自動転写される。CLI/LSP は `hard_bytes` と `scheduler_slots` を用いて 0-1 §1.1 の性能要件を再検証し、設定値が Stage や Capability の制約を満たしているか確認する。
- `MonitoringDigest.metrics` には §6.1 の既定メトリクスを含め、利用者が任意に追加したキーも保持する。`TracingDigest.mode = Conditional` は `trigger` に `@cfg` 条件や `RunConfig.trace_enabled` を記録する。
- `AuditReference` は §3 の監査ログと結合するためのメタデータで、`events` に `AuditEvent::PipelineStarted` などのイベント名を列挙する。`audit_id` が `None` の場合は監査連携されていない診断であると見なす。

| AuditEnvelope.metadata キー | `ConductorDiagnosticExtension` の対応フィールド | 用途 |
| --- | --- | --- |
| `conductor.id` | `conductor_id` | 監査レポートで同一 Conductor の診断を集約する |
| `conductor.node` | `node_id` | ノード単位でのレビュー（例: `transform`） |
| `conductor.capabilities` | `capabilities` | Stage/Capability レビュー (0-1 §1.2 準拠) |
| `conductor.execution` | `execution` | Backpressure/スケジューリング比較 |
| `conductor.resource_limits` | `resource_limits` | リソース制限の追跡と逸脱検出 |
| `conductor.monitoring.metrics` | `monitoring.metrics` | CLI/LSP のメトリクス表示 |
| `conductor.channels` | `channels` | チャネル ID とバッファ設定の参照 |

- CLI/監査ログは `ConductorIssueKind` を Severity 判定の補助として使用し、`CapabilityMismatch` は `Severity::Error`、`Monitoring` は `Severity::Warning` を既定値とする。`Custom` を利用する場合は `issue` と同じ値を `AuditEnvelope.metadata["conductor.issue"]` に保存し、根拠となる仕様ノート（`../notes/` 配下）へのリンクを付与する。
- `snapshot` は DSL ノードの部分構造や `ExecutionPlan` の JSON を格納し、0-1 §2.2 の「分かりやすい診断」を満たすために UI が差分表示できるようにする。個人情報や秘匿データを含む場合は §4 の手順でマスクする。

#### 6.1.3 Config 診断拡張 `config`

```reml
pub type ConfigDiagnosticExtension = {
  source: ConfigSource,
  manifest_path: Option<Path>,
  key_path: List<ConfigKeySegment>,
  profile: Option<Str>,
  compatibility: Option<ConfigCompatibilityDigest>,
  feature_guard: Option<FeatureGuardDigest>,
  schema: Option<Str>,
  diff: Option<ConfigDiffSummary>,
  snapshot: Option<Json>,
}

pub enum ConfigSource = Manifest | Env | Cli | Runtime | Generated | Custom(Str)

pub type ConfigKeySegment = {
  key: Str,
  index: Option<usize>,
}

pub type ConfigCompatibilityDigest = {
  format: Str,
  profile: Str,
  stage: Stage,
}

pub type FeatureGuardDigest = {
  feature: Str,
  expected_stage: Stage,
  actual_stage: Stage,
  cfg_condition: Option<Str>,
}

pub type ConfigDiffSummary = {
  missing: List<Str>,
  unexpected: List<Str>,
  changed: List<ChangedValue>,
}

pub type ChangedValue = {
  key: Str,
  before: Option<Json>,
  after: Option<Json>,
}
```

- `source` は値がどこから供給されたかを明示し、`Cli` > `Env` > `Manifest` > `Runtime` の優先順位（3-7 §1.5.2）を UI 側で再現できるようにする。
- `key_path` は配列インデックスを含む完全な階層を保持し、LSP は `manifest_path` と併用して該当セクションへジャンプする。`index` は 0 起点とする。
- `ConfigCompatibilityDigest.stage` に記録した Stage は 0-1 §1.2 の安全性指針に従い、`Stage::Experimental` の値を `Severity::Warning` 以上で通知する根拠となる。
- `FeatureGuardDigest` は `feature_guard` と `@cfg` の同期状態を比較し、未同期の場合は `actual_stage` と `cfg_condition` を併記する。`cfg_condition = None` の場合は RunConfig がオフラインであることを示す。
- `ConfigDiffSummary` は `load_manifest` / `validate_manifest` で得られた差分を要約し、`missing` / `unexpected` / `changed` を別リストで保持する。`ChangedValue` の `before` / `after` は `Json` 表現で保存し、CLI ではサンプル数を制限して可視化する。
- `snapshot` には検証対象の断片（例: TOML テーブル全体）を格納し、機密情報が含まれる場合は §4 に従ってマスクする。

| AuditEnvelope.metadata キー | `ConfigDiagnosticExtension` の対応フィールド | 用途 |
| --- | --- | --- |
| `config.source` | `source` | 優先順位の再現と監査経路の追跡 |
| `config.path` | `manifest_path` | 監査ログからファイル位置へ遷移 |
| `config.key_path` | `key_path` | 設定キーの特定と差分レビュー |
| `config.profile` | `profile` | プロファイル別の逸脱検知 |
| `config.compatibility` | `compatibility` | Stage と互換モードの整合確認 |
| `config.feature_guard` | `feature_guard` | Feature Gate の審査 |
| `config.diff` | `diff` | 変更点サマリのレビュー |

- CLI/LSP は `source` を基に 3-7 §1.5.2 の優先順位アイコンを表示し、`manifest_path` が `None` の場合は生成値 (`Runtime`) であると判断する。`ConfigDiffSummary` が空の場合は `diff = None` とし、診断は構造的問題（型不一致等）に集中していることを示す。
- `FeatureGuardDigest.actual_stage` が `Stage::Experimental` かつ `expected_stage` が `Stage::Stable` の場合は `Severity::Error` を推奨し、0-1 §1.2 の安全性維持に利用する。`cfg_condition` を `AuditEnvelope.metadata["config.feature_guard.cfg"]` に転写し、ビルドログで追跡可能にする。
- `snapshot` に含める JSON/TOML 断片は `AuditPolicy.anonymize_pii = true` の場合に自動マスクされるべきであり、ツールは `config.snapshot` を出力する前に `redact_pii` を適用する。

推奨診断コード：

| `Diagnostic.code` | 既定 Severity | 発生条件 | 対応 |
| --- | --- | --- | --- |
| `config.feature.mismatch` | `Error`（`missing_in_target` 有り）、それ以外は `Warning` | `feature_guard`, `RunConfigTarget.features`, `RunConfigTarget.feature_requirements` のいずれかに差異がある | CLI/LSP は差集合を提示し、`--fix` で `feature_guard` 同期を提案。`missing_in_target` が発生した場合はビルド停止。 |

### 6.2 トレース統合

```reml
fn start_dsl_span(tracer: Tracer, dsl_id: DslId, context: TraceContext) -> TraceSpan // `effect {trace}`
fn finish_dsl_span(span: TraceSpan, outcome: DslOutcome) -> ()                        // `effect {trace}`
fn attach_channel_link(span: TraceSpan, channel_id: ChannelId, direction: ChannelDirection) -> () // `effect {trace}`
```

- `start_dsl_span` は DSL 実行ごとのトレーススパンを生成し、`TraceContext` を継承して分散トレースに組み込む。
- `attach_channel_link` はチャネル間リンク（`~>`）を可視化するメタデータを追加する。
- `DslOutcome` は成功/失敗/フォールバック経路を表し、`finish_dsl_span` がメトリクス更新と同期する。

### 6.3 ターゲット診断メトリクス

```reml
fn record_target_diagnostics(metrics: DslMetricsHandle, diag: Diagnostic) -> () = {
  let target_errors = diag.extensions.get("cfg").and_then(|cfg| cfg.get("target_config_errors")).unwrap_or(0);
  if target_errors > 0 {
    metrics.error_rate.increment_with_tag("target", target_errors);
  }
}
```

- `Diagnostic.extensions["cfg"]` に格納された `target_config_errors` や `effects_cfg_contract` を読み取り、CI でのポータビリティ回帰を検知する。
- `../guides/ci-strategy.md` に記載の構造化ログと併用し、`RunConfig.extensions["target"]` の変更が期待どおりの挙動を保っているかを定期的に可視化する。
- 重大なポータビリティ診断が発生した場合は `AuditEnvelope.metadata["target"]` にターゲット概要を付与し、監査ログやダッシュボードで迅速に追跡できるようにする。

### 6.4 テンプレート診断ドメイン

> 目的：テンプレート DSL のレンダリング失敗・フィルター欠落・エスケープ逸脱を一元的に検出し、安全性（0-1章 §1.2）と性能（同 §1.1）の要件を満たす運用を支援する。

```reml
pub enum DiagnosticDomain = ... | Template

pub enum TemplateMessageKey =
  | RenderFailure                   // `template.render.failure`
  | FilterUnknown                   // `template.filter.unknown`
  | CapabilityMissing               // `template.capability.missing`
  | EscapeBypassed                  // `template.escape.bypassed`

fn diagnostic_template(key: TemplateMessageKey, span: Option<Span>, data: TemplateDiagData) -> Diagnostic

pub type TemplateDiagData = {
  template_id: Option<Str>,
  segment: Option<TemplateSegmentId>,
  filter: Option<Str>,
  capability: Option<CapabilityId>,
  escape_policy: Option<EscapePolicy>,
  context_snapshot: Option<Json>,
}
```

| `message_key` | 既定 Severity | 発生条件 | 推奨対応 |
| --- | --- | --- | --- |
| `template.render.failure` | Error | `Core.Text.Template.render` が `TemplateError::RenderPanic`/`SinkFailed` を返した | `context_snapshot` を確認し、フィルターコードまたは出力先を修正。CI では `AuditEnvelope` を添付して再発分析を行う。 |
| `template.filter.unknown` | Error | テンプレート内で未登録フィルターが参照された (`TemplateError::FilterMissing`) | `TemplateFilterRegistry.register_*` でフィルターを登録、またはテンプレートを修正。補完候補は `available` から提示。 |
| `template.capability.missing` | Error | フィルターに必要な Capability が欠落 (`TemplateError::CapabilityMissing`) | `conductor.with_capabilities` または `CapabilityRegistry` で該当 Capability を付与。 |
| `template.escape.bypassed` | Warning | `EscapePolicy::None` など緩和ポリシーが明示された | コンテキストに応じて Escaping を再検討し、`EscapePolicy::HtmlStrict` を既定に戻す。監査ポリシーで Warning→Error 昇格を推奨。 |

- `template_id` は `TemplateProgram` の識別子 (`manifest.dsl.id`) を推奨し、IDE での参照・差分比較に利用する。
- `segment` はテンプレート AST 内のセグメント ID を指し、LSP が該当箇所へジャンプできるようハイライト用オフセットを格納する。
- `context_snapshot` は `TemplateError` に添付された JSON を埋め込み、個人情報を含む場合は 4.1 節の `redact_pii` で匿名化する。
- `diagnostic_template` は `Core.Text.Template.to_diagnostic` から呼び出され、`TemplateDiagData` を `Diagnostic.extensions["template"]` に保存する。

### 6.5 監視メトリクスの CLI/LSP 連携

- CLI サブコマンド（例: `reml conductor monitor`）は 3.9 §1.4.5 `snapshot_channel_metrics` と本節 §6.1 `diagnostic_metrics` を併用し、`CliDiagnosticEnvelope.summary.stats` に `channel.queue_depth`, `channel.dropped_messages`, `dsl.latency_p95` 等を格納する。
- LSP は `workspace/diagnosticMetrics` 拡張で、`channel.queue_depth` が高水位を超過したチャネルを `CodeActionKind::QuickFix` と併せて表示し、利用者に `ExecutionPlan.backpressure` の再調整を促す。
- CLI/LSP から発行されるメトリクス通知は `AuditEnvelope.metadata` に `metric_kind`, `channel_id`, `observed_at`, `value` を必須項目として含め、監査ログと可観測性ツールで単一トレースに結合できるようにする。
- メトリクス転送時は §4 のプライバシー保護手順を適用し、個人情報を含む値は `redact_pii` による匿名化後に CLI/LSP へ渡す。これにより 0-1 §1.2 の安全性と §2.2 の分かりやすい診断を両立する。

## 7. ターゲット診断ドメイン (Target) {#diagnostic-target}

> 目的：クロスコンパイルやターゲットプロファイルの整合性に関するエラー／警告を体系化し、CLI・IDE・監査ログで一貫して扱う。

### 7.1 メッセージキー一覧

| `message_key` | 既定 Severity | 発生条件 | 推奨対応 |
| --- | --- | --- | --- |
| `target.profile.missing` | Error | `profile_id` が要求されたにもかかわらず、`RunConfigTarget.profile_id` が `None`（環境変数未設定、CLI オプション欠如等） | `REML_TARGET_PROFILE`・`reml build --target` などでプロファイルを明示。CI ではフェイルストップ。 |
| `target.abi.mismatch` | Error | `TargetProfile.runtime_revision` / `stdlib_version` とコンパイラ生成メタデータが不一致 | `reml toolchain install` で正しいランタイム/stdlib を取得し直し、`RunArtifactMetadata` を更新。 |
| `target.config.mismatch` | Warning（再現性検証時は Error 推奨） | `PlatformInfo` と `RunConfigTarget` の `os`/`arch`/`family`/`triple` などが一致しない | ホスト・クロスターゲット両方の設定値を確認し、CI では `--fail-on-warning` でエラー昇格を推奨。 |
| `target.capability.unknown` | Error | `@cfg(capability = "...")` または `RunConfigTarget.capabilities` に未知の Capability が含まれる | `capability_name(TargetCapability::...)` で定義されたカノニカル名を使用。独自拡張時は Capability Registry に登録。 |
| `target.config.unsupported_value` | Error | `RunConfigTarget.extra` 等で実装が未対応の値が指定された | サポートされる値を `RunConfig::register_target_key` で確認し、プロファイルを修正。 |

- 既定 Severity は CLI で `--fail-on-warning` やポリシーファイルにより変更可能。`Core.Env`・`Core.Runtime` は本表を基準に重大度を設定する。
- `Diagnostic.code` は `TARGET01`（profile missing）、`TARGET02`（ABI mismatch）、`TARGET03`（config mismatch）、`TARGET04`（capability unknown）、`TARGET05`（unsupported value）を推奨する。

### 7.2 拡張データフォーマット

`Diagnostic.domain = Some(DiagnosticDomain::Target)` の診断には、以下の拡張フィールドを `Diagnostic.extensions["target"]` として付与する。

```json
{
  "profile_id": "desktop-x86_64",
  "requested": {
    "os": "linux",
    "arch": "x86_64",
    "triple": "x86_64-unknown-linux-gnu"
  },
  "detected": {
    "os": "macos",
    "arch": "aarch64"
  },
  "capability": "unicode.nfc",
  "runtime_revision": {
    "profile": "rc-2024-09",
    "artifact": "rc-2024-08"
  }
}
```

- `requested` は `TargetProfile` / `RunConfigTarget` から取得した値、`detected` は `PlatformInfo` 由来の値を格納する。
- `capability` は Capability 名（`capability_name` の戻り値）を指し、該当しない場合は省略可能。
- `runtime_revision` や `stdlib_version` は比較対象をペアで格納し、監査ログでの差分追跡を容易にする。
- 監査ログでは `AuditEnvelope.metadata["target"] = extensions["target"]` を推奨し、ポータビリティ検証結果を一元化する。

### 7.3 運用ガイドライン

- CLI は `CliDiagnosticEnvelope.phase = CliPhase::Codegen` または `CliPhase::Execution` に `Target` ドメイン診断を紐付け、`summary.stats["target_failures"]` を更新する。
- CI では `DiagnosticDomain::Target` を優先的に集計し、`../guides/ci-strategy.md` に定義するマトリクス上で失敗ターゲットを特定する。
- IDE/LSP では `Target` ドメイン診断をワークスペースレベル警告として表示し、該当ファイルが無い場合でも `RunConfigTarget` 情報を提示する。
- `Core.Env` と `Core.Runtime` はターゲット診断を発生させた際、`RunArtifactMetadata.hash` を `extensions["target"].hash` に追加し、再ビルドのトレーサビリティを確保する。

## 8. Runtime Bridge 診断 (Runtime) {#diagnostic-bridge}

Runtime Bridge 契約（[3-8-core-runtime-capability.md](3-8-core-runtime-capability.md) §10）で検証される Stage・Capability・ターゲット整合性は `DiagnosticDomain::Runtime` として報告する。CLI/LSP は `RuntimeCapability::ExternalBridge(id)` と紐付けて可用性を表示し、監査ログは `AuditEnvelope` と `RuntimeBridgeAuditSpec` を同期させる。

### 8.1 拡張データ `bridge`

```reml
pub type BridgeDiagnostic = {
  id: RuntimeBridgeId,
  stage_required: Option<Runtime.StageRequirement>,
  stage_actual: Option<Runtime.StageId>,
  target_requested: Option<Str>,
  target_detected: Option<Str>,
  manifest_path: Option<Path>,
  checklist_missing: List<Text>,
}
```

- `id` は `RuntimeBridgeRegistry::describe_bridge` で取得できる識別子。CLI (`reml bridge describe`) はこの値を基に詳細情報を表示する。
- `stage_required` / `stage_actual` は `verify_capability_stage` と同じ列挙で、`RuntimeBridgeError::StageViolation` の根拠を提示する。
- `target_requested` / `target_detected` は `RunConfig.extensions["target"].profile_id` と `RuntimeBridgeDescriptor.target_profiles` の差分を表し、ターゲット不一致時の再現性調査に利用する。
- `manifest_path` は `reml.toml` 等の設定ファイル上の宣言位置。IDE は該当ファイルへのジャンプに使用する。
- `checklist_missing` は `RuntimeBridgeAuditSpec.rollout_checklist`／`mandatory_events` の未達項目を列挙し、監査レビューでの確認を促す。

### 8.2 診断コード一覧

| コード | Severity | 発火条件 | 拡張データ / 監査 | 推奨対応 |
| --- | --- | --- | --- | --- |
| `bridge.contract.violation` | Error | `RuntimeBridgeRegistry::acquire_bridge` が Capability 検証や Stage 条件で失敗。 | `extensions["bridge"].id`, `stage_required`, `stage_actual`, `AuditEnvelope.metadata["bridge.capability"] = capability_id` | 必要 Capability の Stage を昇格させるか、`RuntimeBridgeDescriptor.required_capabilities` を調整する。 |
| `bridge.stage.experimental` | Warning | `RuntimeBridgeDescriptor.stage = Stage::Experimental` のブリッジを起動した際、最低 1 度記録。 | `extensions["bridge"].stage_required = Some(StageRequirement::Exact(Experimental))`, `stage_actual = Some(Experimental)` | ロールバック手順と監査ログ (`bridge.stage`) を確認し、安定化後は Stage 昇格を実施する。 |
| `bridge.target.mismatch` | Error | `RuntimeBridgeDescriptor.target_profiles` と `RunConfig.extensions["target"].profile_id` が不一致。 | `target_requested`, `target_detected`, `AuditEnvelope.metadata["target.profile.requested"]` | ターゲットプロファイルの設定を見直し、互換プロファイルで再登録する。 |
| `bridge.audit.missing_event` | Error | `RuntimeBridgeAuditSpec.mandatory_events` に列挙したイベントが監査ログに存在しない。 | `checklist_missing`, `AuditEnvelope.metadata["bridge.missing_events"]` | 監査ログで `audit.log("bridge.*", …)` を再実行し、`requires_audit_effect = true` を満たす。 |
| `bridge.diff.invalid` | Error | `RuntimeBridgeReloadSpec.diff_format` に合わない差分がホットリロードへ渡された。 | `AuditEnvelope.metadata["bridge.diff.expected"]`, `"bridge.diff.received"` | `Config.compare`（3-7 §4.2）で生成した差分形式を用い、形式不一致時はロールバックを実行する。 |

- すべての `bridge.*` 診断は `Diagnostic.domain = DiagnosticDomain::Runtime` を既定とし、`AuditEnvelope.metadata["bridge.id"] = extensions["bridge"].id` を必須とする。
- `RuntimeCapability::ExternalBridge(id)` が Stage 不整合で無効化された場合は `bridge.contract.violation` が発生し、同時に `PlatformInfo.runtime_capabilities` から該当 ID を除外する。
- CI で実験段階ブリッジを禁止する際は `--deny experimental` を指定し、`bridge.stage.experimental` を検出した時点で失敗させる運用を推奨する。

## 9. 使用例（CLI エラー報告）


```reml
use Core;
use Core.Diagnostics;
use Core.Config;

fn validate_config(cfg: AppConfig, audit: AuditSink) -> Result<(), Diagnostic> =
  ensure(cfg.timeout < 5000, || Diagnostic::invalid_value(cfg.timeout))?
    .tap_diag(|diag|
      emit(
        diag
          |> Diagnostic::builder()
          |> Diagnostic::attach_audit(from_change(cfg.change_set))
          |> Diagnostic::finish(),
        audit,
      ).ok()
    );
  Ok(())
```

- `ensure` と `tap_diag` を組み合わせ、検証失敗時に監査ログへ自動送出。
- `from_change` により `change_set` を `AuditEnvelope` へ変換し、監査と診断に共通語彙を適用する。

## 10. CLI/LSP 連携の具体例

### 10.1 CLI ツール統合

```reml
// CLI コマンドラインオプション
fn setup_cli_diagnostics(args: CliArgs) -> AuditSink {
  let policy = AuditPolicy {
    level: args.verbosity.to_audit_level(),
    anonymize_pii: !args.debug_mode,
    ..AuditPolicy::default()
  };

  match args.output_format {
    OutputFormat::Human => filtered_audit_sink(console_audit_sink, policy.filter),
    OutputFormat::Json => json_audit_sink,
    OutputFormat::Structured => lsp_audit_sink,
  }
}
```

### 10.2 LSP サーバー統合

```reml
// LSP プロトコル対応
fn diagnostic_to_lsp(diag: Diagnostic) -> LspDiagnostic {
  LspDiagnostic {
    range: span_to_lsp_range(diag.primary),
    severity: severity_to_lsp(diag.severity),
    code: diag.code,
    message: diag.message,
    related_information: diag.secondary.map(|span| span_to_related_info(span)),
    data: diag.audit.metadata.to_json(),
  }
}

fn batch_publish_diagnostics(diagnostics: List<Diagnostic>, client: LspClient) -> Result<(), AuditError>
```

### 10.3 メトリクス監視ダッシュボード

```reml
fn diagnostic_metrics(diagnostics: Iter<Diagnostic>) -> DiagnosticMetrics {
  DiagnosticMetrics {
    total_count: diagnostics.count(),
    by_severity: diagnostics.group_by(|d| d.severity).map_values(|group| group.count()),
    by_code: diagnostics.group_by(|d| d.code).map_values(|group| group.count()),
    resolution_time: diagnostics.map(|d| d.timestamp).variance(),
  }
}
```

> 関連: [2.5 エラー設計](2-5-error.md), [3.4 Core Numeric & Time](3-4-core-numeric-time.md), [3.5 Core IO & Path](3-5-core-io-path.md), [3.7 Core Config & Data](3-7-core-config-data.md)

## 11. CLI コマンドプロトコル {#cli-protocol}

`reml build`, `reml test`, `reml fmt`, `reml check` などの公式 CLI は、診断と監査の統合を保証するため `CliDiagnosticEnvelope` 構造を介して出力を共通化する。

```reml
type CliDiagnosticEnvelope = {
  command: CliCommand,
  phase: CliPhase,
  diagnostics: List<Diagnostic>,
  summary: CliSummary,
  exit_code: ExitCode,
}

type CliSummary = {
  inputs: List<Path>,
  started_at: Timestamp,
  finished_at: Timestamp,
  artifact: Option<Path>,
  stats: Map<Str, Json>,
}

enum CliCommand = Build | Test | Fmt | Check | Publish | New

enum CliPhase = Parsing | TypeCheck | EffectCheck | Codegen | Execution | Formatting | Reporting
```

- `command` は CLI が実行した操作。`Fmt` や `Check` を追加しても `Diagnostic` フローは共通。
- `phase` は診断が発生した処理段階を示し、`Diagnostic.expected` や `AuditEnvelope.metadata["phase"]` と合わせて LSP/IDE が不具合箇所を可視化できる。
- `summary.stats` には `parsed_files`, `tests_passed`, `formatting_changes` 等のコマンド固有メトリクスを格納し、JSON モードで `--summary` フラグが `true` の場合に必須。

CLI は出力モードに応じて次のフォーマットで `CliDiagnosticEnvelope` をシリアライズする。

| オプション | 出力 | 説明 |
| --- | --- | --- |
| `--output human` | 標準エラーへ人間向け整形（ターミナル色付け可）。 | `Diagnostic` を逐次表示し、終了時に `summary` をテキスト化。 |
| `--output json` | 標準出力へ JSON 行（NDJSON）。 | 各 `CliDiagnosticEnvelope` を 1 行 JSON で書き出し、`diagnostics` は配列。 |
| `--output lsp` | LSP transport 適合のメッセージ。 | `publishDiagnostics` と `window/logMessage` にマッピング。 |

`CliDiagnosticEnvelope.exit_code` は [3.7 Core Config & Data](3-7-core-config-data.md) の `attach_exit_code` と一致し、`diagnostics` 中で `Severity::Error` が 1 件以上ある場合は `ExitCode::Failure`（非ゼロ）を返す。`severity=Warning` のみの場合は `ExitCode::Warning` を返し、CI ツールが閾値を設定できるようにする。

`AuditEnvelope` の `metadata` には CLI 固有の `command`, `phase`, `run_id` を必ず含めること。`run_id` は `Uuid` で、`reml` サブコマンドの 1 実行あたり 1 つ発行される。これにより CLI/IDE/監査ログ間でトレースを結び付けられる。

CLI は `CliDiagnosticEnvelope` を生成した後、`emit(envelope.diagnostics[i], sink)` を順次呼び出し、構造化ログと人間向け表示の両方を実現する。`summary` の最終書き出し後に `exit_code` をプロセスの終了コードとして使用する。
