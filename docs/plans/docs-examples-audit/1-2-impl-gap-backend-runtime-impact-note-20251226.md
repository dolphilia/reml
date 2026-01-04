# 1.2 実装ギャップの Backend / Runtime 影響調査メモ（2025-12-26）

`docs/plans/docs-examples-audit/1-2-impl-gap-plan-20251224-3.md` の修正内容について、Backend / Runtime 側への影響有無を確認した結果を簡潔に記録する。

## 結論
- 追加された識別子受理範囲（emoji/ZWJ/bidi の取り込み）は **Runtime の識別子境界判定**へ影響するため、Runtime 側の追随が必要。
- Backend は **識別子が非 ASCII を含むケース**で LLVM IR 名の整形が未整備なため、記号名サニタイズが必要。

## 根拠（確認ポイント）
- **Runtime**: キーワード境界判定が `XID_Start/XID_Continue` のみを考慮しており、emoji/ZWJ を識別子継続文字として扱っていない。
  - `compiler/runtime/src/parse/combinator.rs` の `is_ident_continue` / `keyword` 内チェック
- **Backend**: LLVM IR 名にユーザー識別子が直接流入し、非 ASCII を安全に整形する処理がない。
  - `compiler/backend/llvm/src/codegen.rs` の `LlvmBuilder::new_tmp` / `intrinsic_*` / `@{name}` 組み立て
- 既存の型ギャップ（ラベル付きフィールド、デフォルト値、リテラル型、レコード引数）は Frontend で閉じており、実行時表現は変わらない。

## 対象となったギャップ
- 合成型バリアントのラベル付きフィールド
- タプル型のラベル付きフィールド
- 型定義内のデフォルト値
- 文字列リテラル型と型の和
- 引数位置のレコードリテラル

## 補足
- Backend / Runtime の独立課題（例: `@reml_value` / `@reml_index_access`）は本件と無関係。
- 仕様の識別子定義を拡張したため、Runtime/Backend の整合計画を別途作成する。
