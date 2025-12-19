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

## 監査と失敗時の扱い
- `ffi.build.*` と `ffi.bindgen.*` を分離して監査する。
- 失敗時は `input_hash` をログに残し、再生成条件を明確化する。
