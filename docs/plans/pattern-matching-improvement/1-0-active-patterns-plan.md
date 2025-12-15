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
  - **Partial Active Pattern (部分パターン)**: `Option<T>` を返す。`Some(v)` でマッチ成功、`None` で失敗（次のアームへ）。名前は `(|Name|_|)` のように `|_|` を含む。
  - **Total Active Pattern (完全パターン)**: `T` を返す。**常にマッチ**し、網羅性検査では「成功」として扱われる。名前は `(|Name|)` のように `|_|` を含まない。
  - **Fallible Active Pattern (要検討)**: `Result<T, E>` を返す場合、`Err` を「マッチ失敗」とみなすか「実行時エラー（例外）」として伝播させるかの設計が必要。Phase A では `Option` (失敗) と `T` (成功) を基本とし、`Result` は原則非サポート（または `Option` への変換を要求）としてリスクを低減する。
- 副作用: `@pure` 関数内で使用する場合の制約を `docs/spec/1-3-effects-safety.md` と照合。副作用（I/O等）を伴う Active Pattern は、網羅性検査の前に「効果の発生順序」が確定している必要がある。

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

## 仕様差分ドラフトパッチ（準備用スケッチ）
以下は実装前にレビュー用として `docs/spec/1-1-syntax.md` / `1-5-formal-grammar-bnf.md` / `2-5-error.md` に適用を検討する短縮パッチ案。

### 1-1-syntax.md への追記案（抜粋）
```
@@ C.3 パターン（束縛・`match` で共通）
* アクティブパターン：`(|Name|_|)` / `(|Name|)` で定義された分解ロジックを `match` で使用できる。
  - 使用例：`match input with | (|IntLit|_|) n -> n | _ -> 0`
  - ガード/エイリアスとの併用可。評価順は「パターン一致 → ガード → エイリアス」。

@@ C.4 `match` 式
* 例を追加：
  | (|HexInt|_|) n when n > 0xFF -> "large"
  | Some(x) | None as v          -> ...
  | [head, ..tail]               -> ...
  | 1..=10                       -> ...
```

### 1-5-formal-grammar-bnf.md への追記案（抜粋）
```
@@ 4. 式
Primary         ::= ... | ActivePatternApp | ...
MatchArm        ::= "|" Pattern MatchArmTail "->" Expr
MatchArmTail    ::= MatchGuard? MatchAlias? | MatchAlias? MatchGuard?
MatchGuard      ::= ("if" | "when") Expr
MatchAlias      ::= "as" Ident
ActivePatternApp ::= "(|" Ident "|)" Pattern?

@@ 5. パターン
Pattern         ::= "_"
                  | Ident
                  | OrPattern
                  | TuplePattern
                  | RecordPattern
                  | ConstructorPattern
                  | SlicePattern
                  | RangePattern
                  | BindingPattern
                  | RegexPattern
                  | ActivePatternApp

OrPattern       ::= Pattern "|" Pattern { "|" Pattern }
SlicePattern    ::= "[" Pattern? ".." Pattern? "]"
RangePattern    ::= RangeBound ".." RangeBound ["="]
BindingPattern  ::= Ident "@" Pattern | Pattern "as" Ident
RegexPattern    ::= "r\"" RegexBody "\""
RangeBound      ::= Literal | Ident | ConstructorPattern
```

### 2-5-error.md への診断キー追加案（抜粋）
```
- pattern.active.return_contract_invalid : Active Pattern の戻り値が Option/Result 以外
- pattern.active.effect_violation        : @pure 文脈で副作用を持つ Active Pattern を使用
- pattern.exhaustiveness.missing         : 網羅性未達（Or/Slice/Range/Active 追加後も共通）
- pattern.unreachable_arm                : 先行アームにより到達不能
- pattern.range.type_mismatch            : Range 境界の型が一致しない
- pattern.range.bound_inverted           : 下限 > 上限
- pattern.slice.type_mismatch            : Slice パターンに非コレクションを適用
- pattern.slice.too_many_parts           : Slice パターンの `..` が複数など不正形
- pattern.regex.invalid_syntax           : Regex リテラル糖衣の構文エラー
- pattern.regex.unsupported_target       : 対象型が文字列/バイト列でない
- pattern.binding.duplicate_name         : `as` / `@` 併用で同一名を重複束縛
```

### ガード/エイリアス統一方針（Active 併用時の扱い）
- ガードは正規形として `when` を使用し、互換目的で `if` を許容する場合は警告付きエイリアスとする（警告キー案: `pattern.guard.if_deprecated`）。例示・仕様本文は `when` へ統一。
- `MatchGuard` と `MatchAlias` の順序は順不同を受理し、AST 正規化は guard→alias の順で固定する。BNF では `MatchArmTail ::= MatchGuard? MatchAlias? | MatchAlias? MatchGuard?` を採用予定。
- Active Pattern の例示では `(|Name|_|) x when cond as v -> ...` 形式を推奨形として示し、順不同許容に関する注記を `1-5` 側に併記する。

## 次の具体作業ステップ
1. **仕様パッチ作成（下書き）**: 上記ドラフトパッチ案を実際の差分として `docs/spec/1-1-syntax.md` / `1-5-formal-grammar-bnf.md` / `2-5-error.md` に適用する下書きを用意し、`when` 正規形・順不同ガード/エイリアスを明文化。
2. **型/効果セマンティクス追記案**: `1-2-types-Inference.md` と `1-3-effects-safety.md` へ Option/T 返り値の推論規則・@pure での副作用制約を追加する文案を作成し、`Result` 非推奨方針を脚注で明示。
3. **サンプル設計**: `examples/spec_core/chapter1/match_expr/` 用に部分/完全 Active Pattern の成功・失敗・ガード併用の 3 本をプロットし、対応する `phase4-scenario-matrix.csv` 行（`diagnostic_keys`）をドラフト登録。
4. **診断メッセージ雛形**: `pattern.active.return_contract_invalid` / `pattern.active.effect_violation` の短文メッセージ案を作成し、既存診断スタイルと揃える（コード・タイトル・説明の3要素）。
5. **実装連携メモ更新**: Rust/OCaml パーサが `if` ガードを警告付きで受理し、AST 正規化を guard→alias とする実装指針を短文で `docs/plans/pattern-matching-improvement/README.md` に転載（共有しやすくするため）。
