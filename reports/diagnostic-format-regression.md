# 診断フォーマット差分レビュー手順

## 目的
- `diagnostic_serialization` の変更や CLI/LSP 出力の更新時に、旧フォーマットからの差分を可視化しレビューを容易にする。
- CI の `diagnostic-json` ジョブで検知したエラーの調査手順を明文化する。

## 1. ローカル検証手順
1. `npm ci --prefix tooling/lsp/tests/client_compat`
2. `npm run ci --prefix tooling/lsp/tests/client_compat`
3. `bash scripts/validate-diagnostic-json.sh tmp/diagnostics-output/`
4. 差分が発生した場合、`tooling/lsp/tests/client_compat/fixtures/` および `compiler/ocaml/tests/golden/diagnostics/` を比較し、意図した変更か確認する。

## 2. 差分レポートのまとめ方
- 変更前後の JSON を `jq --sort-keys` で整形し、`diff -u` で比較する。
- `effects.*` や `bridge.*` など拡張キーの追加は、対応する仕様書 (`docs/spec/3-6-core-diagnostics-audit.md` 等) を参照して説明を添える。
- `extensions.typeclass.dictionary.*` / `typeclass.dictionary.*` に変更が生じた場合は、辞書監査ゴールデン `compiler/ocaml/tests/golden/typeclass_dictionary_resolved.json.golden` を更新し、`typeclass.dictionary_pass_rate` のトラッキングに差分内容を反映する。
- 期待値の変更がある場合は、CLI テキスト出力も取得し、利用者視点で破壊的でないかを確認する。
- Phase 2-4 以降は `tooling/review/audit-diff.py --base <path> --target <path>` を併用し、Markdown/HTML レポート (`reports/audit/review/<commit>/diff.{md,html}`) を生成する。CI では `tooling/ci/publish-audit-diff.py` が同レポートを PR コメントへ要約投稿するため、レビュー担当者はコメントリンクを起点に確認する。
- `tooling/review/audit-diff.py --query-file tooling/review/presets/stage-regressions.dsl` のようにクエリファイルを指定して重要メタデータのみ抽出し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の指標（`diagnostic_regressions`, `audit_diff.regressions` など）に基づいて影響範囲を判断する。

## 3. CI 連携時の確認ポイント
- GitHub Actions `diagnostic-json` ジョブが失敗した場合、`tooling/lsp/tests/client_compat` のテストログと `scripts/validate-diagnostic-json.sh` の結果を参照する。
- スキーマ違反が発生した場合は `tooling/json-schema/diagnostic-v2.schema.json` を更新し、併せてフィクスチャを追加する。
- Windows/macOS 固有のフィクスチャ（`diagnostic-v2-ffi-macos-sample.json` など）が最新の監査ログと整合しているか確認する。
- `audit-review` 系ジョブ（`audit-diff`, `audit-dashboard`）が失敗した場合は、生成された `reports/audit/review/<commit>/diff.json` とダッシュボードアーティファクト (`reports/audit/dashboard/index.html`) を確認し、`collect-iterator-audit-metrics.py --section review` の出力に警告がないかチェックする。
- `tooling/review/audit-query --query '<dsl>' --from <path>` または `--query-file tooling/review/presets/<preset>.dsl` をローカルで実行し、CI の DSL クエリ結果と一致しているか検証する。差異がある場合は `tooling/review/audit_shared.py` の正規化ロジックまたは DSL プリセット (`tooling/review/presets/*.dsl`) を更新する。

## 4. レビュー用チェックリスト
- [ ] JSON スキーマ (`tooling/json-schema/diagnostic-v2.schema.json`) とサンプル出力が一致している。
- [ ] 全診断で `audit` / `timestamp` フィールドが欠落していない（`scripts/validate-diagnostic-json.sh` および `collect-iterator-audit-metrics.py --require-success` の結果を確認）。
- [ ] `scripts/validate-diagnostic-json.sh` の既定対象（`compiler/ocaml/tests/golden/diagnostics`, `compiler/ocaml/tests/golden/audit`）でエラーがない。
- [ ] `compiler/ocaml/tests/golden/typeclass_dictionary_resolved.json.golden` の辞書フィールド（`kind`, `identifier`, `repr`）が `extensions.*`／`audit_metadata` の双方で欠落していない。
- [ ] `npm run ci --prefix tooling/lsp/tests/client_compat` が成功する。
- [ ] `docs/plans/bootstrap-roadmap/2-4-status.md` に進捗や既知リスクが反映されている。
- [ ] `tooling/review/audit-diff.py` で生成した `diff.md` / `diff.html` の差分サマリが付属し、`diagnostic.regressions`・`metadata.changed` の値がレビュー対象に共有されている。
- [ ] `tooling/review/audit_dashboard.py --render` で最新の指標グラフを生成し、`reports/audit/dashboard/index.{html,md}` を確認した。
- [ ] `tooling/review/audit-query` のプリセットクエリ（例: `tooling/review/presets/stage-regressions.dsl`）を実行し、監査ログの重点領域（Stage/FFI/型クラス）に未確認のレグレッションがない。

---

この文書は Phase 2-4 の診断・監査パイプライン作業のレビュー補助ツールとして運用する。追加の手順や改善案があれば追記すること。
