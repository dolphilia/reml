# Text API エラーシナリオチェックリスト

## 目的
Core.Text / Core.Unicode API が `UnicodeError` や `CollectError` を仕様どおりに返すかを検証し、3-3 計画および Diagnostics 連携の観測項目に反映する。

## 運用メモ
- ケースは `reports/spec-audit/ch1/core_text_examples-*` へリンクし、再現ログを追加する。
- 状況欄は `Pending`/`Green`/`Blocked` の 3 種で記録する。

## チェック表
| ID | API/機能 | 前提条件・入力 | 期待されるエラー / 成果物 | 検証資産 | 担当 | 状況 | 備考 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| TA-01 | `Bytes::from_vec` バリデーション | 末尾が不完全な UTF-8 シーケンス | `UnicodeErrorKind::InvalidUtf8`、`Diagnostic.highlight.display_width` 記録 | `tests/data/unicode/invalid_utf8.bin` | @core-text | Pending | 実装時に `effect {mem}` 打刻可否を記録（W42 再測） |
| TA-02 | `String::clone` メモリ枯渇 | 1GB 超の入力で `try_reserve` 失敗を誘発 | `UnicodeErrorKind::OutOfMemory` → `CollectError::OutOfMemory` への変換 | `tooling/ci/collect-iterator-audit-metrics.py --section text --scenario bytes_clone --text-mem-source reports/text-mem-metrics.json` | @core-text | Pending | `reports/spec-audit/ch1/core_text_mem-20270329.md` に OOM ケースを記録し、CI で監視（W42 再測） |
| TA-03 | `TextBuilder::push_grapheme` decode | 不正な結合シーケンス、Streaming 中 | `UnicodeErrorKind::Decode` + `effect {mut}` | `reports/spec-audit/ch1/text_builder_streaming-*.json` | @core-text | Pending | Capability `core.text.audit` のログ一致を確認 |
| TA-04 | `prepare_identifier` 連携 | Lexer で非正規化の識別子 | `UnicodeErrorKind::InvalidIdentifier` → `ParseErrorKind::InvalidToken` | `compiler/frontend/tests/lexer_unicode_identifier.rs`, `reports/spec-audit/ch1/lexer_unicode_identifier-20270329.json` | @core-text | Green | `lex.identifier_locale`・Bidi 制御・非 NFC・UnsupportedLocale を 10 ケースで検証し、`unicode.error.*` メタデータと `TokenKind::Unknown` の整合を確認 |
| TA-05 | `decode_stream` I/O 障害 | `Reader` が `UnexpectedEof` を返す | `UnicodeErrorKind::DecodeFailure` + `phase=io.decode.eof`、`InvalidSequenceStrategy` が `Error` のまま維持される | `compiler/runtime/tests/text_stream.rs`, `cargo test --manifest-path compiler/runtime/Cargo.toml text_stream` | @core-text | Green | `map_io_error` で `IoErrorKind::UnexpectedEof` → `phase=io.decode.eof` を強制し、`take_text_effects_snapshot` で `effect.io=true` を検証 |
| TA-06 | `decode_stream` 置換経路 | `%FF` をチャンク境界で分割し `replace` を指定 | 書記素列に `�` が挿入され、`effect {unicode}` が記録される | `compiler/runtime/tests/text_stream.rs`, `compiler/runtime/examples/io/text_stream_decode.rs` | @core-text | Green | `TextDecodeOptions::with_buffer_size(4)` + `InvalidSequenceStrategy::Replace` で `unicode=true` を観測し、CLI JSON でも `effects.unicode` を出力 |

## TODO
- [x] ケース追加: `decode_stream` I/O エラー（`IOErrorKind::UnexpectedEof` → `UnicodeErrorKind::Decode`）
- [x] ケース追加: ケース変換で `UnsupportedLocale` を返す tr-TR / az-Latn 向けシナリオ（`compiler/runtime/tests/unicode_case_width.rs` で検証済み）
