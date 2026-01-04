# 書記素セグメンテーション手法比較メモ

## 0. 目的と要件整理
- `segment_graphemes` は UAX #29 に準拠し `Iter<Grapheme>` を返すこと（docs/spec/3-3-core-text-unicode.md:105-112）。
- `grapheme_width`/`GraphemeSeq::width`/`width_map` を通じて表示幅を一貫して算出し、Diagnostics の下線計算や `log_grapheme_stats` の監査出力と同期させる（docs/spec/3-3-core-text-unicode.md:78-144,135-138）。
- Phase 3 計画 (docs/plans/bootstrap-roadmap/3-3-core-text-unicode-plan.md#1.3) で求められる `IndexCache` と `log_grapheme_stats` の `cache_hits/cache_miss` に接続できる内部 API を備える。
- 将来的に `Core.Parse`／`Core.Diagnostics` の Stage/Capability 監査へ同じアルゴリズム/辞書を提供する必要がある。

## 1. 比較表（Segmentation + Width 計測の組み合わせ）
| 候補 | セグメンテーション手段 | 幅計測手段 | ライセンス | 主な利点 | 想定リスク | 評価状況 |
| --- | --- | --- | --- | --- | --- | --- |
| A: `unicode-segmentation` + `unicode-width` | `unicode-segmentation` crate (MIT/Apache-2.0) | `unicode-width` crate (MIT/Apache-2.0) | OSS (MIT/Apache-2.0) | Rust Native で軽量、UAX #29 テスト同梱、依存が小さい | 幅テーブルが Unicode 最新版に追従するまでギャップがある／Segmenter がキャッシュ API を持たない | 推奨案（Phase3 短期導入） |
| B: ICU4X segmenter + display width | `icu_segmenter` (icu4x) | `icu_displaywidth` | Unicode License | 公式辞書・ロケール対応が充実、幅計測も ICU ベースで揃う | バイナリサイズ・データパック管理コスト、`DataProvider` 設計が必要、Phase3 スケジュールに重い | Phase4 以降の拡張候補 |
| C: 自前 DFA + 生成テーブル + wcwidth テーブル | 自前生成 DFA（UAX #29 ルールをビルド済みテーブルへ展開） | 東アジア幅テーブル/emoji width をカスタム実装 | - | 完全制御・`IndexCache` に最適化した API を提供できる | 表生成ツール・CI 同期・Unicode バージョンアップ時の負荷が大きい | 研究継続（docs/notes/text/text-unicode-performance-investigation.md と連携） |

## 2. 候補詳細

### 2.1 A: `unicode-segmentation` + `unicode-width`
- 実装案
  - `compiler/runtime/src/text/grapheme.rs`（新設予定）で `unicode_segmentation::UnicodeSegmentation::grapheme_indices` をラップし、`GraphemeSeq` を `Vec<Grapheme>` へ収集。
  - 幅計測は `unicode_width::UnicodeWidthStr` / `UnicodeWidthChar` をラップして `grapheme_width`・`GraphemeSeq::width` に利用。`WidthMode` (Narrow/Wide) は `width_map` のモードに合わせる。
  - `log_grapheme_stats` 用に `(cluster_len, display_width, script, emoji_flag)` を計測し `cache_hits/cache_miss` を更新。キャッシュは `Vec<usize>`（コードポイント→Grapheme 先頭オフセット）を保持し、Segmenter の結果を再利用する。
- 評価
  - pros: 依存が2 crate のみ、no_std サポートもあり。UAX #29 Conformance テストを `unicode-segmentation` 内部から再利用可能。
  - cons: 幅テーブル更新は crate リリース待ちになる。`unicode-width` は East Asian Width ベースで emoji width が1扱いとなるケースがあり、追加の修正テーブルが必要。
  - 対応策: emoji width 差分を `docs/notes/text/text-case-width-gap.md` と同期し、`log_grapheme_stats` で `avg_width` を監視。Unicode アップグレード時は `cargo update -p unicode-segmentation -p unicode-width` を `docs/notes/text/unicode-upgrade-log.md` に記録。

### 2.2 B: ICU4X `icu_segmenter` + `icu_displaywidth`
- 実装案
  - `icu_segmenter::GraphemeClusterBreakIteratorLatin1/Utf8` を用い、`DataProvider` と Unicode データパックを `runtime/assets/unicode/` に格納。`icu_displaywidth::DisplayWidthFormatter` で幅計測を行い、ロケール別幅調整を `WidthMode::Locale(LocaleId)` 風に拡張。
  - `Core.Runtime` 側で DataProvider のホットリロードやバージョンチェックを `CapabilityRegistry` に登録し、`log_grapheme_stats` に `unicode.version` を記録する。
- 評価
  - pros: 公式 Unicode データと同一の結果となり、幅計測やロケール対応（トルコ語など）をまとめて解決できる。
  - cons: DataProvider のサイズが数 MB～になり Phase3 時点ではバイナリ膨張が懸念。ICU4X API の安定度を監視し続ける必要がある。CI でのビルド時間増。
  - 適用タイミング: Phase4 `docs/plans/bootstrap-roadmap/4-0-phase4-migration.md` でのマルチターゲット検証や `docs/notes/text/unicode-upgrade-log.md` に沿ったバージョンピン留め時に評価。

### 2.3 C: 自前 DFA + 生成テーブル
- 実装案
  - `tools/unicode-table-gen/`（新設予定）で UAX #29 ルールと Unicode Character Database を解析し、グラフェム境界クラスの遷移表を生成。Rust では `&'static [u8]`＋`match` で最適化。
  - 幅計測は東アジア幅(W) + emoji ZWJ シーケンスのテーブルを `build.rs` で生成し `unicode-width` へ依存しない構成にする。
- 評価
  - pros: `IndexCache` への直接アクセス（例: Grapheme boundary + display width + script) を1パスで得られる。`log_grapheme_stats` の `cache_miss` 定義をより厳密にできる。
  - cons: Unicode バージョンごとに独自生成を維持する必要があり、CI/レビュー負荷が高い。生成ツール側のテストと監査が必要。
  - 適用方針: Phase3 では研究メモレベルに留め、`docs/notes/text/text-unicode-performance-investigation.md` で性能差分を計測してから採択可否を決定。

## 3. 推奨構成（2025-11-25 時点）
1. **短期 (Phase3 M3)**: 候補 A を採用。`unicode-segmentation` + `unicode-width` を runtime へ組み込み、GraphemeSeq/width API を実装。表示幅ギャップは `text-case-width-gap.md` と `text-unicode-known-issues.md` へ記録。
2. **中期 (Phase3 後半)**: `unicode-segmentation` の結果を `IndexCache` へ保存し、`tooling/ci/collect-iterator-audit-metrics.py --section text` で `cache_hits/cache_miss` をモニタリング。`reports/spec-audit/ch1/core_text_grapheme_stats.json` をゴールデン化。
3. **長期 (Phase4)**: ICU4X もしくは自前 DFA への切替を再評価。マルチロケール幅表示や CLI/LSP 連携要件（docs/plans/bootstrap-roadmap/4-0-phase4-migration.md）を踏まえて決定。

## 4. TODO
- [x] `unicode-segmentation` + `unicode-width` を利用した PoC を `compiler/runtime/src/text/grapheme.rs` へ実装し、`reports/spec-audit/ch1/grapheme_poc-20251125.md` を作成。
- [ ] `unicode-width` の emoji 幅差分を `docs/notes/text/text-case-width-gap.md` に一覧化し、`width_map` の補正テーブル要否を判断。
- [ ] ICU4X PoC を `examples/text/icux_grapheme.rs` に整理し、データパックのサイズ/ビルド時間を測定。
- [ ] 自前 DFA 生成ツールの仕様を `tools/unicode-table-gen/README.md` にまとめ、UAX #29 参照元とライセンス表記を追記。

## 5. Phase3 W41 実装サマリ
- `compiler/runtime/src/text/grapheme.rs` に `ScriptCategory`/`TextDirection` のヒューリスティクスを導入し、Latin/Han/Kana/Arabic/Emoji/Other の 6 バケットで script mix ratio と `rtl_ratio` を算出。`GraphemeSeq::stats` が `log_grapheme_stats` と `reports/spec-audit/ch1/core_text_grapheme_stats.json` に `primary_script` を書き出す。
- `GraphemeSeq` が `IntoIterator`（`DoubleEndedIterator`/`ExactSizeIterator`）とランダムアクセス API (`byte_offset_at`, `grapheme_at_byte_offset`) を備えたため、Diagnostics 側の幅計算と `width_map` が再走査なしで同期できる。
- UAX #29 rev.40 の GraphemeBreakTest データを `tests/data/unicode/UAX29/GraphemeBreakTest-15.1.0.txt` として同梱し、`cargo test --manifest-path compiler/runtime/Cargo.toml unicode_conformance --features unicode_full` でセグメンテーション互換性を確認。データ投入は `docs/notes/text/unicode-upgrade-log.md` に記録済み。
