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
- 期待値の変更がある場合は、CLI テキスト出力も取得し、利用者視点で破壊的でないかを確認する。

## 3. CI 連携時の確認ポイント
- GitHub Actions `diagnostic-json` ジョブが失敗した場合、`tooling/lsp/tests/client_compat` のテストログと `scripts/validate-diagnostic-json.sh` の結果を参照する。
- スキーマ違反が発生した場合は `tooling/json-schema/diagnostic-v2.schema.json` を更新し、併せてフィクスチャを追加する。
- Windows/macOS 固有のフィクスチャ（`diagnostic-v2-ffi-macos-sample.json` など）が最新の監査ログと整合しているか確認する。

## 4. レビュー用チェックリスト
- [ ] JSON スキーマ (`tooling/json-schema/diagnostic-v2.schema.json`) とサンプル出力が一致している。
- [ ] `scripts/validate-diagnostic-json.sh` の既定対象（`compiler/ocaml/tests/golden/diagnostics`, `compiler/ocaml/tests/golden/audit`）でエラーがない。
- [ ] `npm run ci --prefix tooling/lsp/tests/client_compat` が成功する。
- [ ] `docs/plans/bootstrap-roadmap/2-4-status.md` に進捗や既知リスクが反映されている。

---

この文書は Phase 2-4 の診断・監査パイプライン作業のレビュー補助ツールとして運用する。追加の手順や改善案があれば追記すること。
