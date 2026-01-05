# tooling/ci 概要（下書き）

Phase 1 `docs/plans/bootstrap-roadmap/1-7-linux-validation-infra.md` および `1-5-runtime-integration.md` を起点に、CI ワークフローとローカル再現スクリプトを配置する領域です。現在は Rust 実装に合わせて整理中です。

## CI 自動化ツール（Phase 1-7 完了）

### メトリクス記録スクリプト

`tooling/ci/record-metrics.sh` により、CI 実行結果を `docs/guides/tooling/audit-metrics.md` に記録します（現在は非推奨で、ファイルが存在しない場合はスキップします）：

```bash
./tooling/ci/record-metrics.sh \
  --build-time "5m 32s" \
  --test-count "143" \
  --test-result success \
  --llvm-verify success
```

CI 監査メトリクスの収集は Phase 計画と合わせて別途整備します。

## 完了タスク（Phase 1-7）

- ✅ GitHub Actions ワークフローに対応する補助スクリプトを追加
- ✅ Phase 1 `docs/plans/bootstrap-roadmap/1-7-linux-validation-infra.md` のチェックリストを反映
