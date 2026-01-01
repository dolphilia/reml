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

   #### Step1 棚卸し結果（2025-11-26 更新）
   - ✅ OCaml 実装の診断ドメインは `Parser` / `Type` / `Config` / `Runtime` / `Network` / `Data` / `Audit` / `Security` / `CLI` の 9 件に固定されており、仕様が要求する `Effect` / `Target` / `Plugin` / `Lsp` / `Manifest` / `Syntax` / `Regex` / `Template` / `Text` / `Other(Str)` が未定義のままである（`compiler/ocaml/src/diagnostic.ml:54-63`、`compiler/ocaml/src/diagnostic_serialization.ml:125-139`）。  
   - ✅ JSON/LSP 出力も同じ列挙を前提にしており、未知ドメインを安全に扱うフォールバックが存在しない。CLI と LSP は `domain_to_string` が `None` を返した場合にフィールドを欠落させるため、仕様で必須とされる `Effect` や `Target` 系メタデータを伝播できない（`compiler/ocaml/src/cli/json_formatter.ml:108-198`、`tooling/lsp/lsp_transport.ml:68-126`）。  
   - ✅ `parser_driver` 系の生成経路では `Diagnostic.Parser` / `Diagnostic.Config` のみ利用実績があり、型・効果・プラグイン由来の診断は未分類のまま出力されている。CI でも `diag.get("domain") == "parser"` といった特定値判定に留まっており、新語彙導入時に集計指標の見直しが必要（`compiler/ocaml/src/parser_driver.ml:58-145`、`tooling/ci/collect-iterator-audit-metrics.py:334-352`）。

   **現行実装の棚卸し表**

   | 観点 | 現状 | 主な参照 | 備考 |
   |------|------|----------|------|
   | OCaml 列挙 | `Parser` / `Type` / `Config` / `Runtime` / `Network` / `Data` / `Audit` / `Security` / `CLI` | `compiler/ocaml/src/diagnostic.ml:54-63` | 仕様 3-6 の語彙と最大 8 件乖離。 |
   | JSON/LSP 変換 | 9 件のみを `domain_to_string` で文字列化 | `compiler/ocaml/src/diagnostic_serialization.ml:125-139`<br>`compiler/ocaml/src/cli/json_formatter.ml:108-198`<br>`tooling/lsp/lsp_transport.ml:68-126` | 未知ドメインは `None` 扱いでフィールド欠落。監査メタデータ追跡不可。 |
   | 生成経路 | `parser_driver` が `Parser` / `Config` を固定設定、その他経路は `None` | `compiler/ocaml/src/parser_driver.ml:58-145` | 型/効果/プラグイン診断で適切なドメインが設定されず、CLI/LSP フィルタリングが困難。 |
   | テスト・メトリクス | ゴールデンは `parser` / `config` / `type` / `cli` のみ | `compiler/ocaml/tests/golden/diagnostics/*`<br>`tooling/ci/collect-iterator-audit-metrics.py:334-352` | スキーマ更新時に追加サンプルが必須。 |

   **仕様上の語彙整理**

   | ドメイン / 語彙 | 定義箇所 | 関連メタデータ・備考 |
   |-----------------|----------|-----------------------|
   | 基本 12 項目（`Syntax` / `Parser` / `Type` / `Effect` / `Runtime` / `Config` / `Manifest` / `Target` / `Security` / `Plugin` / `Cli` / `Lsp` / `Other(Str)`） | `docs/spec/3-6-core-diagnostics-audit.md:178-191` | `Other(Str)` は snake_case 推奨。実装は `Manifest` / `Effect` / `Plugin` / `Lsp` を未提供。 |
   | `Effect` ドメイン | `docs/spec/3-6-core-diagnostics-audit.md:324-343`<br>`docs/spec/3-8-core-runtime-capability.md:132-285` | `extensions["effects"]` に `stage.*` / `iterator.*` を必須出力、`AuditEnvelope.metadata["effect.stage.required"]` 等と同期。 |
   | `Target` / `Manifest` | `docs/spec/3-6-core-diagnostics-audit.md:963-999`<br>`docs/spec/3-8-core-runtime-capability.md:254-285` | `extensions["target"]` に `profile_id` / `triple` / `capabilities` を記録し、監査ログ `event.domain = "target"` と整合。 |
   | `Plugin` | `docs/spec/4-7-core-parse-plugin.md:120-188` | `AuditEnvelope.metadata["plugin.signature.*"]` と `extensions["plugin"].bundle_id` が必須。 |
   | `Regex` | `docs/spec/2-2-core-combinator.md:274`<br>`docs/spec/2-6-execution-strategy.md:259`<br>`docs/spec/3-3-core-text-unicode.md:428` | `extensions["regex"].unicode_profile` や RunConfig `extensions["regex"]` と連携。 |
   | `Text` | `docs/spec/3-3-core-text-unicode.md:93` | Unicode 正規化フェーズで `DiagnosticDomain::Text` を利用。 |
   | `Template` | `docs/spec/3-6-core-diagnostics-audit.md:905-939` | テンプレート DSL 用の `extensions["template"]` を定義済み。 |

   - Phase 2-7 以降の RunConfig / lex シム（PARSER-002 / LEXER-002 / EFFECT-003）で共有する `extensions["lex"]` / `extensions["effects"]` / `extensions["plugin"]` / `extensions["target"]` の命名規則はすべて上記仕様に依存する。棚卸し結果を関連計画へ展開し、列挙追加とメタデータ整備を同時に進める必要がある。

2. **OCaml モデル層の列挙拡張と移行補助実装（Week31 Day2-3）**  
   - `type error_domain` を `Effect | Target | Plugin | Lsp | Runtime | Parser | Type | Config | Network | Data | Audit | Security | Cli | Other of string` に改修し、既存の 9 ドメインは旧値を維持したまま `Other _` へ落とし込まない方針を明記する。  
   - `Diagnostic.Builder.set_domain` と `Diagnostic.make` 系 API に `Other of string` を渡すためのヘルパ（`Diagnostic.Domain.other : string -> t`）を追加し、コンパイル エラー箇所を `Week31 Day2` 中に解消する。  
   - `Legacy` 経路（`diagnostic_of_legacy`）で未知ドメインを受け取った際は `Other legacy_domain` として保持し、`AuditEnvelope.metadata["legacy.domain"]` に原文字列を記録する。  
   - 調査: `compiler/ocaml/tests` で `Diagnostic.create_*` を直接呼ぶテストを列挙し、新列挙への置換とゴールデン再生成の要否を洗い出す。

   #### Step2 実装結果（2025-11-26 更新）
   - ✅ `compiler/ocaml/src/diagnostic.ml` の `error_domain` を仕様語彙（`Effect` / `Target` / `Plugin` / `Lsp` など）へ拡張し、既存 9 項目は専用コンストラクタとして維持。未知値は `Other of string` へマップする。
   - ✅ `Diagnostic.Domain` モジュールを新設し、`other` ヘルパで未知ドメイン文字列を安全に登録できるようにした（空文字は `"other"` に正規化）。
   - ✅ `domain_label` / `domain_to_string` / CLI テスト類を新 enum に追従させ、`Cli` へリネームしたコントラクタの参照を更新。
   - ℹ️ `Legacy` 由来の生ドメイン記録は Step4 以降で扱う想定のため、現時点では `Other` 経路のみ整備。

3. **シリアライズ・CLI/LSP 出力とスキーマ更新（Week31 Day3-4）**  
   - `compiler/ocaml/src/diagnostic_serialization.ml` の `domain_to_json`、`domain_of_json` を新列挙へ対応させ、`Other` は `"other"` + `extensions["domain.other"]` に分離してシリアライズする。  
   - `tooling/json-schema/diagnostic-v2.schema.json` の `domain.enum` を仕様語彙へ更新し、`scripts/validate-diagnostic-json.sh` ゴールデンに `Effect` / `Plugin` / `Lsp` / `Other` ケースを追加。  
   - CLI/LSP 出力：`cli/json_formatter.ml` と `tooling/lsp/lsp_transport.ml` でドメインごとのカラーリング・フィルタリングを調整し、`Effect` は `effects` メッセージ群、`Plugin` は `plugin.bundle` 監査へのリンクが辿れるようにする。  
   - 調査: `tooling/ci/collect-iterator-audit-metrics.py` における `domain_counts` の集計列挙を特定し、新 enum へ対応する差分を洗い出す。

   #### Step3 実施結果（2025-11-27 更新）
   - ✅ `compiler/ocaml/src/diagnostic_serialization.ml` で `Other` ドメインを `"other"` 固定文字列として出力しつつ、`extensions["domain.other"]` に元の識別子を保持する正規化ロジックを追加。`domain_of_json` を拡張し、未知ドメインは `Other` 扱いで丸める互換経路を整備。  
   - ✅ CLI/LSP 経路は新しい正規化済み拡張を透過的に利用できるようになり、`compiler/ocaml/tests/test_cli_diagnostics.ml` に `domain = "type"` のアサーションと `Other` ドメイン用のシリアライズ検証（`domain.other = "plugin_bundle"`）を追加。  
   - ✅ `tooling/json-schema/diagnostic-v2.schema.json` の `domain` を仕様語彙（`syntax`〜`other` + 既存互換）へ更新し、新しいゴールデン `compiler/ocaml/tests/golden/diagnostics/domain/multi-domain.json.golden` を追加して Plugin/Lsp/Other ケースが `scripts/validate-diagnostic-json.sh` の既定ターゲットで検証されるようにした。  
   - ⚠ `tooling/ci/collect-iterator-audit-metrics.py` の `domain` 集計は現状読み取り専用のまま。Phase 2-7 で `diagnostics.domain_coverage` 指標を実装する TODO を Step4 以降へ引き継ぐ必要がある。

4. **監査メタデータとメトリクス整合（Week31 Day4-5）**  
   - `Diagnostic.extensions["effects"]` に加えて `extensions["capability"]`, `extensions["plugin"]`, `extensions["lsp"]` を追加し、`RunConfig.extensions["lex"]`（LEXER-002）や `Capability` 監査（EFFECT-003）から受け取った Stage/Capability 情報を格納できるようにする。  
   - `AuditEnvelope.metadata` に `event.domain`, `event.kind`, `capability.ids[*]`, `plugin.bundle_id` を追加し、`docs/spec/3-6-core-diagnostics-audit.md` のキーセットと突合するユニットテストを `compiler/ocaml/tests/test_cli_diagnostics.ml` / `test_type_inference.ml` へ実装。  
   - `tooling/ci/collect-iterator-audit-metrics.py` に `diagnostics.domain_coverage`, `diagnostics.effect_stage_consistency`, `diagnostics.plugin_bundle_ratio` を追加し、`--require-success` で新指標が 1.0 を維持することを確認する。  
   - 調査: `docs/plans/bootstrap-roadmap/2-5-review-log.md` の DIAG-003 エントリを更新し、`Phase 2-7` の Capability/Stage 監査計画と整合性をチェック。

   #### Step4 実施結果（2025-11-28 更新）
   - ✅ `compiler/ocaml/src/diagnostic.ml` で `extensions["capability"]` / `extensions["plugin"]` / `extensions["lsp"]` を実装し、`event.domain`・`event.kind`・`capability.ids`・`plugin.bundle_id` を `audit_metadata` と `AuditEnvelope.metadata` に自動付与するよう統合。  
   - ✅ `compiler/ocaml/tests/test_cli_diagnostics.ml` と `compiler/ocaml/tests/test_type_inference.ml` に新メタデータ検証テストを追加し、`tooling/ci/collect-iterator-audit-metrics.py` へ `diagnostics.domain_coverage` / `diagnostics.effect_stage_consistency` / `diagnostics.plugin_bundle_ratio` を組み込み。  
   - ⚠ フォローアップ: EFFECT-003 で予定している複数 Capability 対応に合わせ、`capability.ids` の配列比較ロジックを Phase 2-7 で再確認する（複数 Stage を返すケースのサンプル拡充が必要）。

5. **ドキュメント・脚注・ハンドオフ更新（Week32 Day1）**  
   - `docs/spec/3-6-core-diagnostics-audit.md` と `docs/spec/0-2-glossary.md` に新ドメインの定義脚注を追加し、OCaml 実装の反映日時を `Phase 2-5` フッタに記録。  
   - `docs/plans/bootstrap-roadmap/2-5-review-log.md` へ Step1〜4 の作業ログと検証結果を追記し、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に Stage/Plugin ドメイン連携 TODO（CI 監査ダッシュボード改修）を登録。  
   - `docs/spec/3-8-core-runtime-capability.md` / `docs/notes/dsl-plugin-roadmap.md` / `docs/guides/runtime/runtime-bridges.md` の参照セクションに `DiagnosticDomain` 変更を周知する脚注を挿入し、RunConfig/lex シムや複数 Capability 計画との依存関係を明文化する。

   #### Step5 実施結果（2025-11-30 更新）
   - ✅ `docs/spec/3-6-core-diagnostics-audit.md` に `Diagnostic.domain` 拡張メモと脚注 `[^diag003-phase25-domain]` を追加し、OCaml 実装の語彙整合を記録。  
   - ✅ `docs/spec/0-2-glossary.md` へ新語彙一覧と脚注 `[^diag003-phase25-glossary]` を追記し、読者が `Effect` / `Target` / `Plugin` / `Lsp` / `Other(Str)` を参照できるようにした。  
   - ✅ `docs/spec/3-8-core-runtime-capability.md`, `docs/guides/runtime/runtime-bridges.md`, `docs/notes/dsl-plugin-roadmap.md` に脚注を挿入し、RunConfig/Capability/Plugin 計画と診断ドメイン拡張の依存関係を共有。  
   - ✅ `docs/plans/bootstrap-roadmap/2-5-review-log.md` に Step5 のサマリ・検証記録を追加し、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` へ CI ダッシュボード改修 TODO を登録。  
   - ℹ️ フォローアップ: EFFECT-003 で複数 Capability を扱うサンプルの収集が完了した段階で脚注の有効性を再確認し、`diagnostics.domain_coverage` グラフ設計のレビューを行う。

## 残課題
- 既存診断との互換性（`Note` → `Info` 等のマッピング）と併せ、ドメイン拡張で既存 JSON 出力に影響がないかを CLI/LSP チームと確認する必要がある。  
- `Other(Str)` を OCaml 実装でどのように表現するか（自由文字列 vs. enum 拡張）を決める必要がある。
