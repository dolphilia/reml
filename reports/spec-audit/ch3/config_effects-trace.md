# Config/Data 効果タグ・診断連携メモ（Run ID: 20251203-config-effects-trace）

`docs/spec/3-7-core-config-data.md` で規定される `effect {config}`, `{audit}`, `{io}`, `{migration}` と `docs/spec/3-6-core-diagnostics-audit.md` §6.1.3 の `config.*` 診断キーの整合を確認した。`compiler/rust/frontend/src/diagnostic`・`compiler/rust/runtime/src/config` を走査し、`config` 向けの Diagnostic 拡張や Audit 連携が未定義であることを記録する。

## 1. 参照資料と実装確認
- 仕様: `docs/spec/3-7-core-config-data.md` §0（効果タグ）、§1.5.3（Config 診断拡張）、§4（ChangeSet/Audit）、§5（Migration）
- 診断基盤: `docs/spec/3-6-core-diagnostics-audit.md` §1.2（`AuditEvent::ConfigCompatChanged`）、§6.1.3（`config.*` 拡張）、`docs/plans/bootstrap-roadmap/3-6-core-diagnostics-audit-plan.md`（Run ID `20290701-effect-tag-policy` / `20290705-cli-lsp-format`）
- 実装確認コマンド:  
  - `grep -R "config" compiler/rust/frontend/src/diagnostic -n`（ヒットなし）  
  - `grep -R "config" compiler/rust/frontend/src/bin/reml_frontend.rs`（`RunConfig` 拡張のみ）  
  - `grep -R "ConfigDiagnostic" -n .`（ヒットなし）

## 2. 効果タグ別の現状

| 効果タグ | 仕様での責務 | Rust 実装の現状 | ギャップ |
| --- | --- | --- | --- |
| `effect {config}` | `load_manifest` / `resolve_compat` / `Schema.validate` など設定専用 API で診断を構築（3-7 §0, §1.5.3） | 当該 API 未実装。`compiler/rust/runtime/src/config` は `ChangeSet` ラッパのみで effect 宣言なし | 設定操作に起因する診断/監査が出せない |
| `effect {audit}` | 互換モード変更時の `AuditEvent::ConfigCompatChanged`（3-6 §1.2）と `ChangeSet` 出力 | `merge_maps_with_audit` が `AuditBridgeError` を返すのみで、`config` 用メタデータ出力が無い | `AuditEnvelope.metadata["config.*"]` が欠落 |
| `effect {io}` | `load_manifest` / `Schema.load` のファイル IO | API 未実装。`reml_frontend` CLI も Config ファイル未対応 | CLI/LSP から `config.source = cli/env/manifest` を渡せない |
| `effect {migration}` | `MigrationPlan` 生成と `effect {migration}` 診断（3-7 §5） | `migration.rs` 不在。`effect {migration}` を出力する経路自体が無い | Phase 5 実装向けの基盤欠如 |

## 3. 診断キーとメタデータ
- `docs/spec/3-6-core-diagnostics-audit.md` §6.1.3 の `config.source` / `config.path` / `config.key_path` / `config.profile` / `config.compatibility` / `config.feature_guard` / `config.diff` を埋める箇所が Rust 実装に存在しない。
- `docs/plans/bootstrap-roadmap/3-6-core-diagnostics-audit-plan.md` の Run ID `20290705-cli-lsp-format` で整備された `CliDiagnosticEnvelope`/LSP 出力にも `config.*` 例が含まれておらず、`structured_hints` への連携も未検証。
- `collect-iterator-audit-metrics.py --section config` のプリセットは未定義。`reports/audit/dashboard/collectors-20251203.json` と同様に Config 用 KPI を作る前提で `AuditEvent::ConfigCompatChanged` を発火させる必要がある。

## 4. 次に必要な設計タスク
1. `compiler/rust/frontend/src/diagnostic/messages/config.rs` を新設し、`config.missing_manifest` / `config.schema_mismatch` / `config.compat.unsupported` など仕様で定義されたコードを `DiagnosticBuilder` へ登録する。
2. `ConfigDiagnosticExtension`（3-6 §6.1.3）のシリアライズ形式を Rust 側で定義し、`load_manifest` や `resolve_compat` の戻り値 `Result<_, Diagnostic>` に付与する。
3. `ChangeSet` / `AuditEnvelope` 生成ルートに `config.*` メタデータキーを追加し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の KPI（`config_diagnostics_pass_rate`）を計測可能にする。
4. Migration API 実装時に `effect {migration}` 用のダミー診断を先に追加し、Phase 4 の移行計画で参照できるログを残す。
