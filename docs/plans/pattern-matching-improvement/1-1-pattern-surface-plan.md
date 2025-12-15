# 1.1 パターン表現拡張計画（ドラフト）

## 目的
- Active Patterns 以外の周辺機能（Or/Slice/Range/Binding/Regex）を優先度付きで整理し、実装順序と診断ポリシーを定義する。
- 既存の `match` 記法（ガード/エイリアス）との互換性を維持しつつ、構文・BNF・サンプルを追加する。

## 対象機能と方針
1. **Or-patterns（最優先）**  
   - 構文: `pat1 | pat2 -> ...`、ネスト可（例: `Some(A | B)`）。  
   - 作業: `docs/spec/1-1-syntax.md` へ使用例を追加し、`1-5-formal-grammar-bnf.md` の `pattern` 規則を拡張。網羅性診断の併用ポリシーを Active Patterns と揃える。
2. **Slice Patterns（高優先）**  
   - 構文: `[head, ..tail]`, `[.., last]`, `[first, .., last]`。  
   - 作業: コレクション型の要件を `docs/spec/1-2-types-Inference.md` に追記（要 `Iterator`/`Slice` トレイト相当）。`examples/spec_core/chapter1/match_expr/` に可変長サンプルを追加する計画を策定。
3. **Range Patterns（高優先）**  
   - 構文: `a..b` / `a..=b`。整数/文字/Enum への適用可否を明示。  
   - 作業: 比較演算の型拘束を `docs/spec/1-2-types-Inference.md` へ追加し、診断キー（境界逆転、型不一致）を定義。`expected` 資産で範囲外診断サンプルを用意。
4. **Binding Operator（中優先）**  
   - 現行の `pat as name` を維持しつつ、`name @ pat` を導入するか検討。  
   - 作業: 両記法を許容する場合は BNF に並列表記し、警告・非推奨ポリシーを決める。`as` 優先で回帰影響を抑える。
5. **Regex パターン糖衣（中優先）**  
   - 構文案: `r"^\\d+" as digits -> ...`。Active Pattern 糖衣として実装位置を決める。  
   - 作業: `docs/spec/1-1-syntax.md` に限定的な使用条件（文字列/バイト列のみ等）を記載し、`docs/spec/3-3-core-text-unicode.md` との整合を確認。

## タスク（ドラフト）
- **BNF 更新**: `pattern` 規則を再構成し、Or/Slice/Range/Binding/Regex の優先順位・結合ルールを明示。  
- **型・診断定義**:  
  - Or/Slice/Range の網羅性・到達不能診断キーを追加。  
  - Range 境界の型チェック、Slice の可変長セマンティクスを型推論章へ追加。  
  - Regex 糖衣は Active Pattern 診断を再利用する方針を明文化。
- **サンプル計画**:  
  - `examples/spec_core/chapter1/match_expr/` に各機能の成功/失敗例を 1 本ずつ追加する案をまとめる。  
  - `reports/spec-audit/ch4` に対応する診断サンプルを設計し、`phase4-scenario-matrix.csv` 用の `diagnostic_keys` を列挙。
- **導入順序**: Or → Slice → Range → Binding → Regex の順で仕様ドラフトを確定し、各ステップで回帰計画（Phase 4）への影響をレビューする。

## 成果物（ドラフト段階の出口条件）
- 対象機能ごとの BNF 追記ポイントと診断キー案が明文化されている。
- サンプル追加計画（ファイルパス、期待診断/標準出力）が文章で用意され、重複回避の方針が決まっている。
- 導入順序と Phase 4 回帰計画のチェックポイントが合意できる状態にある。

## 仕様差分メモ（1-1 / 1-5 に対する追記箇所）
- `docs/spec/1-1-syntax.md` の C.3/C.4 に Or/Slice/Range/Binding/Regex の説明・例が無い。優先度順に使用例と網羅性・到達不能の言及を追記する必要がある。
- `docs/spec/1-1-syntax.md` のガード記法は本文 `pat if cond` と BNF 側 `when` が不一致。どちらに統一するかを決めてサンプルを揃える（`as` 例も本文に未掲載）。
- `docs/spec/1-5-formal-grammar-bnf.md` の `Pattern` は `_`/Ident/タプル/レコード/コンストラクタのみ。以下の非終端を追加し、優先順位表を併記する差分が必要。  
  - `OrPattern ::= Pattern "|" Pattern { "|" Pattern }`  
  - `SlicePattern ::= "[" Pattern? ".." Pattern? "]"` など可変長形式  
  - `RangePattern ::= RangeBound ".." RangeBound ["="]`（閉区間 `..=` を含むか決定する）  
  - `BindingPattern ::= Ident "@" Pattern | Pattern "as" Ident`（`as` と `@` の並列表記と優先順位）  
  - `RegexPattern ::= "r\"" RegexBody "\""`（Active Pattern 糖衣として制約を注記）  
  - Active Pattern の呼び出しが `Pattern`/`Primary` へ入る場合の生成規則（`(|Name|) pat?`）
- `MatchArm` の `MatchGuard? MatchAlias?` 順序は実装側（Phase 4）で順不同を許容済み。BNF を順不同に再構成するか、仕様で順序固定を明文化する必要がある。
- 診断キーは仕様に未登場のため、以下を `2-5-error.md` か `1-1` 診断節へ追加する差分が必要。  
  - 網羅性/到達不能: `pattern.exhaustiveness.missing`, `pattern.unreachable_arm`  
  - Range: `pattern.range.type_mismatch`, `pattern.range.bound_inverted`  
  - Slice: `pattern.slice.type_mismatch`, `pattern.slice.too_many_parts`  
  - Regex: `pattern.regex.invalid_syntax`, `pattern.regex.unsupported_target`  
  - Binding: `pattern.binding.duplicate_name`

## 優先順に沿った BNF 追加案とサンプル候補（Phase4 マトリクス紐付け）
以下は `docs/spec/1-5-formal-grammar-bnf.md` への追加案と、`examples/spec_core/chapter1/match_expr/` に置く想定サンプル、および `phase4-scenario-matrix.csv` で使う診断キー案。

1. **Or-patterns（最優先）**  
   - BNF 追記: `OrPattern ::= Pattern "|" Pattern { "|" Pattern }` を `Pattern` オルタナティブに追加。  
   - サンプル案: `bnf-match-or-pattern-ok.reml`（成功）、`bnf-match-or-pattern-unreachable.reml`（先行アームにより後続が到達不能）。  
   - 診断キー案: `pattern.unreachable_arm`（到達不能）、`pattern.exhaustiveness.missing`（網羅性不足）。

2. **Slice Patterns（高優先）**  
   - 構文: `[p1, p2, ..rest, p3]` のように、先頭/末尾の固定要素と中間の可変長部分（`..` または `..ident`）を記述可能にする。
   - BNF 追記（案）: `SlicePattern ::= "[" SliceElem { "," SliceElem } [","] "]"` / `SliceElem ::= Pattern | ".." [Ident]`。  
     （※正確な定義は `1-5` 策定時に「`..` は1回のみ出現可」などの制約を加える）
   - サンプル案: `bnf-match-slice-head-tail-ok.reml`（`[head, ..tail]` 成功）、`bnf-match-slice-middle-rest.reml`（`[1, ..mid, 9]`）。  
   - 診断キー案: `pattern.slice.type_mismatch`, `pattern.slice.multiple_rest`（`..` が複数回出現）, `pattern.exhaustiveness.missing`。

3. **Range Patterns（高優先）**  
   - BNF 追記: `RangePattern ::= RangeBound ".." RangeBound ["="]`（`..=` を閉区間として明示）。`RangeBound ::= Literal | Ident | ConstructorPattern`。  
   - サンプル案: `bnf-match-range-inclusive-ok.reml`（`1..=10` 成功）、`bnf-match-range-bound-inverted.reml`（下限>上限）。  
   - 診断キー案: `pattern.range.type_mismatch`, `pattern.range.bound_inverted`, `pattern.exhaustiveness.missing`。

4. **Binding Operator（中優先）**  
   - BNF 追記: `BindingPattern ::= Ident "@" Pattern | Pattern "as" Ident` を `Pattern` に追加し、優先順位表で `as`/`@` の結合順を明示。  
   - サンプル案: `bnf-match-binding-as-ok.reml`（`pat as name`）、`bnf-match-binding-at-duplicate.reml`（`as` と `@` 併用で重複）。  
   - 診断キー案: `pattern.binding.duplicate_name`。

5. **Regex パターン糖衣（中優先）**  
   - 構文案: `r"^\\d+" as digits -> ...`。これは **全体マッチ (Whole Match)** と **検証 (Validation)** に特化した糖衣構文とする。  
     ※キャプチャグループの個別の取り出し（`year`, `month` 等）が必要な場合は、Active Pattern `(|Regex|_|) "..." (y, m)` の使用を推奨する。
   - BNF 追記: `RegexPattern ::= "r\"" RegexBody "\""` を `Pattern` に追加。文字列/バイト列限定。  
   - サンプル案: `bnf-match-regex-ok.reml`（数値文字列抽出）、`bnf-match-regex-unsupported-target.reml`。  
   - 診断キー案: `pattern.regex.invalid_syntax`, `pattern.regex.unsupported_target`。

6. **Active Pattern 呼び出しとの統合（優先度: Or/Slice/Range 確定後に併走）**  
   - BNF 追記: `ActivePatternApp ::= "(|" Ident "|)" Pattern?` を `Pattern`/`Primary` に追加し、Or/Slice/Range より高い/低いどちらの優先度にするかを表で明示。  
   - サンプル案: `bnf-match-active-or-combined.reml`（Active と Or/Slice 併用）、`bnf-match-active-effect-violation.reml`（@pure で副作用）。  
   - 診断キー案: `pattern.active.return_contract_invalid`, `pattern.active.effect_violation`, 併用時は上記各パターン診断と組み合わせ。

### ガード/エイリアス方針（仕様側で確定）
- ガード記法は **`when` を正規形** とし、過去互換のため `if` を受理する場合は「非推奨エイリアス」と明記する。本文・例示は `when` へ統一。BNF は `MatchGuard ::= "when" Expr`（実装上 `if` も許可する場合は脚注で記載）。
- `MatchGuard` と `MatchAlias` の順序は **順不同許容** を仕様に明記し、推奨形は `when` → `as`。BNF は `MatchArmTail ::= MatchGuard? MatchAlias? | MatchAlias? MatchGuard?` とする。

### 実装チーム向け共有メモ（短文）
- 解析器は `when` を正規形としつつ `if` を警告付きで許容（警告キー案: `pattern.guard.if_deprecated`）。将来は `when` のみに絞る前提でフェーズアウト計画を検討。
- `MatchGuard`/`MatchAlias` は両順序を受理し、出力 AST では guard→alias の順で正規化する。既存テストは guard-only/alias-only/併用両順を追加して回帰防止。

## 作業ステップ
1. **BNF 拡張パッチ草案**: Or/Slice/Range/Binding/Regex/Active 呼び出しの非終端を `docs/spec/1-5-formal-grammar-bnf.md` に追加するドラフトを作成し、優先順位表で結合順を明記（特に Or vs Active の優先度を決定）。
2. **本文サンプル追加案**: `docs/spec/1-1-syntax.md` C.3/C.4 に各機能の短い使用例を追記する差分案を用意し、ガードは `when` に統一。`as`/`@` 併用例も含める。
3. **サンプルファイル設計**: `examples/spec_core/chapter1/match_expr/` へ追加する `.reml` を優先順にリスト化（成功/失敗を明記）し、`phase4-scenario-matrix.csv` の `diagnostic_keys` を暫定登録する表を作る。
4. **診断キー定義案**: `pattern.exhaustiveness.missing` など既出キー案を `2-5-error.md` のフォーマットで文面化し、Range/Slice/Regex/Binding ごとに短文メッセージを準備。
5. **互換性・フェーズアウト方針明記**: `if` ガード許容を警告付きで残す期間と、順序順不同受理の理由を脚注にまとめ、`docs/plans/pattern-matching-improvement/README.md` からも参照できるよう短文で転載する。
