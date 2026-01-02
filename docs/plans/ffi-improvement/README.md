# Reml FFI 強化計画

Reml の FFI を実用レベルに引き上げるための計画書群です。
`docs/notes/ffi/ffi-improvement-survey.md` の調査結果を起点に、短期・中期・長期の具体的タスクを段階化します。

## 目次
- [0-0-overview.md](0-0-overview.md) — 背景・目的・段階整理
- [0-1-workstream-tracking.md](0-1-workstream-tracking.md) — ワークストリーム管理（暫定）
- [1-0-bindgen-plan.md](1-0-bindgen-plan.md) — `reml-bindgen` 設計・仕様化
- [1-1-ffi-dsl-plan.md](1-1-ffi-dsl-plan.md) — `Core.Ffi.Dsl` 設計・仕様化
- [1-2-build-integration-plan.md](1-2-build-integration-plan.md) — `reml build` 連携の設計・仕様化
- [1-3-wasm-component-model-plan.md](1-3-wasm-component-model-plan.md) — WASM Component Model 調査・方針

## 関連ドキュメント
- 調査メモ: `docs/notes/ffi/ffi-improvement-survey.md`
- 既存仕様: `docs/spec/3-9-core-async-ffi-unsafe.md`
- 監査・Capability: `docs/spec/3-6-core-diagnostics-audit.md`, `docs/spec/3-8-core-runtime-capability.md`

## WS1 完了サマリ（reml-bindgen）
- 仕様: `docs/spec/3-9-core-async-ffi-unsafe.md` に型変換表・未対応型の診断キー案・reml-bindgen 節を反映済み。
- ガイド: `docs/guides/ffi/reml-bindgen-guide.md` に診断メタデータ例・ログ形式・レビュー手順詳細を追記済み。
- サンプル: `examples/ffi/bindgen/minimal` に単一ヘッダの最小サンプルを追加済み。
