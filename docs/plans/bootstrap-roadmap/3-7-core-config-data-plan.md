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

#### 1.1 実施結果（Run ID: 20251203-config-api-diff）
- `reports/spec-audit/ch3/config_data_api_diff.md` に仕様 §1〜§5 で定義された API と `compiler/rust/runtime/src/config/{mod.rs,collection_diff.rs}` の `grep -R "pub "` 結果を整理した表を追加し、Manifest/Schema/Compatibility/Migration API が全て未実装であること、唯一存在する `ChangeSet` ラッパも Core.Collections 依存で Config/Data の効果タグを欠いていることを明示した。
- Appendix の **API Matrix** を更新し、仕様で要求される 9 カテゴリの API を `ステータス`（未着手/一部/既存）と `主要差分` 付きで列挙した。Rust 実装が存在するのは `merge_maps_with_audit` と `SchemaDiff` ラッパのみであり、Manifest/Schema/ConfigCompatibility/Migration/CLI 群はすべて空であることが確認できる。
- `compiler/ocaml/` 以下に `manifest`/`schema`/`config` 系ファイルが存在しない点をログに残し、Rust 実装が完全なグリーンフィールドになることを `docs/plans/rust-migration/appendix/glossary-alignment.md` へフィードバックする前提作業として記録した。

#### 1.2 実施結果（Run ID: 20251203-config-effects-trace）
- `reports/spec-audit/ch3/config_effects-trace.md` に `effect {config,audit,io,migration}` の責務と現状を整理。`compiler/rust/frontend/src/diagnostic` に `config` 系メッセージや `ConfigDiagnosticExtension` が存在しないこと、`AuditEnvelope.metadata["config.*"]` を埋める経路が無いことを `grep -R "config" compiler/rust/frontend/src/diagnostic` の結果とともに記録した。
- `docs/plans/bootstrap-roadmap/3-6-core-diagnostics-audit-plan.md` で既に定義された `config.*` KPI（`config_diagnostics_pass_rate` 等）に合わせ、Config/API 実装で必要となるフロー（`load_manifest` で `effect {config}` 付与 → `resolve_compat` で `effect {io}` → `ChangeSet` 出力で `effect {audit}` → 将来の `MigrationPlan` で `effect {migration}`）を再確認。Plan 本文にも `Config` 診断テンプレート新設と Stage/Audit 連携をフォローアップタスクとして残した。
- CLI で `RunConfig.extensions["config"]` に値を入れている箇所（`compiler/rust/frontend/src/bin/reml_frontend.rs:2243-2265`）は `parser.runconfig` メタデータ生成専用であり、Config/Data API とは未接続であることをログに記載。これにより `effect {config}` のみならず `config.path`/`config.source` などのキーを再利用できる下地が無い点を共有した。

#### 1.3 実施結果（Run ID: 20251203-config-serialization）
- `docs/notes/core-library-outline.md` に「Config/Data シリアライズ検討ログ（2025-12-03）」を追加し、ロード→正規化→検証→差分出力の順序と `effect` 境界、`toml_edit` 採用方針、`Diagnostic`/`AuditEnvelope` で共有する `config.*` メタデータの最小集合を記録。これを Appendix の Serialization Decision Log と同期する。
- `examples/` 配下に `reml.toml` や `schema.reml` が存在しないこと、`Cargo.toml` に `toml` 関連依存が無いことを確認してログへ追記。Manifest/Schema CLI を実装する前提で `serde_json` の既存依存を活用し、`toml_edit` を優先候補に設定した。
- Appendix の **Serialization Decision Log** にライブラリ候補・採用理由・影響範囲（CLI/LSP/Audit）を追記し、`docs/spec/3-7-core-config-data.md#1.5` へのフィードバック先・`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の KPI との関係を整理した。

### 2. Manifest モジュール実装（53-54週目）
**担当領域**: `reml.toml`

2.1. `Manifest`/`ProjectSection`/`DslEntry`/`BuildSection` 等のデータ構造を実装する。
    - `compiler/rust/core/config/manifest.rs` に `serde::Deserialize`/`Serialize` を実装し、`Default`/`Builder` パターンでスキーマ未記載フィールドを補完できるようにする。
    - `docs/spec/3-7-core-config-data.md` の表を基に `ProjectSection::{kind, stage, capabilities}` と `BuildSection::{targets, profiles}` を列挙型で表現し、未知の値に備えた `Unknown(String)` バリアントを用意する。
    - `remlc manifest dump --format json` の暫定 CLI を作り、デシリアライズ結果を `compiler/rust/frontend/tests/fixtures/manifest/*.json` へゴールデン化する。
2.2. `load_manifest`/`validate_manifest`/`declared_effects`/`update_dsl_signature` 等の API を実装し、エラー時に `Diagnostic` を返す仕組みを整備する。
    - ファイル IO と `Diagnostic` の橋渡しを行う `ManifestLoader` を `effect {io,config}` でタグ付けし、`Result<Manifest, Diagnostic>` を返す共通エントリポイントを `compiler/rust/core/config/mod.rs` に配置する。
    - バリデーションでは `spec/3-6-core-diagnostics-audit.md` の `config.missing_field`/`config.invalid_stage` 等のコードを使用し、`core_iter_collectors.snap` を更新してスナップショットテストで検証する。
    - DSL シグネチャ更新 (`update_dsl_signature`) は `compiler/rust/frontend/src/dsl/export.rs` と連携させ、`@dsl_export` 注釈から得た情報を Manifest 側へ逆反映させる手順を `docs/guides/dsl/plugin-authoring.md` にも記載する。
2.3. DSL エクスポートシグネチャとの同期 (`@dsl_export`) を確認し、Capability/Stage 情報が正しく投影されることをテストする。
    - `compiler/rust/frontend/tests/core_iter_collectors.rs` のような既存テストを流用し、Manifest→DSL→Capability の往復経路を `cargo test core_iter_collectors -- --nocapture` で再現し、監査ログ (`reports/spec-audit/ch1/core_iter_collectors.audit.jsonl`) を比較する。
    - Stage 情報は `RuntimeBridgeAuditSpec` のキー（`bridge.stage.required`, `bridge.stage.actual`）が埋まるかを確認し、不足時は `docs/spec/3-8-core-runtime-capability.md` との整合タスクを追加する。
    - `docs/guides/dsl/plugin-authoring.md` と `docs/guides/runtime/runtime-bridges.md` に `@dsl_export` × Manifest 連携例を掲載し、サンプル `examples/core_config/` を用意する。

#### 2.2 実施結果（Run ID: 20250719-manifest-loader）
- `compiler/rust/runtime/src/config/manifest.rs` へ `ManifestLoader`/`load_manifest`/`validate_manifest`/`declared_effects`/`update_dsl_signature` を追加し、`effect {io,config}` 相当のエラーをすべて `GuardDiagnostic` (`config.*` / `manifest.entry.missing`) で返すよう統合した。`config.path`/`config.key_path` メタデータを `Diagnostic.extensions["config"]` と監査メタデータに同時転写することで 3-6 §6.1.3 と整合。
- `compiler/rust/frontend/src/bin/remlc.rs` の `manifest dump` が新ローダを経由して `validate_manifest` を呼び、CLI でも `config.missing_field`/`config.invalid_stage` などの診断が得られるようになった。Manifest の JSON ダンプは従来どおりだが、I/O/解析エラーは `GuardDiagnostic` を整形して出力する。
- `compiler/rust/runtime/tests/manifest.rs` でロード失敗 (`manifest.entry.missing`)、Stage 検証 (`config.invalid_stage`)、署名書き戻し（Capability の重複排除）をカバー。`cargo test -p reml_runtime manifest` を想定したが、ローカル環境では `toml v0.5.11` のチェックサム検証で失敗したため、CI でのフェッチ設定を要確認。

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
    - CLI (`reml config lint`) と LSP で互換性設定が反映されることを `docs/guides/ecosystem/ai-integration.md` §6 のシナリオで QA し、結果を `reports/spec-audit/ch3/config_compatibility-lsp.md` に記録する。

#### 3.3 実施結果（Run ID: 20250214-config-resolve）
- `compiler/rust/runtime/src/config/compat.rs` に Stage/Format 別の `compatibility_profile_for_stage`・優先順位付き `resolve_compat`・`CompatibilityLayer`/`ResolvedConfigCompatibility` を追加し、CLI / Env / Manifest / Default の 4 レイヤーから最終プロファイルを決定できるようにした。`resolve_compat` は `ConfigCompatibilitySource::{Cli,Env,Manifest,Default}` を記録し、診断・監査メタデータで `config.compatibility_source` を共有する。
- `compiler/rust/runtime/src/config/manifest.rs` に `[config.compatibility.<format>]` セクションのシリアライザを実装。`Manifest::compatibility_layer` が `config.compatibility.json` などの値を `CompatibilityLayer` へ変換し、Stage に応じた `ConfigCompatibility` とプロファイルラベルを構築できる。TOML パース時は後方互換性を崩さないよう新フィールドへ `#[serde(default)]` を付与。
- `compiler/rust/runtime/tests/config_compat.rs` へ解決順序/マニフェスト構文の単体テストを追加し、`ConfigCompatibilitySource` が CLI > Env > Manifest > Default で選ばれることと、manifest の `feature_guard` が `ConfigCompatibility` へ反映されることを検証した。
- `compiler/rust/frontend/src/parser/api.rs` の `RunConfig` に `config_compat: Option<ResolvedConfigCompatibility>` を追加、`build_config_extension` で `compatibility` 情報を JSON 化し `parser.runconfig.extensions.config.compatibility_*` を埋めるようにした。CLI 側（`reml_frontend`）は `--config-compat <profile>` オプションを追加し、Stage（`--effect-stage`/`--effect-stage-runtime`）情報から `RuntimeStageId` に変換した上で `resolve_compat` を呼び出して `RunConfig` に保持する。ヘルパ `convert_stage_id` で Chapter1 Stage ID を Runtime Stage へマップしている。
- `docs/spec/3-7-core-config-data.md` §1.5 に manifest テーブル例と Rust CLI の `--config-compat` 設定例を追記し、優先順位の第 1 層（CLI オプション）が実装済みであることを明記した。
- `docs/plans/bootstrap-roadmap/assets/core-runtime-capability-init.md` と `assets/capability-error-matrix.csv` を参照し、Manifest→RunConfig→CapabilityRegistry の初期化順序と契約違反時の診断コードを Config 計画側からも把握できるようになった（Run ID: 20291221-capability-init-seq）。

#### 3.1 実施結果（Run ID: 20251203-schema-core-data）
- `compiler/rust/runtime/src/data/{mod.rs,schema.rs}` に `Schema`/`Field`/`ValidationRule`/`SchemaDiff` を実装し、ビルダー API・差分検出・`FieldAttribute` を仕様 3-7 §2 の構造と整合させた。`SchemaDataType` は JSON/TOML 双方のエイリアスをサポートする列挙体として導入し、`FieldBuilder`/`ValidationRuleBuilder` で `effect {config}` 拡張の基礎を揃えた。
- 差分の可視化用途として `compiler/rust/runtime/examples/schema_diff_demo.rs` を作成し、`SchemaDiff::between` を JSON 化するフローをサンプルコード化。生成物は `reports/spec-audit/ch3/schema_diff-20251203.md` に貼り付け、Cargo のチェックサム問題が解消され次第 `cargo run --manifest-path compiler/rust/runtime/Cargo.toml --example schema_diff_demo` を実行して更新する方針を記載した。
- テスト検証は `cargo test --manifest-path compiler/rust/runtime/Cargo.toml` で試行したが、既知の `toml v0.5.11` チェックサム不一致により停止（`error: checksum for toml v0.5.11 changed between lock files`）。Phase 3 の他タスクでも同一ブロッカーが記録済みのため、本 Run ID でも未解決問題として共有する。
- 今後は `tooling/scripts/generate-schema-diff.sh` の整備と `collect-iterator-audit-metrics.py --section config` への統合を行い、Schema 差分の再取得と監査 KPI への組み込みを行う。

#### 3.2 実施結果（Run ID: 20250318-config-compat-profiles）
- `compiler/rust/runtime/src/config/compat.rs` を新設し、`ConfigCompatibility`/`CompatibilityProfile`/`ConfigTriviaProfile`/`ConfigFormat` など仕様 3-7 §1.5 の型を Rust で定義。`strict_json`/`relaxed_json`/`relaxed_toml` などのビルダーを提供し、`compatibility_profile("strict")` 形式でプロファイルを取得できる API を実装した。互換違反ごとの `Diagnostic.code = "config.compat.*"` 定数と `compatibility_violation_diagnostic`/`CompatibilityDiagnosticBuilder` でメタデータ（`config.source`/`config.compatibility.violation` 等）を一括生成できるようにした。
- `compiler/rust/runtime/tests/config_compat.rs` と `tests/snapshots/config_compat__*.snap` を追加し、`insta` で JSON/TOML 代表ケースの `ConfigCompatibility` をスナップショット固定。`Cargo.toml` へ `insta`（yaml フォーマット）を dev-dependency として登録した。
- `docs/spec/3-6-core-diagnostics-audit.md#推奨診断コード` に `config.compat.trailing_comma`/`config.compat.unquoted_key`/`config.compat.duplicate_key`/`config.compat.number_format` を追記し、監査ログで `config.compatibility` メタデータを参照する根拠と CLI/LSP の対応方針を明文化した。

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

#### 4.3 実施結果（Run ID: 20251210-runconfig-manifest）
- `compiler/rust/runtime/src/run_config.rs` を新設し、`ApplyManifestOverridesArgs` / `RunConfigManifestOverrides` / `apply_manifest_overrides` を追加。`config.manifest.*`、`project.capabilities`、互換プロファイルの情報を `RunConfig.extensions["config"]` に投影できるようになり、`Manifest::compatibility_layer` の結果を `CompatibilityLayer` として保持する。
- CLI 側では `reml_frontend --manifest <path>` を導入し、`ManifestLoader::load` → `apply_manifest_overrides` → `RunSettings::apply_manifest_overrides` の順に連携する経路を実装。`parser.runconfig.extensions.config.manifest` が `audit_metadata["config.*"]` と同期することを `build_config_extension` のスナップショットで確認した。
- `compiler/rust/runtime/tests/run_config_integration.rs` を追加し、マニフェストから生成された拡張に `config.source = "manifest"` / `config.runtime_stage` / 互換情報が含まれることを検証。CLI 統合までは `reml_frontend` の既存テスト（streaming_metrics 等）で `--manifest` を併用し、Packrat / Trace 設定と共存できることを手動確認した。
- ドキュメントは `docs/spec/3-10-core-env.md` §2.1 に `RunConfigManifestOverrides` の流れと `--manifest` オプションを追記し、`docs/spec/3-8-core-runtime-capability.md` §1.3 の `RunConfig` 解説へ `config.manifest` の監査メタデータを追加。これにより Stage/Capability 判定の参照元が仕様レベルで定義された。

#### 4.2 実施結果（Run ID: 20251205-config-diagnostics）
- `compiler/rust/frontend/src/diagnostic/messages/` を新設し、`config.rs` に `config.missing_manifest` / `config.schema_mismatch` / `config.compat.unsupported` のテンプレートを実装した。いずれも `DiagnosticDomain::Config`・`config.*` メタデータ・`AuditMetadata` への書き戻しを共通ヘルパ (`ConfigDiagnosticMetadata`) で扱えるようにし、`extensions["config"]` と `audit_metadata["config.*"]` の欠落を防ぐ単体テストを追加している。
- Config 診断の証跡として `reports/spec-audit/ch3/config_diagnostics-20251203.json` を追加し、`scripts/validate-diagnostic-json.sh --section config` が `config_diagnostics` ゴールデンに含まれる `config.*` 拡張を必須チェックするよう更新した（既存の `schema_diff.*` 判定はファイル名で自動切り替え）。これにより CLI/LSP の Config 診断 JSON が監査用の最小フィールドを欠落させた場合に検出できる。
- `docs/notes/spec-integrity-audit-checklist.md` に `config_diagnostics_pass_rate >= 0.95` の KPI 行を追加し、`--section config` が `extensions["config"]` と `audit_metadata["config.*"]` の両方を満たすことを監査手順として記録した。Phase 3 で Config/Data を組み込む際のベースラインとして Run ID を共有済み。

### 5. データ互換性・マイグレーション支援（55週目）
**担当領域**: 将来互換

5.1. `MigrationPlan`/`MigrationStep` (仕様に記載された実験的 API) のドラフトを実装し、`effect {migration}` の扱いを定義する。
    - `compiler/rust/core/config/migration.rs` を新設し、`MigrationPlan` の JSON/TOML 表現を定義、`#[cfg(feature = "experimental-migration")]` で囲んだ上でドキュメントに opt-in 手順を記す。
    - `effect {migration}` を `docs/spec/1-3-effects-safety.md` の効果テーブルへ追加する提案メモを作成し、`docs/plans/bootstrap-roadmap/3-6-core-diagnostics-audit-plan.md` に連携項目として貼る。
    - `reports/spec-audit/ch3/migration_plan-pilot.md` に PoC 実行ログ（Manifest 差分→MigrationPlan 生成→ChangeSet 適用）を残し、Phase 4 実装者に引き継ぐ。

#### 5.1 実施結果（Run ID: 20251224-migration-plan-alpha）
- `compiler/rust/runtime/Cargo.toml` に `experimental_migration` フィーチャを追加し、`compiler/rust/runtime/src/config/migration.rs` で `MigrationPlan`/`MigrationStep`/`MigrationRiskLevel`/`MigrationDuration`/`ReorganizationStrategy`/`TypeConversionPlan` を `serde` 対応の構造体として実装した。`pub const MIGRATION_EFFECT_TAG` を導入し、`effect {migration}` を利用する API が共通のタグ名を参照できるようにしている。
- `config::mod.rs` で上記モジュールを `#[cfg(feature = "experimental_migration")]` 付きで再輸出し、デフォルトビルドでは未公開、`cargo test -p reml_runtime --features experimental-migration` で PoC を実行できるようにした（`migration.rs` には JSON ラウンドトリップテストを追加済み）。
- `docs/spec/1-3-effects-safety.md` と `docs/spec/3-7-core-config-data.md` に実装メモを追記し、`MigrationPlan` 利用時は `experimental-migration` フィーチャを明示すること、`effect {migration}` が監査ログに出力されることを仕様側で説明した。
- `docs/plans/bootstrap-roadmap/3-6-core-diagnostics-audit-plan.md#1.2` の Run ID ログを更新し、Config/Data 章から出力される `effect {migration}` の監査キーを Diagnostics 計画にも紐付けた。
- `reports/spec-audit/ch3/migration_plan-pilot.md` を新設し、`cargo test -p reml_runtime migration_plan::tests::plan_serialization_roundtrip --features experimental-migration` の実行ログ、`serde_json` で生成されるサンプル JSON、`ChangeSet` 連携 TODO を記録した。Phase 4 の `reml config migrate` CLI へ渡すための PoC ルートとして参照する。
- Manifest と Capability Registry の契約整合は `reports/spec-audit/ch3/manifest_capability-20260225.md` に追記し、3.8 計画 §5.3 の Run ID（20260225-manifest-capability-contract）から本節へ遡れるようにした。
- ✅ 20240712-conductor-contract: `compiler/rust/runtime/src/config/manifest.rs` に `run.target.capabilities` を解析して `ConductorCapabilityContract` を生成するロジックを追加し、`CapabilityRegistry::verify_conductor_contract` 経由で Stage/Capability 契約を突き合わせるテスト (`cargo test -p reml_runtime conductor_contract`) を整備。エラー時には `manifest_path` と `source_span` を `CapabilityError::ContractViolation` と監査メタデータへ転写し、[3-8 Core Runtime Capability 計画](3-8-core-runtime-capability-plan.md#3-3-verify_conductor_contract) の Run ID 参照先として `tests/conductor_contract.rs` と `reports/spec-audit/ch3/conductor_contract-20240712.md` を登録した。
5.2. Manifest/Schema のバージョン互換チェックを追加し、移行シナリオを `docs/notes/dsl-plugin-roadmap.md` に記録する。
    - `Manifest.version` と `Schema.version` を比較し、互換条件（`major` は一致、`minor` は `>=` など）を `docs/spec/3-7-core-config-data.md` に追記するための diff を準備する。
    - 互換性チェックの結果を `reports/dual-write/config_versioning/` に保存し、DSL プラグイン毎の移行ステップを `dsl-plugin-roadmap.md` に表形式で追記する。
    - 重大な非互換が検出された場合は `docs/notes/dsl-plugin-roadmap.md` の TODO セクションへ `MIGRATION-BLOCKER-*` 番号で登録し、Phase 4 のタスクインプットにする。

#### 5.2 実施結果（Run ID: 20250310-config-version）
- `compiler/rust/runtime/src/config/manifest.rs` に `ensure_schema_version_compatibility` を追加し、`project.version` の SemVer 解析エラー (`config.project.version_invalid`) とスキーマ側の先行 (`config.schema.version_incompatible`) を `config.version_reason` メタデータ付きで検出できるようにした。`config/mod.rs` から再エクスポートしたため、CLI/RunConfig API は同じ関数を呼び出せる。
- `tests/manifest.rs` へ 5 つの `schema_version_check_*` テストを追加し、`cargo test schema_version_check` のログを `reports/dual-write/config_versioning/20250310-config-version-check.md` に保存した。互換成功・major mismatch・minor 超過・Schema.version 省略・SemVer 解析失敗の各ケースをカバー。
- 仕様 `docs/spec/3-7-core-config-data.md` と `docs/spec/3-6-core-diagnostics-audit.md`（診断コード補足）へ互換条件と新コードを追記し、`docs/notes/dsl-plugin-roadmap.md` へ移行ステップ表と `MIGRATION-BLOCKER-001` を登録して Phase4 連携の導線を確保した。
5.3. CLI 連携 (`reml config lint`, `reml config diff`) の出力仕様を整備し、サンプルを作成する。
    - `compiler/rust/cli/src/commands/config.rs` に `lint`/`diff` サブコマンドを追加し、`ChangeSet` との整合を `snap` テストで担保する。
    - `docs/guides/runtime/runtime-bridges.md` と `docs/guides/ecosystem/ai-integration.md` に CLI 出力例を掲載し、JSON/TTY 両方のサンプルを示す。
    - `examples/core_config/cli/` に最小構成の Manifest/Schema を配置し、`scripts/run_examples.sh --suite core_config` で検証できるようにする。

#### 5.3 実施結果（Run ID: 20240305-config-cli-diff）
- `compiler/rust/frontend/src/bin/remlc.rs` を拡張し、`config lint`/`config diff` サブコマンドを新設。`GuardDiagnostic` を `LintDiagnostic` へ整形する JSON レポートと TTY 出力を実装し、`ChangeSet` と `SchemaDiff` を同時出力するテスト（`diff_report_contains_expected_change_set` など）を追加した。Exit Code も `lint` で 0/2 を返し、CI へ直接組み込める形にした。
- `examples/core_config/cli/` に Manifest/Schema/DSL と `config_old.json`/`config_new.json`、および `lint.expected.json`/`diff.expected.json` を追加。`tooling/examples/run_examples.sh --suite core_config` で `cargo run --bin remlc` を呼び出し、`--update-golden` 時にゴールデンを更新できるよう Bash スイートを拡張した。
- `docs/guides/runtime/runtime-bridges.md` と `docs/guides/ecosystem/ai-integration.md` に CLI 操作例・JSON スニペットを追記し、AI 連携や Runtime Bridge ガイドから直接 `examples/core_config/cli` を参照できるようにした。

### 6. ドキュメント・サンプル更新（55-56週目）
**担当領域**: 情報整備

6.1. 仕様書内の表・サンプルを実装に合わせて更新し、`examples/` に Manifest/Schema 例を追加する（Core.Text ガイド更新時は `docs/guides/compiler/core-parse-streaming.md` §10 と整合させる）。
    - `docs/spec/3-7-core-config-data.md` の章末サンプルを最新 API で書き換え、`git grep "Manifest::"` で旧記法を洗い出して一括更新する。
    - `examples/core_config/basic_manifest/` を新設し、`README.md` 付きで `reml config lint` の結果を貼る。`tooling/examples/run_examples.sh --suite core_config --update-golden` 手順を説明書として追記する。
    - Core.Text ガイドとの整合確認は `docs/guides/compiler/core-parse-streaming.md` §10 の参照リンクを更新し、Config/Data 例との相互リンクを README に追加する。
6.2. `README.md`/`3-0-phase3-self-host.md` に Config/Data 実装状況を記載し、Phase 4 への連携点をまとめる（AI 入力ポリシーの共有は `docs/guides/ecosystem/ai-integration.md` §6 と同期）。
    - 進捗サマリを `README.md#phase-3-bootstrap-roadmap` に箇条書きで追記し、完了/進行中のモジュールを色分け Legend で説明する。
    - `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` のマイルストーン表に Config/Data 行を追加し、Phase 4 との依存（MigrationPlan, Registry 連携）を注記で示す。
    - AI 連携ポリシー ( `docs/guides/ecosystem/ai-integration.md` §6 ) に Config/Data API の利用例を追記し、AI ツールへ Manifest 情報を渡す際の制約をまとめる。
6.3. `docs/guides/runtime/runtime-bridges.md`/`docs/guides/dsl/plugin-authoring.md` 等で設定連携の記述を更新する。
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

#### 7.1 実施結果（Run ID: 20250609-config-tests）
- `compiler/rust/runtime/tests/manifest_validation.rs` に 10 ケースのスナップショットテストを追加し、`project.name`/`project.version` 欠落、`project.kind`/`stage` の未知値、`build.optimize`/`build.profiles.*.optimize` の未知値、`dsl.entry` 未設定、`dsl.kind` 不一致、`expect_effects_stage` の未知 Stage、`stage_bounds.minimum` の検証失敗まで網羅した。各テストは `compiler/rust/runtime/tests/snapshots/manifest_validation__*.snap` へ JSON で固定し、`INSTA_UPDATE=always cargo test --test manifest_validation` で更新する運用を明文化した。
- `compiler/rust/frontend/tests/manifest_integration.rs` を追加し、`fixtures/config_integration` 配下の `reml.toml`/`schema.json`/`dsl` を読み込んで Manifest→Schema→`apply_manifest_overrides` の一連フローを検証した。`config.compatibility_profile = json-relaxed` が `RunConfig` 拡張に伝搬することと、`ensure_schema_version_compatibility` が `StageId::Beta` で整合することをテストで保証した。
- リスク管理文書 `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` に `config_validation_cases >= 10` を追加し、Config/Data のフォールトテーブルが 2 桁を下回った場合はリスク登録 (ID: `config-data`) を必須化した。

#### 7.2 実施結果（Run ID: 20250609-config-diff-ci）
- `compiler/rust/frontend/tests/core_iter_collectors.rs` に `config_diff_report` ケースを追加し、`examples/core_config/cli/config_{old,new}.json` の差分を `PersistentMap::diff_change_set` で再現して `__snapshots__/core_iter_collectors.snap` へ記録した。これに合わせて `reports/spec-audit/ch1/core_iter_collectors.audit.jsonl` に `case=config_diff_report` を追記し、ChangeSet サマリが監査ログとして残るようにした。
- `tooling/scripts/verify-config-diff.sh` を新設し、`cargo run --bin remlc -- config diff` の出力と `examples/core_config/cli/diff.expected.json` を `diff -u` で比較する自動検証ステップを整備した。`UPDATE_GOLDEN=1` でゴールデン更新も可能。
- `tooling/ci/collect-iterator-audit-metrics.py` に `--section config` と `--config-source` を追加し、Config Diff ゴールデンの ChangeSet に `summary/metadata/items` が揃っているかを `config.diff.change_set` メトリクスで確認できるようにした。

#### 7.3 実施結果（Run ID: 20250609-config-ci-job）
- `.github/workflows/rust-frontend.yml` を追加し、`config-lint` ジョブで `cargo run --bin remlc -- config lint --manifest examples/core_config/reml.toml --schema examples/core_config/cli/schema.json --format json` を実行、`lint-report.json` をアーティファクト化するまでを自動化した。ジョブは `tooling/scripts/verify-config-diff.sh` を併走させ、差分ゴールデンの一致を保証する。
- `README.md` に Rust Frontend CI の存在を追記し、`docs/plans/rust-migration/3-0-ci-and-dual-write-strategy.md` §3.0.11 (新設) で Config/Data Lint ジョブの役割と成果物 (`config-lint-report`) を共有した。
- `docs/plans/bootstrap-roadmap/0-4-risk-handling.md#config-data` に CI 失敗時のフォローアップ手順（Issue 登録＋ Run ID 記録）を追加し、Config/Data のリスクレビューが自走するようにした。

## 成果物と検証
- Manifest/Schema/ConfigCompatibility API が仕様通りに実装され、効果タグ・診断・監査が整合すること。
- DSL エクスポート・Capability 情報がマニフェストから取得でき、Phase 4 の移行処理に再利用できること。
- ドキュメント・サンプルが更新され、設定ファイルの互換性ポリシーが明確であること。

## リスクとフォローアップ
- TOML/JSON パーサの差異で互換性チェックが不安定な場合、フォーマット別に冪等テストを追加し、必要なら構文制限を仕様側へ提案する。
- Migration API が未成熟な場合、Phase 4 で段階的導入する前提で `docs/notes/` に TODO を残す。
- レジストリ連携で追加機能が必要になった場合、`docs/notes/dsl-plugin-roadmap.md` に記録し、エコシステム計画 (5-x) と調整する。

## Appendix（更新指針）

### Appendix A. API Matrix（Run ID: 20251203-config-api-diff）
`reports/spec-audit/ch3/config_data_api_diff.md` で抽出した一覧を計画書側にも掲載し、章横断で参照できるようにする。

| 仕様 API/型 | 参照 | Rust 実装 | ステータス | 主要差分 |
| --- | --- | --- | --- | --- |
| Manifest/ProjectSection/BuildSection | 3-7 §1.1 | 実装なし | ❌ 未着手 | `compiler/rust/runtime/src/config/manifest.rs` が存在せず、TOML デシリアライズができない。 |
| `load_manifest`/`validate_manifest`/`declared_effects`/`update_dsl_signature`/`iter_dsl` | 3-7 §1.2 | 実装なし | ❌ 未着手 | `effect {io,config}` を伴う入口が未定義。 |
| DSL エクスポート同期 (`expect_effects`, `signature.stage_bounds`) | 3-7 §1.3-1.4 | 実装なし | ❌ 未着手 | `compiler/rust/frontend/src/dsl/` との橋渡しが皆無。 |
| `ConfigCompatibility` / `compatibility_profile` / `resolve_compat` | 3-7 §1.5 | 実装なし | ❌ 未着手 | `Cargo.toml` に TOML パーサ依存が無く、互換モード API が無い。 |
| Config 診断 (`ConfigDiagnosticExtension`, `config.*` コード) | 3-7 §1.5.3, 3-6 §6.1.3 | 実装なし | ❌ 未着手 | `compiler/rust/frontend/src/diagnostic/messages/` に `config` 用ファイルが無い。 |
| `Schema<T>` / `SchemaBuilder` / `SchemaDiff<T>` / `diff` / `plan` / `validate` | 3-7 §2, §3 | 実装なし | ❌ 未着手 | Schema 操作用のファイルが無く、`ChangeSet` 連携も不可。 |
| `SchemaDiff` / `ConfigChange` / `ChangeKind` | 3-7 §2.1, §4 | `compiler/rust/runtime/src/config/collection_diff.rs` | ⚠️ 一部 | JSON 変換と `ChangeSet` 往復のみ実装。Stage/Policy 初期値が固定。 |
| `merge_maps_with_audit` / `write_change_set*` / `CollectionsChangeSetEnv` | 3-7 §4 | `compiler/rust/runtime/src/config/mod.rs` | ⚠️ 既存 | `PersistentMap` 前提の差分マージは存在。ただし Config/Data API と直接結合していない。 |
| `MigrationPlan` / `MigrationStep` / `validate_migration` | 3-7 §5 | 実装なし | ❌ 未着手 | `effect {migration}` を生成するルート無し。 |
| CLI (`reml config lint/diff`) / サンプル (`examples/core_config`) | 3-7 §0 | 実装なし | ❌ 未着手 | CLI コマンドとゴールデンファイル不在。 |

### Appendix B. Serialization Decision Log（Run ID: 20251203-config-serialization）
`docs/notes/core-library-outline.md#configdata-シリアライズ検討ログ2025-12-03` と同期する決定事項を抜粋する。

| 項目 | 候補/決定 | 根拠 | 影響範囲 |
| --- | --- | --- | --- |
| TOML パーサ | `toml_edit` を最優先、`toml`（serde フレンドリ）をバックアップ | `reml.toml` でコメント保持・既存書式維持が必須のため。`Cargo.toml` に依存が無かったので追加前提。 | `compiler/rust/runtime` / `compiler/rust/frontend` / CLI (`reml config lint/diff`) |
| JSON 変換 | 既存 `serde_json` を継続利用 | 仕様 3-7 §2/§4 が JSON 例を前提にしており、既に依存済み。 | SchemaDiff/ChangeSet/Audit ログ (`reports/spec-audit/ch3`), CLI/LSP |
| シリアライズ順序 | **Load → 正規化 → 検証 → 差分出力** を固定 | 仕様 3-7 §1.5, §2 の指針を Rust でも忠実に再現。`effect` 境界を分離できる。 | Manifest Loader / Schema Validator / `collect-iterator-audit-metrics --section config` |
| 診断/Audit メタデータ | `config.path`, `config.key_path`, `config.source`, `config.profile`, `config.compatibility`, `config.feature_guard`, `config.diff` を最小集合として統一 | `docs/spec/3-6-core-diagnostics-audit.md#6.1.3` の要求と `RunConfig.extensions["config"]` の再利用性を一致させるため | CLI/LSP/`AuditEvent::ConfigCompatChanged`、`tooling/scripts/validate-diagnostic-json.sh --suite config` |
| TODO / 未決 | `Schema` サンプルと CLI ゴールデンの参照先（`examples/core_config/`）を新設 | `examples/` に `reml.toml`/`schema.reml` がないため。 | `tooling/examples/run_examples.sh --suite core_config`（新設予定） |

> **メモ**: このログはフェーズ毎に追記し、ライブラリ追加・CLI 実装・RunConfig 連携の判断をここから逆引きできるよう維持する。

## 参考資料
- [3-7-core-config-data.md](../../spec/3-7-core-config-data.md)
- [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md)
- [3-4-core-numeric-time.md](../../spec/3-4-core-numeric-time.md)
- [3-5-core-io-path.md](../../spec/3-5-core-io-path.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- [notes/dsl-plugin-roadmap.md](../../notes/dsl-plugin-roadmap.md)
