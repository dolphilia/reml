# サードパーティライセンス告知

この文書は Reml リポジトリで使用する外部データ／ソフトウェアのライセンス情報を集約します。再配布物の更新時は、該当セクションへ追記してください。

## Unicode データ（Unicode Character Database）

- **提供元**: Unicode, Inc.
- **利用ファイル**: `DerivedCoreProperties.txt`, `UnicodeData.txt`, `PropList.txt`
- **バージョン**: 15.1.0（`compiler/ocaml/third_party/unicode/15.1.0/` に配置）
- **ライセンス**: [Unicode License Agreement – Data Files and Software](https://www.unicode.org/copyright.html)（SPDX: `Unicode-Derived-Core-Properties-1.0`）
- **用途**: `scripts/unicode/generate-xid-tables.py` による `compiler/ocaml/src/lexer_tables/unicode_xid_tables.ml` の生成
- **備考**:
  - 生成結果のメタデータは `compiler/ocaml/src/lexer_tables/unicode_xid_manifest.json` に記録されます。
- `scripts/unicode/fetch-unicode-data.sh` を用いるか、Unicode 公式サイトから直接ダウンロードしたファイルを `third_party/unicode/<version>/` 以下に配置してください。
- データ更新時は当文書および manifest の `unicode_version` を更新し、`dune build @check-unicode-tables` で差分を検証してください。

## Unicode Conformance テストデータ

- **提供元**: Unicode, Inc.
- **利用ファイル**:
  - `tests/data/unicode/UAX29/GraphemeBreakTest-15.1.0.txt`
  - `tests/data/unicode/UAX15/NormalizationTest-15.1.0.txt`
  - `tests/data/unicode/UCD/EastAsianWidth-15.1.0.txt`
- **バージョン**: 15.1.0
- **ライセンス**: [Unicode License Agreement – Data Files and Software](https://www.unicode.org/copyright.html)
- **用途**: Core.Text / Core.Unicode 実装の準拠テスト（`cargo test unicode_conformance --features unicode_full`, `unicode_width_mapping`, `text_normalization_metrics`）
- **備考**:
  - 取得・更新ログは `docs/notes/text/unicode-upgrade-log.md` に記録。
  - `THIRD_PARTY_LICENSES.md` に概要表を掲載。詳細を更新した場合は本節と両方に反映する。

---

他のサードパーティ資産を追加した場合は、以下のテンプレートに従って追記してください。

```
## ライブラリ／データ名
- **提供元**:
- **利用ファイル**:
- **バージョン**:
- **ライセンス**:
- **用途**:
- **備考**:
```
