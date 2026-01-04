# 1.7 Backend MIR 型 JSON 取り込み方針（2025-12-27）

Backend の `MirTypeJson` 取り込みと診断ログを具体化し、sum 型（ADT）を `RemlType` に落とすまでの方針を明文化する。

## 目的
- 構造化型トークン（`MirTypeToken`）を Backend が受け入れられるようにする。
- 取り込み不能なケースに対して一貫した診断ログを出す。
- 既存の文字列トークン入力との後方互換を保つ。

## 対象範囲
- Backend: `compiler/backend/llvm`
- 仕様/計画: `docs/plans/docs-examples-audit/1-7-backend-runtime-sum-mir-json-draft-20251227.md`

## 前提・現状
- `parse_reml_type` は文字列トークンのみ対応で、未知トークンは `RemlType::Pointer` にフォールバックする。
- `RemlType::Adt` は存在するが JSON からの生成経路がない。

## 実装方針（ドラフト）

### 1. 型トークンのデシリアライズ
- `MirTypeJson` を `#[serde(untagged)]` で定義し、文字列/構造化どちらも受ける。
  - 文字列: 現行 `parse_reml_type` の互換処理を維持。
  - 構造化: `kind` によって `RemlType` に変換。

### 2. 構造化トークンの変換規則
- `primitive`: `name` を `RemlType` に直マップ。
- `ref`: `mutable` と `to` を `RemlType::Ref` に変換。
- `slice` / `set`: `item` を再帰変換。
- `tuple`: `items` を `RemlType::RowTuple` に変換。
- `adt`:
  - `tag_bits` と `variants` を用いて `RemlType::Adt` を生成。
  - `payload_layout=boxed` は `RemlType::Pointer` にフォールバックし、診断ログに理由を記録。

### 3. 診断ログ方針
- 診断は JSON 取り込み時点で `BackendDiffSnapshot.diagnostics` に追記する。
- 例: `diag backend.type.json.unsupported.kind: record`
- 例: `diag backend.type.json.payload.boxed: adt=Option variant=1`
- 例: `diag backend.type.json.invalid.tag_bits: adt=Result tag_bits=0`

### 4. 互換維持
- 文字列トークン入力は従来通りに解釈し、構造化 JSON が来た場合のみ新処理へ移行する。
- 既存のテストはそのまま通ることを前提とし、新規テストのみ追加する。

## 実行計画

### フェーズ 0: 現状確認
  - [ ] `parse_reml_type` の呼び出し箇所を整理する
  - [ ] `MirParamJson` / `return` / `ffi_calls` の JSON 仕様を確認する

### フェーズ 1: MirTypeJson 定義
  - [ ] `MirTypeJson` / `MirTypeKind` の構造体を追加する
  - [ ] `MirParamJson` / `return` / `ffi_calls` の型トークンを差し替える

### フェーズ 2: 変換ロジック実装
  - [ ] 文字列トークンの互換処理を維持する
  - [ ] 構造化トークンの `RemlType` 変換を実装する
  - [ ] `payload_layout=boxed` のフォールバック診断を追加する

### フェーズ 3: テスト/検証
  - [ ] `adt` の JSON 入力に対する `parse_reml_type` テストを追加する
  - [ ] 既存の JSON スナップショットテストが破壊されないことを確認する

## 受け入れ基準
- `MirTypeJson` が文字列/構造化の両方を受け入れられる。
- `RemlType::Adt` が JSON 経由で生成され、診断ログが意図通り出力される。

## 進捗管理
- 本計画書作成日: 2025-12-27
- 進捗欄（運用用）:
  - [ ] フェーズ 0 完了
  - [ ] フェーズ 1 完了
  - [ ] フェーズ 2 完了
  - [ ] フェーズ 3 完了

## 関連リンク
- `docs/plans/docs-examples-audit/1-7-backend-runtime-sum-mir-json-draft-20251227.md`
- `docs/plans/docs-examples-audit/1-7-frontend-mir-type-token-plan-20251227.md`
- `compiler/backend/llvm/src/integration.rs`
