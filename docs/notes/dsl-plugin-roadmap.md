# DSLプラグイン提供ロードマップ

Reml の DSL ファースト戦略に沿って、プラグイン領域で提供する拡張機能の設計・提供計画をまとめる。公式仕様としては [4-7 Core.Parse.Plugin](../spec/4-7-core-parse-plugin.md) に DSL プラグイン契約を掲載済みであり、本メモは提供ロードマップ・優先順位の追跡を目的とする。

## 1. DSLテンプレート／ジェネレーター

### 1.1 目的

- 小規模プロジェクトでも DSL ファーストアプローチを導入しやすくする。
- Core.Parse と Conductor 構文を最小セットで体験できるスケルトンを生成する。

### 1.2 提供物

- `reml-plugin-dsl-template`（CLI）：`dsl init --kind config` などのサブコマンドで各種テンプレートを生成。
- テンプレート内容：
  - Conductor ブロックを含むメインファイル
  - Core.Parse を利用した基本的な `rule` サンプル
  - Core.Diagnostics 連携済みの監視設定
  - プロジェクト構成案（tests/、guides/ への導線）

### 1.3 実装ステップ

1. **アルファ版 (0-1ヶ月)**: JSON/CSV 向けテンプレートと README を同梱。
2. **ベータ版 (1-2ヶ月)**: テンプレート選択肢の拡充（設定DSL、ETL DSLなど）と CI ワークフロー。
3. **安定版 (2-3ヶ月)**: モジュール式テンプレート（DSL断片の追加インストール）をサポート。

### 1.4 エコシステム連携

- guides/ に設置する各ガイドと相互リンク。
- Capability Registry のサンプル設定を `templates/runtime.toml` として配布。
- 公式プラグインカタログに登録し、`reml plugin install` から入手可能にする。

## 2. 運用監視／Circuit Breaker 拡張

### 2.1 目的

- Conductor 実行中の DSL に対する可観測性とフォールトトレランス機構を提供。
- Core.Async と Core.Diagnostics の橋渡し機能をパッケージ化する。

### 2.2 提供物

- `reml-plugin-dsl-observability`：
  - DSLメトリクス登録のヘルパー（`ExecutionMetricsScope` を受け取る `register_dsl_metrics` のデフォルト実装）
  - Grafana / OpenTelemetry へのエクスポータ設定
  - Circuit Breaker ポリシーを YAML/Reml から読み込むユーティリティ
- `reml-plugin-dsl-fallback`：
  - ExecutionPlan のエラーポリシーを動的に差し替えるランタイムフック
  - カウンタ付き Circuit Breaker の参照実装（半開制御、フェイルファスト）

### 2.3 実装ステップ

1. **PoC (0-1ヶ月)**: DSLメトリクスの自動登録とエラー率の閾値判定を実装。
2. **拡張 (1-3ヶ月)**: Circuit Breaker と Backpressure ポリシー可視化を統合。
3. **安定化 (3-4ヶ月)**: 複数環境（オンプレ／クラウド）向け設定プリセットとドキュメント整備。

### 2.4 依存と配布

- Core.Diagnostics 3.6 で定義した `DslMetricsHandle`, `start_dsl_span` を前提とする。
- Core.Async 3.9 の `ExecutionPlan` API と互換性を維持。
- プラグイン配布は公式レジストリ経由。インストール後に `plugins/dsl-observability/README.md` を展開し、環境変数設定例を案内。

## 3. ネイティブUI / 通知拡張

### 3.1 目的
- 各プラットフォーム（Windows/Mac/Linux）で共通化された通知・ダイアログ API を提供し、DSL 実行結果をユーザー UI と連携させる。
- Core.Platform と Capability Registry を活用し、UI 機能をプラグイン経由で opt-in させる。

### 3.2 提供物
- `reml-plugin-native-ui`：
  - `NativeUI` トレイト（ウィンドウ生成・ダイアログ表示・通知）とプラットフォーム別バックエンド実装。
  - `@cfg(target_os = ...)` に対応したデフォルト設定とフォールバック（CLI モードでは no-op）。
- `reml-plugin-notification-hub`：
  - DSL 実行イベントを OS 通知センターや Webhook に転送するユーティリティ。
  - `docs/guides/runtime-bridges.md` の通知連携セクションと連動した設定テンプレート。

### 3.3 実装ステップ
1. **PoC (0-1ヶ月)**:
   - Win32 API を利用したトースト通知（`Shell_NotifyIcon`）と macOS Cocoa の `NSUserNotification` を実装
   - CLI モードでは stdout ログと LSP 診断にフォールバック
   - WASM / サーバーレス環境では自動的に no-op となることを `@cfg` で保証
2. **拡張 (1-3ヶ月)**:
   - Linux (GTK/DBus) 通知、Webhook 連携、DSL エラー通知テンプレート
   - プラットフォームごとのテーマ設定とホットリロード対応
3. **安定化 (3-4ヶ月)**:
   - i18n メッセージ、利用権限チェック、IDE 連携サンプル（VS Code / JetBrains）

### 3.4 依存と配布
- Core.Platform 3.8 の `platform_info()` を利用してバックエンドを自動選択。
- Core.Env の設定値（通知エンドポイント、UI モード）を読み込むユーティリティを同梱。
- プラグイン配布は公式レジストリ経由で行い、VS Code など IDE 連携を想定した設定ガイドを付属。

## 4. 共通ガバナンス

- バージョン管理: Reml 本体とは独立した `reml-plugins` モノレポで管理、Semantic Versioning 適用。
- 互換性保証: Reml 本体のマイナーバージョンに同期して互換テストを実行。
- 監査対応: プラグイン経由で取得したテレメトリは Core.Diagnostics のポリシーに従い、匿名化と保持期間を設定。

## 5. 効果ハンドリング比較マトリクス {#effect-handling-matrix}

Reml の効果システムを他言語と比較し、Capability とステージ管理の指針を整理する。ここで得た結論は [1-3-effects-safety.md §I.5](../spec/1-3-effects-safety.md#effect-line-ordering) と [3-8-core-runtime-capability.md §1.2](../spec/3-8-core-runtime-capability.md#capability-stage-contract) に反映済み。

| 言語/実装 | ハンドラ探索順序 | ステージ/安定度管理 | 診断・Capability 連携 | Reml への示唆 |
| --- | --- | --- | --- | --- |
| **Reml** | 動的スコープで最内ハンドラから外側へ探索。`resume` は呼び出し順に再入。 | `stage ∈ {Experimental, Beta, Stable}` を Capability Registry で検証。 | `effects.contract.*` 診断と AuditCapability のシンクを連携。 | 効果行整列基準と Stage 検査を仕様に明記。 |
| **Koka 2** | ハンドラは外側優先（call-by-value）。行多相型で残余効果を静的追跡。 | 研究版は Stable 区分なし。 | 編注に留まり公式診断は限定的。 | Stage と Capability の分離によって OSS/商用の判断材料を明示。 |
| **Eff / Multicore OCaml** | 最内ハンドラが優先。`perform` は評価途中で中断し Stack を巻き戻す。 | `experimental` ブランチで段階管理。 | 効果安全性はドキュメント準拠。 | Reml では診断コード化とハンドラ整列規約を追加。 |
| **Rust generators (try blocks)** | `?` による早期戻りでエフェクト類似挙動。ハンドラは存在せず型で合成。 | 安定化済み。 | `?` 使用箇所は警告/エラー診断。 | Result ベースの整列規約と組み合わせて hybrid DSL を設計。 |

### 5.1 今後のアクション

- `effects.contract.stage_mismatch` 診断コードを Core.Diagnostics へ追加し、Capability 監査ログとリンクする。
- `@handles` の再配置を検出する LSP アシストを PoC 実装し、効果行整列提案を自動化する。
- Stage 情報を `reml.toml` の `dsl.expect_effects` に書き戻し、CI で `--deny experimental` を活用できるよう CLI を拡張する。

## 6. 仕様統合ステータス（2024-Q4）

- DSL プラグイン契約は [4-7-core-parse-plugin.md](../spec/4-7-core-parse-plugin.md) へ移管済み。ガイドはベストプラクティスとテンプレート配布に集中し、仕様差分は `docs/guides/DSL-plugin.md` から削除した。
- Capability Stage と監査要件は [3-8-core-runtime-capability.md](../spec/3-8-core-runtime-capability.md) §1.2 / §10 に統合され、`RuntimeCapability::ExternalBridge` を通じてブリッジ経由の拡張 Capability も同じポリシーで検証する。
- Runtime Bridge の診断コード (`bridge.contract.*`, `bridge.target.mismatch` など) と監査メタデータは [3-6-core-diagnostics-audit.md](../spec/3-6-core-diagnostics-audit.md) §8 に整理済み。プラグイン提供側は `checklist_missing` を監査チェックリストのエビデンスとして提出すること。
- `docs/notes/guides-to-spec-integration-plan.md` §4 に掲げたガイド→仕様の移管作業は完了。今後の更新は本ロードマップでステージ昇格と監査テンプレートの整合確認に集中する。
- Phase 2-5 DIAG-003 Step5 で `DiagnosticDomain::Plugin` / `::Lsp` の語彙整備と `AuditEnvelope.metadata["plugin.bundle_id"]` 連携が完了し、仕様・ガイド・CI の同期状況を脚注に固定した[^diag003-phase25-plugin-roadmap]。

## 7. Unicode プロファイル導入準備（Phase 2-5 Step3）

### 7.1 暫定ポリシー

- Phase 2-5 ではプラグイン ID・Capability 別名の公式表記を **ASCII 限定** とし、`identifier(profile=DefaultId)` を運用で使用しないことを開発者ガイドに明記する。[^lexer001-phase25]
- Unicode 別名の利用を希望する場合は `bundle.alias` を宣言せず、Phase 2-7 `lexer-unicode` が完了するまで CLI/LSP の補助表示に留める。
- プラグイン配布時は `plugin.toml` の `id` / `capabilities` / `targets` を ASCII で登録し、別名を README などドキュメント側で案内する。
- CI/CLI は `--lex-profile=ascii|unicode` スイッチでプロファイルを切り替える。Phase 2-5 中は `--lex-profile=ascii` を既定とし、Unicode リハーサルでは `--lex-profile=unicode` を指定して差分メトリクス（`lexer.identifier_profile_unicode`）を確認する。

### 7.2 Stage / Capability 連携

- Capability Stage 監査では ASCII ID のまま `StageRequirement::{Exact, AtLeast}` を評価し、Unicode 別名を導入する際は Stage 表を一括更新する計画とする（参照: `docs/spec/3-8-core-runtime-capability.md` §1.2）。  
- `Runtime Bridge` 連携時は [docs/guides/runtime-bridges.md](../guides/runtime-bridges.md) の Capability 検証フローを流用し、ASCII エイリアスのみを CI 指標 (`diagnostics.effect_stage_consistency`) に集計する。  
- `compiler/ocaml/docs/technical-debt.md` の ID22/ID23（Windows Capability Stage / macOS FFI 検証）で管理している監査ログ更新と矛盾しないよう、ASCII 限定の間は追加ステージやエイリアスの登録を行わない。Unicode 別名導入時には同 ID のフォローアップ項目へ再調整を記録する。

### 7.3 TODO / フォローアップ

- TODO: Phase 2-7 `lexer-unicode` 着手時に `plugin.toml`/`Capability Registry` の許容文字集合を再検証し、`collect-iterator-audit-metrics.py` の `lexer.identifier_profile_unicode` を PASS 状態へ更新する（記録先: `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` Unicode 行）。  
- TODO: CLI/LSP で Unicode 代替表示を開始する前に、`docs/guides/plugin-authoring.md` と VS Code 拡張の両方でエイリアス選択フローを同期する。  
- Phase 2-7 以降で `identifier(profile=...)` を公開 API として扱う際は、`Core.Parse.Plugin` の `RunConfig.extensions["lex"]` へプロファイル名を渡す新しいテンプレートを提供する。

[^diag003-phase25-plugin-roadmap]: Phase 2-5 DIAG-003 診断ドメイン語彙拡張計画（`docs/plans/bootstrap-roadmap/2-5-proposals/DIAG-003-proposal.md`）Step5（2025-11-30 完了）で `Plugin` / `Lsp` ドメインの監査キー（`extensions["plugin"]`, `extensions["lsp"]`, `AuditEnvelope.metadata["plugin.bundle_id"]` など）を仕様・実装・CI 指標へ統一し、`docs/spec/3-6-core-diagnostics-audit.md`, `docs/spec/3-8-core-runtime-capability.md`, `docs/guides/runtime-bridges.md` と同期させた。後続のダッシュボード整備は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` の TODO で追跡する。

[^lexer001-phase25]: `docs/plans/bootstrap-roadmap/2-5-proposals/LEXER-001-proposal.md` Phase 2-5 Step3（DSL/プラグインチーム連携）と Step4（測定指標・CI スイッチ定義）で策定した ASCII 限定ポリシーと Unicode プロファイル導入ロードマップ。
