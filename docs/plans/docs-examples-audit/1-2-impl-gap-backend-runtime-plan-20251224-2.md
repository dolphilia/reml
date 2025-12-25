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
- `@reml_value` の役割を明文化する（単純な cast か、動的変換か）。
- 1 つの関数名で多型を許容しない方針へ統一する。
  - 例: 型別の `@reml_value_i64` / `@reml_value_ptr` を導入する。
  - 例: backend で LLVM の cast 命令へ置換する。
- `@reml_index_access` の引数型（target, index）と戻り値型を決める。
- 参照可能なコレクション（`List`/`Str` 等）を一旦限定する。

### フェーズ 2: Backend 側の呼び出し整理
- `compiler/rust/backend/llvm/src/codegen.rs` の `@reml_value` 呼び出しを ABI 方針に合わせて修正する。
- `@reml_index_access` の引数型を新しい仕様に合わせる。
- 既存の IR スナップショット/テストがある場合は更新する。

### フェーズ 3: Runtime 実装
- `runtime/native/include/reml_runtime.h` に新しい宣言を追加する。
- `runtime/native/src` に実装を追加する（最低限の stub/identity から着手）。
- ランタイム静的ライブラリのビルドへ組み込む。

### フェーズ 4: 検証
- Backend で IR を生成し、`@reml_value` と `@reml_index_access` の宣言が一意な型で出力されることを確認する。
- runtime とリンクできることを確認する（最小例でよい）。

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
