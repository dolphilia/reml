# Phase4: Error Recovery Combinators 計画

## 背景と目的
- `docs/notes/dsl/dsl-enhancement-proposal.md` の「3.7 Error Recovery Combinators」を Phase 4 の実装計画へ落とし込む。
- `docs/spec/0-1-project-purpose.md` の「分かりやすいエラーメッセージ」を満たすため、DSL 作者が安全に回復ポイントを組める最小ヘルパを整備する。
- 既存の `recover`/糖衣（`recover_with_default` など）を前提に、**同期点指定・パニック回復・欠落トークン補挿**の体験を簡潔にする。

## 依存関係
- `docs/plans/bootstrap-roadmap/4-1-core-parse-error-recovery-impl-plan.md`（Core.Parse 回復基盤の実装）
- `docs/plans/bootstrap-roadmap/4-1-core-parse-lex-helpers-impl-plan.md`（`symbol`/`keyword` の同期点利用）

## スコープ
- **含む**: `sync_to` などの回復ヘルパ、パニックモード糖衣、欠落トークン補挿の利用指針、Rust 実装と単体テスト、Phase4 回帰シナリオ追加。
- **含まない**: 新しい回復モードの追加、LSP/Visualizer の拡張、復旧アルゴリズムの大規模刷新。

## 成果物
- 仕様:
  - `docs/spec/2-2-core-combinator.md` に回復ヘルパ（`sync_to`/`panic_until`/`panic_block`/`recover_missing`）を追記。
  - `docs/spec/2-5-error.md` に「同期点指定ヘルパ」「パニック回復の運用規約（action="skip"+context）」を追記。
- 実装:
  - `compiler/runtime/src/parse/combinator.rs` にヘルパ API を追加。
  - `compiler/runtime/tests/parse_combinator.rs` に回復ヘルパの単体テストを追加。
- 回帰:
  - `examples/spec_core/chapter2/parser_core/` に回復ヘルパのサンプルを追加。
  - `expected/spec_core/chapter2/parser_core/` に診断期待値を追加。
  - `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` に新シナリオを登録。

## 仕様ドラフト（最小構成）

```reml
let sync_stmt = sync_to(symbol(";"))

let stmt =
  rule("stmt", expr)
    |> recover_until(sync_stmt, ErrorNode)

let paren_expr =
  between(symbol("("), expr, symbol(")"))
    |> recover_missing(sync_to(lookahead(symbol(")"))), token=")", value=ErrorNode)

let block =
  between(symbol("{"), many(stmt), symbol("}"))
    |> panic_block(symbol("{"), symbol("}"), value=[])
```

### 最小契約（案）
- `sync_to` は **同期点まで読み飛ばし、同期点自身を消費する**ヘルパとする（無限ループ回避）。
- `panic_until`/`panic_block` は **`recover_until` の糖衣**として提供し、`extensions["recover"].action` は `"skip"` のまま、`context` に `"panic"` 等の説明を付与する。
- `recover_missing` は `recover_with_insert` の別名として、**欠落トークン補挿 + FixIt** を標準化する。

## 作業ステップ

### フェーズA: 仕様整理
1. `docs/spec/2-2-core-combinator.md` に回復ヘルパの API 追加と使用指針を追記する。（着手済み）
2. `docs/spec/2-5-error.md` の E-1/E-2 に `sync_to` とパニック回復の運用例を追記する。（着手済み）
3. `docs/spec/2-6-execution-strategy.md` に `recover` 既定 OFF の前提とパニック回復が opt-in である旨を明記する。（着手済み）

### フェーズB: Rust 実装
1. `compiler/runtime/src/parse/combinator.rs` に以下を追加する。（完了）
   - `sync_to(sync: Parser<()>) -> Parser<()>`（同期点まで読み飛ばし + 同期点消費）
   - `panic_until<T>(p: Parser<T>, sync: Parser<()>, value: T) -> Parser<T>`（`recover_until` + `context="panic"`）
   - `panic_block<T>(p: Parser<T>, open: Parser<()>, close: Parser<()>, value: T) -> Parser<T>`（ネストを考慮した同期）
   - `recover_missing<T>(p: Parser<T>, sync: Parser<()>, token: Str, value: T) -> Parser<T>`（`recover_with_insert` の別名）
2. `RecoverMeta` に `context` 付与の経路を揃え、`panic_*` が `extensions["recover"].context` を必ず埋めるようにする。（完了）
3. `compiler/runtime/tests/parse_combinator.rs` に単体テストを追加する。（完了）
   - `sync_to_consumes_sync_token`
   - `panic_block_skips_nested_block`
   - `recover_missing_inserts_token_and_fixit`

### フェーズC: サンプル/回帰接続
1. `examples/spec_core/chapter2/parser_core/` に回復ヘルパのサンプルを追加する。（完了）
   - `core-parse-recover-sync-to.reml`
   - `core-parse-recover-panic-block.reml`
2. `expected/spec_core/chapter2/parser_core/` に診断出力を追加する。（完了）
   - `core-parse-recover-sync-to.diagnostic.json`
   - `core-parse-recover-panic-block.diagnostic.json`
3. `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` に `CH2-PARSE-203/204` を登録し、`diagnostic_keys` と `resolution_notes` を更新する。（完了）

#### フェーズC 追補（Recover 拡張）
- Typeck 診断でも `extensions["recover"]` が出力されるように経路を追加し、`sync`/`context` を含む JSON を CLI で確認した。
- `CH2-PARSE-203/204` は `resolution=ok` とし、`run_id` と recover 拡張の要点をマトリクスへ記録済み。

## リスクと緩和策
| リスク | 影響 | 緩和策 |
| --- | --- | --- |
| 同期点設計ミスで再回復ループ | パフォーマンス劣化・診断スパム | `sync_to` が同期点を必ず消費する仕様を徹底 |
| パニック回復が AST を広げる | 不正な AST が下流へ伝播 | `mode="off"` を既定維持し、`panic_*` は opt-in のみ |
| FixIt の補挿位置ずれ | IDE 補完の誤案内 | `recover_with_insert` の既存仕様に合わせ、位置は `until` の設計指針を厳守 |

## 完了判定
- `sync_to`/`panic_*`/`recover_missing` が `docs/spec/2-2-core-combinator.md` と一致する。
- 回復ヘルパの単体テストが追加され、`parse_combinator` の基礎テストに統合される。
- Phase4 マトリクスに `CH2-PARSE-203/204` が登録され、CLI 出力の期待値が揃う。
- `core.parse.recover.branch` の Typeck 診断に `extensions["recover"]`（`sync`/`context`）が出力される。

## 参照
- `docs/notes/dsl/dsl-enhancement-proposal.md`
- `docs/spec/0-1-project-purpose.md`
- `docs/spec/2-2-core-combinator.md`
- `docs/spec/2-5-error.md`
- `docs/spec/2-6-execution-strategy.md`
- `docs/plans/bootstrap-roadmap/4-1-core-parse-error-recovery-impl-plan.md`
