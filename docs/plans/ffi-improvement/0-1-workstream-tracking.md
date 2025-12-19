# FFI 強化ワークストリーム管理

## 目的
FFI 強化の複数タスクを分割し、仕様更新・ガイド・サンプル更新を一貫して追跡する。

## ワークストリーム定義
- **FFI-WS1: reml-bindgen**
  - 対象: `reml-bindgen` 仕様、型変換ルール、生成コードの監査基準
- **FFI-WS2: Core.Ffi.Dsl**
  - 対象: DSL API、効果・安全境界、診断キー
- **FFI-WS3: Build Integration**
  - 対象: `reml build` / `reml.json` 連携、リンク解決、監査ログ
- **FFI-WS4: WASM Component Model**
  - 対象: WIT 連携、Canonical ABI、境界安全性

## 追跡項目（各 WS 共通）
- 仕様更新: `docs/spec/` 内の更新対象と差分
- ガイド更新: `docs/guides/` の追加・更新案
- サンプル更新: `examples/ffi` の追加/差し替え
- 診断・監査: `docs/spec/3-6-core-diagnostics-audit.md` との整合

## ステータス運用（暫定）
- `draft` / `in_review` / `confirmed` / `blocked`
- 各計画書にステータスを記載し、`README.md` で一覧化する。

## Phase ステータス（確定）
- Phase 1: `confirmed`
- Phase 2: `confirmed`
- Phase 3: `confirmed`
- Phase 4: `confirmed`

## WS1 進捗（reml-bindgen）
- ステータス: `confirmed`
- 仕様更新: `docs/spec/3-9-core-async-ffi-unsafe.md` に型変換表、未対応型の診断キー案、reml-bindgen 節を反映済み。
- ガイド更新: `docs/guides/reml-bindgen-guide.md` に診断メタデータ例、ログ形式、レビュー手順詳細を追記済み。
- サンプル更新: `examples/ffi/bindgen/minimal` に単一ヘッダの最小サンプルを追加済み。

## 初期 TODO
- Phase1/Phase2 の計画書に「仕様差分チェック表」を追加する。
- `docs/plans/README.md` に本計画の導線を追加する。
