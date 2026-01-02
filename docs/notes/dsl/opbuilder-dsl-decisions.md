# OpBuilder DSL 決定ログ

## 背景
- Phase 4 で `precedence(...).level(|lvl| ...)` API へ統一する案が浮上したが、Chapter 2 の DSL 記法（`builder.level(5, :infix_left, ["+"])`）を使ったサンプル・診断資産（`CH2-OP-401` など）が多数存在し、Reader/Implementer 双方に影響が大きい。
- `docs/spec/2-4-op-builder.md` から DSL 断片が薄れた結果、仕様と `examples/spec_core/chapter2/op_builder/*.reml` の乖離が発生し、Rust フロントエンドの `parser.syntax.expected_tokens` が `core.parse.opbuilder.level_conflict` を返せない状態になった。
- フェーズ F の判断として「OpBuilder DSL を継続サポートする」方針を採用し、DSL と `precedence` API を併記する形で仕様・実装・資産を整合させることを決定。

## 決定事項（要約）
- DSL 記法を正式仕様として復元し、`docs/spec/2-4-op-builder.md` に DSL/API 並列表と診断根拠を追加する（`fixity_missing`、`level_conflict`、`duplicate_operator` を含む）。
- Rust フロントエンドでは Lexer/Parser/Typeck/Runtime に fixity シンボル（`:infix_left` ほか）を追加し、`examples/spec_core/chapter2/op_builder` と `expected/` の診断を一致させる。`phase4-scenario-matrix.csv` の `CH2-OP-401` を `ok` で閉じる。
- Dual-write 運用では `docs/plans/rust-migration/1-3-dual-write-runbook.md` の OpBuilder チェックを通じて `diagnostics.*` と `expected_tokens.*` を比較し、差異が出た場合は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` へ即時登録する。
- Self-host/Phase 5 以降に備え、`docs/plans/bootstrap-roadmap/4-4-field-regression-and-readiness-plan.md` で DSL パイプラインを組み込み、実行結果を `phase4-readiness.md` のハンドオーバー材料として扱う。

## 拡張検討メモ
- **追加 fixity**: 将来 `foldl`/`foldr` 相当の畳み込み用 fixity や `ternary` バリアント拡張を検討する場合、既存の `FixityKind` enum と BNF を破壊しない別ラベルを導入する。実装フラグ `dsl_opbuilder` の背後で試験運用し、仕様への昇格は Phase 5/6 の RFC プロセスで判断。
- **API 表記**: `precedence` API と DSL を双方向にマッピングするサンプル（DSL→API、API→DSL）を `docs/spec/2-4-op-builder.md` に併記し、どちらを採用しても診断キーとエラーメッセージが一致することを明示する。
- **診断スナップショット**: `expected/spec_core/chapter2/op_builder/*.diagnostic.json` を DSL 版で再取得し、`reports/spec-audit/ch5/spec-core-dashboard.md` の Pass/Fail と Run ID を紐付ける。差分が発生した場合は `docs/notes/process/examples-regression-log.md` へ記録し、再現手順を残す。

## ハンドオーバー時の確認チェック
- `docs/spec/2-4-op-builder.md` へ DSL 仕様（構文・BNF・診断根拠）が反映済みか。
- `phase4-scenario-matrix.csv` `CH2-OP-401` の `resolution=ok` と `diagnostic_keys=core.parse.opbuilder.*` が一致しているか。
- `reports/spec-audit/ch5/spec-core-dashboard.md` に最新の Run ID と Pass/Fail が記録され、Dual-write の `summary.json` と整合しているか。
- Self-host パイプライン（`4-4-field-regression-and-readiness-plan.md` の節）で DSL ケースが実行され、`phase4-readiness.md` に結果が転記されているか。

## 参照
- `docs/plans/bootstrap-roadmap/4-1-opbuilder-dsl-plan.md`
- `docs/plans/rust-migration/1-3-dual-write-runbook.md`
- `docs/plans/bootstrap-roadmap/4-4-field-regression-and-readiness-plan.md`
- `docs/spec/2-4-op-builder.md`
- `phase4-scenario-matrix.csv` (`CH2-OP-401`)
