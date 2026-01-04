# サードパーティライセンス一覧

Reml リポジトリで再配布している主要サードパーティデータのライセンス情報を以下にまとめる。詳細な履歴や付随メモは `docs/THIRD_PARTY_NOTICES.md` も参照すること。

| 名称 | 対象ファイル | バージョン | 配置パス | ライセンス / 出典 | 用途 |
| --- | --- | --- | --- | --- | --- |
| Unicode UAX #29 GraphemeBreakTest | `GraphemeBreakTest-15.1.0.txt` | Unicode 15.1.0 | `tests/data/unicode/UAX29/` | [Unicode License Agreement - Data Files and Software](https://www.unicode.org/license.txt) / https://www.unicode.org/Public/15.1.0/ucd/auxiliary/GraphemeBreakTest.txt | `Core.Text` の書記素分割テスト (`cargo test unicode_conformance --features unicode_full`) および Grapheme 統計レポート生成 |
| Unicode UAX #15 NormalizationTest | `NormalizationTest-15.1.0.txt` | Unicode 15.1.0 | `tests/data/unicode/UAX15/` | 同上 / https://www.unicode.org/Public/15.1.0/ucd/NormalizationTest.txt | 正規化 API (`Text.normalize_*`) および `text_normalization_metrics` 例の性能検証 |
| Unicode EastAsianWidth | `EastAsianWidth-15.1.0.txt` | Unicode 15.1.0 | `tests/data/unicode/UCD/` | 同上 / https://www.unicode.org/Public/15.1.0/ucd/EastAsianWidth.txt | `width_map` 実装と `unicode_width_mapping` テストの幅クラス定義 |

- 取得日: 2027-03-30 (UTC)
- 作業ログ: `docs/notes/text/unicode-upgrade-log.md` にバージョンとコミット ID を記録済み。
- 追加で Unicode データを導入する場合は、上表に行を追加し、本ファイルと `docs/THIRD_PARTY_NOTICES.md` の双方を更新すること。

