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

## CI 自動化ツール（Phase 1-7 完了）

### ローカル再現スクリプト

`scripts/ci-local.sh` により、GitHub Actions と同じ手順をローカルで実行できます：

```bash
# リポジトリルートから実行
./scripts/ci-local.sh

# 特定のステップをスキップ
./scripts/ci-local.sh --skip-lint --skip-runtime

# ヘルプを表示
./scripts/ci-local.sh --help
```

### メトリクス記録スクリプト

`tooling/ci/record-metrics.sh` により、CI 実行結果を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に記録します：

```bash
./tooling/ci/record-metrics.sh \
  --build-time "5m 32s" \
  --test-count "143" \
  --test-result success \
  --llvm-verify success
```

## 完了タスク（Phase 1-7）

- ✅ GitHub Actions ワークフローに対応する補助スクリプトを追加
- ✅ ローカル検証用の `scripts/ci-local.sh` などユーティリティを整備
- ✅ Phase 1 `docs/plans/bootstrap-roadmap/1-7-linux-validation-infra.md` のチェックリストを反映

## 残りタスク（Phase 2 以降）

- [ ] カバレッジレポート生成の自動化
- [ ] メトリクス可視化ツールの統合
- [ ] Windows 環境への対応（Phase 2-6）
- [ ] macOS 環境への対応（Phase 1-8 または Phase 2-7）
