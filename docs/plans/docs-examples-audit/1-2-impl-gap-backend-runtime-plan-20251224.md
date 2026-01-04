# 1.2 実装ギャップ後続対応計画（Backend / Runtime / 2025-12-24）

`docs/plans/docs-examples-audit/1-2-impl-gap-plan-20251223.md` と `docs/plans/docs-examples-audit/1-2-impl-gap-plan-20251224.md` の追記で明らかになった Backend / Runtime 側の補正ポイントを、実装タスクとして切り出して整理する。

## 目的
- ドキュメント監査で復元した構文・型表記が、Backend / Runtime でも意味を失わない状態へ進める。
- `...` / `[T]` / `&` / `&mut` を含む仕様例が、将来の実行系フェーズで破綻しないよう基盤を整える。

## 対象範囲
- Backend: `compiler/backend/llvm/`
- Runtime: `compiler/runtime/`
- Frontend MIR 連携: `compiler/frontend/src/semantics/mir.rs` と JSON 出力（Backend の `integration.rs` が読むスキーマ）
- 参考仕様: `docs/spec/1-2-types-Inference.md`, `docs/spec/3-2-core-collections.md`, `docs/spec/3-9-core-async-ffi-unsafe.md`

## 背景となるギャップ
1) **`[T]` と `&` / `&mut` の型情報が Backend で消失する**
- Backend 側の `parse_reml_type` が未知トークンを `pointer` へフォールバックするため、型差分が保存されない。
- `RemlType` が `Slice` / `Ref` を持たず、レイアウト算定が不可能。

2) **C 可変長引数 (`...`) の情報が Backend へ届かない**
- Frontend の AST には `varargs` が存在するが、Typed/MIR へ伝搬しておらず、Backend の `FfiCallSignature` にも variadic フラグが存在しない。
- Runtime には `FfiFnSig.variadic` があるため、Frontend → Backend/Runtime の橋渡しが不足。

## 実装修正計画

### フェーズ 1: スキーマと型表現の補強
1) MIR 型表記の仕様化
- 目的: Backend 側が `Slice` / `Ref` を確実に識別できる文字列表現を定める。
- 作業ステップ:
  - Typed / MIR の `ty` 文字列における表記ルールを `docs/spec/1-2-types-Inference.md` の表記と一致させる。
  - `&T` / `&mut T` / `[T]` の表記をそのまま JSON に出す前提を明記する。
  - 確認用サンプルとして `reports/spec-audit/ch1/mir-json-type-sample-20251224.json` を追加する。

2) Backend `RemlType` の拡張
- 目的: `[T]` / `&T` / `&mut T` を `RemlType` とレイアウトに落とす。
- 作業ステップ:
  - `compiler/backend/llvm/src/type_mapping.rs` に `Slice` / `Ref { mutable }` を追加する。
  - `layout_of` に `[T]` の `{ptr,len}` を反映する（既存の `String` と同様のサイズ・アラインメント方針を明記）。

3) `parse_reml_type` の拡張
- 目的: Backend 側で `[T]` / `&` / `&mut` を正しく解析する。
- 作業ステップ:
  - `compiler/backend/llvm/src/integration.rs` の `parse_reml_type` に簡易パーサを追加する。
  - ネスト型の再帰対応（例: `&[T]`）を最低限検討する。

### フェーズ 2: varargs の伝搬
1) Frontend Typed / MIR への varargs 伝搬
- 目的: `extern "C" fn ...` の variadic を JSON へ載せる。
- 作業ステップ:
  - `compiler/frontend/src/semantics/typed.rs` と `compiler/frontend/src/semantics/mir.rs` に `varargs` を追加する。
  - 既存の型チェック診断（`ffi.varargs.*`）と矛盾しない形式で保持する。
 - 対応状況: 完了（Typed/MIR の `varargs` 追加と JSON 出力を反映済み）。

2) Backend FFI 署名の拡張
- 目的: Backend の `FfiCallSignature` に variadic 情報を持たせる。
- 作業ステップ:
  - `compiler/backend/llvm/src/ffi_lowering.rs` の `FfiCallSignature` に `variadic: bool` を追加する。
  - `compiler/backend/llvm/src/integration.rs` の `FfiCallJson` に `variadic` を追加し、MIR JSON から受理する。
 - 対応状況: 完了（JSON 取り込みとテスト `ffi_call_variadic_is_loaded_from_json` を追加済み）。

3) Runtime 連携方針の整理
- 目的: Runtime の `FfiFnSig.variadic` と一致する経路を作る。
- 作業ステップ:
  - `compiler/runtime/src/ffi/dsl/mod.rs` の `fn_sig` への入力元が Backend/MIR から流れてくる箇所を整理する。
  - 実行系が未接続の場合は TODO として導線を記録する。
 - 対応状況: 完了（`FfiCallSpec` と `to_signature` を追加し、MIR JSON から `variadic` と型変換を反映）。
 - 簡易検証: `cargo test ffi_call_spec_from_mir_json_variadic_and_types` を実行し成功。

### フェーズ 3: 監査・検証
1) 監査ログへの追記
- 目的: docs-examples から派生した Backend/Runtime 対応であることを明示する。
- 作業ステップ:
  - `reports/spec-audit/summary.md` に Backend/Runtime 対応の着手ログを追加する。
  - 変更点と該当サンプル（`sec_b_3`, `sec_f`, `sec_h_2-a`, `sec_b_4-f`）の対応関係を整理する。

2) 簡易検証
- 目的: MIR JSON の変更が Backend に読み込まれることを確認する。
- 作業ステップ:
  - `compiler/frontend` 側で `--emit-mir-json` を使ったサンプル出力を取得する。
  - Backend の `generate_snapshot_from_mir_json` で `Slice` / `Ref` / `variadic` が保持されることを確認する。

## 詳細タスクリスト（ファイル単位 TODO）

### Frontend
- `compiler/frontend/src/semantics/typed.rs`: `TypedFunction` に `varargs: bool` を追加し、AST の `FunctionSignature.varargs` を伝搬する TODO。
- `compiler/frontend/src/semantics/mir.rs`: `MirFunction` に `varargs: bool` を追加し、Typed から引き継ぐ TODO。
- `compiler/frontend/src/semantics/mir.rs`: MIR JSON の `schema_version` を更新するか検討する TODO（互換性維持が難しい場合のみ）。

### Backend
- `compiler/backend/llvm/src/type_mapping.rs`: `RemlType::Slice` / `RemlType::Ref { mutable }` を追加する TODO。
- `compiler/backend/llvm/src/type_mapping.rs`: `[T]` のレイアウト（`{ptr,len}`）と `&T` / `&mut T` の ABI 方針を記述する TODO。
- `compiler/backend/llvm/src/integration.rs`: `parse_reml_type` に `[T]` / `&` / `&mut` の簡易パーサを実装する TODO。
- `compiler/backend/llvm/src/ffi_lowering.rs`: `FfiCallSignature` に `variadic: bool` を追加する TODO。
- `compiler/backend/llvm/src/integration.rs`: `FfiCallJson` に `variadic` を追加し、MIR JSON から取り込む TODO。
- `compiler/backend/llvm/src/codegen.rs`: variadic を使う場合の stub 生成方針（未実装なら TODO コメント）を追加する TODO。

### Runtime
- `compiler/runtime/src/ffi/dsl/mod.rs`: `FfiFnSig.variadic` が JSON/DSL 経由で設定される入口を洗い出す TODO。
- `compiler/runtime/src/ffi/dsl/mod.rs`: variadic 呼び出し時の引数数検証・診断方針を仕様と突き合わせる TODO。
- `compiler/runtime/src/ffi/dsl/mod.rs`: `[T]` / `&T` の ABI 表現が必要になった場合の候補構造体をコメントで整理する TODO。

### 検証・ログ
- `reports/spec-audit/summary.md`: Backend/Runtime 追随開始の記録を追加する TODO。
- `reports/spec-audit/ch1/docs-examples-fix-notes-YYYYMMDD.md`: `sec_b_3` / `sec_f` / `sec_h_2-a` / `sec_b_4-f` に紐づく Backend/Runtime 影響メモを追記する TODO。

## 進捗管理
- 本計画書作成日: 2025-12-24
- 進捗欄（運用用）:
  - [x] フェーズ 1 完了
  - [x] フェーズ 2 完了
  - [x] フェーズ 3 完了

## 関連リンク
- `docs/plans/docs-examples-audit/1-2-impl-gap-plan-20251223.md`
- `docs/plans/docs-examples-audit/1-2-impl-gap-plan-20251224.md`
- `docs/spec/1-2-types-Inference.md`
- `docs/spec/3-2-core-collections.md`
- `docs/spec/3-9-core-async-ffi-unsafe.md`
