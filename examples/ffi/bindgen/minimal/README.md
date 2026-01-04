# reml-bindgen 最小サンプル

単一ヘッダから生成した `.reml` と `bindings.manifest.json` を用意し、
生成物と手書きラッパーを分離する構成を確認するための例です。

## 構成
- `headers/counter.h`: 入力ヘッダ
- `reml-bindgen.toml`: 最小構成の設定
- `generated/`: 生成物（`counter_bindings.reml`, `bindings.manifest.json`）
- `wrapper/`: 手書きラッパー（利用側 API の例）

## 生成の想定
`reml-bindgen.toml` を使って `generated/` を再生成し、差分レビューは
`bindings.manifest.json` を一次情報として扱います。

## 実行例
```bash
reml-bindgen --config reml-bindgen.toml
```
