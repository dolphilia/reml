# Unicode 公開データ

このディレクトリには Unicode Consortium が公開している UAX #29 等のテスト
データを格納しています。現在の構成:

| ファイル | バージョン | 出典 | 用途 |
| --- | --- | --- | --- |
| `UAX29/GraphemeBreakTest-15.1.0.txt` | Unicode 15.1.0 | https://www.unicode.org/Public/15.1.0/ucd/auxiliary/GraphemeBreakTest.txt | `compiler/runtime/tests/grapheme_conformance.rs` / `unicode_conformance_grapheme` が参照する Grapheme Cluster Break の完全テストベクタ |
| `UAX15/NormalizationTest-15.1.0.txt` | Unicode 15.1.0 | https://www.unicode.org/Public/15.1.0/ucd/NormalizationTest.txt | `compiler/runtime/tests/normalization_conformance.rs` / `text_normalization_metrics.rs` が参照する UAX #15 Normalization Test Suite |
| `UCD/EastAsianWidth-15.1.0.txt` | Unicode 15.1.0 | https://www.unicode.org/Public/15.1.0/ucd/EastAsianWidth.txt | `compiler/runtime/src/text/width.rs` / `tests/unicode_width_mapping.rs` が参照する幅クラス定義 |

- ライセンス: [Unicode License Agreement - Data Files and Software](https://www.unicode.org/license.txt)
- 取得日: 2027-03-30 (UTC)
- 追加・更新時は `docs/notes/text/unicode-upgrade-log.md` にバージョンとコミット ID を記録し、該当ファイルのパスを更新してください。

## 更新手順
1. https://www.unicode.org/Public/ にある最新リリースから対象バージョンをダウンロード。
2. `tests/data/unicode/<UAX>/` 以下に `GraphemeBreakTest-<version>.txt` のようなバージョン付きファイル名で保存。UCD ベースのファイルは `tests/data/unicode/UCD/` に配置する。
3. `docs/notes/text/unicode-upgrade-log.md` と `docs/plans/bootstrap-roadmap/checklists/unicode-conformance-checklist.md` を更新し、テストが参照するパスを差し替える。
4. `cargo test --manifest-path compiler/runtime/Cargo.toml unicode_conformance --features unicode_full` を実行して互換性を確認。必要に応じて `-- --ignored` を併用してその他の Unicode テストも検証する。
