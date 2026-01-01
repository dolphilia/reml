# Core Text Examples - 2027-03-30

## 1. 実行コマンド

1. `cargo run --manifest-path compiler/rust/runtime/Cargo.toml --bin text_stream_decode -- --input tests/data/unicode/streaming/sample_input.txt --output examples/core-text/expected/text_unicode.stream_decode.golden`
   - `BomHandling=auto`, `InvalidSequenceStrategy=error`。出力 JSON は `graphemes=76` / `avg_width=1.131578947368421` を記録。
2. `cat examples/core-text/expected/text_unicode.tokens.golden`
   - `Identifier("prep_work_𝑒")`, `Number("42.0")`, `Emoji("👩‍💻")`, `DocComment("/** 設定 : AI補助 */")` を並べて Bytes→Str→String 正規化の期待値を固定。
3. `cat examples/core-text/expected/text_unicode.grapheme_stats.golden`
   - `text.grapheme_stats` メタデータ（`cache_hits=1`, `cache_miss=0`, `script_mix_ratio=0.5`, `version="15.1"`）を `log_grapheme_stats` の監査値として保存。

## 2. 監査メモ

- 仕様: `docs/spec/3-3-core-text-unicode.md` §9 のサンプルを `examples/core-text/text_unicode.reml` へ移し、`expected/` 配下のゴールデンで CLI/Streaming/監査の整合を検証する。
- KPI: `tooling/ci/collect-iterator-audit-metrics.py --section text --scenario grapheme_stats --source examples/core-text/expected/text_unicode.grapheme_stats.golden --require-success` を追加して `text.grapheme.cache_hit` を監視する予定。
- リンク: `docs/plans/bootstrap-roadmap/3-3-core-text-unicode-plan.md` §5、`docs/guides/compiler/core-parse-streaming.md` §11、`docs/guides/ecosystem/ai-integration.md` §6。
