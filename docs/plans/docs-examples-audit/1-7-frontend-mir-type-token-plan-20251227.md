# 1.7 Frontend MIR JSON 型トークン導入計画（2025-12-27）

Frontend の MIR JSON 出力に `MirTypeToken`（文字列/構造化の両対応）を導入し、sum 型などの型情報を Backend へ受け渡せるようにするための計画。

## 目的
- 既存の文字列型トークン出力を維持しつつ、構造化型トークンを段階導入する。
- sum 型（ADT）や参照型の情報を JSON で安定して表現できるようにする。
- Backend の `MirTypeJson` 取り込みに必要な JSON 形状を Frontend で確定する。

## 対象範囲
- Frontend: `compiler/frontend`
- 仕様/計画: `docs/plans/docs-examples-audit/1-7-backend-runtime-sum-mir-json-draft-20251227.md`

## 前提・現状
- `compiler/frontend/src/semantics/mir.rs` の MIR JSON 出力は型を文字列トークンで出力する。
- sum 型のタグ幅/variant 情報は MIR JSON に出力されていない。

## 実行計画

### フェーズ 0: 現状の出力経路把握
- MIR JSON 出力の型トークン生成箇所を特定する。
  - [ ] `compiler/frontend/src/semantics/mir.rs` の JSON 出力コードを確認する
  - [ ] `MirExpr.ty` / `MirFunction.params` / `return` の型トークン生成経路を整理する

### フェーズ 1: MirTypeToken 仕様確定
- 文字列/構造化の併用ルールを確定する。
- sum 型の `tag_bits` / `variants` / `payload_layout` の出力方針を決める。
  - [ ] `MirTypeToken` の JSON 形状（文字列 or `{kind: ...}`）を確定する
  - [ ] sum 型の `tag_bits` 算出タイミング（typeck/MIR 生成）を決める
  - [ ] `payload_layout` の判定ルール（inline/boxed）を決める
  - [ ] `type_name` / `module_path` の出力要否を整理する

### フェーズ 2: 出力実装
- MIR JSON 出力に `MirTypeToken` を実装する。
  - [ ] `MirTypeToken` 相当の構造体を追加し、`serde` 出力を整備する
  - [ ] 既存の型トークン文字列出力を保持しつつ、sum 型のみ構造化出力へ切り替える
  - [ ] 既存の JSON スナップショットとの互換性影響を確認する

### フェーズ 3: 検証
- 代表的な sum 型の JSON 出力を確認する。
  - [ ] `Option` / `Result` など代表ケースの JSON 例を `tmp/` に出力して検証する
  - [ ] `docs-examples-audit` のサンプルで型トークン出力が破壊されないことを確認する

## 受け入れ基準
- sum 型が `MirTypeToken` の構造化 JSON として出力できる。
- 既存の文字列トークン出力が維持され、Backend 側の旧取り込みに影響がない。

## 進捗管理
- 本計画書作成日: 2025-12-27
- 進捗欄（運用用）:
  - [ ] フェーズ 0 完了
  - [ ] フェーズ 1 完了
  - [ ] フェーズ 2 完了
  - [ ] フェーズ 3 完了

## 関連リンク
- `docs/plans/docs-examples-audit/1-7-backend-runtime-sum-mir-json-draft-20251227.md`
- `docs/plans/docs-examples-audit/1-7-backend-runtime-type-decl-layout-plan-20251227.md`
