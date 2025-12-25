# 1.2 実装ギャップ後続対応計画（Backend / Runtime / 2025-12-24）

`docs/plans/docs-examples-audit/1-2-impl-gap-plan-20251224-2.md` の `defer`/`propagate` 対応で Backend が参照する runtime/intrinsic の実体が未整備のため、Backend / Runtime 側の整合タスクを整理する。

## 目的
- Backend が生成する IR をランタイムとリンク可能な状態にする。
- `@reml_value` / `@reml_index_access` の ABI と実装方針を確定する。

## 対象範囲
- Backend: `compiler/rust/backend/llvm/src/codegen.rs`
- Runtime: `runtime/native/include/reml_runtime.h`, `runtime/native/src`
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

### フェーズ 2: Backend 側の呼び出し整理
- `compiler/rust/backend/llvm/src/codegen.rs` で `@reml_value` 呼び出し箇所を洗い出す。
- 決定した命名規則に従って、型別関数 or LLVM cast への置換を行う。
- `@reml_index_access` 呼び出しの引数順序と型を仕様に合わせて修正する。
- 既存の型変換ロジックに影響が出ないか、周辺の IR 生成コードを再点検する。
- 既存の IR スナップショット/テストがある場合は差分を更新し、変更理由をメモする。
- 変更箇所に対応する TODO/コメントがあれば整理して削除・更新する。

### フェーズ 3: Runtime 実装
- `runtime/native/include/reml_runtime.h` に ABI 仕様に沿った宣言を追加する。
- `runtime/native/src` に最小限の実装（stub / identity / boundary check）を追加する。
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
  - [ ] フェーズ 1 完了
  - [ ] フェーズ 2 完了
  - [ ] フェーズ 3 完了
  - [ ] フェーズ 4 完了

## 関連リンク
- `docs/plans/docs-examples-audit/1-2-impl-gap-plan-20251224-2.md`
- `compiler/rust/backend/llvm/src/codegen.rs`
- `runtime/native/include/reml_runtime.h`
