# 1.2 実装ギャップ後続対応計画（Backend / Runtime / 2025-12-31）

`docs/plans/docs-examples-audit/1-2-impl-gap-plan-20251231-1.md` で復元した固定長配列型・`struct` 型・関数型引数名が Backend / Runtime に与える影響を整理し、必要な後続対応を計画する。

## 目的
- Frontend で受理された固定長配列型 `[T; N]` が Backend で誤って `pointer` へフォールバックしないようにする。
- Backend の型レイアウト計算が `[T; N]` を扱えるように整備する。
- `struct` 型 / 関数型引数名の影響範囲を明確化し、必要なら後続の計画に切り出す。

## 対象範囲
- Frontend MIR 型トークン: `compiler/frontend/src/semantics/mir.rs`
- Backend 型パース / レイアウト: `compiler/backend/llvm/src/integration.rs`, `compiler/backend/llvm/src/type_mapping.rs`
- Backend コード生成（必要時）: `compiler/backend/llvm/src/codegen.rs`
- Runtime: `compiler/runtime/native`（影響確認のみ）

## 背景
- Frontend の `TypeKind::Array` は `[T; N]` を `TypeAnnot.render()` で文字列化する。
- Backend の `parse_reml_type` は `[T]` のみを `RemlType::Slice` として扱い、`[T; N]` は未知型として `Pointer` に落ちる。
- `struct` 型は Frontend で `TypeDeclKind::Opaque` として扱われており、Backend は既存の挙動（未知型→`Pointer`）で一応破綻しない。
- 関数型引数名は型トークン表記にのみ影響し、Backend の型パースは `fn(...)` 自体を扱えないため、既存ギャップの範囲内に留まる。

## ギャップ一覧（Backend / Runtime 影響）
1) **固定長配列型 `[T; N]` の Backend 未対応**
- `parse_reml_type` が `"i64; 6"` を型として解釈できず `Pointer` にフォールバックする。
- 関数引数/戻り値に `[T; N]` が現れた場合、Backend の型レイアウトが破綻する可能性がある。

2) **`struct` 型のレイアウト未定義**
- Frontend は `struct` を名義型として保持するが、Backend は未知型を `Pointer` として扱うため、現時点では「不透明型」として成立する。
- 値型としての `struct` レイアウトを導入する場合は別計画で扱う。

3) **関数型引数名の影響**
- 型トークンに `fn(fd: i32, ...)` が現れるが Backend は `fn` 型を解析していない。
- 本件は新規追加ではなく既存の未対応領域のため、本計画の対象外とする。

## 実装修正計画

### フェーズ 1: 固定長配列型の型パース拡張
- `parse_reml_type` に `[T; N]` を認識する分岐を追加する。
- `N` は整数リテラルのみ受理し、失敗時は `backend.todo.fixed_array_type` の診断を出す。
- 作業ステップ:
  - `compiler/backend/llvm/src/integration.rs` の `parse_reml_type` の既存分岐（`[T]` や `*T` など）を確認し、`[T; N]` がどこで `Pointer` に落ちているかをメモする。
  - `[` と `]` の内側トークンを分割する処理に `;` 区切りパスを追加し、`[T; N]` を `element` と `length` に分解する。
  - `N` のパースは 10 進整数リテラルのみ受理し、桁区切りや `_` は拒否する方針を明記する（受理範囲を将来拡張しやすいように TODO を残す）。
  - `N` が空・負数・非数値・オーバーフローの場合は `backend.todo.fixed_array_type` を診断し、`RemlType::Pointer` にフォールバックする。
  - 診断には元の型トークン（例: `[i64; X]`）と失敗理由を付与する。

### フェーズ 2: Backend 型レイアウトの拡張
- `RemlType` に固定長配列のバリアントを追加する（例: `Array { element, length }`）。
- `TypeMappingContext::layout_of` で配列長 `N` に応じたサイズ/アラインメントを計算する。
- 作業ステップ:
  - `compiler/backend/llvm/src/type_mapping.rs` の `RemlType` 定義を確認し、既存の `Slice`/`Pointer` と並ぶ形で `Array` を追加する。
  - `TypeMappingContext::layout_of` の分岐に `Array` を追加し、`element` のレイアウト取得 → `size = element.size * N` → `align = element.align` を計算する。
  - `size` 計算時のオーバーフローや `N == 0` の扱いを決め、`backend.todo.fixed_array_layout` などの診断を追加する（挙動が未決なら `Pointer` フォールバック）。
  - `layout_of` が `Array` を返す際、`element` の ABI ルールを逸脱しないこと（`align` の継承）をコメントで補足する。

### フェーズ 3: Backend コード生成方針の整理
- 固定長配列が関数引数/戻り値に現れた場合の LLVM 型生成方針を決める。
  - 例: LLVM の `[N x <elem>]` を使用するか、暫定で `ptr` に落として診断するか。
- 作業ステップ:
  - `compiler/backend/llvm/src/codegen.rs` の型変換（MIR 型 → LLVM 型）の経路を追い、`RemlType::Array` で詰まるポイントを特定する。
  - LLVM の `ArrayType` で表現する場合の利点/制約（ABI、関数引数の取り扱い）を整理し、採用可否をメモする。
  - 直ちに `ArrayType` を使わない場合は、`RemlType::Array` を検出して `backend.todo.fixed_array_codegen` を発行しつつ `Pointer` に落とす方針を明記する。
  - 生成経路が複数ある場合（引数/戻り値/ローカル/メモリコピー）で統一ルールを宣言し、未対応箇所を TODO で列挙する。
 - 確認結果:
   - `LlvmIrBuilder::build_function` は `TypeMappingContext::layout_of` の `description` をそのまま関数引数/戻り値へ反映しており、`RemlType::Array` は `[N x <elem>]` の表記で出力される。
   - `LlvmInstr::Alloca` / `Load` / `Store` も `layout_of` の文字列表現を用いるため、ローカル領域でも配列型は同じ表記で扱われる。
   - 現段階の方針として、Backend の LLVM 風 IR では `ArrayType` 相当の表記を採用し、コード生成段階で `Pointer` へ強制フォールバックしない。
   - ABI 互換性（配列の値渡し/戻り値の扱い）は未検証のため、実 LLVM への移行時に `backend.todo.fixed_array_codegen` 相当の診断 or 変換規則を追加する前提で別計画へ接続する。

### フェーズ 4: Runtime 影響確認
- 固定長配列が Runtime ABI を要求しないことを確認する。
- もし `reml_array_t` 等への変換を採用する場合は別計画へ切り出す。
- 作業ステップ:
  - `compiler/runtime/native/include/reml_runtime.h` の公開 API を確認し、固定長配列専用の ABI 型が存在しないことを記録する。
  - 既存 ABI（ポインタ + length 形式）がある場合は、固定長配列がそれと同一に扱われる想定をドキュメント化する。
  - Runtime 側に変更が不要である旨を本計画書の注記として明記する（必要なら関連計画へのリンクを追加）。
- 確認結果:
  - `compiler/runtime/native/include/reml_runtime.h` には固定長配列専用の ABI 型は定義されていない。
  - `reml_array_t` はヒープ配列（`len` + `items`）の ABI であり、固定長配列（値型）を直接表現する用途ではない。
  - 現段階では Runtime 変更は不要とし、固定長配列を `reml_array_t` へ変換する方針は `docs/plans/docs-examples-audit/1-5-backend-runtime-array-semantics-plan-20251226.md` で検討する。

### フェーズ 5: テストと検証
- Backend の型パーステストに `[i64; 6]` を追加する。
- 診断またはレイアウト計算が期待通りであることを確認する。
- 作業ステップ:
  - `compiler/backend/llvm/src/integration.rs` の既存テストに `[i64; 6]` を追加し、`RemlType::Array` が返ることを確認する。
  - 異常系のテストとして `[i64; X]` / `[i64; -1]` / `[i64; 18446744073709551616]` を追加し、診断が出ることを確認する。
  - `compiler/backend/llvm/src/type_mapping.rs` に `layout_of` のテストを追加し、`size` と `align` が期待値になることを確認する。
  - `backend.todo.*` 診断が出る経路について、ログ/診断の内容が追跡可能であることを確認する。

## 受け入れ基準
- `[T; N]` が Backend で `Pointer` へ無言フォールバックしない。
- Backend の型レイアウト計算が固定長配列を扱える。
- 未対応部分は `backend.todo.*` の診断で明示される。

## 進捗管理
- 本計画書作成日: 2025-12-31
- 進捗欄（運用用）:
  - [x] フェーズ 1 完了
  - [x] フェーズ 2 完了
  - [x] フェーズ 3 完了
  - [x] フェーズ 4 完了
  - [x] フェーズ 5 完了

## 関連リンク
- `docs/plans/docs-examples-audit/1-2-impl-gap-plan-20251231-1.md`
- `docs/plans/docs-examples-audit/1-5-backend-runtime-array-semantics-plan-20251226.md`
- `docs/plans/docs-examples-audit/1-7-backend-runtime-type-decl-layout-plan-20251227.md`
- `compiler/frontend/src/semantics/mir.rs`
- `compiler/backend/llvm/src/integration.rs`
- `compiler/backend/llvm/src/type_mapping.rs`
