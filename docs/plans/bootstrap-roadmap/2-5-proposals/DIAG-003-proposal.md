# DIAG-003 診断ドメイン語彙拡張計画

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
- **タイミング**: Phase 2-5 の中盤でスキーマ・実装調整に着手し、Phase 2-7 の診断ダッシュボード更新が始まる前までに適用する。

## 5. 実施ステップ
1. **現状棚卸しと仕様語彙の再整理（Week31 Day1）**  
   - `compiler/ocaml/src/diagnostic.ml` / `diagnostic_serialization.ml` / `cli/json_formatter.ml` / `tooling/lsp/lsp_transport.ml` を横断し、`error_domain` 列挙のハードコード箇所と分岐網羅率を洗い出す。  
   - `docs/spec/3-6-core-diagnostics-audit.md:160-215` と `docs/spec/3-8-core-runtime-capability.md:210-260`、`docs/spec/4-7-core-parse-plugin.md:120-188` を再確認し、`Effect` / `Target` / `Plugin` / `Lsp` / `Other` の定義根拠と関連メタデータ（例: `effect.stage.required`, `bridge.stage.reload`, `plugin.bundle.signature`）を表形式で整理する。  
   - `rg "error_domain"`、`rg "DiagnosticDomain"`、`rg "extensions\[\"effects\"\]"` を用いて利用箇所を列挙し、Phase 2-7 の RunConfig/lex シム計画（PARSER-002 / LEXER-002 / EFFECT-003）とのインターフェイスを棚卸しする。

2. **OCaml モデル層の列挙拡張と移行補助実装（Week31 Day2-3）**  
   - `type error_domain` を `Effect | Target | Plugin | Lsp | Runtime | Parser | Type | Config | Network | Data | Audit | Security | Cli | Other of string` に改修し、既存の 9 ドメインは旧値を維持したまま `Other _` へ落とし込まない方針を明記する。  
   - `Diagnostic.Builder.set_domain` と `Diagnostic.make` 系 API に `Other of string` を渡すためのヘルパ（`Diagnostic.Domain.other : string -> t`）を追加し、コンパイル エラー箇所を `Week31 Day2` 中に解消する。  
   - `Legacy` 経路（`diagnostic_of_legacy`）で未知ドメインを受け取った際は `Other legacy_domain` として保持し、`AuditEnvelope.metadata["legacy.domain"]` に原文字列を記録する。  
   - 調査: `compiler/ocaml/tests` で `Diagnostic.create_*` を直接呼ぶテストを列挙し、新列挙への置換とゴールデン再生成の要否を洗い出す。

3. **シリアライズ・CLI/LSP 出力とスキーマ更新（Week31 Day3-4）**  
   - `compiler/ocaml/src/diagnostic_serialization.ml` の `domain_to_json`、`domain_of_json` を新列挙へ対応させ、`Other` は `"other"` + `extensions["domain.other"]` に分離してシリアライズする。  
   - `tooling/json-schema/diagnostic-v2.schema.json` の `domain.enum` を仕様語彙へ更新し、`scripts/validate-diagnostic-json.sh` ゴールデンに `Effect` / `Plugin` / `Lsp` / `Other` ケースを追加。  
   - CLI/LSP 出力：`cli/json_formatter.ml` と `tooling/lsp/lsp_transport.ml` でドメインごとのカラーリング・フィルタリングを調整し、`Effect` は `effects` メッセージ群、`Plugin` は `plugin.bundle` 監査へのリンクが辿れるようにする。  
   - 調査: `tooling/ci/collect-iterator-audit-metrics.py` における `domain_counts` の集計列挙を特定し、新 enum へ対応する差分を洗い出す。

4. **監査メタデータとメトリクス整合（Week31 Day4-5）**  
   - `Diagnostic.extensions["effects"]` に加えて `extensions["capability"]`, `extensions["plugin"]`, `extensions["lsp"]` を追加し、`RunConfig.extensions["lex"]`（LEXER-002）や `Capability` 監査（EFFECT-003）から受け取った Stage/Capability 情報を格納できるようにする。  
   - `AuditEnvelope.metadata` に `event.domain`, `event.kind`, `capability.ids[*]`, `plugin.bundle_id` を追加し、`docs/spec/3-6-core-diagnostics-audit.md` のキーセットと突合するユニットテストを `compiler/ocaml/tests/test_cli_diagnostics.ml` / `test_type_inference.ml` へ実装。  
   - `tooling/ci/collect-iterator-audit-metrics.py` に `diagnostics.domain_coverage`, `diagnostics.effect_stage_consistency`, `diagnostics.plugin_bundle_ratio` を追加し、`--require-success` で新指標が 1.0 を維持することを確認する。  
   - 調査: `docs/plans/bootstrap-roadmap/2-5-review-log.md` の DIAG-003 エントリを更新し、`Phase 2-7` の Capability/Stage 監査計画と整合性をチェック。

5. **ドキュメント・脚注・ハンドオフ更新（Week32 Day1）**  
   - `docs/spec/3-6-core-diagnostics-audit.md` と `docs/spec/0-2-glossary.md` に新ドメインの定義脚注を追加し、OCaml 実装の反映日時を `Phase 2-5` フッタに記録。  
   - `docs/plans/bootstrap-roadmap/2-5-review-log.md` へ Step1〜4 の作業ログと検証結果を追記し、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に Stage/Plugin ドメイン連携 TODO（CI 監査ダッシュボード改修）を登録。  
   - `docs/spec/3-8-core-runtime-capability.md` / `docs/notes/dsl-plugin-roadmap.md` / `docs/guides/runtime-bridges.md` の参照セクションに `DiagnosticDomain` 変更を周知する脚注を挿入し、RunConfig/lex シムや複数 Capability 計画との依存関係を明文化する。

## 残課題
- 既存診断との互換性（`Note` → `Info` 等のマッピング）と併せ、ドメイン拡張で既存 JSON 出力に影響がないかを CLI/LSP チームと確認する必要がある。  
- `Other(Str)` を OCaml 実装でどのように表現するか（自由文字列 vs. enum 拡張）を決める必要がある。
