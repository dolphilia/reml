# Core.Parse 強化計画（ドラフト）

`Core.Parse` を **実用的かつ開発者体験の高い基盤**へ引き上げるための、専用計画群を集約するディレクトリです。
本計画は `docs/notes/core-parse-improvement-survey.md` を出発点とした **ドラフト**であり、`docs/plans/bootstrap-roadmap/` の既存 Phase 4.1 計画と混在させずに、検討・分割・優先度付けを行うために作成します。

- 設計指針: `docs/spec/0-1-project-purpose.md`（性能・安全性・分かりやすいエラー）を最優先
- 参照仕様: `docs/spec/2-0-parser-api-overview.md`, `docs/spec/2-1-parser-type.md`, `docs/spec/2-2-core-combinator.md`, `docs/spec/2-3-lexer.md`, `docs/spec/2-5-error.md`, `docs/spec/2-7-core-parse-streaming.md`
- 既存計画との関係: `docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md` を進める前提で、Core.Parse の不足点を先に補強し、回帰の信頼度（診断品質・性能・復旧）を上げる

## 文書一覧（ドラフト）
- [0-0-overview.md](0-0-overview.md): 背景・目的・成功条件・全体方針
- [0-1-workstream-tracking.md](0-1-workstream-tracking.md): 作業ストリーム、成果物、回帰への接続（追跡ルール）
- [1-0-cut-commit-plan.md](1-0-cut-commit-plan.md): Cut/Commit（バックトラック制御）を中核に据えた設計・検証
- [1-1-error-labeling-plan.md](1-1-error-labeling-plan.md): `label`/文脈付与による期待集合と診断の改善
- [1-2-lex-helpers-plan.md](1-2-lex-helpers-plan.md): `Core.Parse.Lex`（lexeme/symbol/literal 等）強化と字句プロファイル連携
- [1-3-error-recovery-plan.md](1-3-error-recovery-plan.md): エラー回復（複数エラー報告/IDE 向け）戦略の整備
- [1-4-input-zero-copy-plan.md](1-4-input-zero-copy-plan.md): `Input` 抽象とゼロコピー前提の整合（性能・安全性）
- [1-5-left-recursion-plan.md](1-5-left-recursion-plan.md): 左再帰への実用的対処（ガード/変換/ビルダー指針）
- [2-0-integration-with-regression.md](2-0-integration-with-regression.md): bootstrap-roadmap（Phase4）回帰計画との接合点

## 注意（ドラフト運用）
- 本ディレクトリは「検討の隔離」と「具体化のための分割」を目的とします。実装を開始した時点で、必要に応じて bootstrap-roadmap の該当 Phase へ成果物を移管（または相互リンク）します。
- 仕様変更が発生する場合は、必ず `docs/spec/2-x` と `docs/guides/core-parse-streaming.md` 等の関連文書へ横断的に反映し、差分の根拠を脚注または TODO として残します。

