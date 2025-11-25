# Unicode Conformance テストチェックリスト

## 目的
UAX #29 (書記素境界) / UAX #15 (正規化) などの準拠状況をトラッキングし、CI での自動検証とリスク管理を容易にする。

## テストマトリクス
| ID | 規格 / バージョン | 対象 API | データソース | 検証手段 | KPI | 状況 | 備考 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| UCNF-29 | UAX #29 rev.40 | `segment_graphemes`, `TextBuilder::finish` | `tests/data/unicode/UAX29/NormalizationTest.txt` | `cargo test grapheme_conformance -- --ignored` | `100% pass` | Pending | `docs/notes/unicode-upgrade-log.md` と同期 |
| UCNF-15 | UAX #15 rev.72 | `normalize_{nfc,nfd,nfkc,nfkd}` | `tests/data/unicode/UAX15/NormalizationTest.txt` | `cargo test normalization_conformance -- --ignored` | `100% pass` | Pending | 大量入力は nightly job で実行 |
| UCNF-Case | Unicode 15.1 CaseFolding | `to_upper`, `to_lower`, `prepare_identifier` | `third_party/unicode/CaseFolding.txt` | `cargo test unicode_casefold` | `99.9%+ pass` | Planned | 例外は `text-case-width-gap.md` に記録 |
| UCNF-Width | East Asian Width 15.1 | `width_map` | `third_party/unicode/EastAsianWidth.txt` | `cargo test unicode_width_mapping` | `100% pass` | Planned | 代替マップは `text-locale-support.csv` 参照 |

## 手順
1. テスト追加時に `docs/notes/unicode-upgrade-log.md` へバージョンとコミットを記録する。
2. 失敗ケースは `reports/spec-audit/ch1/unicode_conformance_failures.md` に残し、`docs/plans/bootstrap-roadmap/0-4-risk-handling.md` にリスク登録する。
3. すべての行が `Green` になるまで Phase 3.3 リリースを凍結する。
