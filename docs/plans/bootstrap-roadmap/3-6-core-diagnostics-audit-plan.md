# 3.6 Core Diagnostics & Audit 実装計画

## 目的
- 仕様 [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) に準拠した `Diagnostic`/`AuditEnvelope`/`Telemetry` API を Reml 実装へ統合し、Chapter 3 全体で共有できる診断・監査基盤を完成させる。
- Parser/TypeChecker/Runtime から発生するエラー・イベントの情報粒度を統一し、監査ログポリシー・ステージ情報との整合を保証する。
- LSP/CLI 用の出力整形とメトリクス報告を整備し、仕様・実装・ドキュメントの差分を解消する。
- 本計画は Rust 版 Reml コンパイラ（`compiler/rust/`）を唯一の実装対象とし、OCaml 実装は具体例参照に留める。

## スコープ
- **含む**: `Diagnostic` 構造・Severity/Domain、`AuditEnvelope` と `AuditEvent`、型制約可視化 (`TraitResolutionTelemetry`)、警告抑制ポリシー、`EffectDiagnostic` 等の実装とテスト、ドキュメント更新。
- **含まない**: LSP プロトコルそのものの実装、外部監査システムとの接続 (Phase 4 移行計画で扱う)。
- **前提**: `Core.Text`/`Core.Numeric`/`Core.Runtime` が整備されており、Phase 2 の診断パイプラインがベースとして存在すること。

## 作業ブレークダウン

### 1. 仕様差分整理と設計更新（50週目）
**担当領域**: 設計調整

1.1. `Diagnostic`/`AuditEnvelope`/`Telemetry` のフィールド一覧を抽出し、既存実装との差分と未実装項目を洗い出す。
1.2. 効果タグ (`effect {diagnostic}`, `{audit}`, `{debug}`, `{trace}`, `{privacy}`) の付与基準を整理し、テスト戦略を決定する。
1.3. LSP/CLI 出力フォーマットの要求 (期待値・ヒント・span ハイライト) を仕様を基に再確認する。

### 2. Diagnostic コア実装（50-51週目）
**担当領域**: 基盤機能

2.1. `Diagnostic` 構造体と補助型 (`SpanLabel`, `Hint`, `ExpectationSummary`) を実装し、`Core.Text` を利用したハイライト生成を統合テストする。
2.2. `DiagnosticBuilder` 相当の生成ユーティリティを実装し、`severity/domain/code` セットが欠落しないようバリデーションを追加する。
2.3. CLI/LSP 出力フォーマットを整備し、`message` テンプレートとローカライズキーの整合を確認する。

### 3. AuditEnvelope / AuditEvent 実装（51週目）
**担当領域**: 監査基盤

3.1. `AuditEnvelope` と `AuditEvent` を実装し、必須メタデータが埋まっているか検証するユーティリティを追加する。
3.2. `PipelineStarted` 等の既定イベントを発火させる API を実装し、監査ログへ記録するテストを整備する。
3.3. `log_grapheme_stats` や IO/Runtime からのイベント連携を確認し、`AuditCapability` との接続をテストする。

### 4. Effect/Capability 診断と Telemetry（51-52週目）
**担当領域**: 高度診断

4.1. `EffectDiagnostic`, `CapabilityMismatch` 等の診断 API を実装し、`Stage`/`Capability` 情報が付与されることを検証する。
4.2. `TraitResolutionTelemetry` など型制約可視化用構造体を実装し、TypeChecker (3-2) から出力を受け取れるようにする。
4.3. `ResolutionState`, `ConstraintGraphSummary` 等のデータを Graphviz 等にエクスポートするヘルパを追加する。

### 5. ポリシー設定とフィルタリング（52週目）
**担当領域**: 運用制御

5.1. 診断抑制ポリシー (`--ack-experimental-diagnostics` 等) を実装し、ステージ別で Severity が切り替わることをテストする。
5.2. `DiagnosticFilter`/`AuditPolicy` を実装し、CLI/LSP で指定できるようにする。
5.3. `0-3-audit-and-metrics.md` と連動するメトリクス収集 (エラー件数、監査イベント件数) を整備する。

### 6. ドキュメント・サンプル更新（52-53週目）
**担当領域**: 情報整備

6.1. 仕様書内の図表・サンプルを実装に合わせて更新し、`examples/` に診断/監査のゴールデン出力を追加する。
6.2. `README.md`/`3-0-phase3-self-host.md` に Diagnostics 実装ステータスと活用ガイドを追記する。
6.3. `docs/guides/ai-integration.md`/`docs/guides/runtime-bridges.md` の診断・監査セクションを更新する。

### 7. テスト・CI 統合（53週目）
**担当領域**: 品質保証

7.1. 単体・統合テストを追加し、診断構造のシリアライズ/デシリアライズが安定していることを確認する。
7.2. 監査ログ出力のスナップショットテストと GDPR/Privacy フラグの検証を行う。
7.3. CI に診断差分検出タスクを追加し、回帰時に `0-4-risk-handling.md` へ自動記録する仕組みを構築する。

## 成果物と検証
- `Diagnostic`/`AuditEnvelope` API が仕様通りに実装され、効果タグ・ステージ情報が正しく扱われること。
- TypeChecker/Runtime/IO からのイベントが統一された監査ログとして出力され、差分が記録されていること。
- ドキュメント・サンプル・メトリクスが更新され、運用ガイドラインが明確であること。

## リスクとフォローアップ
- 監査ログの肥大化が懸念される場合、サンプリングやローテーションを Phase 4 の運用計画に追加する。
- Telemetry 出力が TypeChecker パフォーマンスに影響する場合、デバッグフラグで制御する仕組みを整備する。
- プライバシー要件が未定義な場合、`docs/notes/dsl-plugin-roadmap.md` に TODO を追記し、法務レビューを手配する。

## 参考資料
- [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md)
- [2-5-error.md](../../spec/2-5-error.md)
- [3-4-core-numeric-time.md](../../spec/3-4-core-numeric-time.md)
- [3-8-core-runtime-capability.md](../../spec/3-8-core-runtime-capability.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- [0-4-risk-handling.md](0-4-risk-handling.md)
