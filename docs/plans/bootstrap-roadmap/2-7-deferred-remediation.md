# 2.7 診断パイプライン残課題・技術的負債整理計画

## 目的
- Phase 2-4 で持ち越した診断・監査パイプライン関連タスクと技術的負債（ID 22/23 など）を集中して解消する。
- CLI/LSP/CI の各チャネルで `Diagnostic` / `AuditEnvelope` の新仕様を安定運用できる状態を整え、Phase 2-8 の仕様検証に備える。

## スコープ
- **含む**: Windows/macOS CI での監査ゲート導入、LSP V2 互換テスト整備、CLI フォーマッタの再統合、技術的負債リストで Phase 2 中に解消可能な項目。
- **含まない**: 仕様書の全文レビュー（Phase 2-8 で実施）、新規機能の追加、Phase 3 以降へ移送済みの低優先度負債。
- **前提**:
  - Phase 2-4 の共通シリアライズ層導入と JSON スキーマ検証が完了していること。
  - Phase 2-5 の仕様差分補正で参照する基礎データ（差分リスト草案）が揃っていること。
  - Phase 2-6 の Windows 実装で `--emit-audit` を実行できる環境が CI 上に整備済みであること。

## 作業ディレクトリ
- `compiler/ocaml/src/cli/` : `diagnostic_formatter.ml`, `json_formatter.ml`, `options.ml`
- `compiler/ocaml/src/diagnostic_*` : Builder/API 互換レイヤ
- `tooling/lsp/` : `diagnostic_transport.ml`, `compat/`, `tests/client_compat`
- `tooling/ci/` : `collect-iterator-audit-metrics.py`, `sync-iterator-audit.sh`, 新規検証スクリプト
- `scripts/` : CI 向け検証スクリプト、レビュー補助ツール
- `reports/` : 監査ログサマリ、診断フォーマット差分
- `compiler/ocaml/docs/technical-debt.md` : ID 22/23, H1〜H4 の進捗更新

## 作業ブレークダウン

### 1. 監査ゲート整備（34-35週目）
**担当領域**: Windows/macOS CI

1.1. **Windows Stage 自動検証 (ID 22)**
- `tooling/ci/sync-iterator-audit.sh` を MSYS2 Bash で動作させ、`--platform windows-msvc` 実行パスを整備。
- `tooling/ci/collect-iterator-audit-metrics.py` に Windows プラットフォーム専用プリセット (`--platform windows-msvc`) を追加し、`ffi_bridge.audit_pass_rate` / `iterator.stage.audit_pass_rate` を算出。
- `bootstrap-windows.yml` に `audit-matrix` ジョブを追加し、pass_rate < 1.0 の場合は PR を失敗させる。
- `reports/ffi-bridge-summary.md` と `docs/plans/bootstrap-roadmap/2-3-to-2-4-handover.md` の TODO 欄を更新。

1.2. **macOS FFI サンプル自動検証 (ID 23)**
- `ffi_dispatch_async.reml` / `ffi_malloc_arm64.reml` をビルド可能なよう修正し、`scripts/ci-local.sh --target macos-arm64 --emit-audit` に組み込む。
- `collect-iterator-audit-metrics.py` で `bridge.platform = macos-arm64` の pass_rate 集計を追加し、`ffi_bridge.audit_pass_rate` に反映。
- `bootstrap-macos.yml` に監査ゲートを追加し、成果物 (audit JSON, summary) をアーティファクト化。

**成果物**: Windows/macOS CI 監査ゲート、更新済みレポート、技術的負債リスト反映

### 2. CLI 出力統合とテキストフォーマット刷新（35週目前半）
**担当領域**: CLI フォーマッタ

2.1. **`--format` / `--json-mode` 集約**
- `compiler/ocaml/src/cli/options.ml` で `--format` と `--json-mode` の派生オプションを整理し、`SerializedDiagnostic` を利用するフォーマッタ選択ロジックを再構築。
- `docs/spec/0-0-overview.md` と `docs/guides/ai-integration.md` に新オプションを追記。

2.2. **テキストフォーマット刷新**
- `compiler/ocaml/src/cli/diagnostic_formatter.ml` を `SerializedDiagnostic` ベースへ移行し、`unicode_segment.ml`（新規）を導入して Grapheme 単位のハイライトを実装。
- `--format text --no-snippet` を追加し、CI 向けログを簡略化。
- テキストゴールデン (`compiler/ocaml/tests/golden/diagnostics/*.golden`) を更新し、差分は `reports/diagnostic-format-regression.md` に記録。

**成果物**: CLI オプション整理、テキストフォーマッタ更新、ドキュメント追記

### 3. LSP V2 互換性確立（35週目後半）
**担当領域**: LSP・フロントエンド

3.1. **フィクスチャ拡充とテスト**
- `tooling/lsp/tests/client_compat/fixtures/` に効果診断・Windows/macOS 監査ケースを追加し、AJV スキーマ検証を更新。
- `npm run ci` にフィクスチャ差分のレポート出力を追加し、PR で参照可能にする。

3.2. **`lsp-contract` CI ジョブ**
- GitHub Actions に `lsp-contract` ジョブを追加し、V1/V2 双方の JSON を `tooling/json-schema/diagnostic-v2.schema.json` で検証。
- `tooling/lsp/README.md` と `docs/guides/plugin-authoring.md` に V2 連携手順を追記。

3.3. **互換レイヤ仕上げ**
- `tooling/lsp/compat/diagnostic_v1.ml` を安定化させ、`[@deprecated]` 属性を付与。
- `tooling/lsp/jsonrpc_server.ml` で `structured_hints` の `command`/`data` 変換エラーを `extensions.lsp.compat_error` に記録。

**成果物**: 拡充済み LSP テスト群、CI ジョブ、更新ドキュメント

### 4. 技術的負債の棚卸しとクローズ（36週目前半）
**担当領域**: 負債管理

4.1. **技術的負債リスト更新**
- `compiler/ocaml/docs/technical-debt.md` で ID 22 / 23 を完了扱いに更新し、H1〜H4 の進捗をレビュー。
- Phase 2 以内に解消できなかった項目を Phase 3 へ移送し、`0-4-risk-handling.md` に直結するリスクとして記録。

4.2. **レポート更新**
- `reports/diagnostic-format-regression.md` と `reports/ffi-bridge-summary.md` に完了状況を追記し、差分がないことを確認。
- 監査ログの成果物パスを `reports/audit/index.json` に登録し、`tooling/ci/create-audit-index.py` のテストを更新。

**成果物**: 最新化された技術的負債リスト、報告書更新、移送リスト

### 5. Phase 2-8 への引き継ぎ準備（36週目後半）
**担当領域**: ドキュメント整備

5.1. **差分記録**
- Phase 2-4, 2-7 で実施した変更点・残項目を `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` の前提セクションへ追記。
- 監査ログ/診断の安定化完了を `docs/plans/bootstrap-roadmap/2-8-spec-integrity-audit.md`（新規）から参照できるよう脚注を整備。

5.2. **メトリクス更新**
- `0-3-audit-and-metrics.md` に CI pass_rate の推移と LSP テスト完了状況を記録。
- `tooling/ci/collect-iterator-audit-metrics.py` の集計結果を `reports/audit/dashboard/` に反映し、Phase 2-8 のベースラインとする。

**成果物**: 更新済み前提資料、メトリクス記録、Phase 2-8 用脚注

## 成果物と検証
- Windows/macOS CI で `ffi_bridge.audit_pass_rate` / `iterator.stage.audit_pass_rate` が 1.0 を維持し、監査欠落時にジョブが失敗すること。
- CLI `--format` / `--json-mode` の整合が取れており、テキスト・JSON 双方のゴールデンが更新済みであること。
- LSP V2 の互換テストが `npm run ci` および GitHub Actions `lsp-contract` で成功し、フィクスチャ差分がレポートとして残ること。
- 技術的負債リストと関連レポートに最新状況が反映され、Phase 3 へ移送する項目が明確になっていること。

## リスクとフォローアップ
- CI 監査ゲート導入によるジョブ時間増大: 実行時間を監視し、10% 超過時はサンプル数の調整や並列化を検討。
- CLI フォーマット変更による開発者体験への影響: `reports/diagnostic-format-regression.md` で差分レビューを必須化し、顧客影響を評価。
- LSP V2 導入に伴うクライアント側調整: `tooling/lsp/compat/diagnostic_v1.ml` を一定期間維持し、互換性レイヤ廃止時のスケジュールを Phase 3 で検討。

## 参考資料
- [2-4-diagnostics-audit-pipeline.md](2-4-diagnostics-audit-pipeline.md)
- [2-3-to-2-4-handover.md](2-3-to-2-4-handover.md)
- [2-5-spec-drift-remediation.md](2-5-spec-drift-remediation.md)
- [2-6-windows-support.md](2-6-windows-support.md)
- [compiler/ocaml/docs/technical-debt.md](../../../compiler/ocaml/docs/technical-debt.md)
- [reports/diagnostic-format-regression.md](../../../reports/diagnostic-format-regression.md)
- [reports/ffi-bridge-summary.md](../../../reports/ffi-bridge-summary.md)
