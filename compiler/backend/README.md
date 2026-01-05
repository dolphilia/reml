# backend

Reml のバックエンド実装を置く領域です。現在は `llvm/` に Rust LLVM バックエンドのスケルトンがあり、TargetMachine/TypeMapping/FFI ロワリング/検証の土台を整備しています。

## ディレクトリ
- `llvm/`: `reml-llvm-backend` クレート。コード生成や検証、ランタイムリンクの基盤を実装。

## ビルド/テスト
```
cargo build --manifest-path compiler/backend/llvm/Cargo.toml
cargo test --manifest-path compiler/backend/llvm/Cargo.toml
```

必要に応じて以下の環境変数を利用します。
- `REML_LLVM_TARGET`（例: `x86_64-apple-darwin` / `aarch64-apple-darwin`）
- `REML_BACKEND_VERIFY=1`（`opt -verify` 連携ログを有効化）

## macOS の LLVM セットアップ（概要）
macOS では LLVM ツールチェーンのバージョン整合が重要です。詳細な手順や記録方針は次を参照してください。

- `docs/plans/bootstrap-roadmap/1-8-macos-prebuild-support.md`
- `docs/plans/rust-migration/2-0-llvm-backend-plan.md`
- `docs/notes/backend/llvm-spec-status-survey.md`
