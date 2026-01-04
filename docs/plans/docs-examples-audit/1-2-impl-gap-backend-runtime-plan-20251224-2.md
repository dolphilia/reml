# 1.2 実装ギャップ後続対応計画（Backend / Runtime / 2025-12-24）

`docs/plans/docs-examples-audit/1-2-impl-gap-plan-20251224-2.md` の `defer`/`propagate` 対応で Backend が参照する runtime/intrinsic の実体が未整備のため、Backend / Runtime 側の整合タスクを整理する。

## 目的
- Backend が生成する IR をランタイムとリンク可能な状態にする。
- `@reml_value` / `@reml_index_access` の ABI と実装方針を確定する。

## 対象範囲
- Backend: `compiler/backend/llvm/src/codegen.rs`
- Runtime: `compiler/runtime/native/include/reml_runtime.h`, `compiler/runtime/native/src`
- 監査元計画: `docs/plans/docs-examples-audit/1-2-impl-gap-plan-20251224-2.md`

## 確認できた影響
1) `@reml_value` の実体が Runtime に存在しない
- Backend が多箇所で `@reml_value` を挿入している。
- Runtime 側に宣言/実装が無く、リンク時に未解決になる可能性がある。
- 署名が多型のため、LLVM IR 上で関数型が衝突する懸念がある。

2) `@reml_index_access` の実体が Runtime に存在しない
- `index` 式の lowering で `@reml_index_access` を呼び出している。
- 対象コレクションと ABI が未定義のため、runtime 実装が追随できない。

## 実装修正計画（Backend / Runtime）

### フェーズ 1: ABI/セマンティクス確定
- `@reml_value` の責務を整理し、仕様の位置付け（cast / 動的変換 / 監査目的）を明記する。
- `@reml_value` の型バリエーションを列挙し、命名規則（例: `@reml_value_i64` / `@reml_value_ptr`）を決定する。
- LLVM IR 上での表現方針を決める（専用 intrinsic 化 or 既存 cast 命令へ置換）。
- Backend で必要になる ABI 仕様（引数/戻り値のビット幅、アラインメント、呼び出し規約）を一度表にまとめる。
- `@reml_index_access` の引数型（target, index）と戻り値型を定義し、nullable/境界外の扱いを決める。
- 対象コレクションの初期セット（`List`/`Str` 等）を定義し、将来拡張時の差分方針をメモ化する。
- 決定事項を本計画書に追記し、必要があれば関連仕様 (`docs/spec`) の追記タスクを起票する。

#### 決定事項（フェーズ 1 / Backend・Runtime 共有）

1) `@reml_value` の位置付け
- **責務**: 値の型整形（アンボックス／キャスト／型合わせ）を行うバックエンド専用の補助 intrinsic とし、監査用途では使わない。
- **可視性**: Reml の表層仕様には露出させず、IR lowering の内部境界として扱う。
- **実行時効果**: 原則は **副作用なし**（pure）。失敗時のエラー契約は持たせず、型不一致は Backend 側のバグとして扱う。

2) `@reml_value` の命名とバリエーション
- **命名規則**: `@reml_value_<suffix>` を採用する（`<suffix>` は返却型の短縮名）。
- **Phase 1 の最小セット**:
  - `@reml_value_i64`
  - `@reml_value_bool`
  - `@reml_value_ptr`
  - `@reml_value_str`（`reml_string_t`）
- **将来拡張**: `i32` / `f32` / `f64` / `usize` などは Phase 2 以降で追加。

3) LLVM IR 上の表現方針
- **基本方針**: `@reml_value_*` はコンパイラ intrinsic として扱い、**最終 IR では `load`/`bitcast`/`zext`/`trunc`/`ptrtoint`/`inttoptr` に置換**する。
- **Runtime への依存**: 置換後は runtime に実体を持たせない。置換が完了するまでの暫定期間は `compiler/runtime/native/src` に **identity stub** を置く方針で、ABI は下記の表に合わせる。

4) ABI 仕様（Phase 1 / x86_64 前提）
| 論理型 | LLVM IR 表現 | C 側型 | サイズ | アラインメント | 備考 |
| --- | --- | --- | --- | --- | --- |
| `i64` | `i64` | `int64_t` | 8 | 8 | 符号付き |
| `bool` | `i1`（IR）/`i8`（C ABI） | `uint8_t` | 1 | 1 | C ABI 側は 1 byte で受ける |
| `ptr` | `ptr` | `void*` | 8 | 8 | 不透明ポインタ |
| `Str` | `{ptr, i64}` | `reml_string_t` | 16 | 8 | `data` は NULL 終端を想定 |
- **呼び出し規約**: `C` ABI（SystemV / Win64）に従う。`TargetMachine` の `CallingConvention` を踏襲。

5) `@reml_value_*` の暫定 ABI（置換前提）
- **シグネチャ**: `T @reml_value_<suffix>(const T value)` を基本とし、最終的には IR 側で `T` の値に合わせた `load`/`cast` に置換する。
- **注意**: payload ポインタから `T` を得るケースは Backend 側で `load` を生成してから `@reml_value_*` を呼ぶ方針に寄せる（`@reml_value` に unbox 責務を持たせない）。

6) `@reml_index_access` の ABI とセマンティクス
- **シグネチャ（暫定）**: `ptr @reml_index_access(ptr target, i64 index)`
- **戻り値**: 要素の **payload ポインタ**。呼び出し側で `@reml_value_*` による整形を行う。
- **境界外/NULL**: 例外（panic）を発行し、`@panic(ptr)` を呼ぶ。`null` 戻りは使用しない。
- **対象コレクション（Phase 1）**:
  - `List<T>`: 0-based で線形走査し、該当要素の payload ポインタを返す。
  - `Str`: **byte index** を採用し、`reml_string_t.data + index` のポインタを返す（UTF-8 グラフェム単位の処理は Core.Text API に委譲）。
- **未対応型**: `panic` による即時停止（「unsupported index target」）。

7) 仕様追記タスク（起票予定）
- `docs/spec/1-1-syntax.md`: `index` 式の結果型・境界外契約の明文化。
- `docs/spec/3-2-core-collections.md`: `List` の index 仕様（0-based / O(n) / panic）。
- `docs/spec/3-3-core-text-unicode.md`: `Str` の `[]` が **byte index** であることを明記し、`grapheme` 単位 API との差別化を追記。

### フェーズ 2: Backend 側の呼び出し整理
- `compiler/backend/llvm/src/codegen.rs` で `@reml_value` 呼び出し箇所を洗い出す。
- 決定した命名規則に従って、型別関数 or LLVM cast への置換を行う。
- `@reml_index_access` 呼び出しの引数順序と型を仕様に合わせて修正する。
- 既存の型変換ロジックに影響が出ないか、周辺の IR 生成コードを再点検する。
- 既存の IR スナップショット/テストがある場合は差分を更新し、変更理由をメモする。
- 変更箇所に対応する TODO/コメントがあれば整理して削除・更新する。

#### フェーズ 2 着手チェックリスト（Backend）
- [ ] `@reml_value` 呼び出し位置の棚卸し（`compiler/backend/llvm/src/codegen.rs` の `emit_value_expr` / `lower_*` 系を単位に一覧化）
- [ ] 変換対象の型分類（`i64`/`bool`/`ptr`/`Str`）を整理し、置換優先順位を決める（`emit_value_expr` の `MirExprKind::Literal` / `MirExprKind::Identifier` 参照）
- [ ] `@reml_value` を `@reml_value_<suffix>` へ置換する方針を明記（`@reml_value_i64` など）（`INTRINSIC_VALUE` 定義と呼び出し箇所）
- [ ] `@reml_value` を IR cast/`load` へ置換する箇所の判断基準をメモ化（`emit_value_expr` / `lower_if_else_branch_value_with_defers`）
- [ ] `@reml_value` の戻り型・引数型の整合チェック（`i1`/`i8` の扱いを含む）（`emit_value_expr` と `lower_binary_expr_to_blocks` の戻り型）
- [ ] `@reml_index_access` 呼び出しの引数型を `ptr` + `i64` に合わせる（`emit_value_expr` の `MirExprKind::Index`）
- [ ] `@reml_index_access` の戻り値 `ptr` 前提で downstream の `@reml_value_*` を挿入する箇所を確認（`emit_value_expr` の戻り型と `MirExprKind::Index` の利用箇所）
- [ ] 既存の `@reml_str_data` / `@panic` の引数変換と干渉しないことを確認（`INTRINSIC_STR_DATA` / `INTRINSIC_PANIC` の呼び出し周辺）
- [ ] 置換後に `LlvmInstr::Call`/`LlvmInstr::Cast` の生成が壊れていないかを点検（`LlvmInstr` 生成箇所）
- [ ] 影響範囲のメモ（IR スナップショット/テスト/診断ログ）を更新方針として追記（`compiler/backend/llvm/src/integration.rs` のスナップショット生成）

#### フェーズ 2 完了条件（成果物ベース）
- `compiler/backend/llvm/src/codegen.rs` に `@reml_value_<suffix>` 置換が反映され、`@reml_value` 生呼び出しが残っていないこと
- IR 断片の差分ログ（`reports/backend-ir-diff/` または相当ログ）に `@reml_value_<suffix>` の出力が確認できること
- `@reml_index_access` が `ptr` + `i64` の ABI で呼ばれている IR 断片を記録できること
- 置換前後の差分理由メモを本計画書に追記し、スナップショット更新時の根拠が追跡可能であること

#### フェーズ 2 実施メモ
- `compiler/backend/llvm/src/codegen.rs` の `@reml_value` 呼び出しを `@reml_value_i64`/`@reml_value_bool`/`@reml_value_ptr`/`@reml_value_str` に置換（`intrinsic_value_for_type` を追加）。
- `MirExprKind::Index` の index 引数を `i64` に整形してから `@reml_index_access` を呼ぶように調整（`ptr` + `i64` で統一）。
- `panic` 変換で `Str` 化する箇所は `@reml_value_str` に差し替え。
- IR 差分ログを `reports/backend-ir-diff/reml-value-index-log.json` に保存（`@reml_value_i64` と `@reml_index_access` の出力を確認）。

### フェーズ 3: Runtime 実装
- `compiler/runtime/native/include/reml_runtime.h` に ABI 仕様に沿った宣言を追加する。
- `compiler/runtime/native/src` に最小限の実装（stub / identity / boundary check）を追加する。
- `@reml_index_access` の境界外・null などのエラー契約を簡易的に固定し、後続仕様変更の注釈を残す。
- runtime のビルド設定に新規シンボルが含まれることを確認し、必要ならビルドスクリプトを更新する。
- 追加した API に対応するヘッダコメントを簡潔に記述し、使用条件を明示する。

### フェーズ 4: 検証
- Backend で IR を生成し、`@reml_value` と `@reml_index_access` の宣言が一意な型で出力されることを確認する。
- 型別関数名が衝突しないこと、呼び出し規約が期待通りであることを確認する。
- runtime とリンクできることを最小例で確認し、未解決シンボルがないことを確認する。
- 代表的な index パターン（`List` / `Str`）で実行経路を確認し、戻り値の型が一致することを確認する。
- 確認結果を簡潔に記録し、必要な追加タスク（テスト/仕様追記）を列挙する。

## 進捗管理
- 本計画書作成日: 2025-12-24
- 進捗欄（運用用）:
  - [x] フェーズ 1 完了（ABI/セマンティクス確定・決定事項追記）
  - [x] フェーズ 2 完了
  - [x] フェーズ 3 完了
  - [x] フェーズ 4 完了

### 進捗メモ（実施ログ）
- 2025-12-24: compiler/runtime/native に `@reml_value_*` / `@reml_index_access` の宣言と実装を追加。
- 2025-12-24: `make -C compiler/runtime/native runtime` を実行し、`build/libreml_runtime.a` の生成を確認。
- 2025-12-24: `make -C compiler/runtime/native test` を実行し、`test_ffi_bridge`/`test_mem_alloc`/`test_os`/`test_refcount` がすべて成功。
- 2025-12-24: `reports/backend-ir-diff/reml-value-index-log.json` に `@reml_value_i64` / `@reml_index_access` の呼び出し出力を確認。
- 2025-12-24: `tmp/runtime-link-min.ll` を作成し、`llc` + `clang`（`-isysroot`/`-L` 追加）でリンクに成功。実行結果は `123 / 42 / 98` を出力。

## 関連リンク
- `docs/plans/docs-examples-audit/1-2-impl-gap-plan-20251224-2.md`
- `compiler/backend/llvm/src/codegen.rs`
- `compiler/runtime/native/include/reml_runtime.h`
