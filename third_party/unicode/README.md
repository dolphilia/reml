# Unicode 公開データ

このディレクトリには Unicode Consortium が公開している UAX #29 等のテスト
データを格納しています。現在の構成:

| ファイル | バージョン | 出典 | 用途 |
| --- | --- | --- | --- |
| `UAX29/GraphemeBreakTest-15.1.0.txt` | Unicode 15.1.0 | https://www.unicode.org/Public/15.1.0/ucd/auxiliary/GraphemeBreakTest.txt | `compiler/rust/runtime/tests/grapheme_conformance.rs` が参照する Grapheme Cluster Break の完全テストベクタ |
| `UCD/NormalizationTest-15.1.0.txt` | Unicode 15.1.0 | https://www.unicode.org/Public/15.1.0/ucd/NormalizationTest.txt | `compiler/rust/runtime/tests/normalization_conformance.rs` が参照する UAX #15 Normalization Test Suite |

- ライセンス: [Unicode License Agreement - Data Files and Software](https://www.unicode.org/license.txt)
- 取得日: 2027-03-30 (UTC)
- 追加・更新時は `docs/notes/unicode-upgrade-log.md` にバージョンとコミット ID を記録し、該当ファイルのパスを更新してください。

## 更新手順
1. https://www.unicode.org/Public/ にある最新リリースから対象バージョンをダウンロード。
2. `third_party/unicode/<UAX>/` 以下に `GraphemeBreakTest-<version>.txt` のようなバージョン付きファイル名で保存。
3. `docs/notes/unicode-upgrade-log.md` と `docs/plans/bootstrap-roadmap/checklists/unicode-conformance-checklist.md` を更新し、テストが参照するパスを差し替える。
4. `cargo test --manifest-path compiler/rust/runtime/Cargo.toml grapheme_conformance -- --ignored` を実行して互換性を確認。
