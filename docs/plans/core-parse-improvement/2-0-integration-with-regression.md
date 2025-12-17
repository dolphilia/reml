# Phase4 回帰計画との統合方針（ドラフト）

## 目的
本ディレクトリ（Core.Parse 強化計画）で検討・実施した成果を、`docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md` と `docs/plans/bootstrap-roadmap/4-1-scenario-matrix-plan.md` へ **安全に接続**する方針を定める。

## 現状整理（ドラフト）
- Phase4 は「仕様コア回帰（spec_core）」を軸にしており、Core.Parse はその基盤にあたる
- `docs/plans/bootstrap-roadmap/4-1-core-parse-combinator-plan-v2.md` には Core.Parse の拡張計画が存在するが、本ディレクトリは `docs/notes/core-parse-improvement-survey.md` を起点に **Cut/Label/Lex/Zero-copy/Recovery** を改めて前面化する

## 接合点
- **仕様差分**: `docs/spec/2-x` に追記・修正が入る場合、Phase4 の「期待診断」「成功条件」が変わる  
  → 仕様更新は必ず `docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md` の該当チェックに影響を書き添える
- **シナリオ追加**: 新規シナリオは Phase4 マトリクスへ登録し、run_id/期待出力/診断キーを固定する  
  → 本ディレクトリで一時 ID（`CP-WS*-NNN`）を振り、転写時に `CH2-PARSE-xxx` 等へ割り当てる
- **サンプル追加**: `examples/spec_core/chapter2/parser_core/` を基本置き場とし、必要に応じて `expected/spec_core/...` と同期する

## 運用ルール（暫定）
- 本ディレクトリのドラフトが安定したら、bootstrap-roadmap 側へ次のいずれかで反映する:
  1) 既存 Phase の追記（重複が少ない場合）
  2) 参照リンクの追加のみ（内容は本ディレクトリに保持）
  3) 計画書を移管（正式版として採用する場合）
- どの方式でも、`docs/plans/README.md` の目次リンクを更新し、計画の入口が迷子にならないようにする

## TODO（次の具体作業）
- `docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md` の現在位置（どの章・どのシナリオを進行中か）を確認し、WS1/WS2 を差し込む最小範囲を確定する
- Phase4 マトリクスに追加すべき「Core.Parse 強化シナリオ」の候補を列挙する（Cut/Label/Lex/Recovery を優先）

