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
   - ✅ Week31 Day1-2 棚卸し完了（2025-11-07 更新）。主な確認事項:
     - `compiler/ocaml/src/diagnostic.ml:39` は `Error | Warning | Note` のままで、`Diagnostic.V2` でのみ `Info`/`Hint` を仮想化（`compiler/ocaml/src/diagnostic.ml:803-821`）。`Hint` 相当のバリアントは実装経路が未整備。
     - `compiler/ocaml/src/diagnostic_serialization.ml:249-269` が 3 値前提のシリアライズ (`note` / 1-3) を維持し、CLI/LSP の JSON も同ロジックを利用。`Hint` 出力経路は未定義。
     - `tooling/json-schema/diagnostic-v2.schema.json:14-37` と `tooling/ci/collect-iterator-audit-metrics.py:1004-1025` は `severity` 4 値対応を期待するが、既存フィクスチャに `hint` ケースがなく `note -> info` へ丸めている。
     - 仕様間の差分として Chapter 2 が `Note` を保持（`docs/spec/2-5-error.md:12-55`）、Chapter 3 が `Info`/`Hint` を正式定義（`docs/spec/3-6-core-diagnostics-audit.md:24-43`）。整合化方針の整理が必要。
   - 詳細ログ: [`docs/plans/bootstrap-roadmap/2-5-review-log.md`](../2-5-review-log.md#2-5-レビュー記録--diag-001-week31-day1-2-現状棚卸し2025-11-07-更新) を参照。

2. **列挙型と OCaml 実装の更新（Week31 Day3-4）**  
   - `compiler/ocaml/src/diagnostic.ml` の `type severity` を `Error | Warning | Info | Hint` へ改修し、`Note` を `Info` へ移し替える。`Diagnostic.make` / `Diagnostic.Builder` / `Diagnostic.of_json` / `Diagnostic.to_json` 等の API に対するパターンマッチを全て更新する。  
   - `Diagnostic.V2` 相当の変換ロジック（`diagnostic_serialization.ml` および `compiler/ocaml/src/diagnostic_builder.ml`）で `Info`/`Hint` を保持するように改修し、`Note` フォールバックを削除する。  
   - CLI テキスト出力（`compiler/ocaml/src/diagnostic_printer.ml` など）で Severity ごとの整形・色分けが `Hint` を処理できることを確認し、必要に応じて新しいラベルを追加する。  
   - 調査補助: `compiler/ocaml/tests/test_cli_diagnostics.ml` と `test_type_inference.ml` における Severity 期待値を確認し、追加テストが必要なケースをメモ化する。
   - ✅ Week31 Day3-4 列挙型と OCaml 実装の更新完了（2025-10-27 更新）。主な変更点:
     - `compiler/ocaml/src/diagnostic.ml` の `type severity` を 4 値化し、日本語ラベルを `エラー/警告/情報/ヒント` へ更新。
     - `Diagnostic.V2` と `diagnostic_serialization.ml` のシリアライズで `info`/`hint` をネイティブ出力し、数値レベルを LSP と同じ `1-4` に整理。
     - CLI カラー判定（`compiler/ocaml/src/cli/color.ml`）で `Info` を従来の青、`Hint` をシアンに割り当て、`colorize_pointer` を含む出力経路を手動確認。

3. **スキーマ・フィクスチャ・テストの拡張（Week31 Day4-5）**  
   - `tooling/json-schema/diagnostic-v2.schema.json` の列挙定義を更新し、`Info`/`Hint` の必須化を反映したバージョンを用意する。  
   - `scripts/validate-diagnostic-json.sh` に `Info`/`Hint` を含むゴールデン JSON（`tooling/fixtures/diagnostics/`）を追加し、CI で検証できるようにする。  
   - `compiler/ocaml/tests/test_cli_diagnostics.ml` および `tooling/lsp/tests/diagnostic_adapter_tests.ml`（存在しない場合は新設）に `Info`/`Hint` を含むケースを追加し、`scripts/validate-diagnostic-json.sh` を実行して差分を確認する。  
   - 調査補助: `reports/diagnostic-format-regression.md` と `docs/plans/bootstrap-roadmap/2-4-completion-report.md` のメトリクス欄を読み、既存の監査比率計測がどの Severity を前提にしているか整理する。
   - ✅ Week31 Day4-5 Info/Hint JSON 拡張完了（2025-11-08 更新）  
     - `tooling/json-schema/diagnostic-v2.schema.json` で `severity = {error, warning, info, hint}` と LSP 数値（1-4）の両方を許容する `oneOf` を定義し、オプション `severity_level (1-4)` を追加。  
     - `compiler/ocaml/tests/golden/diagnostics/severity/info-hint.json.golden` を新設し、`cli.audit_id`/`cli.change_set` まで含む Info/Hint の完全スナップショットを登録。  
     - `compiler/ocaml/tests/test_cli_diagnostics.ml` に `test_info_hint_snapshot` を追加し、CI で JSON スナップショット検証が走るようにした。  
     - `scripts/validate-diagnostic-json.sh compiler/ocaml/tests/golden/diagnostics/severity/info-hint.json.golden` を実行して新スキーマで検証済み（要 `tooling/lsp/tests/client_compat` の npm 依存関係）。

4. **CLI/LSP/監査パイプラインの整合確認（Week32 Day1-2）**  
   - ✅ 2025-11-09: `lsp_transport.ml` の `severity_level_of_severity`（compiler/ocaml/src/diagnostic_serialization.ml:252-256）に合わせ、`tooling/lsp/tests/client_compat/tests/client_compat.test.ts:95` で `diagnostic-v2-info-hint.json`（fixtures）を読み込み `severity = [3, 4]` を検証。VS Code 互換フィクスチャを `tooling/lsp/tests/client_compat/fixtures/diagnostic-v2-info-hint.json` として追加し、`npm run ci` で Info/Hint が数値マッピングされることを確認。  
   - ✅ 2025-11-09: `scripts/validate-diagnostic-json.sh` の既定ターゲットと連携する CLI ゴールデン `compiler/ocaml/tests/golden/diagnostics/severity/info-hint.json.golden` をレビュー手順に組み込み、`reports/diagnostic-format-regression.md` に Info/Hint チェックリストを追記。`docs/spec/3-6-core-diagnostics-audit.md` §3.2 と突き合わせ、既定 Severity の降格が発生しないことを確認。  
   - ✅ 2025-11-09: `tooling/ci/collect-iterator-audit-metrics.py:1000-1036` へ `info_fraction` / `hint_fraction` / `info_hint_ratio` を追加し、CI ダッシュボードで Info/Hint の発生率をトラッキング可能にした（`diagnostics.info_hint_ratio`）。Phase 2-6 以降で `0-3-audit-and-metrics.md` をアップデートする下準備済み。  
   - メモ: CLI テキスト出力刷新（Phase 2-7 移管タスク）で Severity 表示の配色・`--fail-on-warning` の挙動を最終調整する。

5. **ドキュメントとメトリクス更新（Week32 Day3）**  
   - `docs/spec/3-6-core-diagnostics-audit.md` へ脚注を追加し、OCaml 実装での適用完了と `Note` 廃止方針を明記する。  
   - `0-3-audit-and-metrics.md` に `diagnostic.info_hint_ratio` と `diagnostic.hint_surface_area`（必要なら）を追加し、収集方法と期待値レンジを定義する。  
   - `docs/plans/bootstrap-roadmap/2-5-review-log.md` に実装完了メモと残課題を記録し、`README.md`（Phase 2-5 カタログ）でステータスを更新する。  
   - 調査補助: `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` を確認し、Phase 2-7 へエスカレーションする項目（例: CLI 表示刷新）が重複しないよう整理する。
   - ✅ Week32 Day3 ドキュメント反映完了（2025-11-10 更新）。`docs/spec/3-6-core-diagnostics-audit.md` へ DIAG-001 脚注を追加し、Severity 4 値化の経緯を記録。`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に `diagnostic.info_hint_ratio` を定義し、`collect-iterator-audit-metrics.py` のサマリ出力項目を文書化。`diagnostic.hint_surface_area` は Phase 2-7 実装予定として暫定登録し、`docs/plans/bootstrap-roadmap/2-5-review-log.md` に更新内容と残課題を追記済み。README の DIAG-001 ハイライトを最新ステータスへ更新。

## 6. 残課題
- 既存 `Note` 表現を `Info` へ移行する際の互換ポリシー（JSON / CLI / LSP 出力）をチーム間で調整する必要がある。  
- `Hint` レベルの診断をどの機能で発行するか（例: 自動修正候補）を Phase 2-7 と相談したい。
