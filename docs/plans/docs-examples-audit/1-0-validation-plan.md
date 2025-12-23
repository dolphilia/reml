# 1.0 検証計画

## 基本方針
- `.reml` の検証は `reml_frontend` を正規ルートとする。
- 検証ログは `reports/spec-audit/` に保存し、`docs/spec/` の監査ノートと対応させる。

## 実行フロー（Chapter 1 例）
```bash
cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin reml_frontend -- \
  --emit-diagnostics examples/docs-examples/spec/1-1-syntax/use_nested.reml \
  --emit-typeck-debug reports/spec-audit/ch1/use_nested-YYYYMMDD-typeck.json
```

## ログ命名
- `reports/spec-audit/<chapter>/<sample>-YYYYMMDD-diagnostics.json`
- `reports/spec-audit/<chapter>/<sample>-YYYYMMDD-typeck.json`
- Streaming 実行時は `streaming_<sample>-YYYYMMDD-diagnostics.json`

## カバレッジの扱い
- `docs/spec/` は P0 として全件検証。
- `docs/guides/` は P1 として主要ガイドから着手。
- `docs/notes` / `docs/plans` は P2 として計画書・ノートの代表例を優先。

## リスクと対応
- 実装未対応の構文は `*_rustcap.reml` のようなフォールバック運用を明示する。
- 差分が出た場合は、元ドキュメントの節・脚注に根拠ログを追記する。

## TODO
- `docs/guides/ai-integration.md` のコード例と監査ログの接続方法を定義する。
