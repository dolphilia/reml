# Unicode Conformance テストチェックリスト

## 目的
UAX #29 (書記素境界) / UAX #15 (正規化) などの準拠状況をトラッキングし、CI での自動検証とリスク管理を容易にする。

## テストマトリクス
| ID | 規格 / バージョン | 対象 API | データソース | 検証手段 | KPI | 状況 | 備考 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| UCNF-29 | UAX #29 rev.40 | `segment_graphemes`, `TextBuilder::finish` | `tests/data/unicode/UAX29/GraphemeBreakTest-15.1.0.txt` | `cargo test unicode_conformance --features unicode_full` | `100% pass` | 緑 (2027-03-31) | Unicode 15.1 GraphemeBreakTest 本体を同梱（Unicode License）。`docs/notes/text/unicode-upgrade-log.md` と同期 |
| UCNF-15 | UAX #15 rev.72 | `normalize_{nfc,nfd,nfkc,nfkd}` | `tests/data/unicode/UAX15/NormalizationTest-15.1.0.txt` | `cargo test --manifest-path compiler/runtime/Cargo.toml unicode_conformance --features unicode_full` | `100% pass` | 緑 (2027-03-31) | 正規化コスト計測（`cargo run --example text_normalization_metrics -- --output reports/text-normalization-metrics.json` → `collect-iterator-audit-metrics.py --scenario normalization_conformance`）と併せて nightly ジョブで実行 |
| UCNF-Case | Unicode 15.1 CaseFolding | `to_upper`, `to_lower`, `prepare_identifier` | `third_party/unicode/CaseFolding.txt` | `cargo test --manifest-path compiler/runtime/Cargo.toml unicode_case_width` | `99.9%+ pass` | 進行中 | ランタイム側で tr-TR/az-Latn/und を検証済み。CaseFolding.txt の全量比較は次フェーズで追加 |
| UCNF-Width | East Asian Width 15.1 | `width_map` | `tests/data/unicode/UCD/EastAsianWidth-15.1.0.txt` | `cargo test --manifest-path compiler/runtime/Cargo.toml unicode_width_mapping` | `100% pass` | 進行中 | `width_corrections.csv` で Emoji 例外を補正し、W/F/A クラスをフルスキャン |

## 手順
1. テスト追加時に `docs/notes/text/unicode-upgrade-log.md` へバージョンとコミットを記録する。
2. 失敗ケースは `reports/spec-audit/ch1/unicode_conformance_failures.md` に残し、`docs/plans/bootstrap-roadmap/0-4-risk-handling.md` にリスク登録する。
3. すべての行が `Green` になるまで Phase 3.3 リリースを凍結する。
