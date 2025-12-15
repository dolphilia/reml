# パターンマッチ強化計画（ドラフト）

パターンマッチ機能を強化するための専用計画群を集約するディレクトリです。背景メモは `../../notes/pattern-matching-improvement.md` を参照してください。

- 設計指針: `docs/spec/0-1-project-purpose.md` に沿って安全性・実用性・DSLファーストを優先
- 参考仕様: `docs/spec/1-1-syntax.md`, `docs/spec/1-5-formal-grammar-bnf.md`, `docs/spec/1-2-types-Inference.md`
- 既存計画との関係: Phase 4 系（例: `../bootstrap-roadmap/4-1-spec-core-regression-plan.md`）の進行を阻害しないよう、仕様差分を明示して連携

## 文書一覧
- [0-0-overview.md](0-0-overview.md): 目的、スコープ、進行フェーズの骨子
- [1-0-active-patterns-plan.md](1-0-active-patterns-plan.md): Active Patterns 導入に関する詳細計画
- [1-1-pattern-surface-plan.md](1-1-pattern-surface-plan.md): Or/Slice/Range/Binding/Regex など周辺機能の拡張計画

> すべてドラフト版です。実施に伴い内容を更新し、`docs/plans/README.md` からも参照します。

## 実装連携メモ（Rust/OCaml パーサ向け短報）

- `match` ガードは正規形を `when` とし、互換目的で `if` を受理する場合は `pattern.guard.if_deprecated` 警告を発行する。
- AST ではガードと `as` エイリアスを **guard → alias** の順に正規化する（記述順は順不同で受理）。
- 上記方針を Rust/OCaml パーサ双方で揃え、診断キーと正規化順が一致することをテストで固定する。
