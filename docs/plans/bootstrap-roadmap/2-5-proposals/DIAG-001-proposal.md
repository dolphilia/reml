# DIAG-001 Severity 列挙拡張計画

## 1. 背景と症状
- Chapter 3 は `Severity = Error | Warning | Info | Hint` を必須フィールドとして定義し（docs/spec/3-6-core-diagnostics-audit.md:21-40）、CLI/LSP が情報診断やヒントを区別できることを前提にしている。  
- 現行 OCaml 実装は `type severity = Error | Warning | Note` としており（compiler/ocaml/src/diagnostic.ml:39-44）、`Info`/`Hint` 相当を表現できない。CLI/LSP では情報診断が `Warning` に丸められ、フェーズ 3 の段階的リリース条件を満たせない。  
- JSON スキーマや監査メタデータでも `Info`/`Hint` が欠落し、`diagnostic_schema.validation_pass` では検出できない。

## 2. Before / After
### Before
- `Info` レベルの診断（例: インラインヒント、性能情報）が `Warning` や `Note` として報告され、意図したフィルタリングが行えない。  
- CLI/LSP のメッセージが仕様と乖離し、`0-3-audit-and-metrics.md` の診断指標でも情報レベルが判別不能。

### After
- `severity` 列挙を `Error | Warning | Info | Hint` に置き換え、`Note` 互換ロジックを廃止。`Diagnostic.V2` の変換ロジックも更新する。  
- JSON スキーマ（`tooling/json-schema/diagnostic-v2.schema.json`）と CLI/LSP のゴールデンを更新し、`Info`/`Hint` が有効に出力されるようにする。  
- 仕様との差分を解消し、情報診断を用いた段階的リリースが可能になる。

## 3. 影響範囲と検証
- **スキーマ検証**: `scripts/validate-diagnostic-json.sh` に `Info`/`Hint` を含むフィクスチャを追加し、CI で検証。  
- **CLI/LSP**: 既存ゴールデンを更新し、新しい Severity が正しくレンダリングされるか確認。  
- **LSP マッピング**: `tooling/lsp/src/diagnostic_adapter.ml`（行番号要確認）で LSP の `DiagnosticSeverity::{Error = 1, Warning = 2, Information = 3, Hint = 4}` へ確実に写像するユニットテストを追加し、VS Code 拡張のサンプルログを `reports/diagnostic-format-regression.md` に追記する。  
- **メトリクス**: `0-3-audit-and-metrics.md` に `diagnostic.info_hint_ratio` を追加し、情報レベル診断の発生状況を監視。

## 4. フォローアップ
- CLI テキスト出力刷新（Phase 2-7）で Severity 表示を再調整し、`--ack-experimental` 等の挙動と整合させる。  
- `docs/spec/3-6-core-diagnostics-audit.md` の脚注に OCaml 実装の更新時期を記録し、導入完了後に脚注を削除。  
- `tooling/lsp` の V2 トランスポートで `Info`/`Hint` を適切な LSP Severity 値へマッピングする。
- **タイミング**: Phase 2-5 の前半で準備が整い次第着手し、Phase 2-7 の診断刷新タスクに入るまでに本番導入を完了させる。

## 5. 実施ステップ
1. **現状棚卸しと仕様突合（Week31 Day1-2）**  
   - `compiler/ocaml/src/diagnostic.ml`, `diagnostic_builder.ml`, `diagnostic_serialization.ml` を中心に、`type severity` の定義とパターンマッチ箇所を洗い出す。`note` 表記が残っている分岐を一覧化し、`docs/plans/bootstrap-roadmap/2-5-review-log.md` に調査結果を記録する。  
   - `tooling/json-schema/diagnostic-v2.schema.json` と `scripts/validate-diagnostic-json.sh` が期待する列挙値を確認し、`Severity = Error | Warning | Info | Hint` に揃えるための差分を把握する。  
   - `tooling/lsp/src/diagnostic_adapter.ml` と VS Code 拡張のログ（`reports/diagnostic-format-regression.md`）から、LSP `DiagnosticSeverity` への写像がどこで行われているか調査する。  
   - 調査補助: `grep -R \"severity\" compiler/ocaml tooling -n` を実行し、直接 `Warning`/`Note` をベタ打ちしている箇所の洗い漏れを防ぐ。

2. **列挙型と OCaml 実装の更新（Week31 Day3-4）**  
   - `compiler/ocaml/src/diagnostic.ml` の `type severity` を `Error | Warning | Info | Hint` へ改修し、`Note` を `Info` へ移し替える。`Diagnostic.make` / `Diagnostic.Builder` / `Diagnostic.of_json` / `Diagnostic.to_json` 等の API に対するパターンマッチを全て更新する。  
   - `Diagnostic.V2` 相当の変換ロジック（`diagnostic_serialization.ml` および `compiler/ocaml/src/diagnostic_builder.ml`）で `Info`/`Hint` を保持するように改修し、`Note` フォールバックを削除する。  
   - CLI テキスト出力（`compiler/ocaml/src/diagnostic_printer.ml` など）で Severity ごとの整形・色分けが `Hint` を処理できることを確認し、必要に応じて新しいラベルを追加する。  
   - 調査補助: `compiler/ocaml/tests/test_cli_diagnostics.ml` と `test_type_inference.ml` における Severity 期待値を確認し、追加テストが必要なケースをメモ化する。

3. **スキーマ・フィクスチャ・テストの拡張（Week31 Day4-5）**  
   - `tooling/json-schema/diagnostic-v2.schema.json` の列挙定義を更新し、`Info`/`Hint` の必須化を反映したバージョンを用意する。  
   - `scripts/validate-diagnostic-json.sh` に `Info`/`Hint` を含むゴールデン JSON（`tooling/fixtures/diagnostics/`）を追加し、CI で検証できるようにする。  
   - `compiler/ocaml/tests/test_cli_diagnostics.ml` および `tooling/lsp/tests/diagnostic_adapter_tests.ml`（存在しない場合は新設）に `Info`/`Hint` を含むケースを追加し、`scripts/validate-diagnostic-json.sh` を実行して差分を確認する。  
   - 調査補助: `reports/diagnostic-format-regression.md` と `docs/plans/bootstrap-roadmap/2-4-completion-report.md` のメトリクス欄を読み、既存の監査比率計測がどの Severity を前提にしているか整理する。

4. **CLI/LSP/監査パイプラインの整合確認（Week32 Day1-2）**  
   - `tooling/lsp/src/diagnostic_adapter.ml` で `Info -> DiagnosticSeverity.Information`, `Hint -> DiagnosticSeverity.Hint` に正しく写像されるテーブルを実装し、ユニットテストと VS Code の手動検証ログを `reports/diagnostic-format-regression.md` に追記する。  
   - CLI の JSON/テキスト出力が `Info`/`Hint` を保持することを `scripts/validate-diagnostic-json.sh` と CLI ゴールデン（`tooling/fixtures/cli/diagnostics/`）で検証し、`--fail-on-warning` オプションとの整合を記録する。  
   - `tooling/ci/collect-iterator-audit-metrics.py` に `diagnostic.info_hint_ratio` 集計を追加し、`0-3-audit-and-metrics.md` に新メトリクスの定義とサンプリング頻度を追記する準備を行う。  
   - 調査補助: `docs/spec/3-6-core-diagnostics-audit.md` の Severity 表（§3.2 以降）を参照し、既定 Severity が `Warning` → `Info` へ変更されるコードがないか確認する。

5. **ドキュメントとメトリクス更新（Week32 Day3）**  
   - `docs/spec/3-6-core-diagnostics-audit.md` へ脚注を追加し、OCaml 実装での適用完了と `Note` 廃止方針を明記する。  
   - `0-3-audit-and-metrics.md` に `diagnostic.info_hint_ratio` と `diagnostic.hint_surface_area`（必要なら）を追加し、収集方法と期待値レンジを定義する。  
   - `docs/plans/bootstrap-roadmap/2-5-review-log.md` に実装完了メモと残課題を記録し、`README.md`（Phase 2-5 カタログ）でステータスを更新する。  
   - 調査補助: `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` を確認し、Phase 2-7 へエスカレーションする項目（例: CLI 表示刷新）が重複しないよう整理する。

## 6. 残課題
- 既存 `Note` 表現を `Info` へ移行する際の互換ポリシー（JSON / CLI / LSP 出力）をチーム間で調整する必要がある。  
- `Hint` レベルの診断をどの機能で発行するか（例: 自動修正候補）を Phase 2-7 と相談したい。
