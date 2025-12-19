# FFI ビルド統合ガイド（ドラフト）

## 目的
`reml build` と `reml.json` を使って FFI 依存と生成フローを一元管理する。

## 想定読者
- FFI を含むプロジェクトのビルド担当者
- CI で再現可能な生成を行いたいチーム

## マニフェスト例
```reml
ffi {
  libraries = ["m"]
  headers = ["/usr/include/math.h"]
  bindgen = { enabled = true, output = "generated/math.reml", config = "reml-bindgen.toml" }
  linker = { search_paths = ["/usr/lib"], frameworks = [], extra_args = [] }
}
```

## 実行フロー（要点）
1. ヘッダ解析
2. `reml-bindgen` 実行
3. 生成物キャッシュ（入力ハッシュ保存）
4. コンパイル/リンク

## 再生成条件
次のいずれかが変化した場合は `input_hash` が更新され、再生成が必要になる。

- `headers` の内容（ヘッダ自体の更新、インクルードパスの差分）
- `reml-bindgen.toml` の内容
- `TargetProfile`（クロスコンパイル時の `target`）
- `reml-bindgen` のバージョン

## キャッシュ破棄の手順
- キャッシュは `cache_dir("reml")/ffi/{input_hash}` に格納される。運用上の目安は `.reml/cache/ffi`。
- キャッシュを破棄する場合は `ffi` 配下を削除し、次回 `reml build` で再生成する。
- CI では `ffi.bindgen` 監査ログの `status = cache_hit` を確認し、意図しないキャッシュ再利用を検知する。

## 監査と失敗時の扱い
- `ffi.build.*` と `ffi.bindgen.*` を分離して監査する。
- 失敗時は `input_hash` をログに残し、再生成条件の確認に利用する。
- `ffi.build.config_invalid` / `ffi.build.link_failed` が出た場合は `reml.json` の `ffi` セクションを優先的に確認する。
- `ffi.bindgen.output_overwrite` は、`cache_hit` 復元時に `output_path` / `manifest_path` が既に存在し、上書きが中止された場合に発火する。

## `tool_version` の解釈例
`reml-bindgen --version` の出力は実装や配布形態で差があるため、監査ログの `tool_version` は次のルールで正規化する。

- 文字列内の「数字を含むトークン」を優先して採用する。
- 該当トークンが見つからない場合は `unknown` を記録する。

例:
- `reml-bindgen 0.3.1` → `0.3.1`
- `reml-bindgen version v0.3.1` → `0.3.1`
- `dev build` → `unknown`
