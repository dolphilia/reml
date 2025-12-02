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
    - 1.1.a `docs/spec/3-6-core-diagnostics-audit.md` と `docs/spec/2-5-error.md` のフィールド定義を表形式に起こし、`compiler/rust/diagnostics/` 直下の `*.rs` を `rg "pub struct"` で走査してマッピング表を更新する。
    - 1.1.b `docs/plans/rust-migration/1-2-diagnostic-compatibility.md` のチェックリストをコピーし、`reports/dual-write/front-end/.../diagnostics-diff.md` を参照しながら不足フィールドへ ❌ ラベルを付ける。
    - 1.1.c 差分表を `docs/plans/bootstrap-roadmap/assets/diagnostics-field-gap.csv` に保存し、更新後に `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の「診断フィールドカバレッジ」指標へ Run ID を追記する。

#### 1.1 実施結果（Run ID: 20290601-diag-field-gap）
- `docs/plans/bootstrap-roadmap/assets/diagnostics-field-gap.csv` に `Diagnostic`/`AuditEnvelope`/`Telemetry` の 13 フィールドを整理。`FrontendDiagnostic` へ未実装の `id`・`span_trace`・`extensions` や、`AuditEnvelope.audit_id` が `Uuid` でない点、`TraitResolutionTelemetry` ファイル未作成といった差分をコメント付きで記録した。
- `reports/dual-write/front-end/poc/2025-11-07-w2-ast-inventory/diagnostic_diff.md` と W4 以降の Run を照合し、`timestamp` 未設定・Stage/Audit キー欠落・`expected_tokens` 不整合といった実際の失敗ログを紐付けた。Rust 側で `schema.version=3.0.0-alpha` へ移行する際の修正箇所として、CSV から直接参照できる状態にした。
- `docs/plans/rust-migration/1-2-diagnostic-compatibility.md#1.2.6` のチェックリストを以下のとおり複製し、欠落した必須フィールドを ❌ で明示した。

| チェック項目 | 参照 | Rust 実装状況 | ラベル | 証跡 |
| --- | --- | --- | --- | --- |
| `expected_tokens` (recover 拡張) | 1-2 §1.2.6 / 2-5 §A | recover ケースで `expected_tokens.diff.json` が解消できず、`parser.expected_summary_presence` が 1.0 未満。 | ❌ | `reports/dual-write/front-end/w4-diagnostics/20280430-w4-diag-cli-lsp/cli_packrat_switch/summary.json` |
| `effects.stage.*` | 1-2 §1.2.6 / 3-6 §2 | Rust 診断に `effect.stage.required/actual` が無く、Stage 監査が `bridge_stage.audit_presence < 1.0` で停止。 | ❌ | `reports/dual-write/front-end/w4-diagnostics/20280601-w4-diag-type-effect-rust-typeck-r7/recover_missing_semicolon/effects-metrics.rust.err.log` |
| `parser.stream.*` | 1-2 §1.2.6 / guides/core-parse-streaming.md | Streaming RunConfig 拡張を `parser.stream.*` に転写できず、`parser.stream_extension_field_coverage=0.0`。 | ❌ | `reports/dual-write/front-end/w4-diagnostics/20271112-w4-diag-m1/stream_pending_resume/parser-metrics.rust.json` |
| `type_row.*` / `typeclass.dictionary.*` | 1-2 §1.2.6 / 1-3-effects-safety.md | JSON には `type_row_mode` のみで、`effect.type_row.*` や `typeclass.dictionary` ブロックが欠落。 | ❌ | `reports/dual-write/front-end/w4-diagnostics/20280601-w4-diag-type-effect-rust-typeck-r7/effect_residual_leak/diagnostics.rust.json` |
| `extensions["recover"]` | 1-2 §1.2.6 / reports/diagnostic-format-regression.md | Recover 代表ケースが `diagnostics=[]` で拡張を組み立てられず、ハンドラ情報がゼロ。 | ❌ | `reports/dual-write/front-end/w4-diagnostics/20271112-w4-diag-m1/recover_else_without_if/diagnostics.rust.json` |

上記ギャップは `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` の DIAG-RUST-05/06/07 とリンク済みであり、次スプリントは `ExpectedTokenCollector` の強制適用、`StageAuditPayload` の共通化、`RunConfigBuilder` による stream/config 拡張を優先実装として扱う。

#### 1.1 追記（Run ID: 20290615-diag-field-coverage）
- `FrontendDiagnostic::ensure_id` を導入し、`DiagnosticBuilder::push_internal` で毎回呼び出すことで CLI/LSP/監査に載るすべての Parser 診断へ UUID を割り当てた（`compiler/rust/frontend/src/diagnostic/mod.rs`）。`span_trace` は `parse_with_options` 内で Streaming State からコピーされるため、`RunConfig.trace = true` のケースでも欠落がなくなった。
- `AuditEnvelope.audit_id` を `Option<Uuid>` へ強化し、`ensure_audit_id` が `Uuid::new_v5` で生成した `audit.id.uuid` をメタデータに記録するよう更新した。既存の `cli.audit_id` 文字列ラベルも残しつつ、CLI 出力では UUID 文字列を JSON に書き出す。
- `build_parser_diagnostics` が `FrontendDiagnostic.extensions` を維持したまま `diagnostic.v2` や `recover` 拡張を上書きし、型検査経路でも `AuditEnvelope` の UUID が JSON へ反映されるようにした。
- 上記の差分を `docs/plans/bootstrap-roadmap/assets/diagnostics-field-gap.csv` に反映し、`id`／`span_trace`／`extensions map`／`audit_id` の 4 行を `整合` へ更新。`diagnostic.field_coverage` は 13 項目中 9 項目の完了扱いとなり、`0-3-audit-and-metrics.md` の KPI 更新履歴にも Run ID を追記した。

#### 1.1 追記（Run ID: 20290620-stage-expected）
- `StageAuditPayload` を `compiler/rust/frontend/src/diagnostic/effects.rs` へ移設し、`Diagnostic` モジュールから再利用できる API として公開した。これにより CLI／TypeChecker／`--emit-effects-metrics` で同一の Stage/Capability 情報を共有し、`collect-iterator-audit-metrics` が参照する `bridge.stage.*`/`effect.stage.*` キーが常に揃う（`effects-metrics.rust.err.log` の `bridge_stage.audit_presence` 警告が解消）。
- `TypecheckViolation::stage_mismatch`／`iterator_stage_mismatch` に `ExpectedTokenCollector` ベースのトップレベル宣言セット（`effect`/`extern`/`fn`/`handler`/`impl`/`let`/`pub`/`trait`/`type`/`var`/`@`/`EOF`）を付与し、OCaml 版が出力する Recover 拡張と同じ配列を JSON へ埋め込むようにした（`compiler/rust/frontend/src/typeck/driver.rs`）。`reports/dual-write/front-end/.../expected_tokens.diff.json` で `ffi_stage_messagebox` などに残っていた差分はこの共通サマリで吸収できる。
- 変更結果を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に追記し、`StageAuditPayload` 共有化と `ExpectedTokenCollector` 適用を Phase3 KPI の更新トリガー（Run ID `20290620-stage-expected`）として登録した。

#### 1.1 追記（Run ID: 20290622-span-timestamp）
- Parser 診断の `timestamp` を必須化し、`FrontendDiagnostic::set_timestamp` を `build_parser_diagnostics` から必ず呼び出すことで CLI/LSP/Audit すべての JSON に同一の ISO8601 値を出力するよう統合した（`compiler/rust/frontend/src/bin/reml_frontend.rs`）。
- `Span` も必須フィールドに変更し、`set_span`/`primary_span` を介して管理。欠落時は `span_trace` 先頭の `TraceFrame`（なければ `[0,0)`）を自動採用するため、`primary`/`location` が欠落しなくなり `diagnostics-field-gap.csv` の `primary span` 行を `整合` とした。
1.2. 効果タグ (`effect {diagnostic}`, `{audit}`, `{debug}`, `{trace}`, `{privacy}`) の付与基準を整理し、テスト戦略を決定する。
    - 1.2.a `docs/spec/1-3-effects-safety.md` と `docs/spec/3-8-core-runtime-capability.md` の Stage/Capability ルールを参照し、タグ別のトリガー条件をスプレッドシート化する。
    - 1.2.b `compiler/rust/frontend/src/effects/` を `rg "effect::"` で走査し、既存タグを洗い出して `StageRequirement::{Exact,AtLeast}` と突合する。
    - 1.2.c `scripts/validate-diagnostic-json.sh` にタグ別フィルタを追加するタスクを Issue 化し、テストケース（`examples/core_effects/*.reml`）を指定して再現手順を計画書へリンクする。
#### 1.2 実施結果（Run ID: 20290701-effect-tag-policy）
- `docs/plans/bootstrap-roadmap/assets/diagnostic-effect-tag-policy.csv` を新設し、 `effect {diagnostic}` / `{audit}` / `{debug}` / `{trace}` / `{privacy}` それぞれのトリガー API、既定の StageRequirement、`CapabilityDescriptor` との対応、検証に使う KPI やスクリプトを列挙した。表内では `docs/spec/3-6-core-diagnostics-audit.md` §1-§6、`docs/spec/1-3-effects-safety.md` §C、`docs/spec/3-8-core-runtime-capability.md` §4 を参照しながら CLI/LSP/Audit の責務境界を整理している。
- Rust Frontend 側の Stage 判定は `compiler/rust/frontend/src/typeck/env.rs`（`StageContext::resolve` が CLI/RunConfig/Runtime Registry の Stage を `StageRequirement::merged_with` で合成）と `compiler/rust/frontend/src/typeck/capability.rs`（`CapabilityDescriptor::resolve` が `core.debug.*` などの識別子を `StageId::beta()` あるいは `panic/unsafe/ffi/runtime` 系は `StageId::experimental()` に正規化）へ集約されているため、1.2.b の走査対象を同ファイルへ差し替えて調査ログを残した。`diagnostic`/`audit`/`trace`/`privacy` はいずれも `AtLeast(beta)` が既定で、CLI から `--effect-stage` を渡した場合は `StageAuditPayload` (`compiler/rust/frontend/src/diagnostic/effects.rs`) が `stage_trace` に統合することを確認している。
- `scripts/validate-diagnostic-json.sh` へ `--effect-tag <tag>` オプションを追加し、Python 段で `effects.*` メタデータに指定タグを含む診断のみ検証するようにした（デフォルトは従来どおり全件検証）。`--suite streaming` でも同じフィルタを共有し、該当診断が無いファイルは `info` ログで明示する。手元での再現手順は `reml_frontend --format json --emit-audit-log examples/core_effects/trace_channel.reml > tmp/examples/core_effects/trace_channel.diagnostics.json` → `scripts/validate-diagnostic-json.sh tmp/examples/core_effects/trace_channel.diagnostics.json --effect-tag trace --pattern parser.stream` を想定しており、`examples/core_effects/privacy_mask.reml` では `--effect-tag privacy --suite audit` を使う。`examples/core_effects/*.reml` が揃い次第 `reports/dual-write/front-end/core_effects/README.md` へ追記する TODO を残し（`#TODO core_effects-trace-privacy`）、Issue 追跡は計画書上の本節で継続する。
1.3. LSP/CLI 出力フォーマットの要求 (期待値・ヒント・span ハイライト) を仕様を基に再確認する。
    - 1.3.a `docs/spec/3-6-core-diagnostics-audit.md#lsp-cli` の期待出力例を抽出し、`compiler/rust/frontend/src/cli/diagnostic_printer.rs` のテンプレートと比較するレビュー表を作成する。
    - 1.3.b LSP 互換性は `tooling/lsp/tests/client_compat` の JSON フィクスチャを `jq` で確認し、欠落フィールドがあれば `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` にリスク ID を追加する。
    - 1.3.c CLI 側は `reml_frontend --format json|human` のサンプル出力を `reports/diagnostic-format-regression.md` へ再収集し、差分が 0 であることを確認して記録する。

#### 1.3 実施結果（Run ID: 20290705-cli-lsp-format）
- 1.3.a 仕様と実装の差異を洗い出すため、`docs/spec/3-6-core-diagnostics-audit.md` §10–11 を起点に CLI/LSP 出力要件の表を作成した。Rust 実装は `compiler/rust/frontend/src/bin/reml_frontend.rs` および `compiler/rust/frontend/src/diagnostic/json.rs` へ集約されているため、該当箇所を精査した結果を下表にまとめた。`structured_hints` のように仕様書では CLI/LSP 双方での提示が前提になっている項目が未実装であること、`CliDiagnosticEnvelope`/`--output` 系フラグが存在しないことを明示し、今後の実装優先度を評価できるようにした。

| 項目 | 仕様参照 | Rust 実装 | 評価 | 補足 |
| --- | --- | --- | --- | --- |
| `CliDiagnosticEnvelope` と `--output human|json|lsp` 切替 | `docs/spec/3-6-core-diagnostics-audit.md` §10–11 | `compiler/rust/frontend/src/bin/reml_frontend.rs` に `OutputFormat`/`CliDiagnosticEnvelope`/`CliExitCode` を実装し、`Run ID: 20290705-cli-output` で `--output {human|json|lsp}` が動作する状態を確認済み。JSON は NDJSON 1 行、Human は stderr 整形、LSP は `publishDiagnostics` + `logMessage` を送出する | ✅ | `reports/diagnostic-format-regression.md#cli-output-note` にサンプル取得手順を記録し、`docs/spec/3-6-core-diagnostics-audit.md` の CLI プロトコルと整合する `summary`/`phase`/`command`/`exit_code` を出力できるようになった。 |
| 期待集合 (`expected`) と Recover ヒント | `docs/spec/3-6-core-diagnostics-audit.md` §2, §10 | `compiler/rust/frontend/src/diagnostic/json.rs:73-168` の `build_expected_field` / `build_recover_extension` で `message_key`/`alternatives`/`hints` を生成し、`build_parser_diagnostics`（`reml_frontend.rs:1304-1412`）から常に埋め込んでいる | ✅ | `ExpectedTokenCollector` 由来のヒントは JSON/LSP 共通で利用可能な状態。 |
| span ハイライト (`primary`/`location`/`span_trace`) | `docs/spec/3-6-core-diagnostics-audit.md` §3, §10 | `LineIndex`（`compiler/rust/frontend/src/diagnostic/json.rs:5-69`）と `span_trace_to_json` が CLI で利用する行/列情報を提供済み | ✅ | `trace_ids` も `build_parser_diagnostics` で付与され、Streaming Trace と連動できる。 |
| `structured_hints` / FixIt コマンド | `docs/plans/bootstrap-roadmap/2-4-diagnostics-audit-pipeline.md` §2, `docs/spec/3-6-core-diagnostics-audit.md` §10 | `compiler/rust/frontend/src/diagnostic/json.rs` に `structured_hints` を追加し、`DiagnosticHint` へ `id/title/kind/span/payload` を持たせることで CLI/LSP 共通の構造化ヒントを出力可能にした | ✅ | `structured_hints` が常に配列として出力されるため、`tooling/lsp/tests/client_compat/fixtures/diagnostic-v2-sample.json` を Rust 出力から再生成できる。`kind` 未指定時は `quick_fix` または `information` を自動補完する。 |

- 1.3.b `tooling/lsp/tests/client_compat/fixtures` を `jq` で確認し、Rust 実装との差異を記録した。`diagnostic-v2-sample.json` および `diagnostic-v2-stream.json` から `schema_version` と `severity` を抽出すると `jq '.[].schema_version' … -> "2.0.0-draft"`、`jq '.[].severity' … -> 1` のように旧スキーマ・数値列挙が残っている。また `jq '.[0] | has("span_trace")' tooling/lsp/tests/client_compat/fixtures/diagnostic-v2-stream.json` が `false` を返し、CLI/LSP 両方で必須になった `span_trace` が欠落している。フィクスチャが古いままだと `npm run ci --prefix tooling/lsp/tests/client_compat` では差分を見落とすため、互換性リスクとして `docs/plans/bootstrap-roadmap/0-4-risk-handling.md#diagnostic-lsp-fixture-drift` に登録した。
- 1.3.c `reml_frontend --output {human|json|lsp}` の挙動を `reports/diagnostic-format-regression.md#cli-output-note` に再記録し、NDJSON/Human/LSP それぞれのサンプルログを `reports/dual-write/front-end/samples/20290705-cli-output.*` へ保存した。`scripts/validate-diagnostic-json.sh` では JSON スキーマ検証、Human/LSP は差分確認（`diff -u`）で監査する手順を追加している。

### 2. Diagnostic コア実装（50-51週目）
**担当領域**: 基盤機能

2.1. `Diagnostic` 構造体と補助型 (`SpanLabel`, `Hint`, `ExpectationSummary`) を実装し、`Core.Text` を利用したハイライト生成を統合テストする。
    - 2.1.a `compiler/rust/frontend/src/diagnostics/model.rs` に構造体定義を追加し、`serde` / `schemars` 互換を `#[cfg_attr]` で確保する。
    - 2.1.b `Core.Text` の `grapheme_clusters` API を `packages/core_text/src/span_highlight.rs` に取り込み、`expect_span_highlight` 単体テストを `cargo test span_highlight` で作成する。
    - 2.1.c `reports/dual-write/front-end/.../span-highlight.md` にテストログを保存し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の `diagnostic.span_highlight_pass_rate` を更新する。
2.2. `DiagnosticBuilder` 相当の生成ユーティリティを実装し、`severity/domain/code` セットが欠落しないようバリデーションを追加する。
    - 2.2.a `DiagnosticBuilder::new()` に `ensure_fields()` を実装し、`debug_assert!` + `Result` 併用で必須項目の欠落を早期検知する。
    - 2.2.b `compiler/rust/frontend/tests/diagnostic_builder.rs` を新設し、`#[test_case]` で Severity/Domain/Code 未設定時の panic/Err を検証する。
    - 2.2.c `docs/spec/3-6-core-diagnostics-audit.md` の §2 表を引用したコメントを Builder に付与し、仕様が更新された際に差分チェックを行う TODO をコメント化する。
2.3. CLI/LSP 出力フォーマットを整備し、`message` テンプレートとローカライズキーの整合を確認する。
    - 2.3.a `compiler/rust/frontend/src/output/cli.rs` と `tooling/lsp/src/handlers/diagnostics.rs` へ `LocalizationKey` を受け取るパスを追加する。
    - 2.3.b `docs/guides/ai-integration.md` へ LSP 診断のキー表を追記し、`README.md` から参照できるようリンクを更新する。
    - 2.3.c `reports/diagnostic-format-regression.md` のローカライズ節に新規ケースを登録し、CI 差分チェック対象に加える。

### 3. AuditEnvelope / AuditEvent 実装（51週目）
**担当領域**: 監査基盤

3.1. `AuditEnvelope` と `AuditEvent` を実装し、必須メタデータが埋まっているか検証するユーティリティを追加する。
    - 3.1.a `compiler/rust/runtime/src/audit/mod.rs` に Envelope/Event 定義を作成し、`serde_json::Value` で拡張フィールドを許可する。
    - 3.1.b `AuditEnvelope::validate()` を実装し、`effect.stage.required` / `bridge.reload` など必須キーをチェックして `anyhow::Result` を返す。
    - 3.1.c `compiler/rust/runtime/tests/audit_validation.rs` で JSON フィクスチャを読み込み、欠落時には明確なエラーになることをスナップショットテストで確認する。

#### 3.2 実施結果（Run ID: 20290715-pipeline-events）
- `reml_runtime` の `audit/mod.rs` へ `AuditEventKind` 列挙体と `AuditEnvelope::from_parts` を追加し、`pipeline_started`/`pipeline_completed`/`pipeline_failed` 等の必須キーを表で管理できるようにした。`AuditEnvelope::validate()` は文字列比較ではなく列挙体に基づいて判定し、`bridge.reload`/`bridge.rollback` の再利用ロジックも共通化している。
- `compiler/rust/frontend/src/pipeline/mod.rs` に `AuditEmitter`・`PipelineDescriptor`・`PipelineOutcome` を実装し、CLI から取得した `run_id`・`command`・入力ファイルを監査メタデータへ集約した。`AuditEmitter::stderr(true)` を経由して `reml_frontend` の `parse → typeck → emit` 全体を挟み込み、`pipeline_started` → `run_frontend` 本体 → `pipeline_completed/pipeline_failed` の順に JSON Lines を出力する。エラー発生時は `"cli.pipeline.failure"` コードで `pipeline_failed` を記録する。
- CLI フラグ `--emit-audit-log`（従来の `--emit-audit` エイリアス込み）を追加し、ヘルプテキストと `parse_args` を更新。`CliArgs.emit_audit` が真のときにのみ `AuditEmitter` が書き込みを行い、失敗時は `[AUDIT] ...` ログを標準エラーへ通知する。`cargo test pipeline_audit` でイベント出力の JSON が `AuditEnvelope::validate` を通過することを確認するユニットテストを追加済み。
- 監査サンプルとして `examples/core_diagnostics/pipeline_success.reml` / `pipeline_branch.reml` を追加し、`tooling/examples/run_examples.sh --suite core_diagnostics --with-audit` で両ケースを連続実行できるようにした。スクリプトは `compiler/rust/frontend` 配下で `cargo run --bin reml_frontend -- [--emit-audit-log] <example>` を実行するよう改修済み。
- コマンド例: `cargo run --quiet --bin reml_frontend -- --emit-audit-log examples/core_diagnostics/pipeline_success.reml` と `tooling/examples/run_examples.sh --suite core_diagnostics --with-audit` の双方で PipelineStarted/PipelineCompleted が NDJSON として出力されることを確認した。
#### 3.1 実施結果（Run ID: 20290712-audit-envelope）
- `compiler/rust/runtime/src/audit/mod.rs` に `AuditEnvelope` / `AuditEvent` を実装し、`metadata["event.kind"]` に基づいてパイプライン／Capability／Bridge Reload 等の必須キーをテーブル駆動で検証する `validate()` を追加した。`effect.stage.*` と `bridge.stage.*` / `bridge.reload` は個別の必須セットとして扱い、欠落時は `"audit metadata validation failed: missing keys [...]"` 形式で特定キーを列挙する。
- 依存解決にネットワークを必要としないよう、最小限の `anyhow::Result` 互換ラッパを `compiler/rust/runtime/src/anyhow.rs` に配置し、フォーマット済みメッセージを `anyhow(...)` で構築できるようにした（将来オンライン環境に戻り次第、公式クレートへ差し替える想定）。
- `compiler/rust/runtime/tests/audit_validation.rs` と `tests/fixtures/audit/*.json` / `tests/expected/audit/*.txt` を追加し、`pipeline_started_valid.json` が成功し、`effect_stage_missing.json` と `bridge_reload_missing.json` が期待どおりのエラーメッセージを返すことを `cargo test -p reml_runtime audit_validation` で確認した。エラー文面はスナップショット (`*.txt`) と一致するため、CI で欠落キーを確実に検出できる。
3.2. `PipelineStarted` 等の既定イベントを発火させる API を実装し、監査ログへ記録するテストを整備する。
    - 3.2.a `AuditEventKind` を列挙体として作成し、`PipelineStarted`/`PipelineCompleted`/`StageMismatch` 等を追加する。
    - 3.2.b `compiler/rust/frontend/src/pipeline/mod.rs` で `AuditEmitter` を注入し、CLI/LSP 経由で `--emit-audit-log` が指定された際のみログを書き出す。
    - 3.2.c `examples/core_diagnostics/pipeline_*.reml` を追加し、`tooling/examples/run_examples.sh --suite core_diagnostics --with-audit` を CI に登録する。
3.3. `log_grapheme_stats` や IO/Runtime からのイベント連携を確認し、`AuditCapability` との接続をテストする。
    - 3.3.a `runtime/logging/grapheme.rs` の `log_grapheme_stats` 呼び出しから `AuditEnvelope` へ `text.utf8.range` メタデータを転送する。
    - 3.3.b `compiler/rust/runtime/src/io/bridge.rs` のイベントフックに `AuditCapability` を注入し、`bridge.stage.*` 診断と一致するかゴールデンを比較する。
    - 3.3.c `collect-iterator-audit-metrics.py --section runtime --require-success` を新シナリオで実行し、結果を `reports/audit/dashboard/core_runtime-YYYYMMDD.md` へ記録する。

### 4. Effect/Capability 診断と Telemetry（51-52週目）
**担当領域**: 高度診断

4.1. `EffectDiagnostic`, `CapabilityMismatch` 等の診断 API を実装し、`Stage`/`Capability` 情報が付与されることを検証する。
    - 4.1.a `compiler/rust/frontend/src/effects/diagnostics.rs` に `EffectDiagnostic` を実装し、`effect.stage.required` / `effect.stage.actual` をフィールドに保持する。
    - 4.1.b `scripts/poc_dualwrite_compare.sh effect_handler --trace` を Rust 実装専用モードで実行し、差分レポートを `reports/spec-audit/ch1/effect_handler-YYYYMMDD-telemetry.md` へ保存して Stage 情報を確認する。
    - 4.1.c Stage/Capability の不一致を `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` の既存リスク ID (DIAG-RUST-05 等) に紐付け、緊急度を更新する。
4.2. `TraitResolutionTelemetry` など型制約可視化用構造体を実装し、TypeChecker (3-2) から出力を受け取れるようにする。
    - 4.2.a `compiler/rust/typeck/src/telemetry.rs` を新設し、`TraitResolutionTelemetry`/`ResolutionState`/`ConstraintGraphSummary` 構造体を `serde` でシリアライズ可能にする。
    - 4.2.b TypeChecker の制約生成ルートに `TelemetrySink` を差し込み、`--emit-telemetry constraint_graph` フラグで JSON を `tmp/telemetry/` に書き出す。
    - 4.2.c 生成した JSON を `scripts/telemetry/render_graphviz.py` で dot → svg に変換し、`docs/spec/3-6-core-diagnostics-audit.md` の図版を差し替える。
4.3. `ResolutionState`, `ConstraintGraphSummary` 等のデータを Graphviz 等にエクスポートするヘルパを追加する。
    - 4.3.a `tooling/telemetry/export_graphviz.rs` を作成し、JSON から dot を生成する CLI を提供する。
    - 4.3.b `examples/core_diagnostics/constraint_graph/*.reml` を追加し、生成 dot/svg を `examples/core_diagnostics/output/` に格納する。
    - 4.3.c `docs/guides/runtime-bridges.md` へ Graphviz エクスポート手順を追記し、TypeChecker 以外のチームも再利用できるよう説明する。

### 5. ポリシー設定とフィルタリング（52週目）
**担当領域**: 運用制御

5.1. 診断抑制ポリシー (`--ack-experimental-diagnostics` 等) を実装し、ステージ別で Severity が切り替わることをテストする。
    - 5.1.a `compiler/rust/frontend/src/config/runconfig.rs` に `AckExperimentalDiagnostics` フラグとステージ紐付けロジックを追加する。
    - 5.1.b `compiler/rust/frontend/tests/diagnostic_filtering.rs` で Stage ごとの Severity 再分類をテーブル駆動テスト化する。
    - 5.1.c `docs/guides/ai-integration.md` の診断抑制フラグ一覧を更新し、AI 支援シナリオでの使用例を追記する。
5.2. `DiagnosticFilter`/`AuditPolicy` を実装し、CLI/LSP で指定できるようにする。
    - 5.2.a CLI 引数 `--diagnostic-filter` `--audit-policy` を `structopt` / `clap` に登録し、Config へ伝搬する。
    - 5.2.b LSP 側は `workspace/configuration` からフィルタ値を受け取る JSON スキーマを定義し、`tooling/lsp/tests` で検証する。
    - 5.2.c フィルタ設定を `reports/diagnostic-format-regression.md` の差分収集スクリプトへ追加し、メトリクスが同時に取得できるようにする。
5.3. `0-3-audit-and-metrics.md` と連動するメトリクス収集 (エラー件数、監査イベント件数) を整備する。
    - 5.3.a `collect-iterator-audit-metrics.py --section diagnostics` に `diagnostic.total_count` と `audit.event_total` を出力する拡張を加える。
    - 5.3.b メトリクス CSV (`docs/plans/bootstrap-roadmap/assets/metrics/diagnostics_ci.csv`) を更新し、CI から日付ごとに append する Git フレンドリーな形式を採用する。
    - 5.3.c `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の KPI 表に新メトリクス ID を追記し、更新トリガー（CI 成功時）を脚注で明記する。

### 6. ドキュメント・サンプル更新（52-53週目）
**担当領域**: 情報整備

6.1. 仕様書内の図表・サンプルを実装に合わせて更新し、`examples/` に診断/監査のゴールデン出力を追加する。
    - 6.1.a `docs/spec/3-6-core-diagnostics-audit.md` のコードサンプルを Rust 実装の最新構造体に合わせ、`reml` コードブロックと JSON 例を再生成する。
    - 6.1.b `examples/core_diagnostics/` に `*.expected.diagnostic.json` と `*.expected.audit.jsonl` を追加し、`tooling/examples/run_examples.sh --update-golden` の手順を README に記載する。
    - 6.1.c `docs/spec/1-0-language-core-overview.md` と `docs/spec/3-0-core-library-overview.md` に診断基盤の概要更新を反映させる。
6.2. `README.md`/`3-0-phase3-self-host.md` に Diagnostics 実装ステータスと活用ガイドを追記する。
    - 6.2.a `README.md` の章一覧に「3.6 Core Diagnostics & Audit」を追加し、ステータスバッジを図示する。
    - 6.2.b `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` の Phase 3 条件テーブルへ診断実装の完了チェックを追加し、CI 連動条件を記述する。
    - 6.2.c `docs/plans/rust-migration/overview.md` の P3 ドキュメント一覧に診断実装ガイドが存在する旨を脚注でリンクする。
6.3. `docs/guides/ai-integration.md`/`docs/guides/runtime-bridges.md` の診断・監査セクションを更新する。
    - 6.3.a AI ガイドでは診断 JSON の解釈手順と `effect.stage.*` の注意点を解説する段落を追加する。
    - 6.3.b Runtime Bridges ガイドでは `AuditEnvelope` の `bridge.stage.*` 例を載せ、プラグイン開発者が参照できるようにする。
    - 6.3.c 更新後に `docs/notes/dsl-plugin-roadmap.md` へ交差参照を追記し、プラグイン昇格審査と診断監査が同じ証跡を共有することを明記する。

### 7. テスト・CI 統合（53週目）
**担当領域**: 品質保証

7.1. 単体・統合テストを追加し、診断構造のシリアライズ/デシリアライズが安定していることを確認する。
    - 7.1.a `compiler/rust/frontend/tests/json_roundtrip.rs` に `serde_json` を用いた round-trip テストを追加し、互換性を保証する。
    - 7.1.b `tests/cli/test_diagnostics.rs` を作成し、CLI 出力（human/json）両方の snapshot を `insta` で管理する。
    - 7.1.c `tooling/lsp/tests/client_compat/diagnostics_roundtrip.json` を更新し、LSP 側でも同じ round-trip が成立することを確認する。
7.2. 監査ログ出力のスナップショットテストと GDPR/Privacy フラグの検証を行う。
    - 7.2.a `compiler/rust/runtime/tests/audit_snapshot.rs` を追加し、`AuditEnvelope` の `privacy.redacted` の有無で差分が出るかチェックする。
    - 7.2.b `reports/audit/privacy/` に GDPR テストケースのログを保存し、`0-4-risk-handling.md` の法務項目とリンクする。
    - 7.2.c `scripts/validate-diagnostic-json.sh --suite audit --require-privacy` を CI に追加し、`privacy` フラグ欠落時は即座に失敗させる。
7.3. CI に診断差分検出タスクを追加し、回帰時に `0-4-risk-handling.md` へ自動記録する仕組みを構築する。
    - 7.3.a `.github/workflows/rust-frontend.yml` に `diagnostic_diff` ジョブを追加し、`reports/dual-write/*` をアーティファクト化する。
    - 7.3.b 差分検出スクリプトを `tooling/ci/diagnostic_diff.py` として実装し、失敗時に `reports/audit/dashboard/ci-regressions.md` を生成する。
    - 7.3.c CI 失敗時に `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` を自動更新できるよう `scripts/ci/risk_append_entry.sh` を用意し、エントリフォーマットを固定する。

### 8. Core Collections effect 連携（39週目）
**担当領域**: `collector.effect.cell` / `collector.effect.rc` / `collector.effect.mem` / `collector.effect.audit`

8.1. `compiler/rust/runtime/src/collections/mutable/{cell,ref,table,vec}.rs` で追加された Effectful コンテナを `reports/spec-audit/ch1/core_iter_collectors.json` へ流し込み、`collector.effect.cell` / `collector.effect.rc` / `collector.effect.mem` / `collector.effect.audit` の必須フィールドを `scripts/validate-diagnostic-json.sh --suite collectors --pattern collector.effect.cell --pattern collector.effect.rc` で検証する。
    - 8.1.a `collector.effect.*` で生成された JSON キー一覧を `scripts/tools/list_collector_keys.py` で収集し、`docs/plans/bootstrap-roadmap/assets/collector-effect-keys.md` に保存する。
    - 8.1.b `reports/spec-audit/ch1/core_iter_collectors.json` を生成する際、`--with-stage` フラグで Stage 情報を含め、`effect.stage.*` が欠けていないか自動チェックを追加する。
    - 8.1.c バリデーション結果を `docs/plans/bootstrap-roadmap/3-2-core-collections-plan.md` の進捗テーブルに反映し、実験項目は Stage=experiment を明示する。
8.2. `collect-iterator-audit-metrics.py --suite collectors --scenario ref_internal_mutation` / `--scenario table_csv_import` を CI に組み込み、`--require-success --require-cell` を指定して `cell_mutations_total`・`ref_borrow_conflict_rate`・`table_insert_throughput`・`csv_load_latency` の KPI を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` / `reports/iterator-collector-summary.md` と `docs/plans/bootstrap-roadmap/assets/metrics/core_collections_persistent.csv`（`RefInternalMutation` 行）に同期させる。
    - 8.2.a KPI の閾値を `docs/plans/bootstrap-roadmap/assets/metrics/core_collections_thresholds.json` として定義し、CI で逸脱した場合にコメントを残すスクリプトを追加する。
    - 8.2.b `reports/iterator-collector-summary.md` の最新 Run ID を列挙し、`collect-iterator-audit-metrics.py --emit-run-id` の出力を自動追記する。
    - 8.2.c `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に collectors KPI のアラート条件を追記し、Phase 3 ハンドオーバー時の必須証跡とする。
8.3. `docs/plans/bootstrap-roadmap/3-2-core-collections-plan.md` の `Cell<T>`/`Ref<T>` 内部可変性セクションと `docs-migrations.log` に記録された `Cell/Ref effect trace` を相互にリンクし、`collect-iterator-audit-metrics.py --require-cell` と `scripts/validate-diagnostic-json.sh --suite collectors` を監査 CI の gate とすることで Effectful コンテナ導入タイミングを明示する。
    - 8.3.a `docs-migrations.log` に `Cell/Ref effect trace` 更新日時と Run ID を追記し、参照元を `reports/spec-audit/ch1/` にリンクする。
    - 8.3.b `docs/plans/bootstrap-roadmap/3-2-core-collections-plan.md#cell-ref-内部可変性` に監査 CI gate の手順を追加し、Diagnostics 計画との依存関係を明文化する。
    - 8.3.c `collect-iterator-audit-metrics.py` と `scripts/validate-diagnostic-json.sh` の実行結果をまとめた `reports/audit/dashboard/collectors-YYYYMMDD.md` を作成し、Phase 3 ドキュメントから参照できるようにする。

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
