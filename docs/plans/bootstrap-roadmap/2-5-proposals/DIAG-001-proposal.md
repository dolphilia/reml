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

## 残課題
- 既存 `Note` 表現を `Info` へ移行する際の互換ポリシー（JSON / CLI / LSP 出力）をチーム間で調整する必要がある。  
- `Hint` レベルの診断をどの機能で発行するか（例: 自動修正候補）を Phase 2-7 と相談したい。
