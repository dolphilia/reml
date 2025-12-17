# パターンマッチ強化計画

パターンマッチ機能を強化するための専用計画群を集約するディレクトリです。背景メモは `../../notes/pattern-matching-improvement.md` を参照してください。

- 設計指針: `docs/spec/0-1-project-purpose.md` に沿って安全性・実用性・DSLファーストを優先
- 参考仕様: `docs/spec/1-1-syntax.md`, `docs/spec/1-5-formal-grammar-bnf.md`, `docs/spec/1-2-types-Inference.md`
- 既存計画との関係: Phase 4 系（例: `../bootstrap-roadmap/4-1-spec-core-regression-plan.md`）の進行を阻害しないよう、仕様差分を明示して連携
- 最新同期: Active Pattern の記法・BNF・診断キーに加え、`match` 評価順（Partial Active の `None` フォールスルー含む）を `docs/spec/1-5-formal-grammar-bnf.md` と `docs/guides/core-parse-streaming.md` 付録Aへ同期。Phase4 の `CH1-ACT-001..003` / `CH1-MATCH-007..018` はマトリクス（run_id 記録含む）と CLI 確認まで完了。クロス実装チェック（`1-2` の M5）は OCaml 実装の更新停止により無効（凍結）。

## 文書一覧
- [0-0-overview.md](0-0-overview.md): 目的、スコープ、進行フェーズの骨子
- [1-0-active-patterns-plan.md](1-0-active-patterns-plan.md): Active Patterns 導入に関する詳細計画
- [1-1-pattern-surface-plan.md](1-1-pattern-surface-plan.md): Or/Slice/Range/Binding/Regex など周辺機能の拡張計画
- [1-2-match-ir-lowering-plan.md](1-2-match-ir-lowering-plan.md): Match/Pattern を IR へ伝搬しコード生成で分岐を組むための計画（Partial Active の miss パス含む）

本計画は承認済みの正式版です。更新時は `docs/plans/README.md` と関連仕様のリンク整合を確認してください。

## 実装連携メモ（Rust/OCaml パーサ向け短報）

- `match` ガードは正規形を `when` とし、互換目的で `if` を受理する場合は `pattern.guard.if_deprecated` 警告を発行する。
- AST ではガードと `as` エイリアスを **guard → alias** の順に正規化する（記述順は順不同で受理）。
- 上記方針を Rust/OCaml パーサ双方で揃え、診断キーと正規化順が一致することをテストで固定する。
