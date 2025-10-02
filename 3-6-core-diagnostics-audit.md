# 3.6 Core Diagnostics & Audit

> 目的：Reml 全体で統一された診断 (`Diagnostic`) と監査 (`audit_id`, `change_set`) モデルを提供し、パーサ・標準ライブラリ・ツールが同一の情報粒度でログ・レポートを生成できるようにする。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | 正式仕様 |
| 効果タグ | `@pure`, `effect {audit}`, `effect {debug}`, `effect {trace}`, `effect {privacy}` |
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
  capability: Option<CapabilityId> // 必要とされる Capability（任意）
}

enum Stage = Experimental | Beta | Stable
```

`Stage` は Capability Registry（3.8 §1）と共有される列挙で、CLI/LSP は `Stage` に基づき表示レベルを調整する。`before` / `handled` / `residual` は 1.3 §I の効果計算結果に対応し、`residual = ∅` の場合は純粋化可能であることを意味する。`unhandled_operations` は `effects.handler.unhandled_operation` 診断（2.5 §B-10）で IDE へ提示する一覧として使用する。

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

### 2.1 `Result`/`Option` との連携

```reml
fn expect_ok<T, E: IntoDiagnostic>(result: Result<T, E>) -> Result<T, Diagnostic> // `@pure`
fn tap_diag<T>(result: Result<T, Diagnostic>, inspect: (Diagnostic) -> ()) -> Result<T, Diagnostic> // `effect {audit}`
```

- `IntoDiagnostic` トレイトにより任意のエラー型を `Diagnostic` へ変換。
- `tap_diag` は監査ログ出力や統計集計に利用でき、`effect {audit}` を明示する。

### 2.2 Core.Parse 連携（`Parse.fail` / `Parse.recover`）

```reml
type ParseDiagnosticOptions = {
  severity: Severity = Severity::Error,
  domain: DiagnosticDomain = DiagnosticDomain::Parser,
  code: Option<Str> = None,
  locale: Option<Locale> = None,
  audit: Option<AuditEnvelope> = None,
  attach_span_trace: Bool = true,
}

fn from_parse_error(src: Str, err: ParseError, opts: ParseDiagnosticOptions) -> Diagnostic      // `@pure`
fn from_parse_errors(src: Str, errs: List<ParseError>, opts: ParseDiagnosticOptions) -> List<Diagnostic> // `@pure`
```

- `locale` は 2.5 §B-11 の手順で `RunConfig.locale` を渡し、未指定時は CLI/LSP 側の既定値を利用する。
- `audit` へ値を渡すと `Diagnostic.audit` が事前に設定され、監査ライン（§3）でそのまま利用できる。`RunConfig.extensions["audit"].envelope` を `Some(AuditEnvelope)` にしておくと、`Core.Parse` は `Parse.fail` 実行時にこの値を引き継ぐ。
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

### 2.4 効果診断メッセージ (Effect Domain) {#diagnostic-effect}

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

これらのキーは `AuditPolicy.exclude_patterns` で除外しない限り永続化され、`CapabilityAudit` レポートや LSP の効果ビューで差分分析に利用できる。

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
- `Diagnostic.primary` は `AsyncError.span` を利用し、値が無い場合は 1-1 §B の合成 Span 規約で生成した位置を割り当てる。
- `AsyncError.cause` の各要素は順序を保持したまま `Diagnostic.secondary` の `SpanLabel` に変換する。`SpanLabel.message` には `AsyncErrorLink.message` を格納し、`origin` と `metadata` から抽出したキー（例: `retry_attempt`, `channel`）を括弧書きで併記する。
- `Diagnostic.extensions["async"]` には上記構造を格納し、`metadata` フィールドに `AsyncError.metadata` をマージする。`diagnostic_id` キーが存在する場合は `AuditEnvelope.metadata["async.diagnostic_id"]` にも反映し、重複報告を避ける。
- `AuditEnvelope.metadata["async.cause_chain"]` へ `AsyncError.cause` を JSON 化して保存し、監査ポリシーが `Trace` 未満でも最初の要素を残す。

これらの手順は 0-1 §1.2 と §2.2 に沿って、原因追跡と再現性を改善する。CLI/LSP は `AsyncDiagnosticExtension` を持つ診断をツリー表示する UI を提供することが推奨される。

## 3. 監査ログ出力

```reml
pub type AuditSink = fn(Diagnostic) -> Result<(), AuditError>          // `effect {audit}`

fn emit(diag: Diagnostic, sink: AuditSink) -> Result<(), AuditError>    // `effect {audit}`
fn with_context(diag: Diagnostic, ctx: Json) -> Diagnostic              // `@pure`
fn redact(diag: Diagnostic, policy: RedactPolicy) -> Diagnostic         // `@pure`
```

- `AuditSink` は CLI/LSP/Runtime の橋渡しを抽象化した関数型。

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

## 6. DSLオーケストレーション向け可観測性

### 6.1 メトリクスプリセット

```reml
pub struct DslMetricsHandle = {
  latency: LatencyHistogram,
  throughput: CounterMetric,
  error_rate: RatioGauge,
  in_flight: GaugeMetric,
}

fn register_dsl_metrics(registry: MetricsRegistry, dsl_id: DslId) -> Result<DslMetricsHandle, Diagnostic> // `effect {trace}`
fn record_dsl_success(handle: DslMetricsHandle, duration: Duration) -> ()                                  // `effect {trace}`
fn record_dsl_failure(handle: DslMetricsHandle, error: Diagnostic, duration: Duration) -> ()               // `effect {trace, audit}`
fn observe_backpressure(handle: DslMetricsHandle, depth: usize) -> ()                                      // `effect {trace}`
```

- デフォルトメトリクス名（指標 → 型）:
  - `dsl.latency` → `LatencyHistogram`（p50/p95/p99 を追跡）
  - `dsl.throughput` → `CounterMetric`（1秒あたりの完了数）
  - `dsl.error_rate` → `RatioGauge`（成功/失敗比率）
  - `dsl.in_flight` → `GaugeMetric`（実行中タスク数）
- `register_dsl_metrics` は `conductor` で宣言された DSL ID ごとにメトリクスを初期化し、`Core.Async` ランタイムへハンドルを戻す。
- 成功/失敗の記録は `ExecutionPlan` のエラーポリシーと連動し、監査ログ `AuditEnvelope` を自動的に添付できる。

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
- `guides/ci-strategy.md` に記載の構造化ログと併用し、`RunConfig.extensions["target"]` の変更が期待どおりの挙動を保っているかを定期的に可視化する。
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
- CI では `DiagnosticDomain::Target` を優先的に集計し、`guides/ci-strategy.md` に定義するマトリクス上で失敗ターゲットを特定する。
- IDE/LSP では `Target` ドメイン診断をワークスペースレベル警告として表示し、該当ファイルが無い場合でも `RunConfigTarget` 情報を提示する。
- `Core.Env` と `Core.Runtime` はターゲット診断を発生させた際、`RunArtifactMetadata.hash` を `extensions["target"].hash` に追加し、再ビルドのトレーサビリティを確保する。

## 8. 使用例（CLI エラー報告）


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

## 9. CLI/LSP 連携の具体例

### 8.1 CLI ツール統合

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

### 8.2 LSP サーバー統合

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

### 8.3 メトリクス監視ダッシュボード

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

## 10. CLI コマンドプロトコル {#cli-protocol}

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
