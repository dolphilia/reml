# 1.0 Active Patterns 導入計画

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

## タスク

1. **構文/BNF 差分策定**  
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

## 成果物（出口条件）

- 上記タスクのアウトラインを各仕様ファイルへの修正ポイント付きで提示できている。
- 最低 2 本の例題案（成功/失敗）が決まり、診断キーと期待挙動が文章で示されている。
- Phase 4 回帰計画（`docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md`）に対し、影響範囲と併走方針をコメントで共有できる。

## 未解決事項 / TODO

- `Result` ベースの Active Pattern を正式に許容するかは網羅性計算コストを評価して決定する。
- LSP での補完候補提示（`(|Name|_|)`）の表示ポリシーは別ガイド（`docs/guides/core-parse-streaming.md` 付録を想定）に委ねる。

## 仕様差分メモ（1-1 / 1-5 に対する追記箇所）

- `docs/spec/1-1-syntax.md` は C.3/C.4 に Active Pattern の説明が存在しない。定義記法と `match` 内での使用例を追加する差分が必要。
- `docs/spec/1-5-formal-grammar-bnf.md` には Active Pattern の生成規則が無い。以下を追加する方針を計画タスクに組み込む。  
  - `ActivePatternDecl ::= "pattern" "(|" Ident ("|_|")? "|)" "(" ParamList? ")" "=" Expr`（呼び出し規約と戻り値契約を注記）  
  - `ActivePatternApp  ::= "(|" Ident "|)" Pattern?` を `Pattern` もしくは `Primary` に統合（優先順位を表で明記）  
  - `MatchArm` のガード/エイリアス順は `MatchGuard? MatchAlias?` 固定だが Phase 4 実装は順不同を許容しているため、どちらに揃えるか決定する必要あり。
- 診断キーは仕様本文に未登場。最低限 `pattern.active.return_contract_invalid`（Option/Result 以外の戻り値）、`pattern.active.effect_violation`（@pure で副作用を持つ場合）を `2-5-error.md` か `1-1` 診断節に追加する差分を要検討。

## 仕様差分パッチ（準備用スケッチ）

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

## 実行前の下準備

1. **仕様パッチ作成（下書き）**: 上記パッチ案を実際の差分として `docs/spec/1-1-syntax.md` / `1-5-formal-grammar-bnf.md` / `2-5-error.md` に適用する下書きを用意し、`when` 正規形・順不同ガード/エイリアスを明文化。
2. **型/効果セマンティクス追記案**: `1-2-types-Inference.md` と `1-3-effects-safety.md` へ Option/T 返り値の推論規則・@pure での副作用制約を追加する文案を作成し、`Result` 非推奨方針を脚注で明示。
3. **サンプル設計**: `examples/spec_core/chapter1/match_expr/` 用に部分/完全 Active Pattern の成功・失敗・ガード併用の 3 本をプロットし、対応する `phase4-scenario-matrix.csv` 行（`diagnostic_keys`）を登録。
4. **診断メッセージ雛形**: `pattern.active.return_contract_invalid` / `pattern.active.effect_violation` の短文メッセージ案を作成し、既存診断スタイルと揃える（コード・タイトル・説明の3要素）。
5. **実装連携メモ更新**: Rust/OCaml パーサが `if` ガードを警告付きで受理し、AST 正規化を guard→alias とする実装指針を短文で `docs/plans/pattern-matching-improvement/README.md` に転載（共有しやすくするため）。

## Rust実装Remlコンパイラのパターンマッチ実装を強化するための具体的な作業ステップ
1. **構文パーサ拡張（frontend/parser）**  
   - `(|Name|_|)` / `(|Name|)` 定義と呼び出しをパーサに追加し、`MatchGuard`/`MatchAlias` の順不同受理と `if` ガード警告（`pattern.guard.if_deprecated`）を実装する。  
   - BNF 差分（`ActivePatternDecl`/`ActivePatternApp`）を parser テーブル・テストに反映し、既存 `match` サンプルがレグレッションしないことを確認する。
   - **進捗**: Rust Parser/Lexer へ Active Pattern 定義・適用を追加し、`when` 正規形 + `if` 非推奨警告を実装済み。`match` ガード/エイリアス順不同受理も導入し、`spec_core` テストを追加（`bnf-activepattern-partial-ok` ほか）して受理を確認。  
     残件: BNF 表への同期・`expected/` ゴールデンの更新は次ステップで実施。
2. **AST/HIR 拡張と IR 正規化**  
   - Active Pattern 定義ノード（部分/完全の区別を持つ）と適用ノードを AST/HIR に追加し、ガード→エイリアス順で正規化する共通パスを整備する。  
   - パターン内の Active 呼び出しと通常関数呼び出しの混同を避けるタグ付けを行い、IR 生成で Option/値返却の分岐を明示する。
   - **進捗**: AST に ActivePatternDecl/PatternKind::ActivePattern を追加し、MatchArm に `guard_used_if` を保持。ガード→エイリアス順で正規化するパーサ実装を導入済み。  
     残件: HIR/IR 伝播と Option/値の戻り値分岐の明示化は未着手。
3. **型・効果検査の実装（typeck/effects）**  
   - 戻り値契約: 部分パターンは `Option<T>`、完全パターンは `T` のみ許容し、`Result`/その他は `pattern.active.return_contract_invalid` で失敗させる。  
   - `@pure` 文脈で副作用を持つ Active Pattern を検出し `pattern.active.effect_violation` を発火、効果タグ伝播を既存 `perform` チェックと共有する。  
   - パターンバインディングの型付け（`(|Name|_|) x` の束縛型推論）を既存 Binding/Or/Slice のロジックに組み込む。
   - **進捗**: TypecheckDriver に戻り値契約検証と @pure 時の副作用検出を実装し、`pattern.active.return_contract_invalid` / `pattern.active.effect_violation` を発火させる経路を追加。パターン束縛の環境挿入は従来どおり。効果タグの IR 連携は未着手。
4. **網羅性・到達不能解析の拡張（exhaustiveness pass）**  
   - 部分 Active Pattern を「失敗し得るパターン」として扱い、網羅性不足は `pattern.exhaustiveness.missing`、重複は `pattern.unreachable_arm` で報告する。  
   - 完全 Active Pattern は常時成功パスとして扱い、Range/Slice/Or と併用した場合のカバレッジ計算を回帰テストで固定する。
   - **進捗**: TypecheckDriver に簡易カバレッジ判定を追加し、総称パターン（`_` / 変数 / 完全 Active Pattern）以降を `pattern.unreachable_arm` で報告、欠落時に `pattern.exhaustiveness.missing` を発火。専用の網羅性パスおよび複合パターン対応は今後実施。
5. **診断メッセージとキーの統合（diagnostics crate）**  
   - `pattern.active.return_contract_invalid` / `pattern.active.effect_violation` を診断レジストリに追加し、コード・タイトル・短文説明を既存パターン系メッセージと揃える。  
   - `pattern.guard.if_deprecated` を警告レベルで登録し、将来のフェーズアウト方針（when 正規形）をメッセージ内に明示する。
   - **進捗**: `pattern.guard.if_deprecated` の警告発火は継続。TypecheckDriver から `pattern.active.return_contract_invalid` / `pattern.active.effect_violation` / `pattern.exhaustiveness.missing` / `pattern.unreachable_arm` を生成し、CLI で JSON 出力を確認済み。diagnostics crate（共通レジストリ）への文面登録は未対応。
6. **サンプル・E2E テスト連携**  
   - `examples/spec_core/chapter1/match_expr/` に Active Pattern 成功/失敗サンプルを追加し、`tooling/examples/run_examples.sh --suite spec_core` で実行する期待結果 (`expected/` と `reports/spec-audit/ch4`) を更新。  
   - `compiler/rust/tests`（もしくは `frontend/tests`）で AST 正規化・網羅性診断・効果違反のユニット/スナップショットテストを追加し、`phase4-scenario-matrix.csv` の該当行に `diagnostic_keys` を登録する。  
   - **進捗**: Typecheck 連携テストを追加（戻り値契約違反、@pure 副作用、網羅性欠落、到達不能を検証）。`expected/spec_core/chapter1/active_patterns/bnf-activepattern-return-contract-error.diagnostic.json` と `phase4-scenario-matrix.csv` の CH1-ACT-* 行を更新済み。その他サンプルの expected/ 再取得は未実施。
7. **移行・互換性ガード**  
   - 既存コードとの衝突を防ぐため、Active Pattern 名の予約衝突チェック（通常関数との重複時の警告方針）を実装し、ドキュメントの命名規則と同期させる。  
   - `docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md` と `rust-migration` 計画に着手タイミングを記録し、Phase4 回帰スイートでの確認手順を追記する。
   - **進捗**: 未着手（予約衝突チェック/計画書同期は今後実施）。

## 推奨順 (1〜5) 対応の具体方針

1. **予約衝突ガード（Active Pattern 名 vs 通常関数）**  
   - **ポリシー**: 同一モジュール内で `(|Name|_|)` / `(|Name|)` と `fn Name` が衝突した場合は **エラー** とする（警告では抑止できず曖昧さが残るため）。外部モジュールからの import による衝突は `use` 解決時に ModulePath へ正規化し、型検査側で重複シンボル診断（既存のシンボル重複キーを再利用）を発火させる。  
   - **検出ポイント**: パーサは Active Pattern 定義を `DeclKind::ActivePattern` で保持し、シンボルテーブル登録時に `fn` と同一名前空間へ挿入する。Typeck は関数テーブルへの insert 時に `ActivePatternKind` を確認し、既存関数・外部 import と衝突した場合にエラーを生成する。  
   - **命名ルール明示**: `docs/spec/1-1-syntax.md` へ「Active Pattern 名は通常関数と共有できない」旨を追記する前提で、Rust 実装は `pattern.active.name_conflict`（仮）を新設する場合でも既存の重複診断キーを再利用する。

2. **診断レジストリ統合（diagnostics crate 反映）**  
   - diagnostics crate へ登録する文面を以下で固定し、`docs/spec/2-5-error.md` のフォーマットに合わせてコード・タイトル・説明をそろえる。  
     - `pattern.active.return_contract_invalid`（Error, domain=typeck）: 「Active Pattern は `Option<T>`（部分）または `T`（完全）を返す必要があります。`Result` / それ以外の戻り値は許可されません。」  
     - `pattern.active.effect_violation`（Error, domain=effects）: 「`@pure` 文脈で副作用を含む Active Pattern を呼び出すことはできません。副作用を除去するか純粋な Active Pattern に置き換えてください。」  
     - `pattern.exhaustiveness.missing`（Warning 初期、将来 Error）: 「`match` の網羅性が不足しています。未処理のケースを追加してください。」  
     - `pattern.unreachable_arm`（Warning 初期）: 「先行パターンによりこのアームは到達不能です。順序を見直すか冗長なアームを削除してください。」  
   - Typeck/Parser から出力する際は上記 Severity を既定値とし、Phase4 のゲートで Error 昇格を選択できるよう `resolution_notes` に記載する。

3. **HIR/IR 伝播（Option/値タグ付け）**  
   - HIR に `ActivePatternKind::{Partial,Total}` と `ReturnCarrier::{OptionLike,Value}` を保持し、Typeck での戻り値検証結果を IR へ渡す。Partial は `Option<T>` を要求し、IR では `Some/None` の分岐を明示する。Total は `T` をそのまま束縛し、網羅性検査で「常に成功」扱いとする。  
   - IR 生成時は `ActivePatternBranch::{Matched,NotMatched}` を付け、実行時に `None` を検出した場合は即座に次アームへジャンプする分岐を生成する。`Result` など非許容の戻り値は Typeck 手前で拒否し、IR 側に例外パスを持ち込まない。

4. **ゴールデン/マトリクス更新（bnf-activepattern-*.reml）**  
   - 対象: `expected/spec_core/chapter1/active_patterns/*.stdout|*.diagnostic.json` と `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` の `CH1-ACT-00{1,2,3}` / `CH1-MATCH-018` 行。  
   - 手順: `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter1/active_patterns/bnf-activepattern-*.reml` を再実行し、戻り値契約・副作用違反の診断文面が上記レジストリ案と一致することを確認して expected を再取得。マトリクスの `resolution_notes` に再取得コマンドと `diagnostic_keys` 一致確認ログを記録する。

5. **網羅性精度向上（Guard/複合パターン）**  
   - Guard 付き完全パターンや Range/Slice/Or/Active 併用時のカバレッジ計算を専用パスへ分離し、`pattern.exhaustiveness.missing` の生成ロジックを Typeck 本体から切り出す。  
   - 優先順位: (a) Guard を含む完全 Active Pattern を「常に成功」集合へ折り込み、(b) Range/Slice/Or を束ねたカバレッジマージ関数を実装、(c) 上記を IR 分岐と整合させ、到達不能計算で `pattern.unreachable_arm` を確実に返す。
