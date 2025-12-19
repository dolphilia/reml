# reml-bindgen ガイド（ドラフト）

## 目的
C/C++ ヘッダから Reml の `extern "C"` 定義を生成し、低レベル FFI の導入コストを下げる。

## 想定読者
- C ライブラリを Reml から呼び出したい開発者
- `examples/ffi` の生成物を保守する担当者

## 前提
- 生成物は `unsafe` 前提であり、安全化は `Core.Ffi.Dsl`（別ガイド）で行う。
- 生成物の差分レビューは `bindings.manifest.json` を一次情報とする。

## 基本フロー
1. `reml-bindgen.toml` を用意する。
2. `reml-bindgen` を実行し `.reml` と `bindings.manifest.json` を生成する。
3. `examples/ffi` に生成物を配置し、手書きラッパーと分離する。

## 設定ファイル（最小構成）
```toml
headers = ["path/to/header.h"]
include_paths = ["/usr/include"]
output = "generated/bindings.reml"
manifest = "generated/bindings.manifest.json"
```

## 生成物の扱い
- `.reml` は自動生成領域として扱い、手書き編集を避ける。
- `bindings.manifest.json` に型変換と修飾子の情報が記録される。

## 診断キーの使い方
- `ffi.bindgen.unknown_type`: 型変換表にない型が見つかった
- `ffi.bindgen.parse_failed`: ヘッダ解析に失敗した
- `ffi.bindgen.unresolved_symbol`: シンボル解決に失敗した

## レビュー手順（要点）
1. `bindings.manifest.json` の差分を確認する。
2. 生成された `.reml` の `extern "C"` 宣言のみを確認する。
3. 手書きラッパーの API が維持されているかを確認する。
