# 3.6 Core Diagnostics & Audit

Status: 正式仕様（2025年版）

> 目的：Reml 全体で統一された診断 (`Diagnostic`) と監査 (`audit_id`, `change_set`) モデルを提供し、パーサ・標準ライブラリ・ツールが同一の情報粒度でログ・レポートを生成できるようにする。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | 正式仕様 |
| 効果タグ | `@pure`, `effect {audit}`, `effect {debug}`, `effect {trace}`, `effect {privacy}` |
| 依存モジュール | `Core.Prelude`, `Core.Text`, `Core.Numeric & Time`, `Core.Config`, `Core.Data`, `Core.IO` |
| 相互参照 | [2.5 エラー設計](2-5-error.md), [3.4 Core Numeric & Time](3-4-core-numeric-time.md), [3.5 Core IO & Path](3-5-core-io-path.md), [3.7 Core Config & Data](3-7-core-config-data.md) |

## 1. `Diagnostic` 構造体

既存の Chapter 2.5 で提示した構造を標準ライブラリ側で正式定義する。

```reml
pub type Diagnostic = {
  id: Option<Uuid>,
  message: Str,
  severity: Severity,
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

## 2. 診断生成ヘルパ

```reml
fn diagnostic(message: Str) -> DiagnosticBuilder                     // `@pure`

struct DiagnosticBuilder {
  diag: Diagnostic,
}

impl DiagnosticBuilder {
  fn with_span(self, span: Span) -> Self;                             // `@pure`
  fn with_severity(self, severity: Severity) -> Self;                 // `@pure`
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

## 5. 使用例（CLI エラー報告）

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

## 6. CLI/LSP 連携の具体例

### 6.1 CLI ツール統合

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

### 6.2 LSP サーバー統合

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

### 6.3 メトリクス監視ダッシュボード

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
