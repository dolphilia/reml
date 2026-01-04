# Phase4: Core.Parse Error Labeling 実装計画

## 背景と目的
- `docs/plans/core-parse-improvement/1-1-error-labeling-plan.md`（WS2）が Step0-3 を完了し、仕様・ラベル語彙・サンプル/期待ゴールデンを用意済み。
- 仕様側では `docs/spec/2-2-core-combinator.md` に推奨ラベル集合と付与ポリシーを追記し、`docs/spec/2-5-error.md` で `label` が期待差し替え＋context push を行うことを明記した。
- Phase4（spec_core 回帰）で Rust フロントエンドの期待集合整形を **概念ラベル中心**に揃え、`CP-WS2-001` の回帰（Rule("expression") を含む）を成立させる。

## スコープ
- 対象: Rust フロントエンド `compiler/frontend/` の Parser / 診断整形。
- シナリオ: `examples/spec_core/chapter2/parser_core/core-parse-label-vs-token-no-label.reml` / `...with-label.reml`（新規）。Phase4 マトリクスへ `CP-WS2-001` として登録する。
- 診断キー: 既存 `parser.syntax.expected_tokens` を維持（新規キーは増やさない）。期待集合・context の内容を改善する。

## 成果物
- 実装: `label` 付きパーサが失敗した際、`Expectation::Rule(name)` が `expected_summary.alternatives` に含まれ、`context` にも積まれる（Token だけの表示に退行しない）。
- ゴールデン: `expected/spec_core/chapter2/parser_core/core-parse-label-vs-token-with-label.diagnostic.json`（Rule を含む）と `...no-label.diagnostic.json`（Token 中心）を CLI 出力と一致させる。
- 回帰: `phase4-scenario-matrix.csv` に `CP-WS2-001`（with-label/no-label のペア）を登録し、`resolution_notes` に実行コマンドと期待条件を記録。
- 記録: 実装変更点を `docs/notes/parser/core-parse-api-evolution.md` または `docs/plans/core-parse-improvement/2-0-integration-with-regression.md` に短く追記。

## 実装ステップ
1. **期待集合整形の確認と修正**
   - `label` 付き失敗で `Expectation::Rule` を必ず保持するよう `ParseError.expected` / `ExpectationSummary` 生成経路を確認し、必要なら `compiler/runtime/src/parse/combinator.rs`（期待縮約）と `compiler/frontend/src/parser/mod.rs`（診断整形）を調整。
   - Token/Rule 混在時の並び替え（B-6/B-7: 具体トークン優先だが Rule を落とさない）をテーブル化し、`context_note` に「`+` の後に expression」などラベル名が残ることを確認。
2. **CLI 表示と LSP データの確認**
   - `expected_summary.alternatives` に Rule が含まれる場合でも `humanized` が自然文になるようローカライズテンプレート適用を確認（フォールバック時の文言調整）。
   - `ParseError.context` の順序（外側→内側）を表示に反映し、`then/andThen` 後段で積まれているかをスポットテスト。
3. **ゴールデン更新・回帰登録**
   - 実行:  
     - `cargo run --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter2/parser_core/core-parse-label-vs-token-no-label.reml`  
     - `cargo run --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter2/parser_core/core-parse-label-vs-token-with-label.reml`
   - 出力を `expected/spec_core/chapter2/parser_core/core-parse-label-vs-token-*.diagnostic.json` に反映し、`phase4-scenario-matrix.csv` へ `CP-WS2-001` を転記（with-label: Rule を含む / no-label: Token 中心）。`resolution_notes` に上記コマンドと期待条件（Span + Rule("expression") を含む）を記録。
   - `4-1-spec-core-regression-plan.md` に追記（任意）し、Phase4 ダッシュボードへの接続を明示。

## 進捗状況（2025-12-18）
- Step1 実装: `compiler/runtime/src/parse/combinator.rs` の `label` が元エラーの期待集合を保持したままラベル名を追加するように修正済み。`compiler/frontend/src/parser/mod.rs` で chumsky の `label()` を `ExpectedToken::rule` として `alternatives`/`context_note` へ反映する整形を追加済み。
- ✅ 動作確認メモ: CLI にパース専用ドライバを追加（`--parse-driver` / `--parse-driver-label`）。これを使うと `core-parse-label-vs-token-*.reml` で期待集合を JSON 出力でき、with-label では Rule("expression") を含み、no-label は identifier/integer-literal のみになることを確認済み。
- ✅ 2026-03-09: parse-driver の humanized/context を B-6/B-7 に合わせて整形し、`core-parse-label-vs-token-*.diagnostic.json` を更新。`phase4-scenario-matrix.csv` に `CP-WS2-001` を登録して with-label/no-label の期待集合差を固定。
- ✅ 2026-03-09: `tooling/examples/run_phase4_suite.py` で CP-WS2-001 だけ `--parse-driver --parse-driver-label expression` を用いる分岐を追加し、spec_core/practical スイートを再実行して全件成功（レポート: `reports/spec-audit/ch5/spec-core-dashboard.md`, `.../practical-suite-index.md`）。
- ✅ 2025-12-18: LSP/Human 出力をスポット確認し、`expected.humanized` / `context_note` が B-6/B-7 どおり（with-label は Rule("expression") を保持、no-label は token/class のみ）であることを確認。フォールバック文言移植は現状不要と判断。

## 依存関係
- 仕様: `docs/spec/2-2-core-combinator.md`（推奨ラベル語彙・付与ポリシー）、`docs/spec/2-5-error.md`（label と context/expected の扱い、縮約 B-6/B-7）。
- 計画・サンプル: `docs/plans/core-parse-improvement/1-1-error-labeling-plan.md`（WS2）、`examples/spec_core/chapter2/parser_core/core-parse-label-vs-token-*.reml`、期待ゴールデン2件。

## リスクと対策
- **他シナリオの期待集合が変わる**: `parser.syntax.expected_tokens` 依存シナリオへの影響を確認するため、Phase4 spec_core/practical を一度通し、差分は `phase4-scenario-matrix.csv` に `impl_fix` として記録。
- **Rule が落ちる/重複する**: 期待縮約の優先度を B-6/B-7 に合わせる回帰テスト（CP-WS2-001）で検出し、Token 優先表示でも Rule を `alternatives` に保持する実装を維持する。

## 完了判定
- `core-parse-label-vs-token-with-label.reml` の診断に `Expectation::Rule("expression")` が含まれ、`core-parse-label-vs-token-no-label.reml` ではトークン中心の期待集合になることを CLI で確認済み。
- `expected/...` と CLI 出力が一致し、`phase4-scenario-matrix.csv` に `CP-WS2-001` が登録・緑化。
- 実装メモを `core-parse-api-evolution.md` 等に反映。
