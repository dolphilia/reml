# 0.2 再ビルドと検証計画

## 目的
- ツールチェーン更新後に `compiler/rust/` 配下の主要バイナリがビルドできることを確認する。

## ビルド対象（最低限）
- `compiler/rust/frontend`（`reml_frontend`）
- `compiler/rust/runtime`
- `compiler/rust/tooling`（存在する場合）

## 推奨コマンド例
- `cargo build --manifest-path compiler/rust/frontend/Cargo.toml --bin reml_frontend`
- `cargo build --manifest-path compiler/rust/runtime/Cargo.toml`
- `cargo build --manifest-path compiler/rust/tooling/Cargo.toml`

## 検証ログの記録
- 実行コマンドと結果を `reports/spec-audit/summary.md` に追記する。
- 重大な依存更新があれば `reports/spec-audit/ch0/` などにメモを作成し、更新理由と差分を記録する。

## 判定基準
- 上記ビルドが全て成功すること。
- `Cargo.lock` の差分が計画書に記録され、更新理由が説明されていること。
