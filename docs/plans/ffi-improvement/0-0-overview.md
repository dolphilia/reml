# FFI 強化計画の概要

## 背景と目的
- `docs/notes/ffi/ffi-improvement-survey.md` で、Reml の FFI が低レベル `extern "C"` に偏り、
  大規模ライブラリ対応や安全性確保に課題があることを整理した。
- Reml の設計指針（`docs/spec/0-1-project-purpose.md`）に沿って、
  **実用性・安全性・DSL ファースト**を満たす FFI 体験を構築する。

## ゴール
- C/C++ など既存資産を **低コストで取り込める**自動生成経路を用意する。
- FFI を **Reml 言語内 DSL**で安全に記述できる仕組みを整備する。
- ビルド・リンク・監査を **一貫したフロー**で運用できる状態にする。

## 非ゴール（現段階）
- C コンパイラやパーサを Reml に内蔵する（Zig 型統合）の実装。
- 既存 `extern` の破壊的な置換。
- すべての ABI を短期で網羅すること。

## フェーズ構成（確定）
1. **Phase 1: 自動生成ツールの仕様化**
   - `reml-bindgen` の入力・出力・型変換・診断キーの設計を確定する。
   - 完了条件: `docs/spec/3-9-core-async-ffi-unsafe.md` に型変換表と生成ルールが反映済み。
2. **Phase 2: FFI DSL の設計**
   - `Core.Ffi.Dsl` の API と安全境界（`effect {ffi, unsafe}`）を確定する。
   - 完了条件: DSL API 仕様と安全境界の記述が仕様に反映済み。
3. **Phase 3: ビルド統合の設計**
   - `reml build` / `reml.json` でのヘッダ解析・リンク定義・監査ログの流れを確定する。
   - 完了条件: `reml.json` の FFI セクション仕様と監査キー案が文書化済み。
4. **Phase 4: 相互運用の拡張**
   - WASM Component Model / WIT 連携の調査と設計指針の整理までを対象とする（実装は別計画）。
   - 完了条件: WIT 連携の設計メモと対応表の一次案が `docs/notes/` に整理済み。

## 影響範囲
- 仕様: `docs/spec/3-9-core-async-ffi-unsafe.md`, `docs/spec/3-8-core-runtime-capability.md`
- ガイド: `docs/guides/` 配下（FFI ガイド新設を検討）
- サンプル: `examples/ffi` の拡充

## 進行上の前提
- 既存の `4-1-spec-core-regression-plan.md` を進める前に、
  FFI 仕様の改善方針を確定し、関連する仕様更新をまとめて行う。
- 仕様変更に伴う監査・診断のキーは `docs/spec/3-6-core-diagnostics-audit.md` と整合を取る。

## 成果物
- Phase 1〜4 の計画書と依存関係の明文化
- 仕様更新項目の一覧化（差分チェック表）
- FFI サンプル更新案（`examples/ffi`）
