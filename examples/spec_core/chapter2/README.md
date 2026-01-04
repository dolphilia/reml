# Chapter 2 spec_core ケース

Chapter 2 の `Core.Parse` API（コンビネータ・Streaming・OpBuilder）を `.reml` で再現するスイートです。`docs/spec/2-2-core-combinator.md`、`2-5-error.md`、`2-7-core-parse-streaming.md`、`2-4-op-builder.md` を対応付けています。

- `parser_core/`: `Parse.or` + `commit` の成功例、`Parse.recover` 診断 (`core.parse.recover.branch`)、および演算子欠落（`(1 +)`）/括弧閉じ忘れ（`(1 + 2`）での `cut` 境界による期待集合縮約を固定化
- `streaming/`: `run_stream` / `DemandHint::More` の往復を再現し、Chunk 供給時の長さを検証
- `op_builder/`: 優先度レベルに異なる fixity を登録した際の `core.parse.opbuilder.level_conflict` を固定化

`expected/spec_core/chapter2/` 配下に `stdout` または `diagnostic.json` を用意し、`phase4-scenario-matrix.csv` の `CH2-*` 行と 1:1 に対応させています。
