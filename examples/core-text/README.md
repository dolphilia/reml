# Core Text & Unicode サンプル

`docs/spec/3-3-core-text-unicode.md` の §9「使用例」で紹介している `Core.Text`/`Core.Unicode` API を 1 つの ReML ファイル（`text_unicode.reml`）へまとめ、Bytes→Str→String の三層モデル、`GraphemeSeq`、`TextBuilder`、`log_grapheme_stats`、ストリーミング decode の連携例を確認できるようにしました。

## ファイル構成

| パス | 内容 |
| --- | --- |
| `text_unicode.reml` | `Token` 列挙体と `normalize_identifier` / `emoji_token` / `doc_comment` / `unicode_pipeline` / `sample_decode` を含むサンプル本体。 |
| `expected/text_unicode.tokens.golden` | `unicode_pipeline()` が返す `Token` 列の期待値（`Identifier`/`Number`/`Emoji`/`DocComment`）。 |
| `expected/text_unicode.grapheme_stats.golden` | `log_grapheme_stats` のメタデータ例。監査ログ (`text.grapheme_stats`) が含むキーを把握するためのゴールデン。 |
| `expected/text_unicode.stream_decode.golden` | `compiler/runtime/examples/io/text_stream_decode.rs` を `tests/data/unicode/streaming/sample_input.txt` で実行した結果。BOM 処理と `InvalidSequenceStrategy::Replace` の挙動を把握できる。 |

## 実行とゴールデン更新

1. `text_unicode.reml` の出力確認（Phase 3 以降の `reml` CLI を想定）:

```sh
cargo run --bin reml -- examples/core-text/text_unicode.reml > /tmp/core-text.tokens
diff -u examples/core-text/expected/text_unicode.tokens.golden /tmp/core-text.tokens
```

2. Grapheme 監査ログの再取得:

```sh
scripts/collect-text-grapheme-stats.sh --input /tmp/core-text.tokens \
  | tee examples/core-text/expected/text_unicode.grapheme_stats.golden
```

3. ストリーミング decode 例（Rust runtime の PoC バイナリを利用）:

```sh
cargo run --manifest-path compiler/runtime/Cargo.toml --bin text_stream_decode \
  -- --input tests/data/unicode/streaming/sample_input.txt \
  --output examples/core-text/expected/text_unicode.stream_decode.golden
```

※ 自動 CLI が未整備の間は `examples/core-text/expected/` に保存した最新ゴールデンを参照し、`docs/plans/bootstrap-roadmap/3-3-core-text-unicode-plan.md#5` に従って更新履歴を `docs-migrations.log` へ追記してください。

## 関連ドキュメント

- `docs/spec/3-3-core-text-unicode.md` §2〜§5（三層モデル・正規化・ストリーミング）
- `docs/plans/bootstrap-roadmap/3-3-core-text-unicode-plan.md` §5（サンプル／ドキュメント更新タスク）
- `docs/guides/compiler/core-parse-streaming.md` §10（`decode_stream` と TextBuilder の連携例）
- `docs/guides/ecosystem/ai-integration.md` §6（AI 入力正規化の注意点）
