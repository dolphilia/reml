# Phase4: Core.Parse Error Recovery 実装計画

## 背景と目的
- `docs/plans/core-parse-improvement/1-3-error-recovery-plan.md`（WS4）で、**回復の責務境界**・**cut との整合**・**糖衣 API**・**回復メタ（最小スキーマ）**・**複数診断回帰（CP-WS4-001）**を確定した。
- Phase4（spec_core 回帰）側では、上記決定を Rust 実装へ反映し、IDE/LSP 用の「複数エラー収集」を **再現可能な RunConfig と期待出力**で固定する必要がある。

本計画のゴールは、`CH2-PARSE-201`（単発回復）に加えて、`CH2-PARSE-202`（複数回復）を Phase4 回帰として成立させ、Phase4 に戻って実装・ゴールデン更新・緑化まで行える導線を提供することである。

## スコープ
- 対象実装: Rust フロントエンド/ランタイム（`compiler/rust/frontend/`, `compiler/rust/runtime/`）の Core.Parse 回復経路。
- 対象シナリオ（Phase4）:
  - `CH2-PARSE-201`（`core-parse-recover-diagnostic` / 単発回復）
  - `CH2-PARSE-202`（`core-parse-recover-multiple-errors-semicolon` / 文末 `;` 同期で複数回復、計画起点 ID: `CP-WS4-001`）
- 仕様根拠（決定事項）:
  - `docs/spec/2-1-parser-type.md`（`RunConfig.extensions["recover"]` と `ParseResult.recovered`）
  - `docs/spec/2-2-core-combinator.md`（`recover` と 4 糖衣）
  - `docs/spec/2-5-error.md`（committed 超え回復、同期点指針、`extensions["recover"]` 最小スキーマ、FixIt）
  - `docs/spec/2-6-execution-strategy.md`（運用指針: mode 切替と recovered/diagnostics 蓄積）

## 成果物
- 実装:
  - `RunConfig.extensions["recover"].mode = "off"|"collect"` を解釈し、`mode="off"`（既定）では回復しない（fail-fast）。
  - `recover` は committed（`cut`）を含む失敗でも捕捉できる（`mode="collect"` の場合）。ただし分岐（`or` の右枝）は試さない。
  - 糖衣 `recover_with_default/recover_until/recover_with_insert/recover_with_context` を実装し、`Diagnostic.extensions["recover"]` の `action`/`sync`/`inserted`/`context` を出力する。
  - 回復が 1 回でも起きたら `ParseResult.recovered=true`、回復のたびに `ParseResult.diagnostics` を蓄積する。
- ゴールデン:
  - `expected/spec_core/chapter2/parser_core/core-parse-recover-diagnostic.diagnostic.json`（既存）と、
    `expected/spec_core/chapter2/parser_core/core-parse-recover-multiple-errors-semicolon.diagnostic.json`（新規）を CLI 出力と一致させる。
- 回帰:
  - `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` の `CH2-PARSE-202` を `resolution=ok` に更新できる状態にする（コマンドとログを `resolution_notes` に残す）。

## 実装ステップ（優先順）
1. **RunConfig の recover 拡張を実装（mode/sync_tokens/上限）**
   - `extensions["recover"].mode`（`"off"|"collect"`）を解析・既定 `"off"` を保持。
   - `extensions["recover"].sync_tokens` を読み、回復経路がどの同期点を使ったかを `Diagnostic.extensions["recover"].sync` に記録できるようにする。
   - `max_diagnostics/max_resync_bytes/max_recoveries` は best-effort の安全弁として実装し、超過時は回復停止（fail-fast へフォールバック）。
   - 実装メモ（Rust runtime / Core.Parse）:
     - 実装箇所: `compiler/rust/runtime/src/parse/combinator.rs`
       - `decode_recover_config` を追加し、`ParseState` 構築時に `RunConfig.extensions["recover"]` を解釈する。
       - `Parser::recover` は `mode!="collect"` の場合は回復せず、元の `Err` を返す（fail-fast）。
       - 同期点は `until` 成功時の消費スライスから推定し、`ParseError.recover.sync` → `GuardDiagnostic.extensions["recover"].sync` に露出する（`sync_tokens` が空なら消費スライスを採用）。
       - 安全弁: `max_diagnostics`（診断件数）、`max_resync_bytes`（全回復の総スキップ量）、`max_recoveries`（成功回復回数）を超えた場合は回復を打ち切り、元の失敗を返す（best-effort）。
     - 回帰（ユニットテスト）: `compiler/rust/runtime/tests/parse_combinator.rs`
       - 既定 `mode="off"` で recover が発火しないこと、`mode="collect"` で回復し `sync` が記録されること、上限で fail-fast に戻ることを固定。
2. **recover の意味論を仕様通りに揃える（committed 超え回復）**
   - `recover(p, until, with)` が committed 失敗も捕捉して同期できること（`mode="collect"` の場合）。
   - ただし `or` の分岐挙動は `cut` に従い、右枝は試さない（回復は「分岐再探索」ではない）。
   - 実装メモ（Rust runtime / Core.Parse）:
     - 実装箇所: `compiler/rust/runtime/src/parse/combinator.rs`
       - `Parser::recover` の `committed` 早期リターンを廃止し、**committed 失敗でも同期して `Ok(with)` へ回復**できるようにする。
       - 回復を諦めて `Err` を返す経路（上限超過・EOF 到達など）では、`committed` を潰さず元の値を保持し、`or` が右枝へ進まないことを保証する。
     - 回帰（ユニットテスト）: `compiler/rust/runtime/tests/parse_combinator.rs`
       - `recover_collect_mode_can_recover_committed_failure_without_trying_fallback` を追加し、committed 超え回復と `or` の短絡を固定する。
3. **糖衣 4 種の実装と recover メタ/ FixIt の出力**
   - `recover_with_default`: `action="default"`
   - `recover_until`: `action="skip"`
   - `recover_with_insert`: `action="insert"` + `inserted=token` + `FixIt::InsertToken(token)`（等価表現可）
   - `recover_with_context`: `action="context"` + `context=message`（`notes=true` 運用では `Diagnostic.notes` へも露出）
   - `Diagnostic.extensions["recover"]` は `docs/spec/2-5-error.md` の E-2-1 を最小保証として満たす。
4. **Runner / CLI 経路の整備（`run_with_recovery` の取り扱い）**
   - `examples/spec_core/chapter2/parser_core/core-parse-recover-*.reml` が前提としている `Parse.run_with_recovery(...)` を Phase4 実装で再現可能にする。
     - 方針案: `run_with_recovery(p, src)` は `RunConfig.extensions["recover"].mode="collect"` を有効化した `run(p, src, cfg)` の薄いラッパ。
     - 同期点集合は `extensions["recover"].sync_tokens=[";"]` を既定とするか、サンプル側で注入する。
5. **ゴールデン更新と Phase4 マトリクス緑化**
   - 実行（例）:
     - `cargo run --quiet --manifest-path compiler/rust/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter2/parser_core/core-parse-recover-multiple-errors-semicolon.reml`
   - `diagnostics[].code` が `["core.parse.recover.branch","core.parse.recover.branch"]`（順序含む）になることを確認し、`expected/.../core-parse-recover-multiple-errors-semicolon.diagnostic.json` を更新。
   - `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` の `CH2-PARSE-202` を `resolution=ok` に更新し、`resolution_notes` に CLI コマンド・ログパス・RunConfig 前提（mode/ sync_tokens）を記録する。

## 作業対象ファイル（対応表）
- シナリオ入力:
  - `examples/spec_core/chapter2/parser_core/core-parse-recover-diagnostic.reml`（既存: CH2-PARSE-201）
  - `examples/spec_core/chapter2/parser_core/core-parse-recover-multiple-errors-semicolon.reml`（新規: CH2-PARSE-202）
- 期待出力:
  - `expected/spec_core/chapter2/parser_core/core-parse-recover-diagnostic.diagnostic.json`
  - `expected/spec_core/chapter2/parser_core/core-parse-recover-multiple-errors-semicolon.diagnostic.json`
- Phase4 マトリクス:
  - `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv`（`CH2-PARSE-201/202`）

## リスクと対策
- **回復が既定 ON になって誤 AST が広がる**: `mode="off"` 既定を厳守し、回復は opt-in（WS4 Step0）を維持する。
- **cut と回復が衝突して診断が不安定になる**: `cut` は分岐抑止、`recover` は同期・継続と責務を分離し、`committed` 超え回復は「同じ枝のまま同期」だけ許す（WS4 Step1）。
- **期待出力の揺れ**: 初期は `diagnostic_keys`（件数/コード列）中心で固定し、`extensions["recover"]` の詳細固定は段階導入する（WS4 Step3 方針）。

## 完了判定
- `CH2-PARSE-201` と `CH2-PARSE-202` の CLI 出力が `expected/` と一致し、`phase4-scenario-matrix.csv` の `CH2-PARSE-202` が `resolution=ok`。
- `docs/spec/2-5-error.md` E-2（回復糖衣と FixIt 最小スキーマ）の要求を満たす `extensions["recover"]` が出力される。
