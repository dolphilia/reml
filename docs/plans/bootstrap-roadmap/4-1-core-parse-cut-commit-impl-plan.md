# Phase4: Core.Parse Cut/Commit 実装計画

## 背景と目的
- `docs/plans/core-parse-improvement/1-0-cut-commit-plan.md`（WS1）が **Cut/Commit の指針・診断ゴールデン・仕様整合**を完了。
- Phase4（spec_core 回帰）側で、Rust フロントエンド実装へ反映し、`CH2-PARSE-102/103` を **Cut 有無比較が成立する形**で自動検証できるようにする。

## スコープ
- Rust フロントエンド (`compiler/frontend/`) の Parser 実装に限定。
- 対象シナリオ: `CH2-PARSE-101/102/103`（Phase4 マトリクス、spec_core）。
- 診断キーは既存 `parser.syntax.expected_tokens` を維持し、notes/context で Cut 境界を反映する（新規キーは増やさない）。

## 成果物
- 実装: Cut 境界の明示（`cut_here()` 挿入など）で、期待集合が WS1 の指針と一致する。
- ゴールデン: `expected/spec_core/chapter2/parser_core/` で Cut 有り版の期待集合が安定。
- 回帰: Phase4 マトリクス `CH2-PARSE-102/103` が「Cut 有無比較ができる」状態で CI を通過。
- 記録: 実装変更点を `docs/notes/parser/core-parse-api-evolution.md` または `docs/plans/core-parse-improvement/2-0-integration-with-regression.md` に短く追記。

## 実装ステップ（優先順）
1. **境界挿入の洗い出し（Rust Parser）**
   - 配列/オブジェクト/区切り: `[` / `{` / `:` / `,` / `]` / `}` 直後の `cut_here()`（JSON/YAML 指針）。
   - 演算子: `+` など演算子消費後の右項開始地点で `cut_here()`（expr builder / chainl 系）。
   - 括弧ペア: `between(open, p, close)` の `open` 消費直後に `cut_here()`（D-1）。
   - 進捗: **完了**。`compiler/frontend/src/parser/mod.rs` に `delimited_with_cut` を追加し、括弧/配列/レコード/属性/パラメータ/ハンドラ/エフェクト等の囲み構造と演算子チェーン・代入・引数列で `cut()` を挿入して committed 境界を明示化した。
2. **診断メッセージの調整（最小）**
   - `parser.syntax.expected_tokens` の notes/context を、WS1 D-1/D-5 の文脈に揃える（例: 「`(` に対応する `)` が必要です」「`+` の後に式が必要です」）。
   - 新規診断キーは追加しない。
   - 進捗: **完了（最小調整）**。`compiler/frontend/src/parser/mod.rs` の期待集合組み立てで、`RParen` 期待時に「`(` に対応する `)` が必要です」、式リカバー文脈（演算子右項欠落など）で「演算子の後に式が必要です」を `context_note` として付与。診断キーは追加せず `parser.syntax.expected_tokens` の notes/context のみ更新。
3. **ゴールデン更新と確認**
   - `cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter2/parser_core/core-parse-cut-branch-mislead.reml`
   - `cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter2/parser_core/core-parse-cut-unclosed-paren.reml`
   - 必要に応じて `expected/...` を更新（Cut 無し版は比較用で更新しない）。
   - 進捗: **実行済み**。上記コマンドを実行し、出力を `expected/spec_core/chapter2/parser_core/core-parse-cut-branch-mislead.diagnostic.json` と `.../core-parse-cut-unclosed-paren.diagnostic.json` に更新。
4. **回帰計画への反映**
   - `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` の `resolution_notes` に実行コマンド・期待集合の要約を追記。
   - `docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md` に実装完了メモを追加（任意）。
   - 進捗: **完了**。`phase4-scenario-matrix.csv` の `CH2-PARSE-102/103` に最新 CLI コマンド（run_id=c9f29fff-58c7-4849-9bb3-999562132bbf / f9fb5aaa-f93d-42d9-b149-301a34f61485）と期待ファイル更新を記録し、`4-1-spec-core-regression-plan.md` へ cut/commit 回帰メモを追記。

## 依存関係
- 仕様: `docs/spec/2-1-parser-type.md`（committed 独立性）、`docs/spec/2-2-core-combinator.md`（D 節の指針）、`docs/spec/2-5-error.md`（B-5 / D-1）、`docs/spec/2-6-execution-strategy.md`（期待再初期化）。
- 計画: `docs/plans/core-parse-improvement/1-0-cut-commit-plan.md`, `docs/plans/core-parse-improvement/2-0-integration-with-regression.md`。

## リスクと対策
- **誤挿入による過剰コミット**: 回復系（recover）との干渉を避けるため、演算子・括弧・一意トークンのみに限定して cut を入れる。
- **期待集合の変動で他シナリオが影響**: Phase4 フルスイートを一度流し、影響範囲を確認（特に `parser.syntax.expected_tokens` 依存シナリオ）。

## 完了判定
- `CH2-PARSE-101/102/103` が最新実装で CLI 実行し、ゴールデンと一致。
- `phase4-scenario-matrix.csv` の `scenario_notes` / `resolution_notes` に最新実装結果が記録済み。
- `core-parse-api-evolution.md` または `2-0-integration-with-regression.md` に実装反映の一文を追記済み。
