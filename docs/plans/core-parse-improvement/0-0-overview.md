# Core.Parse 強化計画: 概要

## 背景
- `docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md` を進めている途中で、回帰の土台となる `Core.Parse` の **診断品質** と **実装・運用の実用性**（DSL 開発の書きやすさ、復旧、性能）が不足しうると判断した。
- 調査メモ `docs/notes/parser/core-parse-improvement-survey.md` は、既存の優れたパーサーコンビネーター（Parsec/Megaparsec/nom/chumsky/Angstrom/FastParse）から Reml が取り込むべき要素を整理している。

本計画は上記メモを具体タスクへ落とし込み、Core.Parse の強化を **回帰計画の前提整備（Phase4 の信頼性向上）** として実施するための正式計画である。

## 目的
`docs/spec/0-1-project-purpose.md` に沿って、次を同時に満たす状態を目標とする。

1) **分かりやすいエラー**: 文脈に沿った期待表示、Cut/Commit に基づく「最も正しい失敗」を提示できる  
2) **実用性能**: 入力が大きくても線形に近い特性を維持し、ゼロコピー前提（`Input`）を破らない  
3) **DSL での書きやすさ**: Lex ヘルパと回復戦略により、サンプルが自前実装を必要としない  
4) **回帰可能性**: シナリオ/期待出力/診断キーが揃い、改修の副作用を検出できる

## 非目的（この計画ではやらない）
- コンパイラ全体の最適化（JIT、全面的ストリーミング化等）のような大規模刷新
- パーサー生成（parser generator）への移行
- 仕様外の暗黙挙動（例: 既定で無制限バックトラックする）を導入して診断品質を落とす変更

## 成功条件
- Cut/Commit の慣習が API とガイドに反映され、代表的な分岐点で「別ルールへ誤って逃げる」診断が減る
- `label`/文脈付与が導入され、期待集合がトークン列ではなく **人間可読な単位**（例: "expression"）で提示できる
- `Core.Parse.Lex` の標準ヘルパで、サンプル DSL の多くが自前 `lexeme/symbol/literal` を定義せずに書ける
- 回復戦略により、1 ファイル内の複数箇所のエラーを報告でき、IDE/LSP の解析に耐える
- `Input` のゼロコピー前提が仕様と実装に整合し、部分文字列生成がホットパスにならない

## 参照
- 調査メモ: `docs/notes/parser/core-parse-improvement-survey.md`
- パーサ仕様: `docs/spec/2-0-parser-api-overview.md`, `docs/spec/2-1-parser-type.md`, `docs/spec/2-2-core-combinator.md`, `docs/spec/2-5-error.md`
- 字句ヘルパ: `docs/spec/2-3-lexer.md`
- ストリーミング: `docs/spec/2-7-core-parse-streaming.md`, `docs/guides/compiler/core-parse-streaming.md`

## 進行方針（ワークストリーム）
本計画は、機能を「一括投入」せず、回帰と連動する単位へ分割する。
詳細は `0-1-workstream-tracking.md` を参照。

- Cut/Commit（バックトラック制御）: `1-0-cut-commit-plan.md`
- Error Labeling（文脈・期待集合）: `1-1-error-labeling-plan.md`
- Lex Helpers（scannerless ヘルパ群）: `1-2-lex-helpers-plan.md`
- Error Recovery（複数エラー・IDE）: `1-3-error-recovery-plan.md`
- Input/Zero-copy（入力抽象と性能）: `1-4-input-zero-copy-plan.md`
- Left Recursion（式などの文法）: `1-5-left-recursion-plan.md`
