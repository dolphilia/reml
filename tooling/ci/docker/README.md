# tooling/ci/docker

Phase 1 ランタイム連携で利用する x86_64 Linux コンテナ資産を管理するディレクトリです。

## ファイル構成

- `bootstrap-runtime.Dockerfile` — Ubuntu 22.04 ベースのビルド環境
- `metrics.json` — イメージサイズやビルド時間の最新計測値
- `README.md` — この文書

## ビルド手順

```bash
scripts/docker/build-runtime-container.sh --tag ghcr.io/reml/bootstrap-runtime:dev
```

`USE_BUILDX=1` を指定すると `docker buildx` を利用します。Podman を使う場合は
`CONTAINER_TOOL=podman` を環境変数で指定してください。

## 利用方法

```bash
scripts/docker/run-runtime-tests.sh --tag ghcr.io/reml/bootstrap-runtime:dev
```

- リポジトリルートが `/workspace` としてマウントされます。
- デフォルトコマンドは `dune build`, `dune runtest`, `make -C runtime/native runtime` を実行し、
  LLVM ゴールデンファイルの検証を行います。
- 任意のコマンドを実行する場合は `--` 以降に Bash コマンドを指定します。
- クロスコンパイル済みバイナリのスモークテストは `scripts/docker/run-cross-binary.sh --tag ghcr.io/reml/bootstrap-runtime:dev -- artifacts/cross/hello-linux`
  のように実行できます（`artifacts/cross/` 配下の成果物を想定）。

スモークテスト用のショートカット:

```bash
scripts/docker/smoke-linux.sh --tag ghcr.io/reml/bootstrap-runtime:dev
```

## 現在の検証ステータス（2025-10-10）

- `dune runtest`: GREEN — Let 多相 A2 の型一般化バグを修正済み。
- `llvm_golden`: GREEN — `print_i64` 宣言追加に合わせてゴールデン更新済み。
- `smoke-linux`: GREEN — `basic_interpreter.reml` はスタブ実装（完全版はコメント化、パーサ拡張後に復元予定）。

## CVE スキャン

`docker scout cves ghcr.io/reml/bootstrap-runtime:dev` あるいは `trivy image ...` を利用して
月次で脆弱性を確認し、重大度 High 以上は `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` に登録します。
