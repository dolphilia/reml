# Rust 版コンパイラ作業ディレクトリ

このディレクトリは、OCaml 実装から Rust 実装へ移植するための成果物を集約する作業領域である。

## 役割
- Rust 製フロントエンド（パーサー、型推論、LLVM IR 生成）のソースコード配置
- Rust 向けビルドスクリプト、CI 設定、サンプルプロジェクトの管理
- OCaml 実装との比較検証に用いる補助ツールの設置

## 今後のタスク
- `docs/plans/rust-migration/` で定義する計画書に従い、初期スケルトン（Cargo プロジェクト）を作成する
- Windows / Linux / macOS 向けのビルドフローを段階的に整備し、CI から `cargo build` を実行できるようにする
- OCaml 実装と Rust 実装の差分メトリクスを収集する自動化スクリプトを追加する
- `backend/llvm/` に TargetMachineBuilder/TypeMappingContext/FFI ロワリングのスケルトンを置き、W2 の数値設計との整合を追跡する

## macOS での Rust 版ビルド

`docs/plans/bootstrap-roadmap/1-8-macos-prebuild-support.md` で定義した macOS プレビルド対応と `docs/plans/rust-migration/2-0-llvm-backend-plan.md` の LLVM バックエンド要件を踏まえ、本リポジトリで Rust 版 Reml を macOS ローカルに構築するための手順を記録しておきます。

### 前提
- Xcode Command Line Tools (`xcode-select --install`) を導入済みであること。
- Homebrew で `llvm@19`, `lld`, `cmake`, `pkg-config`, `bindgen`（将来の FFI 拡張）をインストール。`docs/notes/llvm-spec-status-survey.md` に記載された LLVM のバージョンと整合するようバイナリを固定します。
- `rustup` が導入済みで `stable` をデフォルトに設定していること（`rustup default stable` / `rustup show` で確認）。
- `docs/plans/rust-migration/2-1-runtime-integration.md` や `2-2-adapter-layer-guidelines.md` で想定されている Capabilities/FFI API を理解し、フェーズ P2 に向けた検証基盤の一部として位置づける。

### 環境を整える
```
brew install llvm@19 lld cmake pkg-config

# パスとフラグを明示的に設定（`llvm-sys` や `inkwell` を使う予定が生まれたときにもこの設定に従う）
export PATH="/opt/homebrew/opt/llvm@19/bin:${PATH}"
export LDFLAGS="-L/opt/homebrew/opt/llvm@19/lib"
export CPPFLAGS="-I/opt/homebrew/opt/llvm@19/include"
export PKG_CONFIG_PATH="/opt/homebrew/opt/llvm@19/lib/pkgconfig:${PKG_CONFIG_PATH}"
export LLVM_CONFIG="/opt/homebrew/opt/llvm@19/bin/llvm-config"

# 確認
rustup show
llvm-config --version
opt --version
```

`llvm-config --version` や `opt --version` の結果は `reports/backend-verify/macOS/toolchain.json` などのバージョン記録に転記し、`docs/notes/llvm-spec-status-survey.md` と `docs-migrations.log` へ追記しておくとフェーズ間の整合性が保てます。

### ビルド手順
1. フロントエンド
   ```
   cargo build --manifest-path compiler/rust/frontend/Cargo.toml
   cargo test --manifest-path compiler/rust/frontend/Cargo.toml
   ```
2. LLVM バックエンド
   ```
   REML_LLVM_TARGET=x86_64-apple-darwin \
   REML_BACKEND_VERIFY=1 \
   cargo build --manifest-path compiler/rust/backend/llvm/Cargo.toml
   ```
   `REML_LLVM_TARGET` は `arm64-apple-darwin` も併記して検証する（Apple Silicon 上で `rustup target add aarch64-apple-darwin` しておく）。`REML_BACKEND_VERIFY=1` によって `opt -verify` との統合ログが `reports/backend-verify/macOS/opt-verify.log` に残るようにしておくと、`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` が要求するドリフト監査につながる。

### 検証と連携
- `scripts/poc_dualwrite_compare.sh` を `--frontend rust --backend rust` で実行すると `reports/dual-write/front-end/` 以下に JSON が生成される。差分を `docs/plans/rust-migration/1-3-dual-write-runbook.md` に記録し、ステージごとの `audit` メタデータと `docs/spec/3-6-core-diagnostics-audit.md` の期待値を照合する。
- `REML_LLVM_PATH` や将来的な `REML_LLVM_DISTRIBUTION` を使う際は `docs/plans/bootstrap-roadmap/windows-llvm-build-investigation.md` を参考にし、macOS でも CLI のパス・`DataLayout` の記録を `reports/backend-ir-diff/macOS/` に残す。
- `docs/plans/rust-migration/2-1-runtime-integration.md` に記載した FFI ケースや `reports/runtime-bridge/` に関連する検証を Rust 版で再現する観点を持ち、結果を `reports/runtime-bridge/macOS` に追記する（FFI ハーネスは今後整備）。

このような手順を README に残し、macOS 開発者が `cargo build` で Rust 版をローカル再現できる状態を早めることで P2 以降の Windows/Linux 連携を支える土台とします。
