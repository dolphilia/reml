# 1.0 Active Patterns 導入計画（ドラフト）

## 目的と背景
- `docs/notes/pattern-matching-improvement.md` で **最優先** とされた Active Patterns を、Reml の DSL ファースト思想に適合する形で導入する。
- 既存の `match` ガード/エイリアス仕様（`docs/spec/1-1-syntax.md` で再構成済み）と干渉しない構文・診断ポリシーを設計し、Phase 4 回帰計画と整合する実装ガイドを用意する。

## スコープ
- **含む**: 記法決定、BNF/構文セクション更新、型・効果セマンティクスの定義、診断ポリシー、例題/テスト計画、移行指針。
- **含まない**: Rust/OCaml 実装の具体コード、ランタイム最適化、LSP 連携の詳細。

## 記法と仕様設計の前提
- 定義記法は F# 互換案を第一候補: `pattern (|Name|_|)(args) = expr`。部分/完全パターンを `_|` の有無で区別する案を比較。
- パターン使用例: `match input with | Name value -> ...`。`as` エイリアスや `when` ガードと併用可能とする。
- 戻り値契約:  
  - 部分パターン: `Option<T>`（`None` でマッチ失敗）。  
  - 完全パターン: `T` もしくは `Result<T, E>` だが、網羅性計算と衝突しないように `Result` は診断設計で扱いを決める。
- 副作用: `@pure` 関数内で使用する場合の制約を `docs/spec/1-3-effects-safety.md` と照合。

## タスク（ドラフト）
1. **構文/BNF ドラフト**  
   - `docs/spec/1-1-syntax.md` に Active Pattern 定義/呼び出し構文を追加。  
   - `docs/spec/1-5-formal-grammar-bnf.md` へ `active_pattern_decl` / `active_pattern_app` の生成規則を追記し、`match_arm` に組み込む。  
   - 優先順位表に `when`/`as` との結合順を明記。
2. **型・効果セマンティクス**  
   - `docs/spec/1-2-types-Inference.md` へ戻り値契約（Option/Result）と型推論ルールを追加。  
   - 効果安全性 (`@pure` / `perform`) との整合を `docs/spec/1-3-effects-safety.md` に明文化。
3. **診断ポリシー設計**  
   - 部分パターンの網羅性警告（推奨: `warning` から段階的に `error`）、パターン戻り値型不一致のエラーコードを新設。  
   - Active Pattern 本体が例外/エラーを返した場合の診断を `core.parse` 系と揃える。
4. **アセット計画**  
   - `examples/spec_core/chapter1/match_expr/` へ最小サンプル（成功/失敗/ガード併用）を追加する計画を整理。  
   - `reports/spec-audit/ch4` に診断サンプルを追加し、`phase4-scenario-matrix.csv` 用の `diagnostic_keys` を定義。
5. **移行・互換性メモ**  
   - 既存の関数呼び出しと Active Pattern 名の衝突を避ける命名ルール案を提示。  
   - `docs/guides/ai-integration.md` へ導入時の LLM 提案ガイドライン追記案を準備。

## 成果物（ドラフト段階の出口条件）
- 上記タスクのアウトラインを各仕様ファイルへの修正ポイント付きで提示できている。
- 最低 2 本の例題案（成功/失敗）が決まり、診断キーと期待挙動が文章で示されている。
- Phase 4 回帰計画（`docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md`）に対し、影響範囲と併走方針をコメントで共有できる。

## 未解決事項 / TODO
- `Result` ベースの Active Pattern を正式に許容するかは網羅性計算コストを評価して決定する。
- LSP での補完候補提示（`(|Name|_|)`）の表示ポリシーは別ガイド（`docs/guides/core-parse-streaming.md` 付録を想定）に委ねる。

## 仕様差分メモ（1-1 / 1-5 に対する追記箇所）
- `docs/spec/1-1-syntax.md` は C.3/C.4 に Active Pattern の説明が存在しない。定義記法と `match` 内での使用例を追加する差分が必要。
- `docs/spec/1-5-formal-grammar-bnf.md` には Active Pattern の生成規則が無い。以下をドラフトとして追加する方針を計画タスクに組み込む。  
  - `ActivePatternDecl ::= "pattern" "(|" Ident ("|_|")? "|)" "(" ParamList? ")" "=" Expr`（呼び出し規約と戻り値契約を注記）  
  - `ActivePatternApp  ::= "(|" Ident "|)" Pattern?` を `Pattern` もしくは `Primary` に統合（優先順位を表で明記）  
  - `MatchArm` のガード/エイリアス順は `MatchGuard? MatchAlias?` 固定だが Phase 4 実装は順不同を許容しているため、どちらに揃えるか決定する必要あり。
- 診断キーは仕様本文に未登場。最低限 `pattern.active.return_contract_invalid`（Option/Result 以外の戻り値）、`pattern.active.effect_violation`（@pure で副作用を持つ場合）を `2-5-error.md` か `1-1` 診断節に追加する差分を要検討。
