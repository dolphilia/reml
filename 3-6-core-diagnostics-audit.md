# 4.7 Core Diagnostics & Audit（フェーズ3 ドラフト）

Status: Draft（内部レビュー中）

> 目的：Reml 全体で統一された診断 (`Diagnostic`) と監査 (`audit_id`, `change_set`) モデルを提供し、パーサ・標準ライブラリ・ツールが同一の情報粒度でログ・レポートを生成できるようにする。

## 0. ドラフトメタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | Draft（フェーズ3） |
| 効果タグ | `@pure`, `effect {audit}`, `effect {debug}`, `effect {trace}` |
| 依存モジュール | `Core.Prelude`, `Core.Text`, `Core.Numeric & Time`, `Core.Config`, `Core.Data` |
| 相互参照 | [2.5 エラー設計](2-5-error.md), [4.5 Core Numeric & Time](4-5-core-numeric-time.md), [4.8 Core Config & Data](4-8-core-config-data.md) |

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

- `timestamp` は [4.5](4-5-core-numeric-time.md) の `Timestamp` を利用し、診断生成時に `Core.Numeric.now()` を呼び出す。
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

## 4. 差分・監査テンプレート

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

> 関連: [2.5 エラー設計](2-5-error.md), [4.5 Core Numeric & Time](4-5-core-numeric-time.md), [4.8 Core Config & Data](4-8-core-config-data.md)
