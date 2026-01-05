# compiler ディレクトリ構成

Reml の Rust 実装を集約する作業領域です。各サブディレクトリの概要は、それぞれの README を入口として参照してください。

## サブディレクトリ
- `adapter/`: プラットフォーム差異吸収レイヤ（Capability/監査の基盤）
- `backend/`: バックエンド実装（現在は LLVM スケルトン）
- `ffi_bindgen/`: FFI バインディング生成ツール（reml-bindgen）
- `frontend/`: パーサ/型推論/フロントエンド CLI
- `runtime/`: ランタイム実装（Rust + native/ffi）
- `xtask/`: 開発支援用の xtask

## 主要な README への導線
- `compiler/adapter/README.md`
- `compiler/backend/README.md`
- `compiler/ffi_bindgen/README.md`
- `compiler/frontend/README.md`
- `compiler/runtime/README.md`
- `compiler/xtask/README.md`

## ビルドとテスト（基本）
ルートには `Cargo.toml.ws` のみがあるため、原則 `--manifest-path` を使って実行します。

```
cargo build --manifest-path compiler/frontend/Cargo.toml
cargo test --manifest-path compiler/frontend/Cargo.toml

cargo build --manifest-path compiler/backend/llvm/Cargo.toml
cargo test --manifest-path compiler/backend/llvm/Cargo.toml

cargo build --manifest-path compiler/runtime/Cargo.toml
cargo test --manifest-path compiler/runtime/Cargo.toml

cargo test --manifest-path compiler/adapter/Cargo.toml
```

macOS でリンクが不安定な場合は `lld` を利用します。

```
RUSTFLAGS="-C link-arg=-fuse-ld=lld" cargo test --manifest-path compiler/frontend/Cargo.toml
```

## 注意点
- adapter のネットワーク試験は `127.0.0.1:0` への TCP bind が必要です。権限が付与できない場合は `REML_ADAPTER_SKIP_NETWORK_TESTS=1` を利用し、強制実行は `REML_ADAPTER_FORCE_NETWORK_TESTS=1` を併用します。
- LLVM ツールチェーン要件や macOS での詳細手順は `compiler/backend/README.md` を参照してください。
- xtask の使い方は `compiler/xtask/README.md` にまとめています。
