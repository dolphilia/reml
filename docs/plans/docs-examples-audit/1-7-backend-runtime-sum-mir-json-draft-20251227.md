# 1.7 Backend/Runtime sum 型 MIR JSON 構造ドラフト（2025-12-27）

sum 型（ADT）を Backend へ渡すための MIR JSON 形状案と、Backend 取り込み手順のドラフト。

## 目的
- sum 型のタグ幅/ペイロード情報を JSON で安定的に表現する。
- 既存の文字列型トークンと共存しながら段階的に導入する。
- Backend 側の取り込み手順（`parse_reml_type` 拡張）を明文化する。

## 前提
- 現行 Backend は MIR JSON の型トークン文字列のみを `RemlType` に変換する。
- sum 型は `RemlType::Adt { tag_bits, variants }` を前提としているが、JSON から渡す経路がない。

## MIR JSON 構造ドラフト

### MirTypeToken
既存との後方互換を優先し、型は **文字列 or 構造化オブジェクト** のどちらでも表現可能とする。

```json
// 既存互換（文字列トークン）
"i64"

// 構造化（推奨）
{"kind":"primitive","name":"i64"}
```

#### kind 一覧（ドラフト）
- `primitive`:
  - フィールド: `name`（例: `i64`, `bool`, `string`, `unit`, `pointer`）
- `ref`:
  - フィールド: `mutable` (bool), `to` (MirTypeToken)
- `slice`:
  - フィールド: `item` (MirTypeToken)
- `set`:
  - フィールド: `item` (MirTypeToken)
- `tuple`:
  - フィールド: `items` (MirTypeToken[])
- `adt`:
  - フィールド: `tag_bits` (u32), `variants` (MirAdtVariant[]), `type_name` (string, 任意), `module_path` (string, 任意)

#### MirAdtVariant（ドラフト）
```json
{
  "payload": "unit",
  "payload_layout": "inline"
}
```

- `payload`: MirTypeToken
- `payload_layout`: `inline` | `boxed`
  - `inline`: ペイロードを値として保持（現行 `RemlType::Adt` の前提）
  - `boxed`: ペイロードはヒープ参照として保持（Backend では `pointer` に落とす）

### 例: Option<i64>
```json
{
  "kind": "adt",
  "tag_bits": 1,
  "type_name": "Option",
  "variants": [
    {"payload": "unit", "payload_layout": "inline"},
    {"payload": "i64", "payload_layout": "inline"}
  ]
}
```

### 例: Result<i64, string>
```json
{
  "kind": "adt",
  "tag_bits": 1,
  "type_name": "Result",
  "variants": [
    {"payload": "i64", "payload_layout": "inline"},
    {"payload": "string", "payload_layout": "inline"}
  ]
}
```

### 例: レコード/タプル payload を boxed で扱う
```json
{
  "kind": "adt",
  "tag_bits": 2,
  "variants": [
    {
      "payload": {"kind":"tuple","items":["i64","i64"]},
      "payload_layout": "inline"
    },
    {
      "payload": {"kind":"record","fields":[{"name":"x","ty":"i64"}]},
      "payload_layout": "boxed"
    }
  ]
}
```

注: `record` の kind は Backend では未対応のため、`boxed` 指定で `pointer` として扱う想定。

## Backend 取り込み手順（ドラフト）

1. `compiler/backend/llvm/src/integration.rs` に `MirTypeJson`（構造化型）を追加する。
   - `#[serde(untagged)]` で `String` と `Structured` を受ける。
   - `MirParamJson` / `return` / `ffi_calls` の型トークンを `MirTypeJson` に置き換える。
2. `parse_reml_type` を `MirTypeJson` 対応に拡張する。
   - 文字列: 既存ロジックを維持。
   - 構造化:
     - `primitive`/`ref`/`slice`/`set`/`tuple` を `RemlType` に変換。
     - `adt` は `RemlType::Adt { tag_bits, variants }` に変換し、各 `payload` を `RemlType` に再帰変換。
     - `payload_layout=boxed` は `RemlType::Pointer` に落とし、診断ログを追加。
3. `TypeMappingContext::layout_of` の ADT 仕様に合わせて、tag/payload の合算レイアウトを維持する。
   - `tag_bits` と `variants` が JSON 由来であることを前提にする。
4. `compiler/backend/llvm/src/intrinsics.rs` の `format_reml_type` に `adt` の情報を簡易表示する。
   - 例: `adt(tag_bits=1, variants=2)`
5. Frontend の MIR JSON 出力に `MirTypeToken` を導入する（別計画で反映）。
   - 既存の型トークン文字列はそのまま出力しつつ、sum 型のみ構造化出力へ移行する。

## 未決事項
- `record` を `MirTypeToken` として正式対応するか（現行は boxed で回避）。
- `payload_layout` の命名（`inline`/`boxed` の名称と意味の確定）。
- `tag_bits` の算出・丸め規則を Frontend でどの段階で固定するか。

## 関連リンク
- `docs/plans/docs-examples-audit/1-7-backend-runtime-type-decl-layout-plan-20251227.md`
- `compiler/backend/llvm/src/integration.rs`
- `compiler/backend/llvm/src/type_mapping.rs`
