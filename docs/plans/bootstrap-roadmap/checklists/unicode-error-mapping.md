# UnicodeError → Diagnostics/Parser 変換マッピング

## 方針
- `UnicodeErrorKind` ごとに Parser/Diagnostics/IO での最終エラーを記録し、差分が発生した場合は `docs/plans/bootstrap-roadmap/2-4-diagnostics-audit-pipeline.md` に反映する。
- 参照元: `docs/spec/3-3-core-text-unicode.md`, `docs/spec/2-3-lexer.md`, `docs/spec/3-6-core-diagnostics-audit.md`。

## マッピング表
| UnicodeErrorKind | Parser 変換 | Diagnostics 変換 | IO 変換 | 主スパン生成 | Audit/AuditEnvelope キー | 関連テスト | 状況 | 備考 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| InvalidUtf8 | `ParseErrorKind::InvalidToken` | `DiagnosticCode::unicode.invalid_utf8` | `IOErrorKind::InvalidData` | `Span::new(offset, offset + len)`（`len` は `Str::iter_graphemes` で 1 書記素分） | `unicode.error.kind=invalid_utf8`, `unicode.error.offset` | `reports/spec-audit/ch1/lexer_unicode_identifier-*.json` | Pending | `display_width` が 0 になる場合の扱いを決める |
| InvalidIdentifier | `ParseErrorKind::InvalidToken` | `DiagnosticCode::unicode.invalid_identifier` | n/a | `prepare_identifier` の `SpanTagged` から継承 | `unicode.error.kind=invalid_identifier`, `unicode.identifier.raw` | `compiler/rust/frontend/tests/lexer_unicode_identifier.rs` | Pending | `prepare_identifier` 統合テスト待ち |
| UnsupportedLocale | `ParseErrorKind::UnicodeOption` | `DiagnosticCode::unicode.unsupported_locale` | `IOErrorKind::UnsupportedLocale` | `LocaleId` 宣言箇所の `Span`（`token.span`） | `unicode.locale.requested`, `unicode.locale.supported[]` | `examples/core-text/locale_cases.reml`(予定) | Planned | `text-locale-support.csv` と同期 |
| Decode | `ParseErrorKind::UnicodeDecode` | `DiagnosticCode::unicode.decode_error` | `IOErrorKind::UnexpectedEof` | Streaming `TextBuilder` が返す `SpanTagged<Grapheme>` | `unicode.stream.chunk`, `unicode.effect.mem_bytes` | `reports/spec-audit/ch1/text_builder_streaming-*.json` | Pending | Streaming decode と `effect {audit}` を同時記録 |
| OutOfMemory | `ParseErrorKind::SystemResource` | `DiagnosticCode::unicode.alloc_failed` | `IOErrorKind::OutOfMemory` | `Span::new(offset, offset)`（ゼロ幅） | `unicode.alloc.bytes_requested`, `unicode.effect.mem_bytes` | `tooling/ci/collect-iterator-audit-metrics.py --scenario bytes_clone` | Pending | KPI を `0-3-audit-and-metrics.md` に追加 |

## TODO
- [ ] `UnicodeErrorKind::WidthMappingMissing` の新設要否を検討し、ケース追加。
- [ ] Diagnostics での `help` メッセージ案を `docs/notes/text-unicode-known-issues.md` に追記。
