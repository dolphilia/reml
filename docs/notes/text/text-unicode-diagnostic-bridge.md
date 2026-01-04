# Text/Unicode Diagnostics ブリッジメモ

## 目的
- `UnicodeError` → `ParseError` → `FrontendDiagnostic` → JSON/Audit のパイプラインで欠落しているメタデータ（Span/AuditEnvelope/効果タグ）を補完する。
- `docs/plans/bootstrap-roadmap/3-3-core-text-unicode-plan.md` §4.1 で定義したタスクを実装フェーズと連携させ、`unicode_error_to_parse_error` の変換規約を可視化する。

## 主要データ構造
| 型 | 定義箇所 | 利用箇所 | メモ |
| --- | --- | --- | --- |
| `Span` | `compiler/frontend/src/span.rs`【F:../../compiler/frontend/src/span.rs†L7-L45】 | `ParseError`, `DiagnosticSpanLabel`, `DiagnosticFixIt` | 半開区間 `[start, end)`。`UnicodeError::offset` から `Span::new(offset, offset+len)` を生成する。 |
| `SpanTagged<T>` | 同上【F:../../compiler/frontend/src/span.rs†L47-L70】 | `ExpectedToken`, `StreamTrace` | `TextBuilder`/Streaming decode で `Grapheme` と `Span` を同時に保持する。 |
| `ParseError` | `compiler/frontend/src/parser/api.rs`【F:../../compiler/frontend/src/parser/api.rs†L104-L152】 | Parser → Diagnostic 変換 | `unicode: Option<UnicodeError>` / `span_trace: Vec<Span>` を追加予定。 |
| `AuditEnvelope` | `compiler/frontend/src/diagnostic/mod.rs`【F:../../compiler/frontend/src/diagnostic/mod.rs†L22-L57】 | `FrontendDiagnostic` JSON 出力 | `metadata` に `unicode.error.*` を記録し、`audit_id`/`change_set` と突合する。 |
| `FrontendDiagnostic` | `compiler/frontend/src/diagnostic/mod.rs`（下部） | CLI/LSP JSON | `expected`, `notes`, `extensions` に Unicode 情報を展開。 |
| `UnicodeError` | `compiler/runtime/src/text/error.rs`【F:../../compiler/runtime/src/text/error.rs†L1-L54】 | Text API, Parser, IO | `kind`, `message`, `offset`, `phase` を提供。`with_phase` で `parser`, `io`, `builder` を識別する。 |

## スキーマ連携
1. **UnicodeError → ParseError**
   - `UnicodeError::offset` を `Span` に変換し、`ParseError.at` または `span_trace` に格納する。
   - `UnicodeErrorKind` を `ParseError.notes` に `unicode.error.kind` プレフィックスで記録し、後段が `DiagnosticNote` に昇格できるようにする。
2. **ParseError → Diagnostic**
   - `ExpectedToken` / `ExpectedTokensSummary` を `expected` フィールドにマップする（既存実装）。`unicode_error_to_parse_error` では `context`/`notes` に Unicode 情報を追加し、`FrontendDiagnostic.extensions["unicode"]` に同じ JSON を複写する。
   - `AuditEnvelope.metadata` に `unicode.error.kind`, `unicode.error.offset`, `unicode.error.phase`, `unicode.effect.mem_bytes`, `unicode.locale.*` を同時に書き込み、`collect-iterator-audit-metrics.py --section text` の KPI で検証する。
3. **Diagnostic → JSON/LSP**
   - `LineIndex`（`diagnostic/json.rs`）を使って `Span` から `line`/`column` を算出し、`unicode.display_width` などの属性を `extensions` に配置する。
   - JSON Schema 更新時は `docs/spec/3-6-core-diagnostics-audit.md` の付録に `unicode.*` キーの説明を追加し、`schema.version` をインクリメントする。

### 進捗ログ（2027-03-29）
- `compiler/frontend/tests/lexer_unicode_identifier.rs` を 12 ケースに拡張し、`UnicodeErrorKind::{InvalidIdentifier,UnsupportedLocale,InvalidUtf8}` が `Span`・`AuditEnvelope.metadata["unicode.error.*"]` と揃って出力されることを確認。結果は `reports/spec-audit/ch1/lexer_unicode_identifier-20270329.json` に保存した。
- KPI `unicode.diagnostic.display_span` を `0-3-audit-and-metrics.md` に登録し、`scripts/validate-diagnostic-json.sh --pattern unicode.error.kind --pattern unicode.identifier.raw` を通じて `ParseError`→`Diagnostic` の橋渡しを自動チェックする運用を開始。
- `display_width` は `Diagnostic.pretty` 再実装（`docs/plans/bootstrap-roadmap/2-4-diagnostics-audit-pipeline.md` §1.4/§2.3）で扱うため、本メモでは `Span`/列オフセットの変換規約のみ確定とし、幅計算は `TODO` として残す。

### 進捗ログ（2027-03-30）
- `FrontendDiagnostic`/`ParseError` に `UnicodeDetail` を追加し、Lexer で検出した `UnicodeError` が raw/locale/profile 付きで伝搬できるようになった。【F:../../compiler/frontend/src/diagnostic/mod.rs†L244-L276】【F:../../compiler/frontend/src/parser/api.rs†L104-L159】
- `diagnostic/unicode.rs` の `integrate_unicode_metadata` が `extensions["unicode"]` と `AuditEnvelope.metadata["unicode.*"]` を同時更新し、`unicode.display_width`・`unicode.grapheme.start/end` を含めて JSON/Audit 双方に出力する仕組みを実装した。【F:../../compiler/frontend/src/diagnostic/unicode.rs†L1-L223】
- `reports/spec-audit/ch1/unicode_diagnostics-20270330.json` を追加し、`scripts/validate-diagnostic-json.sh --pattern unicode.display_width reports/spec-audit/ch1/unicode_diagnostics-20270330.json` で display_width/GraphemeSpan の書き出しを検証できるようにした。

## TODO
- ~~[ ] `ParseError` へ `unicode: Option<UnicodeError>` を追加し、`parser::State` が `UnicodeError` を受け取れるようにする。~~（2027-03-30 完了）
- ~~[ ] `FrontendDiagnostic` から `AuditEnvelope` へのコピーで `unicode.*` を標準化する。~~（`integrate_unicode_metadata` で対応）
- [ ] `scripts/validate-diagnostic-json.sh --pattern unicode.error.kind` を追加し、CI での欠落検知を自動化。
