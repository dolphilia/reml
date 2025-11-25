# Unicode データ更新ログ

Unicode テーブルやテストデータ (`NormalizationTest.txt`, `CaseFolding.txt` など) を更新した際の履歴を管理する。

## エントリテンプレート
| 日付 | バージョン | 対象ファイル | 対応コミット | 備考 |
| --- | --- | --- | --- | --- |

### 例
| 2025-11-21 | Unicode 15.1 | `NormalizationTest.txt`, `CaseFolding.txt` | (pending) | `docs/plans/bootstrap-roadmap/checklists/unicode-conformance-checklist.md` を更新 |

### 履歴
| 日付 | バージョン | 対象ファイル | 対応コミット | 備考 |
| --- | --- | --- | --- | --- |
| 2027-03-30 | Unicode 15.1 | `third_party/unicode/UAX29/GraphemeBreakTest-15.1.0.txt` | (pending) | UAX #29 GraphemeBreakTest 正式データ（Unicode License）を追加し、`cargo test grapheme_conformance -- --ignored` のゴールデンとして利用 |

## TODO
- [ ] 既存の `third_party/unicode/` 更新履歴を洗い出し、この表へ移行する。
- [ ] テーブル生成スクリプトのハッシュ値を欄に追加するか検討。
