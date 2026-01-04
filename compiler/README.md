# compiler ディレクトリ構成

Reml の Rust 実装を集約する作業領域です。現在は Rust 版コンパイラのみを管理します。

## サブディレクトリ
- `adapter/`: プラットフォーム差異吸収レイヤ
- `backend/`: バックエンド実装（LLVM など）
- `ffi_bindgen/`: FFI バインディング生成ツール
- `frontend/`: パーサ/型推論/フロントエンド CLI
- `runtime/`: ランタイム実装
- `xtask/`: 開発支援用の xtask

## macOS での Rust 版ビルド

`docs/plans/bootstrap-roadmap/1-8-macos-prebuild-support.md` で定義した macOS プレビルド対応と `docs/plans/rust-migration/2-0-llvm-backend-plan.md` の LLVM バックエンド要件を踏まえ、本リポジトリで Rust 版 Reml を macOS ローカルに構築するための手順を記録しておきます。

### 前提
- Xcode Command Line Tools (`xcode-select --install`) を導入済みであること。
- Homebrew で `llvm@19`, `lld`, `cmake`, `pkg-config`, `bindgen`（将来の FFI 拡張）をインストール。`docs/notes/backend/llvm-spec-status-survey.md` に記載された LLVM のバージョンと整合するようバイナリを固定します。
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

`llvm-config --version` や `opt --version` の結果は `reports/backend-verify/macOS/toolchain.json` などのバージョン記録に転記し、`docs/notes/backend/llvm-spec-status-survey.md` に追記しておくとフェーズ間の整合性が保てます。

### ビルド手順
1. フロントエンド
   ```
   cargo build --manifest-path compiler/frontend/Cargo.toml
   cargo test --manifest-path compiler/frontend/Cargo.toml
   ```
   macOS でリンクが不安定な場合は `lld` を利用する。
   ```
   RUSTFLAGS="-C link-arg=-fuse-ld=lld" cargo test --manifest-path compiler/frontend/Cargo.toml
   ```
2. LLVM バックエンド
   ```
   REML_LLVM_TARGET=x86_64-apple-darwin \
   REML_BACKEND_VERIFY=1 \
   cargo build --manifest-path compiler/backend/llvm/Cargo.toml
   ```
   `REML_LLVM_TARGET` は `arm64-apple-darwin` も併記して検証する（Apple Silicon 上で `rustup target add aarch64-apple-darwin` しておく）。`REML_BACKEND_VERIFY=1` によって `opt -verify` との統合ログが `reports/backend-verify/macOS/opt-verify.log` に残るようにしておくと、`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` が要求するドリフト監査につながる。

### 検証と連携
- `REML_LLVM_PATH` や将来的な `REML_LLVM_DISTRIBUTION` を使う際は `docs/plans/bootstrap-roadmap/windows-llvm-build-investigation.md` を参考にし、macOS でも CLI のパス・`DataLayout` の記録を `reports/backend-ir-diff/macOS/` に残す。
- `docs/plans/rust-migration/2-1-runtime-integration.md` に記載した FFI ケースや `reports/runtime-bridge/` に関連する検証を Rust 版で再現する観点を持ち、結果を `reports/runtime-bridge/macOS` に追記する（FFI ハーネスは今後整備）。

### ネットワークテストとポート権限
- `cargo test --manifest-path compiler/adapter/Cargo.toml` では `network::tests::tcp_connect_roundtrip` が `127.0.0.1:0` での待ち受けと接続を行うため、ループバックアドレスへの TCP bind 権限が必要です。CI/ローカルともに BSD/macOS の Application Sandbox や企業向け Endpoint Security ツールで `bind(2)` が拒否される場合があるので、Runner に `network-outbound`/`network-server` 権限を付与してください。
- どうしても権限を付与できない環境では `REML_ADAPTER_SKIP_NETWORK_TESTS=1 cargo test --manifest-path compiler/adapter/Cargo.toml` としてネットワーク試験のみをスキップできます。監査目的で強制的に実行したい場合は `REML_ADAPTER_FORCE_NETWORK_TESTS=1` を併用し、スキップ設定を上書きしてください。
- ネットワーク試験をスキップした場合は `reports/adapter/` に記録される監査ログから `adapter.net` 系メトリクスが欠落するため、週次の非サンドボックス Runner（例: self-hosted macOS/Linux）でフルテストを走らせ、`docs/plans/bootstrap-roadmap/2-8-spec-integrity-audit.md` の差分リストへ反映することを推奨します。
このような手順を README に残し、macOS 開発者が `cargo build` で Rust 版をローカル再現できる状態を早めることで P2 以降の Windows/Linux 連携を支える土台とします。

## xtask コマンド
- `.cargo/config.toml` で `cargo xtask prelude-audit` を `compiler/xtask` に紐づけている。`docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` を検証する場合は `cargo xtask prelude-audit --wbs 2.1b --strict --baseline docs/spec/3-1-core-prelude-iteration.md` を実行し、`reports/spec-audit/ch0/links.md` に結果を貼り付ける。
- `--wbs` フィルタを変更すれば Phase 3 の他タスク（例: `2.2a`）に関する項目だけを抽出できる。未実装が存在する状態で `--strict` を付けた場合は非ゼロ終了となるため、CI やローカルゲートで利用する。
