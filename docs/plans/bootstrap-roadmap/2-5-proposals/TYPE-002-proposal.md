# TYPE-002 効果行統合ポリシー計画

## 1. 背景と症状
- 仕様は関数型に効果集合を含める（`A -> B ! {io, panic}`）と定義し、行多相や残余効果計算を規定している（docs/spec/1-2-types-Inference.md:155-169, docs/spec/1-3-effects-safety.md:236-303）。  
- 現行型表現 `ty` は `TArrow` のみで効果情報を保持せず（compiler/ocaml/src/types.ml:48-58）、実際の効果は `typed_fn_decl.tfn_effect_profile` に別管理される（compiler/ocaml/src/type_inference.ml:2380-2404）。  
- この乖離により、型比較や `@handles` 契約では効果集合を参照できず、仕様上の「型と効果が対で扱われる」という前提が崩れている。

## 2. Before / After
### Before
- 効果解析は `Effect_analysis` で行うが、型スキームは効果集合を持たないため `let` 多相や値制限で効果差分をチェックできない。  
- ドキュメント上は効果行が型の一部と説明されているが、実装では診断メタデータ扱いであり、自動整合（`Σ_after` 等）が不可能。

### After
- Phase 2-5 では仕様に脚注を追加し「OCaml 実装は効果行を型スキームに統合する準備中」と明記。  
- 効果集合を `ty` へ統合する設計案を作成し、`compiler/ocaml/docs/effect-system-design-note.md` に `TArrow` 拡張（`TArrow of ty * effect_row * ty` など）のドラフトを追記。  
- 実装ロードマップを Phase 2-7 効果チームと共有し、効果行統合の段階的導入（診断 → 型表現 → 行多相）を調整する。

## 3. 影響範囲と検証
- **型比較**: 効果を考慮した型等価・部分順序の仕様を整理し、`Type_unification` テストを追加。  
- **残余効果**: EFFECT-002 / EFFECT-003 の実装と連動し、効果集合を型内で扱えるか PoC を実施。  
- **ドキュメント**: Chapter 1/3 の効果行説明に実装ステージを明記し、読者が差分状態を把握できるよう脚注を追加。
- **設計ノート**: `compiler/ocaml/docs/effect-system-design-note.md` に `effect_row` のデータ構造比較（リスト/ビットセット/マップ）の評価結果を追記し、仕様更新時の根拠を残す。

## 4. フォローアップ
- 効果行を型へ組み込む際、`generalize` / `instantiate` を更新する必要があるため、Phase 2-7 の型クラスチームへ事前連絡する。  
- 型表現の変更に伴う Core IR や LLVM バックエンドへの影響を調査し、行多相を導入する際の性能評価計画を立てる。  
- 仕様側脚注を解除する時期と、typeclass 差分（TYPE-003）との整合を Phase 3 手前で再評価する。
- `docs/notes/effect-system-tracking.md` に行多相導入ロードマップを追記し、型チームと効果チームで共有するチェックポイントを明記する。
- **タイミング**: Phase 2-5 では設計検討と脚注整備を完了し、実装は Phase 2-7 の効果システム統合スプリント開始時に着手、必要に応じて Phase 3 序盤まで延長する。

## 残課題
- 効果行を `ty` に含める際の表現形式（リスト / 集合 / 位置付きタグ）をどこまで詳細化するか、型推論チームの合意が必要。  
- 行多相の完全導入をどのフェーズで行うか（Phase 3 へ繰越すか）を PM と相談したい。
