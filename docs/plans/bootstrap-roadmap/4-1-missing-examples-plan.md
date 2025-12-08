# 4.1 Phase 4 欠落シナリオ補完計画

## 1. 目的

`docs/plans/bootstrap-roadmap/4-1-scenario-matrix-plan.md` および `4-0-phase4-migration.md` の要件である「Chapter 1 全構文規則に対する正例/境界例/負例の網羅」を達成するため、現状の `examples/spec_core/chapter1` に不足しているシナリオを特定し、実装する。

現状の調査により、`let`, `match`, `conductor` 等の代表的な構文は正例が存在するが、以下の領域で「4パターン（正例・境界例・ギリギリエラー・明確なエラー）」が不足していることが判明した。

## 2. 不足しているカテゴリと対応方針

### 2.1 制御構文 (`If`, `Loop`, `While`, `For`)
現状: `examples/spec_core/chapter1` 直下にディレクトリが存在しない。
対応: `examples/spec_core/chapter1/control_flow/` を新設し、以下のパターンを追加する。

| BNF Rule | 正例 (OK) | 境界例/負例 (Error) |
|---|---|---|
| `IfExpr` | `bnf-ifexpr-blocks-ok.reml` (if-then-else) | `bnf-ifexpr-missing-else-type-mismatch.reml` (elseなしでUnit以外を返す等) |
| `LoopExpr` | `bnf-loopexpr-break-value-ok.reml` | `bnf-loopexpr-unreachable-code.reml` (明確なエラー診断) |
| `WhileExpr` | `bnf-whileexpr-condition-bool-ok.reml` | `bnf-whileexpr-condition-type-error.reml` (非Bool条件) |
| `ForExpr` | `bnf-forexpr-iterator-pattern-ok.reml` | `bnf-forexpr-iterator-invalid-type.reml` |

### 2.2 リテラルと詳細宣言 (`Type`, `Fn`)
現状: `type_decl` は `sum-record` のみ。`fn_decl` は `generic` のみ。リテラル専用のテストがない。
対応: `literals/`, `type_decl/`, `fn_decl/` を拡充する。

| BNF Rule | 追加シナリオ |
|---|---|
| `Literal` | `bnf-literal-int-boundary-max.reml` (i64 max), `bnf-literal-float-forms.reml` (指数), `bnf-literal-string-raw-multiline.reml` |
| `TypeDecl` | `bnf-typedecl-alias-generic-ok.reml` (type Alias<T> = ...), `bnf-typedecl-new-struct-ok.reml` (new struct) |
| `FnDecl` | `bnf-fndecl-no-args-ok.reml`, `bnf-fndecl-return-inference-error.reml` (推論不一致) |
| `Lambda` | `bnf-lambda-closure-capture-ok.reml`, `bnf-lambda-arg-pattern.reml` |

### 2.3 明示的な構文エラー (Negative Syntax Tests)
現状: `parser.syntax.expected_tokens` を発生させる意図的な構文エラーが少ない。
対応: 各カテゴリに `*-syntax-error.reml` を追加する。

- `let` : `bnf-valdecl-missing-initializer-error.reml`
- `match`: `bnf-matchexpr-missing-arrow-error.reml`
- `block`: `bnf-block-unclosed-brace-error.reml`

## 3. 作業項目

1. **ディレクトリ作成**:
   - `examples/spec_core/chapter1/control_flow`
   - `examples/spec_core/chapter1/literals`
   - `examples/spec_core/chapter1/lambda` (または `fn_decl` に統合)

2. **`.reml` ファイル作成**:
   - 上記表に基づく約 15〜20 ファイルの作成。
   - 各ファイルは `docs/spec/1-5-formal-grammar-bnf.md` の対応する Rule ID を意識した命名にする。

3. **Scenario Matrix への登録**:
   - 本計画完了後、`phase4-scenario-matrix.csv`（別タスク管理）へ ID を発行して登録する。

## 4. 完了条件

- `docs/spec/1-5-formal-grammar-bnf.md` の主要な非終端記号（Section 2 Declarations, Section 3 Statements, Section 4 Expressions）のすべてに対して、最低 1 つの `.reml` ファイルが `examples/spec_core` に存在すること。
- 特に `If`, `Loop`, `While`, `Literal` の欠落が解消されていること。
