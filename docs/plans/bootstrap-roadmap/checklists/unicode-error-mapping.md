# UnicodeError → Diagnostics/Parser 変換マッピング

## 方針
- `UnicodeErrorKind` ごとに Parser/Diagnostics/IO での最終エラーを記録し、差分が発生した場合は `docs/plans/bootstrap-roadmap/2-4-diagnostics-audit-pipeline.md` に反映する。
- 参照元: `docs/spec/3-3-core-text-unicode.md`, `docs/spec/2-3-lexer.md`, `docs/spec/3-6-core-diagnostics-audit.md`。

## マッピング表
| UnicodeErrorKind | Parser 変換 | Diagnostics 変換 | IO 変換 | 主スパン生成 | Audit/AuditEnvelope キー | 関連テスト | 状況 | 備考 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| InvalidUtf8 | `ParseErrorKind::InvalidToken`（[2-3 章 D-1](../../spec/2-3-lexer.md#d-1) の `identifier` フェーズ準拠） | `DiagnosticCode::unicode.invalid_utf8` | `IOErrorKind::InvalidData` | `Span::new(offset, offset + len)`（`len` は `Str::iter_graphemes` で 1 書記素分） | `unicode.error.kind=invalid_utf8`, `unicode.error.offset` | `reports/spec-audit/ch1/lexer_unicode_identifier-20270329.json` | Pending | `display_width` が 0 になる場合の扱いを決める |
| InvalidIdentifier | `ParseErrorKind::InvalidToken`（`prepare_identifier` が `identifier` プロファイルで失敗した場合） | `DiagnosticCode::unicode.invalid_identifier` | n/a | `prepare_identifier` の `SpanTagged` から継承 | `unicode.error.kind=invalid_identifier`, `unicode.identifier.raw`, `unicode.display_width` | `compiler/frontend/tests/lexer_unicode_identifier.rs`, `reports/spec-audit/ch1/lexer_unicode_identifier-20270329.json`, `reports/spec-audit/ch1/unicode_diagnostics-20270330.json` | Green | 非正規化/禁止 Bidi/UnsupportedLocale を 10 ケース以上で検証、`span`/`audit_metadata` の整合を確認。`display_width`/`grapheme_span` は 2027-03-30 の実装で `extensions["unicode"]` へ記録済み。 |
| UnsupportedLocale | `ParseErrorKind::UnicodeOption` | `DiagnosticCode::unicode.unsupported_locale` | `IOErrorKind::UnsupportedLocale` | `LocaleId` 宣言箇所の `Span`（`token.span`） | `unicode.locale.requested`, `unicode.locale.supported[]` | `compiler/runtime/tests/unicode_case_width.rs` | In Progress | `text-locale-support.csv` と同期。Parser との統合テストは `examples/core-text/locale_cases.reml` 追加後に実施 |
| Decode | `ParseErrorKind::UnicodeDecode` | `DiagnosticCode::unicode.decode_error` | `IOErrorKind::UnexpectedEof` | Streaming `TextBuilder` が返す `SpanTagged<Grapheme>` | `unicode.stream.chunk`, `unicode.effect.mem_bytes` | `reports/spec-audit/ch1/text_builder_streaming-*.json` | Pending | Streaming decode と `effect {audit}` を同時記録 |
| OutOfMemory | `ParseErrorKind::SystemResource` | `DiagnosticCode::unicode.alloc_failed` | `IOErrorKind::OutOfMemory` | `Span::new(offset, offset)`（ゼロ幅） | `unicode.alloc.bytes_requested`, `unicode.effect.mem_bytes` | `tooling/ci/collect-iterator-audit-metrics.py --scenario bytes_clone` | Pending | KPI を `docs/guides/tooling/audit-metrics.md` に追加 |

### 変換メモ（2027-03-29 更新）
- `ParseErrorKind::InvalidToken` への写像は `docs/spec/2-3-lexer.md` §D-1 の `identifier(profile)` 規約と一致する。`prepare_identifier` で `UnicodeErrorKind::InvalidIdentifier` が発生した場合、Lexer は `TokenKind::Unknown` を挿入しつつ `ParseError` 側では `identifier` の期待集合へフォールバックする。
- `reports/spec-audit/ch1/lexer_unicode_identifier-20270329.json` では `UnicodeErrorKind::{InvalidUtf8,InvalidIdentifier,UnsupportedLocale}` が `unicode.error.*` メタデータとして記録され、`diagnostic/audit_metadata` の双方で列挙済み。`ParseError` へは `Span::new(offset, offset+grapheme_len)` を移譲し、`LineIndex` が列情報を復元できることを確認した。

## TODO
- [ ] `UnicodeErrorKind::WidthMappingMissing` の新設要否を検討し、ケース追加。
- [ ] Diagnostics での `help` メッセージ案を `docs/notes/text/text-unicode-known-issues.md` に追記。
