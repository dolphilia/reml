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
- `bindings.manifest.json` に型変換・修飾子・入力ハッシュの情報が記録される。

## 診断キーの使い方
- `ffi.bindgen.unknown_type`: 型変換表にない型が見つかった
- `ffi.bindgen.parse_failed`: ヘッダ解析に失敗した
- `ffi.bindgen.unresolved_symbol`: シンボル解決に失敗した

## ログ形式（要点）
- 生成ログは JSON Lines（1行1イベント）を基本とする。
- 主要イベント: `bindgen.start`, `bindgen.parse`, `bindgen.generate`, `bindgen.finish`
- 診断は `diagnostics` 配列として出力し、`code` / `symbol` / `c_type` / `reason` / `hint` を保持する。

```json
{
  "event": "bindgen.finish",
  "status": "success",
  "generated": "generated/bindings.reml",
  "manifest": "generated/bindings.manifest.json",
  "diagnostics": [
    {
      "code": "ffi.bindgen.unknown_type",
      "symbol": "my_callback",
      "c_type": "int (*)(const char*, size_t)",
      "reason": "unsupported_fn_ptr",
      "hint": "phase2"
    }
  ]
}
```

## 未対応型の診断メタデータ例
```json
{
  "code": "ffi.bindgen.unknown_type",
  "symbol": "my_callback",
  "c_type": "int (*)(const char*, size_t)",
  "reason": "unsupported_fn_ptr",
  "hint": "phase2"
}
```

## レビュー手順（詳細）
1. `bindings.manifest.json` の差分を確認し、`types` / `diagnostics` / `qualifiers` の変化を整理する。
2. `diagnostics` の `code` と `reason` が想定どおりか、未対応型が増えていないかを確認する。
3. 生成 `.reml` は `extern "C"` と `repr(C)` 定義だけを確認し、手書き領域を触っていないかを確認する。
4. 手書きラッパーの API 変更が必要な場合は、差分理由を `bindings.manifest.json` の変更と対応づける。
5. `input_hash` と `headers` / `include_paths` / `defines` の差分を確認し、再生成条件として記録する。
