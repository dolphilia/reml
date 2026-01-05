# Reml クロスツールチェーン管理ガイド

このディレクトリは Phase 1-5 の「macOS → Linux x86_64 クロスコンパイル環境整備」で利用するツールチェーン資産を管理します。`docs/plans/bootstrap-roadmap/1-5-runtime-integration.md` §10 を参照しながら、ここに記載された手順で環境構築と監査を行ってください。

## ディレクトリ構成（初期状態）
- `cache/` — sysroot など大容量アーカイブの配置先（Git 管理対象外）。既定アーカイブは `debian-bookworm-x86_64.tar.zst`。
- `x86_64-unknown-linux-gnu/` — `scripts/toolchain/prepare-linux-x86_64.sh` 実行後に生成されるツールチェーン本体（`bin/`, `sysroot/`, `env.sh` など）。
- `versions.toml` — Homebrew パッケージとアーカイブのバージョン・ハッシュを記録するメタデータ。

## 準備スクリプトの使い方
1. Homebrew 依存を確認  
   ```bash
   scripts/toolchain/prepare-linux-x86_64.sh --dry-run
   ```
2. 本構築を実行（Homebrew 依存を使う場合）  
   ```bash
   scripts/toolchain/prepare-linux-x86_64.sh --cache
   ```
   - `cache/` にアーカイブが存在しない場合はエラーになります。事前に社内共有のストレージから取得するか、`--archive <PATH>` で明示指定してください。
   - Homebrew を使用せず既存の LLVM/LLD を使う場合は `--no-brew` を指定し、以下の環境変数でパスを指示します。
     ```bash
     LLVM_PREFIX_OVERRIDE=/opt/homebrew/opt/llvm@18 \
     LLD_PREFIX_OVERRIDE=/opt/homebrew/opt/lld \
     BINUTILS_PREFIX_OVERRIDE=/opt/homebrew/opt/binutils \
     scripts/toolchain/prepare-linux-x86_64.sh --no-brew --cache
     ```
     `binutils` が無い場合は `BINUTILS_PREFIX_OVERRIDE` を省略できます（ラッパ生成がスキップされます）。
3. 生成物の確認  
   - `x86_64-unknown-linux-gnu/env.sh` が生成され、`PATH` などの環境変数が定義されていること。
   - `x86_64-unknown-linux-gnu/.stamp-prepared` が更新され、構築時刻と使用ソース（cache / archive）が記録されていること。

## サンプルバイナリの生成
- Reml コンパイラとクロスツールチェーンが準備できたら、`scripts/toolchain/build-linux-sample.sh` を実行すると `tooling/toolchains/examples/hello-linux`（ELF）と同名の `.ll` を自動生成できます。
- ランタイムを同時にクロスビルドしたい場合は `--build-runtime` を付与してください（sysroot に glibc のヘッダが必要です）。
- 出力された ELF は macOS 上では実行できないため、`scripts/cross/run-linux-qemu.sh` などで QEMU 経由のスモークテストを行ってください。

## バージョンとハッシュの管理
- `versions.toml` に記載された値は、Homebrew 依存のリビジョンや sysroot アーカイブのハッシュを追跡する基準です。更新を行った場合は、差分を `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` に登録し、四半期レビューで確認します。
- sysroot アーカイブのハッシュは `tooling/toolchains/checksums.txt` と同期させ、`shasum -a 256 --check` で検証できる状態を維持してください。
- QEMU 実行によるメトリクス更新は `scripts/cross/run-linux-qemu.sh --metrics tooling/toolchains/metrics.jsonl` を使用し、`docs/guides/tooling/audit-metrics.md` へ転記します。

## トラブルシューティング
- **ツールが見つからない場合**: `env.sh` を `source` しているか確認し、Homebrew の prefix が `versions.toml` の記載と一致しているかを照合します。
- **sysroot の整合性エラー**: `versions.toml` に記載された `sha256` が最新か確認し、差異があればキャッシュを更新して再実行します。
- **QEMU 実行に失敗する場合**: `scripts/cross/run-linux-qemu.sh --dump-env` で設定値を確認し、ライブラリパスが `sysroot/lib`, `sysroot/lib64` を指しているか検証してください。

## 関連資料
- `docs/plans/bootstrap-roadmap/1-5-runtime-integration.md`
- `docs/notes/backend/llvm-spec-status-survey.md`
- `scripts/toolchain/prepare-linux-x86_64.sh`
- `scripts/cross/run-linux-qemu.sh`
