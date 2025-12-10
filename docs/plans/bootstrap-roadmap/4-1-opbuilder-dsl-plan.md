# 4.1 OpBuilder DSL 復元計画

## 背景と決定事項

- Chapter 2 §2.4 では `OpBuilder` DSL（`builder.level(5, :infix_left, ["+"])` 形式）を用いた演算子宣言を紹介しているが、Phase 4 の Rust フロントエンドでは Parser/Typeck から当該 DSL が外れており `CH2-OP-401` が常に `parser.syntax.expected_tokens` で失敗している。
- `docs/spec/2-4-op-builder.md` の本文は `precedence(...).level(|lvl| ...)` API の記法しか残っていないため、仕様と `examples/spec_core/chapter2/op_builder/*.reml`（DSLベース）の間に乖離が発生している。
- フェーズFの判断として「OpBuilder DSL を継続サポートする」方針を確定したため、仕様本文・BNF・Glossary・サンプル・実装を含む回収計画を別途策定する。

## 目的

1. `docs/spec/2-4-op-builder.md` を DSL 記法（`:infix_left` 等）と `precedence` API の二系統に対応する形へ拡張し、Reader/Implementer がどちらの語彙でも読み解けるようにする。
2. Rust フロントエンド Parser/Typeck/Runtime に DSL 構文を復元し、`core.parse.opbuilder.level_conflict` などの診断を CLI とテストの両方で再現できる状態に戻す。
3. `examples/spec_core/chapter2/op_builder/**`・`expected/**`・`phase4-scenario-matrix.csv` を DSL 仕様に合わせて更新し、フェーズF チェックリストを完遂する。
4. 今後の Self-host/プラグイン/他実装へも方針を共有できるよう、`docs/notes` / `docs/plans` にハンドオーバー情報を残す。

## スコープ

- **含む**: `docs/spec/2-4-op-builder.md`・`docs/spec/1-5-formal-grammar-bnf.md`・`docs/spec/0-2-glossary.md` の改訂、`compiler/rust/frontend/src/{lexer,parser,typeck}` の DSL 復元、`compiler/rust/runtime` の API 再接続、`examples/` / `expected/` / `phase4-scenario-matrix.csv` / `reports/spec-audit/ch4/` の同期、CI/テスト整備。
- **含まない**: OCaml 実装の即時改修、`precedence` API の破棄、Chapter 2 以外の DSL（Conductor など）へ波及する仕様変更。必要があれば別計画 (`4-2` 以降) で扱う。

## 成果物

- OpBuilder DSL の正式仕様（サンプル/BNF/テンプレートコード、`docs/spec/2-4`）。
- Rust Parser/Typeck/Runtime が DSL の AST を生成し、`core.parse.opbuilder.level_conflict` / `core.parse.opbuilder.fixity_missing` などの診断を返せる実装。
- `examples/spec_core/chapter2/op_builder/core-opbuilder-level-conflict-error.reml` など DSL を用いた `.reml` と `expected/` ゴールデンの整備、`phase4-scenario-matrix.csv` の `CH2-OP-401` を `ok` へ更新。
- `reports/spec-audit/ch4/spec-core-dashboard.md` で DSL シナリオが `pass` になったログ、および `docs/notes/examples-regression-log.md` への記録。

## 作業ステップ

### フェーズA: 仕様ドキュメントの再整備

1. `docs/spec/2-4-op-builder.md`
   - `A-2. レベル宣言` に DSL 版の構文例（`:infix_left` 等）と `precedence` API 版の対照表を追加。
   - 図解/コード例を DSL 控えめ→ DSL + API 並列表記に再構成。
   - `F. エラー設計` に DSL 特有の診断（`core.parse.opbuilder.level_conflict` / `fixity_missing` / `duplicate_operator` 等）の根拠文を追加。
2. `docs/spec/1-5-formal-grammar-bnf.md`
   - `op_builder_decl` などの規則を新設し、`:infix_left` / `:infix_right` / `:infix_nonassoc` / `:prefix` / `:postfix` / `:ternary` のトークン定義を追加。
3. `docs/spec/0-2-glossary.md`
   - 「OpBuilder DSL」「fixity symbol」「Level Builder」などの語彙を登録し、`precedence` API との差異を説明。
4. `docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md`
   - フェーズF チェックリスト／背景節に DSL 継続方針と仕様改訂タスクへのリンクを追記。

### フェーズB: Parser/Lexer 実装

1. Lexer (`compiler/rust/frontend/src/parser/lexer.rs` など) に `:infix_left` 等のキーワードを追加し、`TokenKind::FixitySymbol` を新設。
2. Parser (`src/parser/mod.rs`)
   - `OpBuilderDecl` ノードを追加し、`builder.level(<int>, :fixity, ["+","-"])` の呼び出しを AST へ変換。
   - `ExpectedTokenCollector` に fixity シンボルを登録し、構文エラー時の候補に含める。
   - `UseDecl`/`Expr` ヘルパーと競合しないよう `:` の扱いを調整。
3. `compiler/rust/frontend/src/parser/ast.rs`
   - `FixityKind` enum (`InfixLeft`/`InfixRight`/`InfixNonAssoc`/`Prefix`/`Postfix`/`Ternary`) と `OpLevelDecl` 構造体を追加。
4. 単体テスト
   - `compiler/rust/frontend/tests/spec_core/op_builder.rs`（新規）で `builder.level` 呼び出しが AST に反映されるか、fixity ごとに `assert_ast!` する。
   - `spec_core::ch2_op_401_reports_level_conflict` など CLI 互換テストを追加。

### フェーズC: Typeck/Runtime/診断

1. Typeck (`compiler/rust/frontend/src/typeck/op_builder.rs` 仮)
   - `FixityKind` を `TypeckOpBuilder` に伝播し、レベルごとの fixity を検証。
   - 同一レベルに異なる fixity が混在した場合に `core.parse.opbuilder.level_conflict` を生成。
   - Fixity が存在しないまま `.build()` された際に `core.parse.opbuilder.fixity_missing` を返す。
2. Runtime (`compiler/rust/runtime/src/parse/op_builder.rs` 等)
   - DSL 記法用の builder API を実装し、`precedence` API への薄いラッパーとして動作させる。将来的に DSL を `reml_runtime` に輸出できるよう `cfg(feature = "dsl_opbuilder")` で切り替えを想定。
3. Diagnostics
   - `docs/spec/3-6-core-diagnostics-audit.md` に `core.parse.opbuilder.*` を追加し、`expected/spec_core/chapter2/op_builder/*.diagnostic.json` と整合を取る。

### フェーズD: 資産・テスト・レポート更新

1. `examples/spec_core/chapter2/op_builder`
   - `core-opbuilder-level-conflict-error.reml` を DSL 仕様に合わせて整備し、`expected/...diagnostic.json` を再取得。
   - 必要に応じて成功例 (`core-opbuilder-level-ok.reml`) / 追加エラー例（fixity missing）を追加。
2. `phase4-scenario-matrix.csv`
   - `CH2-OP-401` の `resolution` を `pending → ok` へ更新（完了時）。
   - `diagnostic_keys` / `scenario_notes` に DSL 仕様の anchor (`docs/spec/2-4-op-builder.md§A-2`) を追記。
3. `reports/spec-audit/ch4/spec-core-dashboard.md`
   - フェーズF ログに DSL シナリオの pass/diagnostics を記録。
4. `docs/notes/examples-regression-log.md`
   - OpBuilder DSL 復元の判断・CLI コマンド・ログ ID を追記。

### フェーズE: ハンドオーバーと継続運用

1. `docs/plans/rust-migration/1-3-dual-write-runbook.md` に OpBuilder DSL チェック項目を追加。
2. `docs/plans/bootstrap-roadmap/4-4-field-regression-and-readiness-plan.md` に OpBuilder フォローアップ（Self-host で DSL を用いたパイプライン作成）を記載。
3. `docs/notes/opbuilder-dsl-decisions.md`（新規）で仕様変更理由、今後の拡張（`foldl`/`foldr` 以外の fixity など）をメモ。

## タイムライン（目安）

| 週 | タスク |
| --- | --- |
| 72 週 | フェーズA 仕様追記（docs/spec 更新・レビュー申請） |
| 73 週 | フェーズB Parser/Lexer 実装 + 単体テスト |
| 74 週 | フェーズC Typeck/Runtime/診断復元 |
| 75 週 | フェーズD 資産更新（examples/expected/matrix/logs） |
| 76 週 | フェーズE ハンドオーバー & PhaseF チェック完了報告 |

## リスクと緩和策

| リスク | 影響 | 緩和策 |
| --- | --- | --- |
| DSL と `precedence` API の文書が重複し読者が混乱 | 仕様理解に時間がかかる | §2.4 に比較表と移行ガイドを載せ、`precedence` API を内部実装・DSL を宣言的ラッパと位置付ける |
| Parser/Typeck 実装が複雑化 | 回帰（`parser.syntax.expected_tokens` など）が再発 | `spec_core::op_builder` テストを追加し、固定文字列/AST をスナップショットで保護。CI に `cargo test -p reml_frontend spec_core::op_builder_*` を追加 |
| 他プラットフォーム（OCaml 等）との乖離 | ドキュメントと実装がずれる | `docs/plans/rust-migration/1-3-dual-write-runbook.md` に DSL チェックを組み込み、OCaml 側へも TODO を起票 |
| DSL の将来拡張が塞がる | Stage 5 以降の拡張余地が減少 | `docs/notes/opbuilder-dsl-decisions.md` で設計判断を管理し、必要に応じて `spec_fix` として Phase5 backlog へ送る |

## 参照

- `docs/spec/2-4-op-builder.md`
- `docs/spec/1-5-formal-grammar-bnf.md`
- `docs/spec/0-2-glossary.md`
- `docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md`
- `examples/spec_core/chapter2/op_builder/core-opbuilder-level-conflict-error.reml`
- `phase4-scenario-matrix.csv` (`CH2-OP-401`)
- `reports/spec-audit/ch4/logs/spec_core-20251210T081000Z.md`

