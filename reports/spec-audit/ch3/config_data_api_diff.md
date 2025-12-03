# Config/Data API 差分メモ（Run ID: 20251203-config-api-diff）

Core Config/Data 章の API を仕様（3-7）と Rust 実装（`compiler/rust/runtime/src/config` 他）で突き合わせ、53 週目タスク「1. API 差分整理と構造設計」に必要なギャップを可視化した。`docs/spec/3-7-core-config-data.md` §1（Manifest）、§1.5（ConfigCompatibility）、§2（Schema & ChangeSet）を確認し、`grep -R "pub "` と `find compiler/rust -name '*manifest.rs'` などの走査結果を反映している。

## 1. 対象資料
- 仕様: `docs/spec/3-7-core-config-data.md`（Manifest / Schema / ConfigCompatibility / Migration）
- 監査・診断整合: `docs/spec/3-6-core-diagnostics-audit.md` §6.1.3（Config Diagnostic Extension）、`docs/plans/bootstrap-roadmap/3-6-core-diagnostics-audit-plan.md`（Run ID: 20290701-effect-tag-policy 他）
- 実装: `compiler/rust/runtime/src/config/{mod.rs,collection_diff.rs}`（ChangeSet ラッパのみ実装済み）
- 旧実装: `compiler/ocaml/` 以下に `Core.Config`/`Core.Data` 相当のソースは存在せず（`find compiler/ocaml -name '*manifest*'` で該当なし）

## 2. API 実装状況サマリ

| 仕様 API/型 | 参照 | Rust 実装 | ステータス | 主要差分 |
| --- | --- | --- | --- | --- |
| `Manifest` / `ProjectSection` / `BuildSection` 等の構造体 | 3-7 §1.1 | 実装なし | ❌ 未着手 | `compiler/rust/runtime/src/config/` に `manifest.rs` 等が存在せず、`Serde` 導入も未実施。 |
| `load_manifest` / `validate_manifest` / `declared_effects` / `update_dsl_signature` / `iter_dsl` | 3-7 §1.2 | 実装なし | ❌ 未着手 | ファイル IO + `effect {config, io}` を要求する API がコードベースに存在しない。`reml.toml` サンプルも `examples/` に未配置。 |
| DSL エクスポート同期 (`expect_effects`, `signature.stage_bounds`) | 3-7 §1.3-1.4 | 実装なし | ❌ 未着手 | `compiler/rust/frontend/src/dsl/` に Manifest との橋渡しロジックが無い。 |
| `ConfigCompatibility` 構造体と `compatibility_profile`/`resolve_compat` 群 | 3-7 §1.5 | 実装なし | ❌ 未着手 | `Cargo.toml`（runtime/frontend）にも `toml`/`toml_edit` 依存が無く、互換モード切替 API が未定義。 |
| Config 診断 (`ConfigDiagnosticExtension`, `config.*` コード) | 3-7 §1.5.3, 3-6 §6.1.3 | 実装なし | ❌ 未着手 | `compiler/rust/frontend/src/diagnostic/messages/` に `config.rs` が存在せず、`config.*` コードも未定義。 |
| `Schema<T>` / `SchemaBuilder` / `SchemaDiff<T>` / `diff` / `plan` | 3-7 §2.0-2.2 | 実装なし | ❌ 未着手 | Rust 側に `schema.rs` 等が存在しないため、DSL/CLI でのスキーマ操作ができない。 |
| `SchemaDiff` / `ConfigChange` / `ChangeKind` | 3-7 §2.1, §4 | `compiler/rust/runtime/src/config/collection_diff.rs` | ⚠️ 一部 | JSON 変換と `ChangeSet` 変換のみ実装。`type_tag` の定義や Stage/Policy 初期値は暫定固定（`DEFAULT_*`）。 |
| `merge_maps_with_audit` / `write_change_set` / `CollectionsChangeSetEnv` | 3-7 §4 | `compiler/rust/runtime/src/config/mod.rs` | ⚠️ 既存 | `PersistentMap` 専用の差分合成と一時ファイル連携は存在。ただし Config/Data API から再利用するための facade が無い。 |
| `MigrationPlan` / `MigrationStep` / `validate_migration` | 3-7 §5 | 実装なし | ❌ 未着手 | `compiler/rust/runtime/src/config/migration.rs` 未作成。`effect {migration}` を出力する箇所も無い。 |
| CLI (`reml config lint/diff`) / Schema/Manifest ゴールデン | 3-7 §0, Appendix | 実装なし | ❌ 未着手 | `compiler/rust/cli` 以下に Config コマンドが存在しない。 |

## 3. 差分メモ
- Manifest 系 API はファイル自体が無いため、`serde::Deserialize`/`toml_edit` 前提の構造定義から実装する必要がある。`cargo metadata` にも TOML パーサ依存が未登録なので、依存ライブラリの合意が前提となる。
- `collection_diff.rs` は `ChangeSet`→JSON のラッパに留まり、`SchemaDiffMetadata` も `origin/policy/category/stage` を固定ストリングで埋めるのみ。仕様で要求される `snapshot`・`diff.missing` などのキーは欠落している。
- `merge_maps_with_audit` 周辺は `Core.Collections` 由来 (`PersistentMap`) 前提で `Core.Config` 観点の effect/tag 付与が無い。Config/Data 側で使うには `effect {config}` を付与した facade が必要。
- `docs/spec/3-7` で言及される CLI (`reml config lint`, `reml config diff`) は Rust CLI (`compiler/rust/cli/src`) に未登場であり、`examples/` 配下にも `reml.toml` ファイルが無い。実装完了まで CLI 連携・ガイド更新フローを追加する必要がある。
- 旧 OCaml 実装にも `Core.Config`/`Core.Data` の実体が無いため、Rust 側でゼロから設計・実装する必要がある。`docs/plans/rust-migration/appendix/glossary-alignment.md` に対応語彙を登録しつつ進める。

## 4. フォローアップ候補
1. `compiler/rust/runtime/src/config/manifest.rs` / `schema.rs` を新設し、仕様の型定義と API を素直に移植する。
2. `Cargo.toml` へ `toml_edit`（コメント保持目的）と `serde_json`（既存依存あり）を組み合わせる構成を提案し、`docs/plans/bootstrap-roadmap/3-7-core-config-data-plan.md` Appendix の Serialization Decision Log で記録する。
3. `docs/spec/3-6-core-diagnostics-audit.md` §6.1.3 の `config.*` 診断と `effect {config/audit/io/migration}` の出力点を `compiler/rust/frontend/src/diagnostic/` に新設する。
4. `reports/spec-audit/ch3` に Manifest/Schema ゴールデンと `collect-iterator-audit-metrics --section config` の枠を追加し、P55 以降のテスト連携を先に確立する。
