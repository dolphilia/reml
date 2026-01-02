# Core.Parse 強化計画

ステータス: 承認済み（2025-12-17）

`Core.Parse` を **実用的かつ開発者体験の高い基盤**へ引き上げるための、専用計画群を集約するディレクトリです。
本計画は `docs/notes/parser/core-parse-improvement-survey.md` を出発点に、`docs/plans/bootstrap-roadmap/` の Phase 4 系回帰（spec_core）を前提として、Core.Parse の改善作業を追跡可能な粒度へ分割します。

- 設計指針: `docs/spec/0-1-project-purpose.md`（性能・安全性・分かりやすいエラー）を最優先
- 参照仕様: `docs/spec/2-0-parser-api-overview.md`, `docs/spec/2-1-parser-type.md`, `docs/spec/2-2-core-combinator.md`, `docs/spec/2-3-lexer.md`, `docs/spec/2-5-error.md`, `docs/spec/2-7-core-parse-streaming.md`
- 既存計画との関係: `docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md` を進める前提で、Core.Parse の不足点を先に補強し、回帰の信頼度（診断品質・性能・復旧）を上げる

## 文書一覧
- [0-0-overview.md](0-0-overview.md): 背景・目的・成功条件・全体方針
- [0-1-workstream-tracking.md](0-1-workstream-tracking.md): 作業ストリーム、成果物、回帰への接続（追跡ルール）
- [1-0-cut-commit-plan.md](1-0-cut-commit-plan.md): Cut/Commit（バックトラック制御）を中核に据えた設計・検証
- [1-1-error-labeling-plan.md](1-1-error-labeling-plan.md): `label`/文脈付与による期待集合と診断の改善
- [1-2-lex-helpers-plan.md](1-2-lex-helpers-plan.md): `Core.Parse.Lex`（lexeme/symbol/literal 等）強化と字句プロファイル連携
- [1-3-error-recovery-plan.md](1-3-error-recovery-plan.md): エラー回復（複数エラー報告/IDE 向け）戦略の整備
- [1-4-input-zero-copy-plan.md](1-4-input-zero-copy-plan.md): `Input` 抽象とゼロコピー前提の整合（性能・安全性）
- [1-5-left-recursion-plan.md](1-5-left-recursion-plan.md): 左再帰への実用的対処（ガード/変換/ビルダー指針）
- [2-0-integration-with-regression.md](2-0-integration-with-regression.md): bootstrap-roadmap（Phase4）回帰計画との接合点

## 運用ルール
- 本計画の成果物（サンプル/期待出力/シナリオ登録）は、原則として `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` と同期し、Phase4 回帰の監視対象として扱います。
- 仕様変更が発生する場合は、必ず `docs/spec/2-x` と `docs/guides/compiler/core-parse-streaming.md` 等の関連文書へ横断的に反映し、差分の根拠（判断理由・影響範囲）を脚注または TODO として残します。
- 本ディレクトリの文書を更新した場合は、`docs/plans/README.md` と `docs/plans/bootstrap-roadmap/` 側の参照リンクの整合も確認します。

## 変更履歴
- 2025-12-17: 承認に伴い正式版へ昇格（ドラフト表記を整理、運用ルールを明確化）
