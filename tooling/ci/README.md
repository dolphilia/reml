# tooling/ci 概要（下書き）

Phase 1 `docs/plans/bootstrap-roadmap/1-7-linux-validation-infra.md` および `1-5-runtime-integration.md` を起点に、CI ワークフローとローカル再現スクリプトを配置する領域です。

## Docker ベースの Linux 検証環境

- `tooling/ci/docker/bootstrap-runtime.Dockerfile` — x86_64 Linux (Ubuntu 22.04) 向けビルド環境
- `scripts/docker/build-runtime-container.sh` — Docker/Podman でのイメージビルド (`--push`, `--build-arg` 対応)
- `scripts/docker/run-runtime-tests.sh` — コンテナ内で `dune build`, `dune runtest`, `make -C runtime/native runtime`, LLVM IR 検証を実行
- `scripts/docker/smoke-linux.sh` — `examples/language-impl-comparison/reml/basic_interpreter.reml` を対象にしたスモークテスト
- `tooling/ci/docker/metrics.json` — イメージサイズ・テスト時間などの最新計測値（更新必須）

### 推奨ワークフロー

```bash
# イメージビルド（必要に応じて ghcr.io へ push）
scripts/docker/build-runtime-container.sh --tag ghcr.io/reml/bootstrap-runtime:dev

# フルテスト（Phase 1 標準パイプライン）
scripts/docker/run-runtime-tests.sh --tag ghcr.io/reml/bootstrap-runtime:dev

# スモークテスト（5 分以内の動作確認）
scripts/docker/smoke-linux.sh --tag ghcr.io/reml/bootstrap-runtime:dev
```

- `docker scout cves ghcr.io/reml/bootstrap-runtime:dev` などで脆弱性を月次確認し、重大度 High 以上は `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` に登録する。
- メトリクス (`tooling/ci/docker/metrics.json`) を更新し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に転記する。

## TODO
- [ ] GitHub Actions ワークフローに対応する補助スクリプトを追加
- [ ] ローカル検証用の `scripts/ci-local.sh` などユーティリティを整備
- [ ] Phase 1 `docs/plans/bootstrap-roadmap/1-7-linux-validation-infra.md` のチェックリストを反映したテンプレートを用意
