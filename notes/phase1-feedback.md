# Phase 1 Draft Feedback Notes

レビュー観点と改善アイデアを整理するメモ。フェーズ1のドラフト更新に合わせて内容を反映する。

## 1-1 Syntax (Schema / Plugin)
- [x] スキーマ継承・部分適用のサンプルを追加（例: `schema Base { ... }` を拡張）。
- [x] 条件付き構成での優先順位（複数 `when` の場合）の説明。
- [x] `package` / `use plugin` にバージョン指定、互換情報の語彙を追加（将来計画として明記）。

## 1-2 Types & Inference
- [x] テンソル/スキーマ/リソースID型の例を追加。
- [ ] `effect` タグ付き関数について、合成ルールを擬似コードで提示（ex: `combine_effects(a, b)`）。
- [x] `SchemaDiff` の利用例を明確化（マイグレーション DSL の予定記述）。

## 1-3 Effects & Safety
- [x] 効果分類表、ホットリロードサンプルを追加。
- [x] 各効果タグの相互作用（例: `audit` + `config`）を表形式か図で示す計画。
- [x] `unsafe` ガイドラインでクラウド/GPU/組み込みのチェックリストを箇条書きにする。

（✅ 完了 / ◻ 未対応）
