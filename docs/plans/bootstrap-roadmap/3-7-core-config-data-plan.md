# 3.7 Core Config & Data 実装計画

## 目的
- 仕様 [3-7-core-config-data.md](../../spec/3-7-core-config-data.md) に準拠した `Core.Config`/`Core.Data` API を Reml 実装へ統合し、マニフェスト・スキーマ・差分管理の標準モデルを確立する。
- `reml.toml` マニフェスト、DSL エクスポート情報、互換性プロファイル、データモデリング (`Schema`, `ChangeSet`) を実装し、監査・診断と連携する。
- Config 互換性ポリシーとステージ管理を提供し、Phase 4 の移行計画へ滑らかに接続する。
- すべての実装・テストは Rust 版 Reml コンパイラ（`compiler/rust/`）を対象とし、OCaml 実装は過去の仕様確認に限って参照する。

## スコープ
- **含む**: Manifest ロード/検証、DSL エクスポート連携、Schema/Manifest API、ConfigCompatibility 設定、差分・監査 API、ドキュメント更新。
- **含まない**: 外部レジストリ連携のネットワークコード、マイグレーションツール自動生成 (Phase 4 で扱う)。
- **前提**: Core.Collections/Diagnostics/IO/Numeric が整備済みであり、Phase 2 の仕様差分解決タスクが完了していること。

## 作業ブレークダウン

### 1. API 差分整理と構造設計（53週目）
**担当領域**: 設計調整

1.1. Manifest/Schema/Data API の公開リストを作成し、既存実装との差分・未実装項目を洗い出す。
    - `docs/spec/3-7-core-config-data.md` の API 一覧と型表を抽出し、`compiler/rust/core/config/` 配下の既存ファイル（`manifest.rs`, `schema.rs`, `compat.rs` 等）と照合した一覧表を本計画書末尾（Appendix: API Matrix）へ追記する。
    - `rg "pub" compiler/rust/core/config -n` などで公開 API を列挙し、未定義のシグネチャは TODO コメント付きで仮スタブを作成、進捗を `reports/spec-audit/ch3/config_data_api_diff.md` へログ化する。
    - API マッピング表について、Phase 2 以前の OCaml 実装 (`compiler/ocaml/runtime/config/`) との乖離がある場合は diff を取り、`docs/plans/rust-migration/appendix/glossary-alignment.md` の該当語彙に脚注を追加する。
1.2. 効果タグ (`effect {config}`, `{audit}`, `{io}`, `{migration}`) と `Diagnostic` との連携ポイントを整理する。
    - `docs/spec/3-6-core-diagnostics-audit.md` の `config.*` 診断キーと照らし合わせ、各タグがどの `DiagnosticBuilder` で発火するかをシーケンス図に落とし込み、`reports/spec-audit/ch3/config_effects-trace.md` へ保存する。
    - `compiler/rust/frontend/src/diagnostics/` で既存の `effect` 連携を調査し、Config/Data 追加分の Hook (`emit_config_error`, `attach_audit_metadata` 等) を洗い出してタスクリストへ登録する。
    - `effect {migration}` に紐づく監査ログ項目を `docs/spec/3-8-core-runtime-capability.md` §10 と比較し、欠落しているメタデータキーを `docs/notes/dsl-plugin-roadmap.md` へ TODO として書き出す。
1.3. Manifest/Schema のシリアライズ形式 (TOML/JSON) とバリデーション順序を仕様と照合する。
    - `examples/` 配下の `reml.toml` / `schema.reml` サンプルを収集してリスト化し、`serde` / `toml_edit` / `serde_json` のどれを採用するか決定メモを本計画書末尾（Serialization Decision Log）として残す。
    - ロード→正規化→検証→差分出力の順序をシーケンシャルに書き出し、各フェーズで必要な型（`ManifestBuilder`, `SchemaValidator`, `ChangeSetEmitter` 等）を割り当てる。
    - TOML/JSON 変換で許容するコメント・未使用キーの挙動を `docs/spec/3-7-core-config-data.md` へフィードバックするため、発見事項は `docs/notes/core-library-outline.md` に暫定記録する。

### 2. Manifest モジュール実装（53-54週目）
**担当領域**: `reml.toml`

2.1. `Manifest`/`ProjectSection`/`DslEntry`/`BuildSection` 等のデータ構造を実装する。
    - `compiler/rust/core/config/manifest.rs` に `serde::Deserialize`/`Serialize` を実装し、`Default`/`Builder` パターンでスキーマ未記載フィールドを補完できるようにする。
    - `docs/spec/3-7-core-config-data.md` の表を基に `ProjectSection::{kind, stage, capabilities}` と `BuildSection::{targets, profiles}` を列挙型で表現し、未知の値に備えた `Unknown(String)` バリアントを用意する。
    - `remlc manifest dump --format json` の暫定 CLI を作り、デシリアライズ結果を `compiler/rust/frontend/tests/fixtures/manifest/*.json` へゴールデン化する。
2.2. `load_manifest`/`validate_manifest`/`declared_effects`/`update_dsl_signature` 等の API を実装し、エラー時に `Diagnostic` を返す仕組みを整備する。
    - ファイル IO と `Diagnostic` の橋渡しを行う `ManifestLoader` を `effect {io,config}` でタグ付けし、`Result<Manifest, Diagnostic>` を返す共通エントリポイントを `compiler/rust/core/config/mod.rs` に配置する。
    - バリデーションでは `spec/3-6-core-diagnostics-audit.md` の `config.missing_field`/`config.invalid_stage` 等のコードを使用し、`core_iter_collectors.snap` を更新してスナップショットテストで検証する。
    - DSL シグネチャ更新 (`update_dsl_signature`) は `compiler/rust/frontend/src/dsl/export.rs` と連携させ、`@dsl_export` 注釈から得た情報を Manifest 側へ逆反映させる手順を `docs/guides/plugin-authoring.md` にも記載する。
2.3. DSL エクスポートシグネチャとの同期 (`@dsl_export`) を確認し、Capability/Stage 情報が正しく投影されることをテストする。
    - `compiler/rust/frontend/tests/core_iter_collectors.rs` のような既存テストを流用し、Manifest→DSL→Capability の往復経路を `cargo test core_iter_collectors -- --nocapture` で再現し、監査ログ (`reports/spec-audit/ch1/core_iter_collectors.audit.jsonl`) を比較する。
    - Stage 情報は `RuntimeBridgeAuditSpec` のキー（`bridge.stage.required`, `bridge.stage.actual`）が埋まるかを確認し、不足時は `docs/spec/3-8-core-runtime-capability.md` との整合タスクを追加する。
    - `docs/guides/plugin-authoring.md` と `docs/guides/runtime-bridges.md` に `@dsl_export` × Manifest 連携例を掲載し、サンプル `examples/core_config/` を用意する。

### 3. Schema & ConfigCompatibility 実装（54週目）
**担当領域**: データモデリング

3.1. `Schema`/`Field`/`ValidationRule` など Core.Data の主要構造を実装し、差分 (`SchemaDiff`) の出力を確認する。
    - `compiler/rust/core/data/schema.rs` に `FieldKind`, `Constraint`, `Collection` などの enum/struct を実装し、`serde_json::Value` ベースの `SchemaExample` を保持できるようにする。
    - `SchemaDiff` 生成用に `diff::Changeset` もしくは独自トラバースロジックを追加し、差分結果を `reports/spec-audit/ch3/schema_diff-YYYYMMDD.md` として保存するスクリプトを `tooling/scripts/generate-schema-diff.sh` へ追加する。
    - `docs/spec/3-7-core-config-data.md` §5 の例を `compiler/rust/frontend/tests/schema_diff.rs` に再現し、差分出力が仕様の疑似コードと一致するか確認する。
3.2. `ConfigCompatibility` 設定 (`trailing_comma`, `unquoted_key`, `duplicate_key` 等) を実装し、フォーマット別既定値をテストする。
    - `compiler/rust/core/config/compat.rs` に `ConfigCompatibility` 構造体と `CompatibilityProfile` 列挙を定義し、`serde(default)` と `#[cfg(feature = "json")]` 等でフォーマット差を整理する。
    - `tests/config_compat.rs` を新設し、TOML/JSON の代表ケースを `insta` スナップショットで比較、`compatibility_profile("strict")` 等のヘルパーを通じて期待値を固定する。
    - 互換性違反時の挙動を `Diagnostic.code = "config.compat.*"` 系で統一し、`docs/spec/3-6-core-diagnostics-audit.md` のエントリへリンクを追加する。
3.3. `compatibility_profile`/`resolve_compat` 等の API を実装し、Manifest/RunConfig からの利用を確認する。
    - Manifest → `RunConfig` 伝搬の制御フロー図を作成し、`compiler/rust/runtime/run_config.rs` で `ConfigCompatibility` が `Core.Env` へ渡ることを確認する。
    - `resolve_compat` は `Result<CompatibilitySet, Diagnostic>` を返すよう統一し、実験的フラグは環境変数 `REML_CONFIG_EXPERIMENTAL` で切り替え可能にする。
    - CLI (`reml config lint`) と LSP で互換性設定が反映されることを `docs/guides/ai-integration.md` §6 のシナリオで QA し、結果を `reports/spec-audit/ch3/config_compatibility-lsp.md` に記録する。

### 4. 差分・監査・診断連携（54-55週目）
**担当領域**: Quality & Audit

4.1. `ChangeSet` や `AuditEvent::ConfigCompatChanged` の発火条件を実装し、監査ログと連携する。
    - `compiler/rust/core/data/change_set.rs` に `ChangeSet` 構造体を実装し、Manifest/Schema の変更種別（追加/削除/更新）と `severity` を記録するフィールドを追加する。
    - `AuditEvent::ConfigCompatChanged` が発火する条件を `Core.DiagnosticsAudit` 仕様に従い pseudo code 化し、`reports/audit/dashboard/collectors-20251203.json` を使用してメトリクス化する。
    - 監査ログの整合テストを `compiler/rust/frontend/tests/__snapshots__/core_iter_collectors.snap` に追加し、`cargo insta review` で差分を承認するフローを手順化する。
4.2. Config 解析エラー (`Diagnostic.code = "config.*"`) のテンプレートとメタデータを実装し、LSP/CLI 出力を確認する。
    - `compiler/rust/frontend/src/diagnostics/messages/config.rs` を新設し、`config.missing_manifest`, `config.schema_mismatch`, `config.compat.unsupported` などのテンプレートを定義する。
    - `scripts/validate-diagnostic-json.sh` に Config セクションを追加し、`reports/spec-audit/ch3/config_diagnostics-YYYYMMDD.json` を生成して LSP/CLI 表示と突き合わせる。
    - `docs/notes/spec-integrity-audit-checklist.md` に Config 診断のカバレッジ行を追加し、リリース判定基準へ `config_diagnostics_pass_rate >= 0.95` を設定する。
4.3. `RunConfig` との連携 API を整備し、`Core.Env`/`Core.Runtime` との接合をテストする。
    - `compiler/rust/runtime/run_config.rs` に Config/Data 由来の値を注入する `apply_manifest_overrides` を実装し、`Core.Env` のステージ情報と同期を取る。
    - `runtime/tests/run_config_integration.rs` を追加し、`reml run --config fixtures/*.toml` の挙動を再現して CLI 経由の統合テストを実施する。
    - `docs/spec/3-10-core-env.md` と `docs/spec/3-8-core-runtime-capability.md` の関連節へ参照リンクを追記し、Config/Data/API の接合を明文化する。

### 5. データ互換性・マイグレーション支援（55週目）
**担当領域**: 将来互換

5.1. `MigrationPlan`/`MigrationStep` (仕様に記載された実験的 API) のドラフトを実装し、`effect {migration}` の扱いを定義する。
    - `compiler/rust/core/config/migration.rs` を新設し、`MigrationPlan` の JSON/TOML 表現を定義、`#[cfg(feature = "experimental-migration")]` で囲んだ上でドキュメントに opt-in 手順を記す。
    - `effect {migration}` を `docs/spec/1-3-effects-safety.md` の効果テーブルへ追加する提案メモを作成し、`docs/plans/bootstrap-roadmap/3-6-core-diagnostics-audit-plan.md` に連携項目として貼る。
    - `reports/spec-audit/ch3/migration_plan-pilot.md` に PoC 実行ログ（Manifest 差分→MigrationPlan 生成→ChangeSet 適用）を残し、Phase 4 実装者に引き継ぐ。
5.2. Manifest/Schema のバージョン互換チェックを追加し、移行シナリオを `docs/notes/dsl-plugin-roadmap.md` に記録する。
    - `Manifest.version` と `Schema.version` を比較し、互換条件（`major` は一致、`minor` は `>=` など）を `docs/spec/3-7-core-config-data.md` に追記するための diff を準備する。
    - 互換性チェックの結果を `reports/dual-write/config_versioning/` に保存し、DSL プラグイン毎の移行ステップを `dsl-plugin-roadmap.md` に表形式で追記する。
    - 重大な非互換が検出された場合は `docs/notes/dsl-plugin-roadmap.md` の TODO セクションへ `MIGRATION-BLOCKER-*` 番号で登録し、Phase 4 のタスクインプットにする。
5.3. CLI 連携 (`reml config lint`, `reml config diff`) の出力仕様を整備し、サンプルを作成する。
    - `compiler/rust/cli/src/commands/config.rs` に `lint`/`diff` サブコマンドを追加し、`ChangeSet` との整合を `snap` テストで担保する。
    - `docs/guides/runtime-bridges.md` と `docs/guides/ai-integration.md` に CLI 出力例を掲載し、JSON/TTY 両方のサンプルを示す。
    - `examples/core_config/cli/` に最小構成の Manifest/Schema を配置し、`scripts/run_examples.sh --suite core_config` で検証できるようにする。

### 6. ドキュメント・サンプル更新（55-56週目）
**担当領域**: 情報整備

6.1. 仕様書内の表・サンプルを実装に合わせて更新し、`examples/` に Manifest/Schema 例を追加する（Core.Text ガイド更新時は `docs/guides/core-parse-streaming.md` §10 と整合させる）。
    - `docs/spec/3-7-core-config-data.md` の章末サンプルを最新 API で書き換え、`git grep "Manifest::"` で旧記法を洗い出して一括更新する。
    - `examples/core_config/basic_manifest/` を新設し、`README.md` 付きで `reml config lint` の結果を貼る。`tooling/examples/run_examples.sh --suite core_config --update-golden` 手順を説明書として追記する。
    - Core.Text ガイドとの整合確認は `docs/guides/core-parse-streaming.md` §10 の参照リンクを更新し、Config/Data 例との相互リンクを README に追加する。
6.2. `README.md`/`3-0-phase3-self-host.md` に Config/Data 実装状況を記載し、Phase 4 への連携点をまとめる（AI 入力ポリシーの共有は `docs/guides/ai-integration.md` §6 と同期）。
    - 進捗サマリを `README.md#phase-3-bootstrap-roadmap` に箇条書きで追記し、完了/進行中のモジュールを色分け Legend で説明する。
    - `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` のマイルストーン表に Config/Data 行を追加し、Phase 4 との依存（MigrationPlan, Registry 連携）を注記で示す。
    - AI 連携ポリシー ( `docs/guides/ai-integration.md` §6 ) に Config/Data API の利用例を追記し、AI ツールへ Manifest 情報を渡す際の制約をまとめる。
6.3. `docs/guides/runtime-bridges.md`/`docs/guides/plugin-authoring.md` 等で設定連携の記述を更新する。
    - runtime bridge ガイドに Config/Data の流れ図を追加し、`RuntimeBridgeRegistry` が Manifest 由来の Capability を参照するステップを説明する。
    - plugin authoring ガイドで `@dsl_export` → Manifest → Schema → CLI/LSP という手順をケーススタディとしてまとめ、ステージ別注意点を列挙する。
    - ガイド更新後は `docs/notes/dsl-plugin-roadmap.md` へ変更ログをリンクし、再編履歴 (`docs-migrations.log`) に章番号と日付を記載する。

### 7. テスト・CI 統合（56週目）
**担当領域**: 品質保証

7.1. Manifest/Schema の単体・統合テストを追加し、バリデーションエラーや互換性チェックのケースを網羅する。
    - `compiler/rust/core/config/tests/manifest_validation.rs` に 10 ケース以上のフォールトテーブル（欠落フィールド、型不一致、ステージ矛盾等）を実装し、`insta` で `Diagnostic` を固定する。
    - 統合テストでは `cargo test -p reml_frontend manifest::integration --features schema` を新設し、CLI を通じて Manifest→Schema→RunConfig の一連フローを検証する。
    - `docs/spec/0-4-risk-handling.md` のチェックリストへ Config/Data テストのカバレッジ指標 (`config_validation_cases >= 10`) を追記する。
7.2. 差分出力のスナップショットテストと監査ログ検証を行う。
    - `compiler/rust/frontend/tests/__snapshots__/core_iter_collectors.snap` を更新し、Config/Data 関連差分が追加された際のレビュー手順を `reports/spec-audit/ch1/core_iter_collectors.audit.jsonl` に記載する。
    - `tooling/scripts/verify-config-diff.sh` を用意し、`ChangeSet` 出力を JSON で比較、リグレッション発生時は `reports/dual-write/config_diff/YYYYMMDD.md` へ自動保存する。
    - 監査ログ検証は `python3 tooling/ci/collect-iterator-audit-metrics.py --section config` を追加し、CI 失敗時に該当ログがアーティファクト化されるようにする。
7.3. CI へ Config Lint を組み込み、回帰時に `0-4-risk-handling.md` へ自動記録する。
    - `.github/workflows/rust-frontend.yml` に `reml config lint --manifest examples/core_config/basic_manifest/reml.toml` を追加し、成果物として `lint-report.json` をアップロードする。
    - 失敗時は `docs/plans/bootstrap-roadmap/0-4-risk-handling.md#config-data` へ Issue 番号を追記する GitHub Actions を実装し、既存の Diagnostics 連携と同じテンプレートを用いる。
    - CI 追加後は `README.md` のバッジや `docs/plans/rust-migration/3-0-ci-and-dual-write-strategy.md` のフローチャートにも Config Lint ジョブを追記する。

## 成果物と検証
- Manifest/Schema/ConfigCompatibility API が仕様通りに実装され、効果タグ・診断・監査が整合すること。
- DSL エクスポート・Capability 情報がマニフェストから取得でき、Phase 4 の移行処理に再利用できること。
- ドキュメント・サンプルが更新され、設定ファイルの互換性ポリシーが明確であること。

## リスクとフォローアップ
- TOML/JSON パーサの差異で互換性チェックが不安定な場合、フォーマット別に冪等テストを追加し、必要なら構文制限を仕様側へ提案する。
- Migration API が未成熟な場合、Phase 4 で段階的導入する前提で `docs/notes/` に TODO を残す。
- レジストリ連携で追加機能が必要になった場合、`docs/notes/dsl-plugin-roadmap.md` に記録し、エコシステム計画 (5-x) と調整する。

## Appendix（更新指針）
- **API Matrix**: `docs/spec/3-7-core-config-data.md` と Rust/OCaml 実装の API 対応表をここで管理し、章番号・ファイルパスを列に持たせる。
- **Serialization Decision Log**: TOML/JSON/独自表現に関する選定理由、採用ライブラリ、互換性リスクを時系列で記録する。

## 参考資料
- [3-7-core-config-data.md](../../spec/3-7-core-config-data.md)
- [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md)
- [3-4-core-numeric-time.md](../../spec/3-4-core-numeric-time.md)
- [3-5-core-io-path.md](../../spec/3-5-core-io-path.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- [notes/dsl-plugin-roadmap.md](../../notes/dsl-plugin-roadmap.md)
