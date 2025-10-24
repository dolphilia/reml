# DIAG-003 診断ドメイン語彙拡張提案

## 1. 背景と症状
- Chapter 3 は `DiagnosticDomain` として `Effect` / `Target` / `Plugin` / `Lsp` / `Other(Str)` 等を定義している（docs/spec/3-6-core-diagnostics-audit.md:40,173-188）。  
- OCaml 実装は `error_domain` 列挙を `Parser` / `Type` / `Config` / `Runtime` / `Network` / `Data` / `Audit` / `Security` / `CLI` に限定しており（compiler/ocaml/src/diagnostic.ml:39-64）、効果やプラグイン、LSP などの診断ドメインを表現できない。  
- `Diagnostic.extensions["effects"]` の Stage 情報や `AuditEnvelope.metadata` の `event.kind` などが限定的にしか出力されず、監査ダッシュボードで仕様通りの分析が困難。

## 2. Before / After
### Before
- 診断ドメインがコア実装の列挙に固定され、仕様で追加されたドメインとのマッピングが不十分。  
- `effect.stage.*` などの監査メタデータが CLI/LSP で利用されず、Phase 2-8 の監査要件と乖離している。

### After
- `Diagnostic.error_domain` を Chapter 3 の列挙に合わせて拡張し、`Effect` / `Target` / `Plugin` / `Lsp` / `Other` 等を追加。  
- 昇格が難しい場合は仕様側に脚注を追加し、Phase 2-7 `diagnostic-domain` タスクとして実装計画を共有。  
- 監査メタデータのキー（`effect.stage.required` 等）と CLI/LSP 出力を整合させ、ドメイン別分析が可能になるよう補強する。

## 3. 影響範囲と検証
- **スキーマ**: `tooling/json-schema/diagnostic-v2.schema.json` でドメイン列挙を更新し、`scripts/validate-diagnostic-json.sh` に新ドメインのサンプルを追加。  
- **監査**: `collect-iterator-audit-metrics.py` など監査スクリプトを更新し、新しいドメインが集計対象になるか確認。  
- **CLI/LSP**: ドメイン別ハイライトやフィルタリングが仕様通りに動作するか手動テストを追加。
- **用語集**: `docs/spec/0-2-glossary.md` に `DiagnosticDomain::Effect` / `Plugin` などの定義を追加し、監査レポートでも同一語彙で記録できるようにする。

## 4. フォローアップ
- Phase 2-7 `diagnostic-domain` タスクと連携して、`DiagnosticDomain` 拡張の実装スケジュールを決定。  
- ドメインを拡張した後、`docs/spec/3-6-core-diagnostics-audit.md` に脚注を追記し、OCaml 実装が追従したタイミングで脚注を削除。  
- `docs/notes/dsl-plugin-roadmap.md` にプラグイン診断のドメイン整理項目を追加しておく。
- `docs/plans/bootstrap-roadmap/2-5-review-log.md` にレビュー結果と残課題（`Other(Str)` の表現方針など）を記載し、Phase 2-8 以降での追跡と承認判断に備える。

## 確認事項
- 既存診断との互換性（`Note` → `Info` 等のマッピング）と併せ、ドメイン拡張で既存 JSON 出力に影響がないかを CLI/LSP チームと確認する必要がある。  
- `Other(Str)` を OCaml 実装でどのように表現するか（自由文字列 vs. enum 拡張）を決める必要がある。
