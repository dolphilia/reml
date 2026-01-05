# 3.6 Core Diagnostics & Audit

> 目的：Reml 全体で統一された診断 (`Diagnostic`) と監査 (`audit_id`, `change_set`) モデルを提供し、パーサ・標準ライブラリ・ツールが同一の情報粒度でログ・レポートを生成できるようにする。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | 正式仕様 |
| 効果タグ | `@pure`, `effect {diagnostic}`, `effect {audit}`, `effect {debug}`, `effect {trace}`, `effect {privacy}`, `effect {migration}` |
| 依存モジュール | `Core.Prelude`, `Core.Text`, `Core.Numeric & Time`, `Core.Config`, `Core.Data`, `Core.IO`, `Core.Net` |
| 相互参照 | [2.5 エラー設計](2-5-error.md), [3.4 Core Numeric & Time](3-4-core-numeric-time.md), [3.5 Core IO & Path](3-5-core-io-path.md), [3.7 Core Config & Data](3-7-core-config-data.md), [3.17 Core Net](3-17-core-net.md) |

> **段階的導入ポリシー**: 新しい効果カテゴリや Capability と連携する診断は、`Diagnostic.extensions["effects"].stage`に `Experimental` / `Beta` / `Stable` を記録し、実験フラグで有効化した機能を明示する。CLI と LSP は `stage` が `Experimental` の診断をデフォルトで `Warning` に落とし、`--ack-experimental-diagnostics` を指定した場合のみ `Error` へ昇格させる運用を推奨する。`effect {migration}` は [3-7 Core Config & Data](3-7-core-config-data.md) で追加された `MigrationPlan` API が Configuration 差分を適用するときに用いるタグであり、Config CLI (`reml config migrate`) が `Diagnostic.extensions["migration"]` と `AuditEnvelope.metadata["config.migration.*"]` を同時に書き込めることを保証する。

## 1. `Diagnostic` 構造体

既存の Chapter 2.5 で提示した構造を標準ライブラリ側で正式定義する。

```reml
pub type Diagnostic = {
  schema_version: Str,
  id: Option<Uuid>,
  message: Str,
  severity: Severity,
  severity_hint: Option<SeverityHint>,
  domain: Option<DiagnosticDomain>,
  source_dsl: Option<Str>,
  code: Option<Str>,
  codes: List<Str>,
  primary: Span,
  location: Option<Location>,
  span_trace: List<TraceFrame>,
  secondary: List<SpanLabel>,
  notes: List<DiagnosticNote>,
  hints: List<Hint>,
  structured_hints: List<StructuredHint>,
  fixits: List<FixIt>,
  expected: Option<ExpectationSummary>,
  recoverability: Recoverability,
  audit_metadata: Map<Str, Json>,
  audit: AuditEnvelope,
  timestamp: Timestamp,
}

pub enum Severity = Error | Warning | Info | Hint

pub enum SeverityHint = Rollback | Retry | Ignore | Escalate

pub type SpanLabel = { span: Span, message: Option<Str> }
pub type DiagnosticNote = { label: Str, message: Str, span: Option<Span> }
pub type StructuredHint = {
  id: Str,
  title: Str,
  kind: Str,
  span: Option<Location>,
  payload: Option<Json>,
  actions: List<FixIt>,
}

pub enum FixIt =
  | Insert { span: Span, text: Str }
  | Replace { span: Span, text: Str }
  | Delete { span: Span }

pub type Location = {
  file: Path,
  line: Int,
  column: Int,
  endLine: Int,
  endColumn: Int,
}

pub type TraceFrame = { label: Option<Str>, span: Span }
```

- `severity` は CLI・LSP・監査ログで共通の 4 値（`Error` / `Warning` / `Info` / `Hint`）を採用し、情報診断とヒント診断を区別したフィルタリングを可能にする。
- `schema_version` は `Diagnostic` JSON の互換性を示す識別子であり、Rust Frontend CLI/LSP では `3.0.0-alpha` を固定で書き込む（`CliDiagnosticEnvelope.schema_version` と同じ値を共有する）。
- `domain` は診断が属する責務領域（構文、型、ターゲット等）を表す。`None` の場合はコンポーネント既定値を利用する。
- `source_dsl` は埋め込み DSL で発生した診断の発生源を明示する識別子。`embedded_dsl` 実行中は `dsl_id` を記録し、親 DSL 由来の診断は `None` とする。
- `primary` はハイライト付きの範囲情報、`location` は IDE/LSP 互換の簡易座標（`file`/`line`/`column`/`endLine`/`endColumn`）であり、双方を揃えておくことで CLI/LSP/監査ログから同じ座標へジャンプできる。
- `severity_hint` は CLI/LSP/AI が「Retry 可能か」「即時 Rollback が必要か」を判断するための追加ヒント。
- `codes` は補助的なエイリアスを持つ診断コード（例: `parser.syntax_error` + `syntax.eof`）を全て列挙したリストであり、`code` がメイン識別子、`codes` が LSP/AI 向けの全文検索対象となる。
- `span_trace` は Streaming/Packrat の実行経路を `TraceFrame` として保持し、CLI/LSP/監査ログで共通の `span_trace[*].span` 情報を提供する。
- `notes` は診断の補足情報を複数保持し、`label` と `message` を並列で提示する。`reml_frontend --output human` ではこれらが 2 次情報として出力され、LSP では `relatedInformation` へ変換される。
- `structured_hints` は `hint.id`・`hint.title`・`hint.kind quick_fix|information`・`hint.actions` を持つ UI フレンドリーなヒント集合であり、`Diagnostic.hints`（メッセージ主体）と同じデータを構造化した形で保持する。CLI/LSP/AI は `structured_hints` を参照して FixIt UI や「自動修正」ボタンを生成する。
- `fixits` は `Insert` / `Replace` / `Delete` の 3 種を標準化し、`structured_hints.actions` でも同じ JSON を再利用する。`tooling/lsp/tests/client_compat/fixtures/diagnostic-v2-sample.json` では複数 FixIt を含むサンプルを用意している。
- `timestamp` は [3.4](3-4-core-numeric-time.md) の `Timestamp` を利用し、診断生成時に `Core.Numeric.now()` を呼び出す。
- `recoverability` は `core::error::Recoverability` 列挙と一致し、`fatal` / `retryable` / `report-only` 等の状態を取りうる。`CliDiagnosticEnvelope.summary.stats.diagnostic_count_by_recoverability` で集計され、AI/CI が自動ロールバックの可否を判断する。
- `AuditEnvelope` は監査情報を同梱する構造（後述）。
- `ExpectedSummary` は LSP/CLI でメッセージを国際化するための鍵と引数を保持する。期待集合を集約する `ExpectationSummary` 出力により、CLI/LSP/監査ログで同じ候補一覧を提示できるようになる。[^err001-phase25-core]
- `audit_metadata` は `AuditEnvelope.metadata` と同じキーバリューを `Diagnostic` 側に複製したものであり、CLI/LSP/AI/CI から監査情報へアクセスする際に JSON Lines をパースせず参照できる。`effects.stage.*`、`pipeline.*`、`config.migration.*` はこのフィールドと `audit.metadata` の両方に出力する。

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

#### 1.1.1 WIT 監査キー命名方針

WASM Component Model / WIT 連携に関する監査メタデータは、`AuditEnvelope.metadata` と `Diagnostic.audit_metadata` の両方に同一キーで出力する。キーは `ffi.wit.*` をプレフィックスに持ち、短い名詞を `snake_case` で連結する。

必須キー（WIT 経由の FFI 呼び出し時）:
- `ffi.wit.package`: WIT パッケージ名（例: `vendor:module`）
- `ffi.wit.world`: `world` 名
- `ffi.wit.interface`: `interface` 名
- `ffi.wit.direction`: `import` / `export`

推奨キー（型・所有権の監査用）:
- `ffi.wit.function`: 呼び出し対象の関数名
- `ffi.wit.type.kind`: `record` / `variant` / `list` / `resource` など主要型種別
- `ffi.wit.resource`: `resource` 名（対象が resource の場合のみ）

#### 1.1.2 埋め込み DSL 監査キー（草案）

埋め込み DSL の境界と発生源を追跡するため、`AuditEnvelope.metadata` と `Diagnostic.audit_metadata` に以下のキーを同一値で出力する。`dsl.id` は標準キーとして扱い、埋め込み DSL の診断では省略不可とする。

必須キー:
- `dsl.id`: 子 DSL の `dsl_id`
- `dsl.embedding.span`: 埋め込み区間の `Span`（`start`/`end` の範囲を含む）
- `dsl.embedding.mode`: `ParallelSafe` / `SequentialOnly` / `Exclusive`

推奨キー:
- `dsl.parent_id`: 親 DSL の `dsl_id`（複数階層の場合は直近の親）
- `dsl.embedding.start`: 境界開始トークン（`start` の原文）
- `dsl.embedding.end`: 境界終了トークン（`end` の原文）

`dsl.embedding.span` の JSON 形式は `Span` と同じ `{ "start": Int, "end": Int }` とする。

例:

```json
{
  "dsl.id": "reml",
  "dsl.parent_id": "markdown",
  "dsl.embedding.span": { "start": 120, "end": 240 },
  "dsl.embedding.mode": "ParallelSafe",
  "dsl.embedding.start": "```reml",
  "dsl.embedding.end": "```"
}
```
- `ffi.wit.ownership`: `own` / `borrow` / `copy`
- `ffi.wit.lift_lower`: `lift` / `lower`

不足キーがある場合は `Diagnostic.code = Some("ffi.wit.audit_missing")` を推奨し、`AuditPolicy` により警告または失敗として扱う。

### 1.2 Rust Frontend CLI 更新（Phase 2-8）

- `reml_frontend` CLI は `schema_version = "3.0.0-alpha"` を `Diagnostic.audit_metadata["schema.version"]` と `typeck/typeck-debug.*.json` の双方で宣言し、Stage/Audit の記録形式を Phase 3 以降の監査基準へ合わせる。`StageAuditPayload` から収集した `stage_trace`・`runtime_capabilities`・`bridge` メタデータは `typeck-debug` にも同梱され、`reports/spec-audit/ch1/<sample>-YYYYMMDD-typeck.json` で直接参照できる。
- AST 生成に失敗した際の型推論は `typeck.aborted.ast_unavailable`（domain: `type`, severity: `error`）として停止する。エンドユーザはこの診断を手掛かりに構文エラーや入力不備を解消し、再実行してから Stage/Audit の `used_impls`・`stage_trace` を確認する。Rust Frontend では `TypecheckDriver::infer_module(None, ..)` が常にこの診断を返し、Fallback による統計捏造を行わない。
- `typeck/typeck-debug.rust.json` は `schema_version`、`stage_trace`、`used_impls` を必須項目とし、CLI 引数 `--emit-typeck-debug <path>` を経由して `reports/spec-audit/ch1/use_nested-YYYYMMDD-typeck.json` などに保存する。`collect-iterator-audit-metrics.py --section diagnostics` はこれらキーを前提に監査 KPI を抽出し、Phase 3 の Core Diagnostics 計画で再利用する。

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

#### 1.1.3 Core.Dsl 監査イベント

`Core.Dsl.*` が発行する監査イベントは `AuditEnvelope.metadata["event.kind"]` に `dsl.*` 名前空間を設定する。埋め込み DSL 由来の場合は §1.1.2 の `dsl.id` と `dsl.embedding.*` を同時に付与し、親 DSL との相関を維持する。

| `event.kind` | 主な発火条件 | 必須メタデータ (`AuditEnvelope.metadata`) | 関連章 |
| --- | --- | --- | --- |
| `dsl.object.dispatch` | `DispatchTable` からのメソッド解決 | `"dsl.id"`, `"dsl.node"`, `"dsl.object.handle"`, `"dsl.object.class"`, `"dsl.dispatch.method"`, `"dsl.dispatch.cache"` | 3-16 §2 |
| `dsl.gc.root` | `RootScope` によるルート登録/解除 | `"dsl.id"`, `"dsl.node"`, `"dsl.gc.heap"`, `"dsl.gc.action"`, `"dsl.gc.root_id"` | 3-16 §3 |
| `dsl.actor.mailbox` | `MailboxBridge` 経由の送受信/監視 | `"dsl.id"`, `"dsl.node"`, `"dsl.actor.id"`, `"dsl.mailbox.id"`, `"dsl.mailbox.action"`, `"dsl.mailbox.depth"` | 3-16 §4 |
| `dsl.vm.execute` | `VMCore` の命令ディスパッチ | `"dsl.id"`, `"dsl.node"`, `"dsl.vm.id"`, `"dsl.vm.instruction"`, `"dsl.vm.frame.depth"`, `"dsl.vm.pc"` | 3-16 §5 |

- `dsl.node` は `ExecutionPlan.node_path`（3-6 §6.1.2）または `pipeline.node` と同一の表現を用いる。導線が無い場合は `dsl.node = "unknown"` を許容するが、監査レポートでは欠落として扱う。
- `dsl.dispatch.cache` は `hit` / `miss` / `disabled` のいずれかを `snake_case` で記録する。`dsl.gc.action` は `register` / `release` / `promote` を想定し、`dsl.mailbox.action` は `enqueue` / `dequeue` / `spawn` / `shutdown` を想定する。
- Stage/Bridge/Effect の監査メタデータが付随する場合は [3-8 §1.4](3-8-core-runtime-capability.md#audit-required-fields) の必須キーを同時に満たすこと。

### 1.2 診断ドメイン `DiagnosticDomain`

### 1.3 効果診断拡張 `effects`

効果宣言やハンドラに由来する診断では `Diagnostic.extensions["effects"]` を使用し、次の構造を格納する。

```reml
type EffectsExtension = {
  stage: Stage,                    // Experimental | Beta | Stable
  before: Set<EffectTag>,          // ハンドラ適用前の潜在効果集合
  handled: Set<EffectTag>,         // 捕捉に成功した効果集合
  residual: Set<EffectTag>,        // ハンドラ適用後に残った効果集合
  handler_name: Option<Str>,       // ハンドラ名（存在する場合）
  unhandled_operations: List<Str>, // 未実装 operation の一覧
  capability: Option<CapabilityId>,                    // 必要とされる Capability（任意）
  required_stage: Option<Runtime.StageRequirement>,    // Stage 要件（3-8 §1.2）
  actual_stage: Option<Stage>,                         // Capability Registry に登録された Stage
  capability_metadata: Option<Runtime.CapabilityDescriptor>, // `describe` から得たメタデータ
}

enum Stage = Experimental | Beta | Stable
```

`Stage` は Capability Registry（3.8 §1）と共有される列挙で、CLI/LSP は `Stage` に基づき表示レベルを調整する。`before` / `handled` / `residual` は 1.3 §I の効果計算結果に対応し、`residual = ∅` の場合は純粋化可能であることを意味する。`unhandled_operations` は `effects.handler.unhandled_operation` 診断（2.5 §B-10）で IDE へ提示する一覧として使用する。`required_stage` と `actual_stage` は Capability 要件と実際の Stage の差分を記録し、0-1 §1.2 の安全性指針に基づく是正アクションを促す基礎データとなる。`capability_metadata` には `Runtime.CapabilityDescriptor` を保持し、提供主体・効果タグ・最終検証時刻を監査ログへ転写する。

Config/Data 章の `MigrationPlan` API（3-7 §5.1）も同じ `EffectsExtension` を利用して `effect {migration}` を発火し、`AuditEnvelope.metadata["config.migration.*"]` と `Diagnostic.audit_metadata["config.migration.*"]` を同期させる。CLI やガイドでは `migration` タグの診断を `--effect-tag migration` でフィルタできる。

### 1.4 型クラス診断拡張 `typeclass`

型クラス制約の解決に関する診断では `Diagnostic.extensions["typeclass"]` に次の JSON オブジェクトを格納し、辞書渡し・モノモルフィゼーション双方の挙動を監査できるようにする。V2 形式では同じ内容をフラット化したキー（`typeclass.*`）として `extensions` / `audit_metadata` / `AuditEnvelope.metadata` にも転写する。

```json
"typeclass": {
  "trait": "Iterator",
  "type_args": ["SampleStream", "SampleItem"],
  "constraint": "Iterator<SampleStream, SampleItem>",
  "resolution_state": "stage_mismatch",
  "dictionary": {
    "kind": "none",
    "identifier": null,
    "trait": null,
    "type_args": [],
    "repr": null
  },
  "candidates": [],
  "pending": [],
  "generalized_typevars": [],
  "graph": { "export_dot": null }
}
```

| キー | 説明 |
| --- | --- |
| `trait` | 診断対象となったトレイト名（例: `Iterator`, `Eq`） |
| `type_args` | 1-2 §A/B の表記で整形した型引数のリスト |
| `constraint` | `trait<type_args...>` 形式で直列化した制約表示 |
| `resolution_state` | 解決状態を表す文字列。`resolved` / `stage_mismatch` / `unresolved` / `ambiguous` / `unresolved_typevar` / `cyclic` / `pending` を想定 |
| `dictionary` | 採用された辞書（存在しない場合は `kind = "none"` を含むプレースホルダ）。`kind`（implicit/parameter/local など）、`identifier`、`trait`、`type_args`、`repr`、`parameter_index`（必要な場合）を保持する |
| `candidates` | 曖昧性調査や統計計測に用いる候補辞書の配列。要素は `dictionary` と同じ構造 |
| `pending` | 循環検出や未解決制約の一覧（`TraitConstraintFailure` の補助情報） |
| `generalized_typevars` | 一般化・未解決の型変数を文字列表現で列挙 |
| `graph.export_dot` | 制約グラフを Graphviz DOT として出力したパスまたは `null` |

補助キーとして `typeclass.span.start` / `typeclass.span.end` をフラット化し、制約が導入されたソース位置（`Ast.span` のオフセット値）を記録する。モノモルフィゼーション経路など辞書が存在しない場合でも、`dictionary.kind = "none"` を必ず出力し、`candidates` や `resolution_state` で差分を説明する。

`resolution_state` は診断コードと 1 対 1 に対応させる想定である。例として:

[^err001-phase25-core]: Phase 2-5 ERR-001 期待集合出力整備計画（`docs/plans/bootstrap-roadmap/2-5-proposals/ERR-001-proposal.md`）S5「ドキュメントと共有タスク」（2025-11-17 完了）で `ExpectationSummary` 出力とガイド整備が完了し、`docs/plans/bootstrap-roadmap/2-5-review-log.md` に導入ログを保存。
[^err002-phase25]: Phase 2-5 ERR-002 `recover`/FixIt 情報拡張計画 Step3/Step4（`docs/plans/bootstrap-roadmap/2-5-proposals/ERR-002-proposal.md#step4-ドキュメント更新とレビュー共有week-33-day3-4`）で CLI/LSP/ストリーミング/CI の各経路を検証し、`docs/plans/bootstrap-roadmap/2-5-review-log.md#err-002-step4-ドキュメント更新とレビュー共有2025-12-15` に共有結果と Phase 2-7 への引き継ぎ事項を登録。

- `resolved`: 辞書参照が確定し、監査ログとして事後分析に利用したい場合。
- `stage_mismatch`: `typeclass.iterator.stage_mismatch` のように Capability Stage が不足している場合。
- `unresolved`: `TraitConstraintFailure` で実装が見つからなかった場合。
- `ambiguous`: `AmbiguousTraitImpl` により候補が複数あった場合。
- `unresolved_typevar`: 型変数の未解決に起因する失敗。
- `cyclic`: 制約グラフに循環が検出された場合。
- `pending`: 将来の再試行や遅延解決に回された制約（現状はプレースホルダ）。

監査ログ（`AuditEnvelope.metadata`）には上記フィールドがそのまま出力され、CI では `typeclass.metadata_pass_rate` を用いて欠落キーや状態遷移の異常を検出する。`dictionary.kind = "parameter"` の場合は `parameter_index` が含まれ、`kind = "implicit"` では `trait` と `type_args` に実際の辞書化対象が記録される。

```reml
pub enum DiagnosticDomain = {
  Syntax,
  Parser,
  Type,
  Effect,
  Runtime,
  Net,
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
- `Net` は `Core.Net` に由来する通信/URL 解析の診断を扱い、`net.*` キーと監査イベントを統一する。
- `Target` はクロスコンパイルやターゲットプロファイル整合性に関する診断を表し、本節 §7 でメッセージ定義を示す。
- `Other(Str)` は将来の拡張やユーザープロジェクト固有の分類に使用し、名前は `snake_case` 推奨とする。

> **運用メモ**: `Effect` / `Target` / `Plugin` / `Lsp` / `Other(Str)` などの語彙に対応した監査メタデータ（`extensions["plugin"]`, `extensions["lsp"]`, `extensions["capability"]` 等）を運用で揃える。

#### 1.4.1 制約グラフテレメトリ

Rust Frontend では `reml_frontend --emit-telemetry constraint_graph input.reml` を実行すると、型制約解決の様子を `TraitResolutionTelemetry` として JSON に記録する。既定の保存先は `tmp/telemetry/<入力ファイル名>-constraint_graph.json` で、`graph.nodes`／`graph.edges`／`graph.export_dot`（推奨 DOT 出力先）が含まれる。`graph.export_dot` は Graphviz へ変換した成果物をドキュメントへ添付する際の標準パスとして扱い、CI で DOT ファイルの所在を追跡できるようにする。

JSON から DOT / SVG を生成する場合は `scripts/telemetry/render_graphviz.py` を利用する。

```bash
python3 scripts/telemetry/render_graphviz.py tmp/telemetry/example-constraint_graph.json \
  --svg-out docs/spec/assets/typeclass-constraint.svg
```

`dot` コマンド（Graphviz）が利用可能な環境では SVG まで自動生成でき、`graph.export_dot` に示されたファイル名は DOT のデフォルト出力先としても再利用できる。CI や `docs/spec/3-6-core-diagnostics-audit.md` の図版更新作業では、同スクリプトの出力を用いて差分確認や可視化を行う。

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
- Phase 2-5 ERR-002 Step3/Step4 で `extensions["recover"]`・FixIt・notes の出力を仕様どおり整備し、CI 指標 `parser.recover_fixit_coverage` が 1.0 を維持している[^err002-phase25]。

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

##### OpBuilder DSL 向け診断

`builder.level(priority, :fixity, ["token"])` を利用する OpBuilder DSL では、Rust Typeck/Runtime が次の診断コードを出力する。Parser/Lexer が DSL 構文を受理した後、`TypecheckDriver::collect_opbuilder_violations` が AST を走査して矛盾を検出し、診断を CLI/LSP へ渡す。Runtime 側は `RuntimeBridgeRegistry` 同様に監査メタデータへ `bridge.stage.*` を記録する。

| `Diagnostic.code` | 既定 Severity | 発生条件 | 監査メタデータ |
| --- | --- | --- | --- |
| `core.parse.opbuilder.level_conflict` | Error | 同じ `priority` レベルに複数の fixity（`:infix_left` 等）が登録された。Typeck は AST を追跡し、最初の fixity と次の fixity を比較して衝突を報告する。 | `AuditEnvelope.metadata["parser.opbuilder.priority"] = priority`, `parser.opbuilder.fixity.existing`, `parser.opbuilder.fixity.next` |
| `core.parse.opbuilder.fixity_missing` | Error | DSL レベル宣言でトークン配列が空、`:ternary` の head/mid が不足、または文字列以外のトークンが指定された。Typeck の `validate_opbuilder_tokens` が検証し、Runtime の `OpBuilder` も同じ制約で `OpBuilderErrorKind::EmptyTokenList` 等を返す。 | `AuditEnvelope.metadata["parser.opbuilder.fixity"] = ":prefix" など`, `parser.opbuilder.reason`（`"empty_tokens"` など） |

これらの診断は `Diagnostic.domain = Parser` を既定とし、`docs/spec/2-4-op-builder.md` §F のエラー設計と整合する。`expected` フィールドは存在しないが、`notes` に「各レベルにつき 1 種類の fixity のみを指定してください」「`:ternary` には head/mid の 2 トークンが必要です」といった修正方針を記録する。CLI/LSP では `priority` と fixity シンボルを強調表示し、`OpBuilder` から生成された CLI ログ（`examples/spec_core/chapter2/op_builder/*.diagnostic.json`）と一致することを保証する。

#### 2.4.2 効果診断メッセージ (Effect Domain) {#diagnostic-effect}

> 1-3-effects-safety.md §I.5 と 3-8-core-runtime-capability.md §1.2 で定義した効果行整列・Stage/Capability 検査を `Diagnostic` と監査ログに落とし込むための共通仕様。

| `message_key` | 既定 Severity | 発生条件 | 監査メタデータ | 推奨対応 |
| --- | --- | --- | --- | --- |
| `effects.contract.stage_mismatch` | Error | `@handles` や `@requires_capability` による Stage 宣言と、Capability Registry が認証した Stage（`EffectsExtension.stage`）が一致しない。 | `AuditEnvelope.metadata` に `effect.stage.required`, `effect.stage.actual`, `effect.capability` を格納し、`effects` 拡張の `residual` を JSON として添付する。 | ハンドラ／呼び出し元に正しい `@requires_capability(stage=...)` を付与し、Stage 昇格フローと整合させる。CI では `--deny experimental` を併用して検出を強制。 |
| `effects.contract.reordered` | Warning（`Σ_after` が変化する場合は Error に昇格） | 効果ハンドラの並び替えによって `EffectsExtension.residual` が変化、捕捉対象が曖昧になる、または 1-3-effects-safety.md §I.5 の整列規約から逸脱。 | `AuditEnvelope.metadata` に `effect.order.before`, `effect.order.after`, `effect.residual.diff` を格納し、必要なら `recommendation` に最小修正案を記録する。 | 関連テストとリスク評価を添えたうえで整列規約へ戻すか、差分許容時は仕様書へ根拠を追記。CI では `--fail-on-warning` でブロックを推奨。 |
| `effects.type_row.integration_blocked` | Error | `RunConfig.extensions["effects"].type_row_mode = "ty-integrated"` が要求されたが、互換環境が `metadata-only` モード固定でビルドされているため切り替えが拒否された。 | `AuditEnvelope.metadata` に `effect.type_row.requested_mode`, `effect.type_row.available_mode`, `effect.type_row.guard_stage` を格納し、`Diagnostic.extensions["effects"]["type_row"]` に同一情報を JSON で写像する。 | 互換モードで運用する場合は `type_row_mode` を `"metadata-only"` に明示し、CI では `effect_row_guard_regressions` が 0 件であることを確認する。 |

上記診断は `DiagnosticDomain::Effect` を既定とし、`Diagnostic.extensions["effects"]` に Stage・効果集合・未処理 operation・type_row ガード情報を記録する。`AuditCapability` はこれらのメタデータを利用して Stage 昇格レビューを自動起案し、`RunConfig.extensions["effects"]` のポリシーで拒否された場合は `effects.contract.stage_mismatch` または `effects.type_row.integration_blocked` をエミットする。

監査ログに出力する最低限のキーは次の通り。

- `effect.stage.required` / `effect.stage.actual`: Stage 不一致の根拠。
- `effect.residual.diff`: ハンドラ順序変更による残余効果の差分。空集合であれば情報系ログとして扱い、Severity を Warning に留める。
- `effect.capability`: Stage チェックと紐づく Capability ID。`CapabilityRegistry::register` の記録と突き合わせて整合性を検証する。
- `effect.type_row.requested_mode` / `effect.type_row.available_mode`: `type_row_mode` 切り替え時の要求値と実際に許可された値。ガードが作動した場合は `"ty-integrated"` と `"metadata-only"` の組み合わせが記録される。
- `effect.type_row.guard_stage`: 効果行モードのガード段階（例: `"phase2-7-ty-integrated"`）。レガシーツールチェーンが `metadata-only` を強制する際は `"compatibility-metadata-only"` など環境識別子を記録する。

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

#### 2.4.3 FFI 契約診断 (Type Domain) {#diagnostic-ffi-contract}

Phase 2-3 で導入する `Core.Ffi` ブリッジ検証は、`extern` 宣言の所有権・ABI 契約がターゲットの ABI 表と一致しない場合に静的エラーを発生させる。診断は `Diagnostic.domain = Some(Type)` を既定とし、`Diagnostic.extensions["bridge"]` と `AuditEnvelope.metadata["bridge"]` に共通メタデータを出力する。必須キーは `bridge.status`, `bridge.target`, `bridge.arch`, `bridge.platform`, `bridge.abi`, `bridge.expected_abi`, `bridge.ownership`, `bridge.extern_symbol`, `bridge.return.*`（`ownership` / `status` / `wrap` / `release_handler` / `rc_adjustment`）の計 9 項目であり、監査ログでは `bridge.source_span` を追加して宣言位置を追跡する。

| `code` | 既定 Severity | 発生条件 | 監査メタデータ | 推奨対応 |
| --- | --- | --- | --- | --- |
| `ffi.contract.symbol_missing` | Error | `extern` 項目に `link_name`（または `ffi_link_name`）が指定されておらず、ランタイムが参照すべきシンボルを決定できない。 | `bridge.*` の必須キーと `bridge.source_span` を出力し、`extern_name` と `link_name` の両方を記録する。 | `#[link_name("…")]` 属性を追加し、ターゲット側シンボルと一致させる。自動生成ツールは欠落時に CI を失敗させる。 |
| `ffi.contract.ownership_mismatch` | Error | `#[ownership("…")]` が `borrowed` / `transferred` / `reference` 以外、または未指定。 | `bridge.ownership` に加え `bridge.return.*`（所有権・wrap・release_handler・rc_adjustment）を拡張フィールドに複写し、CI では `ffi_bridge.audit_pass_rate` を使用して欠落キーを検知する。 | 対応する所有権ポリシーを指定し、`Core.Ffi` の参照カウント規約と整合させる。未対応の所有権を導入する場合は 3-9 §2.4 に従い仕様を更新する。 |
| `ffi.contract.unsupported_abi` | Error | ブロックターゲット（`#[target("triple")]` or `extern { target = … }`）に対応する ABI と `#[calling_convention("…")]` が一致せず、`system_v`/`msvc`/`darwin_aapcs64` 以外の呼出規約を要求している。 | `bridge.expected_abi` を追加で出力し、Windows では `msvc`、Apple Silicon では `darwin_aapcs64` が選ばれていることを CI で検証する。 | ターゲットに合わせて `#[calling_convention]` を更新するか、独自 ABI を導入する場合はランタイム拡張と一体で RFC を提出する。 |

これらの診断は 3-9 §2.7 の ABI 表と `tooling/runtime/capabilities/*.json` に定義されたターゲットオーバーライドを参照する。監査ログは `ffi_bridge.audit_pass_rate` メトリクスで検証され、欠落キーがある場合は CI を失敗させることを推奨する。成功時のイベントは `AuditEnvelope.category = "ffi.bridge"` で記録し、`bridge.status = "ok"` / `"error"` のほか `bridge.platform`（例: `macos-arm64`）を付与することでターゲット別リグレッションを可視化する。

#### 2.4.4 Native Escape Hatches 診断と監査キー {#diagnostic-native}

`@intrinsic` や埋め込み API（`Core.Embed.*`）に由来する診断は `Diagnostic.domain = Some(Type)` または `Some(Runtime)` を既定とし、監査ログには `native.*` 名前空間のキーを必ず記録する。これらのキーは `AuditEnvelope.metadata` と `Diagnostic.audit_metadata` の双方に同一値で出力する。

| `Diagnostic.code` | 既定 Severity | 発生条件 | 監査メタデータ |
| --- | --- | --- | --- |
| `native.intrinsic.invalid_type` | Error | `@intrinsic` が許容されない型（非 `Copy` / 未定義 ABI 型など）を含む。 | `native.intrinsic.invalid_type = true`, `intrinsic.name`, `intrinsic.signature` |
| `native.intrinsic.signature_mismatch` | Error | intrinsic 名と関数シグネチャが LLVM マッピングと一致しない。 | `native.intrinsic.signature_mismatch = true`, `intrinsic.name`, `intrinsic.signature` |
| `native.embed.abi_mismatch` | Error | 埋め込み ABI の要求バージョンと実行環境が一致しない。 | `native.embed.abi_mismatch = true`, `embed.abi.version` |
| `native.embed.unsupported_target` | Error | 埋め込み API が未対応ターゲットで呼び出された。 | `native.embed.unsupported_target = true`, `embed.abi.version` |
| `native.inline_asm.disabled` | Error | `feature = "native-unstable"` が無効、または Capability が無効な状態で Inline ASM を利用した。 | `native.inline_asm.disabled = true`, `asm.template_hash`, `asm.constraints` |
| `native.inline_asm.invalid_constraint` | Error | 制約文字列が LLVM のルールと一致しない。 | `native.inline_asm.invalid_constraint = true`, `asm.constraints` |
| `native.llvm_ir.verify_failed` | Error | LLVM IR 検証で失敗した。 | `native.llvm_ir.verify_failed = true`, `llvm_ir.template_hash`, `llvm_ir.inputs` |
| `native.llvm_ir.invalid_placeholder` | Error | `$0` などのプレースホルダが `inputs` と一致しない。 | `native.llvm_ir.invalid_placeholder = true`, `llvm_ir.template_hash`, `llvm_ir.inputs` |

`native.intrinsic.used` / `native.embed.entrypoint` / `native.inline_asm.used` / `native.llvm_ir.used` は **監査イベント相当のキー**として扱い、成功時にも必ず記録する。`Diagnostic.code` を伴わない場合でも監査ログに残し、呼び出し経路と対象を追跡できるようにする。

| 監査キー | 値の型 | 生成規則 | 記録粒度 |
| --- | --- | --- | --- |
| `native.intrinsic.used` | `Json.Bool` | intrinsic を IR へマッピングした時点で `true` を記録する。 | 関数単位 |
| `native.embed.entrypoint` | `Json.String` | 埋め込み API のエントリ（例: `reml_create_context`）名。 | エントリポイント単位 |
| `native.inline_asm.used` | `Json.Bool` | Inline ASM が IR へ変換された時点で `true` を記録する。 | 関数単位 |
| `native.llvm_ir.used` | `Json.Bool` | LLVM IR テンプレートが IR に組み込まれた時点で `true` を記録する。 | 関数単位 |
| `intrinsic.name` | `Json.String` | `@intrinsic("...")` の文字列値を正規化して格納する。 | 関数単位 |
| `intrinsic.signature` | `Json.String` | Reml の関数シグネチャを ABI 正規化形式で格納する。 | 関数単位 |
| `embed.abi.version` | `Json.String` | 埋め込み ABI のバージョン（例: `"0.1"`）。 | エントリポイント単位 |
| `asm.template_hash` | `Json.String` | Inline ASM テンプレートのハッシュ（改行正規化後）。 | 関数単位 |
| `asm.constraints` | `Json.Array` | 制約文字列（順序維持）の配列。 | 関数単位 |
| `llvm_ir.template_hash` | `Json.String` | LLVM IR テンプレートのハッシュ（改行正規化後）。 | 関数単位 |
| `llvm_ir.inputs` | `Json.Array` | `inputs(...)` に与えた式の型情報または表示名。 | 関数単位 |

`native.intrinsic.used` / `native.embed.entrypoint` は監査ダッシュボードで **呼び出し数と対象ターゲット**を集計するための基準キーとし、`AuditEnvelope.metadata["target.*"]` と組み合わせて Phase 4 の KPI に反映する。

CLI/LSP の `Diagnostic.extensions["effects"]["iterator"]` へも同じキー集合を転写し、人間向け出力と監査ログが同じ語彙で比較できるようにする。CI メトリクス `iterator.stage.audit_pass_rate` はこれらのキーが揃っていることを前提に算出され、欠落時は `AuditPolicy` が `Warning` を昇格させる。

これらのキーは `AuditPolicy.exclude_patterns` で除外しない限り永続化され、`CapabilityAudit` レポートや LSP の効果ビューで差分分析に利用できる。

#### 2.4.5 ネットワーク診断プリセット (Net Domain) {#diagnostic-net}

`Core.Net` の診断は `Diagnostic.domain = Some(DiagnosticDomain::Net)` を既定とし、`AuditEnvelope.metadata` に `net.*` を必ず記録する。`event.kind` には `net.*` を設定し、HTTP/TCP/UDP のイベントを共通の形式で追跡できるようにする。

| `event.kind` | 必須メタデータ | 補足 |
| --- | --- | --- |
| `net.http.request` | `net.url`, `net.method`, `net.request_bytes`, `net.elapsed_ms` | 送信開始または完了時に記録する。 |
| `net.http.response` | `net.url`, `net.status`, `net.response_bytes`, `net.elapsed_ms` | 受信完了時に記録する。 |
| `net.tcp.connect` | `net.url`, `net.elapsed_ms` | 接続完了時に記録する。 |
| `net.tcp.listen` | `net.url`, `net.listen_port` | リスナー確立時に記録する。 |
| `net.udp.bind` | `net.url`, `net.listen_port` | バインド完了時に記録する。 |
| `net.udp.send` | `net.peer`, `net.request_bytes`, `net.elapsed_ms` | 送信完了時に記録する。 |

| 診断キー | 既定 Severity | 発生条件 | 必須メタデータ |
| --- | --- | --- | --- |
| `net.http.timeout` | Error | HTTP 送受信がタイムアウトした | `net.url`, `net.elapsed_ms`, `net.method` |
| `net.http.connection_failed` | Error | HTTP 接続に失敗した | `net.url`, `net.elapsed_ms`, `net.method` |
| `net.tcp.connect_refused` | Error | TCP 接続が拒否された | `net.url`, `net.elapsed_ms` |
| `net.tcp.timeout` | Error | TCP/UDP の送受信がタイムアウトした | `net.url`, `net.elapsed_ms` |
| `net.dns.failure` | Error | DNS 解決に失敗した | `net.url`, `net.elapsed_ms` |
| `net.url.invalid_scheme` | Error | URL スキームが不正 | `net.url` |

`AuditEnvelope.metadata` と `Diagnostic.audit_metadata` には `net.url`, `net.method`, `net.status`, `net.request_bytes`, `net.response_bytes`, `net.elapsed_ms`, `net.peer`, `net.listen_port` を該当ケースで必須とし、値が存在しない場合は `net.audit.missing_metadata` を `Warning` で発行する。

#### 2.4.6 Stage 差分プリセット `EffectDiagnostic` {#effect-diagnostic-stage}

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

### 2.6 Core Prelude ガード診断 {#diagnostic-core-prelude}

`Core.Prelude` の `ensure` / `ensure_not_null`（[3-1-core-prelude-iteration.md](3-1-core-prelude-iteration.md) §2.2）の呼び出しが `Err` を返した場合、ランタイムは `core.prelude.ensure_failed` 診断を構築して監査へ報告する。これらの API は `@pure` な `Result` を返し、呼び出し側が `?` で早期復帰するだけで診断記録を残せるため、例外禁止ポリシー（0-1 §1.2）と監査要件（0-1 §2.2）を同時に満たすための標準パターンである。

| `Diagnostic.code` | 既定 Severity | 発生条件 | 監査メタデータ | 推奨対応 |
| --- | --- | --- | --- | --- |
| `core.prelude.ensure_failed` | Error（`Stage=Experimental` の場合は Warning へダウングレード可） | `ensure` の条件式が偽、または `ensure_not_null` へ `None` が渡された | `core.prelude.guard.kind` / `core.prelude.guard.trigger` / `core.prelude.guard.pointer_class` / `core.prelude.guard.stage` / `core.prelude.guard.module` | 条件式を満たすよう入力を正す、もしくは guard 自体を Stage 要件に合わせて書き換える。FFI 由来の `None` は `RuntimeCapability::FfiBridge` の契約を再確認する。 |

`core.prelude.ensure_failed` の診断は `Diagnostic.domain = DiagnosticDomain::Runtime` を既定とし、`Diagnostic.extensions["prelude.guard"]` に次の構造体を保存する。

```reml
type PreludeGuardExtension = {
  kind: PreludeGuardKind,        // ensure | ensure_not_null
  trigger: Str,                  // 失敗した条件式や識別子
  pointer_class: Option<Str>,    // ffi | plugin | core など
  stage: Option<Stage>,          // Stage Requirement が紐付く場合
  module_path: Option<Str>,      // `Core.Prelude.ensure` の呼び出し元モジュール
}

enum PreludeGuardKind = Ensure | EnsureNotNull
```

監査ログでは同じ情報を `AuditEnvelope.metadata` の `core.prelude.guard.*` キーとして必須保存する。キーごとの規約は以下の通り。

| 監査キー | 値の型 | 生成規則 |
| --- | --- | --- |
| `core.prelude.guard.kind` | `Json.String` | `PreludeGuardExtension.kind` を `snake_case` 化（`ensure`, `ensure_not_null`）。 |
| `core.prelude.guard.trigger` | `Json.String` | 失敗した条件式・識別子・ポインタ名。`ensure(cond, ..)` は `cond` を文字列化し、`ensure_not_null(ptr, ..)` は `ptr` 名と呼び出し元を `::` で連結する。 |
| `core.prelude.guard.pointer_class` | `Json.String` / `Json.Null` | `None` でなければ `ffi`/`plugin`/`core` などの分類を格納。FFI で取得したポインタの場合は `ffi` を必須化し、`tooling/ci/collect-iterator-audit-metrics.py --section prelude-guard` が統計に利用する。 |
| `core.prelude.guard.stage` | `Json.String` / `Json.Null` | `ensure` が Stage 要件（例: `StageRequirement::AtLeast(Beta)`) と共に利用された場合、その Stage 名を記録。条件が Stage 非依存なら `Null`。 |
| `core.prelude.guard.module` | `Json.String` | `ModulePath::display()` の結果。スクリプト／DSL で guard を使う場合は仮想モジュール名（例: `dsl.examples.ensure`）を与える。 |

`scripts/validate-diagnostic-json.sh` は上記キーの存在を検証し、欠落時は CI を失敗させる。Nightly CI の `tooling/ci/collect-iterator-audit-metrics.py --section prelude-guard --require-success` は `core_prelude.guard.failures` と `core_prelude.guard.ensure_not_null` カウンタを集計し、`reports/spec-audit/ch0/links.md` に JSON 結果をリンクさせることで `Phase 3 M1` の KPI（`core_prelude.guard` セクション）を監査する。

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

### 3.4 型推論監査成果物（W3 拡張）

P1 W3 で導入した型推論監査では、診断ログに加えて以下の成果物を `--dualwrite-root`（既定: `reports/type-inference/`）へ保存し、CI とローカル検証の双方で共有する。

| 成果物 | 内容 | 生成元 | 用途 |
| --- | --- | --- | --- |
| `typeck/typed-ast.json` | 型推論後 AST（`TyId` と `StageRequirement` を含む） | `remlc --emit typed-ast` | 型推論出力の受入判定 |
| `typeck/constraints.json` | `Scheme`/`ConstraintSet` のシリアライズ | `remlc --emit constraints` | 制約解決順序の比較 |
| `typeck/impl-registry.json` | 型クラス辞書（Impl Registry）のスナップショット | `remlc --emit impl-registry` | determinism（行順・キー一致）の検証 |
| `typeck/effects-metrics.json` | 効果監査メトリクスの集計結果 | 監査メトリクス集計ツール | 効果監査 KPI（下表）を 0.5pt 以内に維持 |
| `typeck/typeck-debug.json` | `Type_inference_effect` / `Constraint_solver` の詳細ログ (`effect_scope`, `residual_effects`, `recoverable` 等) | `remlc --emit typeck-debug <dir>` | `effects.stage_mismatch.*` 診断との突合、Recover 設定の検証 |
| `typeck/diagnostic-validate.log` | `scripts/validate-diagnostic-json.sh` の結果 | `scripts/validate-diagnostic-json.sh` | JSON Schema 検証ログの監査証跡 |

成果物のディレクトリ構造と更新手順は `reports/type-inference/` 配下で整理し、CI も同じ命名規約に従う。

#### 3.4.1 `effects` セクションの必須メトリクス

`effects-metrics.json` には次のキーを出力し、各メトリクスを ±0.5pt 以内のばらつきに抑える。

| メトリクスキー | 説明 | 合格条件 |
| --- | --- | --- |
| `effects.unify.match` / `effects.unify.delta` | 単一化（`Constraint_solver.unify`）の一致率と差分件数 | `effects.unify.match = 1.0` かつ `|delta| ≤ 0.5` |
| `effects.impl_resolve.match` / `effects.impl_resolve.delta` | Impl Registry 解決結果の一致率と差分件数 | `effects.impl_resolve.match = 1.0`、`effects.impl_resolve.delta = 0` |
| `effects.stage_mismatch.match` / `effects.stage_mismatch.delta` | `StageRequirement` 判定の一致率と Stage 差分件数 (`Type_inference_effect.effect_scope`) | `effects.stage_mismatch.match = 1.0`、`effects.stage_mismatch.delta = 0` |

`typeck-debug.json` の `effect_scope`, `residual_effects`, `recoverable` フィールドは上記メトリクスの根拠データであり、`effects.stage_mismatch.delta > 0` の場合は `Diagnostic.extensions["effects"]` と照らして差分内容を追跡する。CLI から `--recover-disable` を指定した場合でも `effects.impl_resolve.match` と `effects.stage_mismatch.match` は 1.0 を維持する必要がある。

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
| `wrapper` | Str | Optional | `ffi.wrap` 経由の場合は `"ffi.wrap"` を設定し、低レベル呼び出しと区別する | 3-9 §2.4.1 |

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
- `wrapper = "ffi.wrap"` の場合は §5.1.1 の `ffi.wrapper` メタデータを必須とし、未設定の場合は `ffi.wrap.audit_missing` 診断を `Warning` で記録する。

#### 5.1.1 FFI ラッパ監査メタデータ

`AuditEnvelope.metadata["ffi.wrapper"]` に次の JSON オブジェクトを格納する。

| フィールド | 型 | 必須 | 説明 | 参照 |
| --- | --- | --- | --- | --- |
| `name` | Str | Required | ラッパー名（例: `"libm.cos"`） | 3-9 §2.4.1 |
| `null_check` | Bool | Required | 戻り値の NULL 検査を有効化したか | 3-9 §2.4.1 |
| `ownership` | Str | Optional | `Ownership` を文字列化した値。未指定時は省略 | 3-9 §2.6 |
| `error_map` | Str | Optional | `FfiWrapSpec.error_map` を識別するキー | 3-9 §2.4.1 |
| `call_mode` | Str | Required | `wrapped` / `raw` のいずれか。`wrap` 生成物では `wrapped` | 3-9 §2.4.1 |

- `ffi.wrap` が引数数や型に失敗した場合は `Diagnostic.code = Some("ffi.wrap.invalid_argument")` を使用し、`extensions["ffi.wrap"].expected_signature` に `FfiFnSig` をシリアライズした値を格納する。
- `null_check = true` で NULL が返った場合は `Diagnostic.code = Some("ffi.wrap.null_return")` を設定し、`extensions["ffi.wrap"].symbol` と `extensions["ffi.wrap"].ownership` を含める。
- `ownership` の前提が満たされない場合は `Diagnostic.code = Some("ffi.wrap.ownership_violation")` とし、監査ログへ `ffi.wrapper.ownership` と `ffi.wrapper.call_mode` を複写する。

### 5.2 FFI ビルド・生成監査テンプレート {#ffi-ビルド生成監査テンプレート}

> 目的：`reml build` が行う FFI 生成・リンクを監査可能にし、入力ハッシュと生成物キャッシュの再現性を保証する。

`AuditEnvelope.metadata["ffi.bindgen"]` と `AuditEnvelope.metadata["ffi.build"]` に次の JSON オブジェクトを格納する。

#### 5.2.1 `ffi.bindgen` 監査メタデータ

| フィールド | 型 | 必須 | 説明 | 参照 |
| --- | --- | --- | --- | --- |
| `event` | Str | Required | 常に `"ffi.bindgen"` を設定 | 3-9 §2.10 |
| `status` | Str | Required | `success` / `failed` / `cache_hit` / `skipped` | 3-9 §2.10.3 |
| `input_hash` | Str | Required | `headers`/`bindgen.config`/`TargetProfile`/`reml-bindgen` バージョンを正規化したハッシュ | 3-9 §2.10.3 |
| `manifest_path` | Str | Optional | `reml.json` の相対パス | 3-9 §2.10 |
| `headers` | List<Str> | Optional | 実際に解決されたヘッダパス。`cache_hit` の場合は省略可 | 3-9 §2.10.1 |
| `config_path` | Str | Optional | `reml-bindgen.toml` のパス | 3-9 §2.8 |
| `output_path` | Str | Optional | 出力 `.reml` ファイルのパス | 3-9 §2.10.1 |
| `cache_path` | Str | Optional | キャッシュ格納先ディレクトリ | 3-9 §2.10.3 |
| `duration_ms` | u64 | Optional | 実行に要した時間（ミリ秒） | 3-9 §2.10 |
| `tool_version` | Str | Optional | `reml-bindgen` のバージョン | 3-9 §2.10.3 |
| `error.code` | Str | Optional | 失敗時の診断コード（例: `ffi.bindgen.generate_failed`） | 3-9 §2.10.3 |
| `error.message` | Str | Optional | 失敗時のメッセージ | 3-9 §2.10.3 |

#### 5.2.2 `ffi.build` 監査メタデータ

| フィールド | 型 | 必須 | 説明 | 参照 |
| --- | --- | --- | --- | --- |
| `event` | Str | Required | 常に `"ffi.build"` を設定 | 3-9 §2.10 |
| `status` | Str | Required | `success` / `failed` | 3-9 §2.10.3 |
| `input_hash` | Str | Required | `libraries`/`linker`/`TargetProfile` を正規化したハッシュ | 3-9 §2.10.3 |
| `manifest_path` | Str | Optional | `reml.json` の相対パス | 3-9 §2.10 |
| `libraries` | List<Str> | Optional | 解決対象のライブラリ名 | 3-9 §2.10.1 |
| `linker_paths` | List<Str> | Optional | `linker.search_paths` の解決結果 | 3-9 §2.10.1 |
| `frameworks` | List<Str> | Optional | `linker.frameworks` の解決結果 | 3-9 §2.10.1 |
| `extra_args` | List<Str> | Optional | `linker.extra_args` の最終値 | 3-9 §2.10.1 |
| `duration_ms` | u64 | Optional | リンクに要した時間（ミリ秒） | 3-9 §2.10 |

```json
{
  "ffi.bindgen": {
    "event": "ffi.bindgen",
    "status": "cache_hit",
    "input_hash": "b3a1c9d4b65f1f27",
    "manifest_path": "reml.json",
    "output_path": "generated/openssl.reml",
    "cache_path": ".reml/cache/ffi/b3a1c9d4b65f1f27",
    "tool_version": "0.4.2"
  }
}
```

```json
{
  "ffi.build": {
    "event": "ffi.build",
    "status": "failed",
    "input_hash": "9a77e14271d0e8aa",
    "manifest_path": "reml.json",
    "libraries": ["ssl", "crypto"],
    "linker_paths": ["/usr/lib", "/opt/local/lib"],
    "frameworks": [],
    "extra_args": ["-Wl,-rpath,/opt/local/lib"],
    "duration_ms": 310
  }
}
```

- `input_hash` は `ffi.bindgen` / `ffi.build` ともに必須であり、`Diagnostic.extensions["ffi.build"].input_hash` と同じ値を保存する。
- `ffi.bindgen` が `cache_hit` の場合でも監査イベントは出力し、`headers` は省略可能とする。
- `Diagnostic.code` は以下を既定とする。
  - `ffi.build.config_invalid` / `ffi.build.link_failed` / `ffi.build.path_outside_project` / `ffi.build.framework_unsupported`
  - `ffi.bindgen.config_invalid` / `ffi.bindgen.generate_failed` / `ffi.bindgen.output_overwrite`

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
  execution_plan: Option<ExecutionPlanDigest>,
  resource_limits: List<ResourceLimitDigest>,
  monitoring_digest: MonitoringDigest,
  channel_links: List<ChannelLinkDigest>,
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

- `conductor_id` と `node_id` は診断対象の DSL ノードを一意に示し、LSP/CLI は `depends_on` と `channel_links` を利用して依存グラフをハイライトする。
- `ExecutionPlanDigest` は 3-9 §1.4 の構成要素を縮約し、`BackpressureWindow` が `None` の場合でも `ExecutionPlan.strategy` を表示する。値は `ExecutionMetricsScope.execution_plan` から自動取得され、閾値不正（例: `high_watermark <= low_watermark`）は `ConductorIssueKind::ExecutionPlan` を設定する。
- `ResourceLimitDigest.memory` と `ResourceLimitDigest.cpu` は 3.5 §9 の `MemoryLimitResolved` / `CpuQuotaNormalized` を縮約して格納し、値は `ExecutionMetricsScope.resolved_limits()` から自動転写される。CLI/LSP は `hard_bytes` と `scheduler_slots` を用いて 0-1 §1.1 の性能要件を再検証し、設定値が Stage や Capability の制約を満たしているか確認する。
- `MonitoringDigest.metrics` には §6.1 の既定メトリクスを含め、利用者が任意に追加したキーも保持する。`TracingDigest.mode = Conditional` は `trigger` に `@cfg` 条件や `RunConfig.trace_enabled` を記録する。
- `AuditReference` は §3 の監査ログと結合するためのメタデータで、`events` に `AuditEvent::PipelineStarted` などのイベント名を列挙する。`audit_id` が `None` の場合は監査連携されていない診断であると見なす。

| AuditEnvelope.metadata キー | `ConductorDiagnosticExtension` の対応フィールド | 用途 |
| --- | --- | --- |
| `conductor.id` | `conductor_id` | 監査レポートで同一 Conductor の診断を集約する |
| `conductor.node` | `node_id` | ノード単位でのレビュー（例: `transform`） |
| `conductor.capabilities` | `capabilities` | Stage/Capability レビュー (0-1 §1.2 準拠) |
| `conductor.execution` | `execution_plan` | Backpressure/スケジューリング比較 |
| `conductor.resource_limits` | `resource_limits` | リソース制限の追跡と逸脱検出 |
| `conductor.monitoring.metrics` | `monitoring_digest.metrics` | CLI/LSP のメトリクス表示 |
| `conductor.channels` | `channel_links` | チャネル ID とバッファ設定の参照 |

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
| `config.project.version_invalid` | `Error` | `project.version` が SemVer として解析できない | `extensions["config"]` に `manifest_version`, `schema_name`, `version_mismatch="parse_error"` を付与し、`config.version_reason="parse_error"` とともに監査ログへ記録する。Manifest を修正しない限りビルドを停止する。 |
| `config.schema.version_incompatible` | `Error` | Schema の `(major, minor, patch)` がマニフェストより新しい、または major が一致しない | `schema_version`, `schema_name`, `version_mismatch ∈ {"major","schema_ahead"}` と `config.version_reason` を付与し、`MIGRATION-BLOCKER-*` 登録の根拠とする。CI では互換性が解消されるまでマージを禁止する。 |
| `config.feature.mismatch` | `Error`（`missing_in_target` 有り）、それ以外は `Warning` | `feature_guard`, `RunConfigTarget.features`, `RunConfigTarget.feature_requirements` のいずれかに差異がある | CLI/LSP は差集合を提示し、`--fix` で `feature_guard` 同期を提案。`missing_in_target` が発生した場合はビルド停止。 |
| `config.compat.trailing_comma` | `Error` | `ConfigCompatibility.trailing_comma = Forbid` なのに JSON/TOML 入力の末尾に余分なカンマが存在する | `docs/spec/3-7-core-config-data.md` §1.5.2。CLI/LSP は `config.compatibility.violation = "trailing_comma"` を拡張メタとして提示し、`--compat relaxed` で上書きした場合は `AuditEvent::ConfigCompatChanged` を記録する。 |
| `config.compat.unquoted_key` | `Error` | `KeyPolicy::Forbid` もしくは `AllowAlpha` より厳格な設定時に bare key が検出された | `ConfigCompatibility::unquoted_key` の閾値を表示し、`key_path` をハイライト。Manifest/Env 指定のどちらから緩和されたかを `config.source` で追跡する。 |
| `config.compat.duplicate_key` | `Warning`（`CollectAll` 時）/`Error`（`Error`/`LastWriteWins` 時） | 同一テーブル/オブジェクトでキーが重複し、互換ポリシーが許容していない | 3-7 §1.5.1。`DuplicateKeyPolicy::CollectAll` では `config.diff.changed` に衝突一覧を出力し、レビューで解消させる。 |
| `config.compat.number_format` | `Error` | `NumberCompatibility::Strict` で 16 進浮動小数や `+1` のようなフォーマットが入力された | 0-1 §1.2 の安全性に従い Stage::Stable では拒否する。Stage::Experimental では `Warning` 化し、`config.compatibility.stage` に `experimental` を設定して監査ログへ送る。 |

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
| `bridge.stage.backpressure` | Error | ストリーミングランナーが `PendingReason::Backpressure` を報告した際に、ブリッジ Stage が要求値を満たさない。 | `extensions["bridge"].stage_required`, `stage_actual`, `extensions["bridge"].signal.kind = "pending"`, `AuditEnvelope.metadata["bridge.stream.signal"]` | Stage の昇格または `RuntimeBridgeDescriptor.stage` の更新を行い、`effects.contract.stage_mismatch` と合わせて CI の KPI（`parser.stream.bridge_backpressure_diagnostics`）が 1.0 になることを確認する。 |
| `bridge.target.mismatch` | Error | `RuntimeBridgeDescriptor.target_profiles` と `RunConfig.extensions["target"].profile_id` が不一致。 | `target_requested`, `target_detected`, `AuditEnvelope.metadata["target.profile.requested"]` | ターゲットプロファイルの設定を見直し、互換プロファイルで再登録する。 |
| `bridge.audit.missing_event` | Error | `RuntimeBridgeAuditSpec.mandatory_events` に列挙したイベントが監査ログに存在しない。 | `checklist_missing`, `AuditEnvelope.metadata["bridge.missing_events"]` | 監査ログで `audit.log("bridge.*", …)` を再実行し、`requires_audit_effect = true` を満たす。 |
| `bridge.diff.invalid` | Error | `RuntimeBridgeReloadSpec.diff_format` に合わない差分がホットリロードへ渡された。 | `AuditEnvelope.metadata["bridge.diff.expected"]`, `"bridge.diff.received"` | `Config.compare`（3-7 §4.2）で生成した差分形式を用い、形式不一致時はロールバックを実行する。 |

- すべての `bridge.*` 診断は `Diagnostic.domain = DiagnosticDomain::Runtime` を既定とし、`AuditEnvelope.metadata["bridge.id"] = extensions["bridge"].id` を必須とする。
- `RuntimeCapability::ExternalBridge(id)` が Stage 不整合で無効化された場合は `bridge.contract.violation` が発生し、同時に `PlatformInfo.runtime_capabilities` から該当 ID を除外する。
- `bridge.stage.backpressure` は `effects.contract.stage_mismatch` と同時に収集され、`collect-iterator-audit-metrics.py --section streaming` の `parser.stream.bridge_backpressure_diagnostics` / `parser.stream.bridge_stage_propagation` を 1.0 に保つことで Stage 逸脱を早期検知できる。
- CI で実験段階ブリッジを禁止する際は `--deny experimental` を指定し、`bridge.stage.experimental` を検出した時点で失敗させる運用を推奨する。

## 9. 使用例（CLI エラー報告）

### 9.1 `pipeline_*` サンプルと CLI/監査ログ

Chapter 3 で定義した `AuditEvent` と `CliDiagnosticEnvelope` の挙動を検証するために、`examples/core_diagnostics/pipeline_success.reml` / `pipeline_branch.reml` を `reml_frontend` で実行する。Rust 実装の CLI は `--output json` と `--emit-audit-log` を同時に指定することで診断（stdout）と監査（stderr, NDJSON）を1回の実行で収集できる。

```reml
// examples/core_diagnostics/pipeline_branch.reml
use Core;

fn choose(flag: Bool) -> Int =
  if flag then 1 else -1

fn pipeline_branch(flag: Bool) -> Int =
  let base = 10;
  base + choose(flag)

fn main() -> Int = pipeline_branch(true)
```

- `tooling/examples/run_examples.sh --suite core_diagnostics --with-audit --update-golden` を実行すると、`examples/core_diagnostics/*.expected.diagnostic.json` と `*.expected.audit.jsonl` が再生成される。
- CLI の標準出力は `CliDiagnosticEnvelope`（NDJSON 1 行）であり、`python -m json.tool` などで整形した内容をゴールデンとして保存する。Rust 実装では `summary.stats.parse_result` や `stream_meta` のような補助メトリクスも同じ JSON に含まれる。
- 監査ログは `AuditEmitter::stderr(true)` が `pipeline_started` / `pipeline_completed` などのイベントを 1 行ずつ JSON で出力する。`*.expected.audit.jsonl` では JSON Lines 形式を維持し、テキスト比較で差分を確認する。

#### 診断出力の例（`pipeline_branch.expected.diagnostic.json` 抜粋）

```jsonc
{
  "schema_version": "3.0.0-alpha",
  "command": "Check",
  "phase": "Reporting",
  "run_id": "2d3b5d70-a4c2-4a5e-93c2-fc1ec51f93bf",
  "diagnostics": [],
  "summary": {
    "inputs": [
      "examples/core_diagnostics/pipeline_branch.reml"
    ],
    "started_at": "2025-07-01T10:32:10Z",
    "finished_at": "2025-07-01T10:32:10Z",
    "artifact": null,
    "stats": {
      "cli_command": "target/debug/reml_frontend --output json --emit-audit-log examples/core_diagnostics/pipeline_branch.reml",
      "diagnostic_count": 0,
      "parse_result": {
        "recovered": false,
        "farthest_error_offset": null
      },
      "run_config": {
        "packrat": true,
        "trace": false,
        "effects": {
          "type_row_mode": "ty-integrated"
        },
        "lex": {
          "identifier_profile": "unicode",
          "profile": "strict_json"
        }
      },
      "stream_meta": {
        "packrat_enabled": true,
        "bridge": null
      }
    }
  },
  "exit_code": {
    "label": "success",
    "value": 0
  }
}
```

#### 監査ログの例（`pipeline_success.expected.audit.jsonl` 抜粋）

```jsonc
{"timestamp":"2025-07-01T10:31:55Z","envelope":{"audit_id":"b36c299e-5938-4d72-90bd-6b7dc2fcf7e2","capability":"core.diagnostics","metadata":{"audit.channel":"cli","audit.policy.version":"rust.poc.audit.v1","cli.command":"Check","cli.command_line":"target/debug/reml_frontend --output json --emit-audit-log examples/core_diagnostics/pipeline_success.reml","cli.input":"examples/core_diagnostics/pipeline_success.reml","cli.phase":"Reporting","cli.program":"target/debug/reml_frontend","cli.run_id":"e42f7a4d-c933-4a97-ab6d-2820d1d676d7","event.kind":"pipeline_started","pipeline.dsl_id":"pipeline_success.reml","pipeline.id":"dsl://examples/core_diagnostics/pipeline_success.reml","pipeline.node":"pipeline_success","schema.version":"3.0.0-alpha"}}
{"timestamp":"2025-07-01T10:31:55Z","envelope":{"audit_id":"b36c299e-5938-4d72-90bd-6b7dc2fcf7e2","capability":"core.diagnostics","metadata":{"audit.channel":"cli","audit.policy.version":"rust.poc.audit.v1","cli.command":"Check","cli.command_line":"target/debug/reml_frontend --output json --emit-audit-log examples/core_diagnostics/pipeline_success.reml","cli.input":"examples/core_diagnostics/pipeline_success.reml","cli.phase":"Reporting","cli.program":"target/debug/reml_frontend","cli.run_id":"e42f7a4d-c933-4a97-ab6d-2820d1d676d7","event.kind":"pipeline_completed","pipeline.dsl_id":"pipeline_success.reml","pipeline.id":"dsl://examples/core_diagnostics/pipeline_success.reml","pipeline.node":"pipeline_success","pipeline.outcome":"success","pipeline.count":1,"schema.version":"3.0.0-alpha"}}}
```

Pipeline 成功時は `diagnostics=[]` で CLI が正常終了し、`pipeline_completed` イベントが `pipeline.outcome=success` を記録する。一方で `pipeline_branch` は `effects.contract.stage_mismatch` を再現するために非ゼロ終了し `pipeline.exit_code=failure` を残すが、`AuditEnvelope.metadata.pipeline.node` や `pipeline.outcome` のキーは成功ケースと同じ形式で埋められるため、CI で `pipeline.*` キーの欠落を検知できる。

ここで示した JSON には `schema_version = "3.0.0-alpha"`・`run_config.lex.identifier_profile`・`stream_meta.packrat_enabled` が含まれており、`tooling/examples/run_examples.sh --suite core_diagnostics --update-golden` を実行するとこれらの値がゴールデンファイルへ自動的に反映される。LSP/AI/CI は `run_config.lex` から字句プロファイルを、`stream_meta` から Packrat/Streaming の実際の実行パスと Bridge 有無を読み取れるため、`collect-iterator-audit-metrics.py --section streaming` の KPI と整合させやすい。

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

### 11.1 監査ログ出力フラグと永続ストア {#cli-audit-flags}

- `--emit-audit`: 監査ログ（`AuditEnvelope` の JSON Lines）を生成する恒久フラグ。Phase 2-3 では暫定扱いだったが、Phase 2-4 以降は既定で有効とし、無効化する場合のみ `--emit-audit=off` を指定する。
- `--audit-store=<profile>`: 出力先と保持ポリシーを切り替える。`ci` は `reports/audit/<target>/<YYYY>/<MM>/<DD>/` へ永続化し、`local` は `tooling/audit-store/local/<timestamp>/`、`tmp` は互換目的で `tmp/cli-callconv-out/<target>/` を用いる。各プロファイルは `AuditEnvelope.build_id = "<utc timestamp>-<commit sha>"` の命名規約を共有する。
- `--audit-dir=<path>`: プロファイル既定のルートを上書きしつつ命名規約を維持する。相対パスは CLI 実行ディレクトリを基準に解決する。
- `--audit-level={summary,full,debug}`: 書き出すメタデータ量を制御する。`summary` は必須キー（`command`, `phase`, `run_id`, `bridge.*`, `effect.stage.*` 等）のみに制限し、`full` は Phase 2-3 で合意された監査フィールドをすべて含み、`debug` は `extensions.*` を含む完全ログを生成する。
- `--audit-store=ci` 時は CI 成功で最新 20 件を `reports/audit/history/<target>.jsonl.gz` として圧縮保存し、失敗時は `reports/audit/failed/<commit>/` へ退避する。削除したビルド ID は `reports/audit/index.json` の `pruned` 配列へ追記する。
- 永続ストアは `reports/audit/index.json` および `reports/audit/usage.csv` を更新し、総容量が 500 MB を超えた場合は `0-4-risk-handling.md` に記録する。`tooling/ci/collect-iterator-audit-metrics.py` はこれらのメタデータを前提に `ffi_bridge.audit_pass_rate` と `iterator.stage.audit_pass_rate` を集計する。

本節の CLI フラグ仕様は `docs/plans/bootstrap-roadmap/2-4-diagnostics-audit-pipeline.md` と `docs/guides/tooling/audit-metrics.md` に記載された運用手順と同期させ、計画書と仕様の齟齬を防止する。

## 12. Streaming Recover メトリクス

Streaming ランナーの Recover 収束状況は `parser.runconfig.extensions["stream"]` と `AuditEnvelope.metadata["parser.runconfig.extensions.stream.*"]` に同じ値を記録し、`parser.stream_extension_field_coverage` の必須項目をすべて満たすことで担保する。とくに Packrat 利用と checkpoint クロージャは次のように扱う。

| フィールド | 型 | 取得元 / 意図 |
| --- | --- | --- |
| `packrat_enabled` | `Bool` | CLI/LSP の `--packrat` フラグ、または `RunConfig.packrat`。Streaming Recover が Packrat キャッシュを利用できる状態かどうかを示し、`parser.expected_summary_presence` が 1.0 の場合にはこの値も両フロントエンドで一致している必要がある。 |
| `flow.checkpoints_closed` | `Int` | ストリーミング実行中にクローズされた checkpoint 件数。`StreamFlowState` が収集した実測値があればそれを出力し、まだ未計測の環境では `0` を既定値とする。 |

- これらの値は `diagnostics.json` と `parser-metrics.*.json` の両方に直列化し、監査レポートで差分を追跡する。
- `tooling/ci/collect-iterator-audit-metrics.py --section streaming` は上記フィールドを前提に `parser.stream_extension_field_coverage`, `parser.stream.outcome_consistency`, `ExpectedTokenCollector.streaming` を評価し、`require-success` 実行時に 1.0 を必須とする。
- Packrat／checkpoint 情報は `ExpectedTokenCollector.streaming`（期待トークン補完）と `parser.runconfig_switch_coverage`（CLI スイッチ覆い）と連携して計測され、Streaming Recover が Packrat キャッシュと checkpoint 制御の両方と同期しているかを保証する。
