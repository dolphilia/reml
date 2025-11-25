# 書記素セグメンテーション手法比較メモ

## 目的
Core.Text の `segment_graphemes` 実装方針を決める際に候補となるアルゴリズム・ライブラリを比較し、性能/互換性/実装コストを評価する。

## 比較表
| 候補 | ライセンス | 主な利点 | 想定リスク | 評価状況 | 備考 |
| --- | --- | --- | --- | --- | --- |
| `unicode-segmentation` crate | MIT/Apache-2.0 | 実績豊富、UAX #29 テスト同梱 | `no_std` 対応に制約、キャッシュ API が無い | 調査中 | Rust Native 実装を再利用予定 |
| ICU break iterator (via icu4x) | Unicode | 公式データ、ロケール対応充実 | バイナリサイズ、FFI レイヤが必要 | 未評価 | Phase 4 再検討 |
| 自前 DFA + 表生成 | - | 完全制御、キャッシュ最適化可能 | 生成ツール整備が必要 | 研究メモ作成中 | `docs/notes/text-unicode-performance-investigation.md` と連携 |

## TODO
- [ ] `unicode-segmentation` 最新版でのベンチ結果を `benchmarks/text/grapheme.rs` に追加。
- [ ] ICU4X PoC のリンクを `reports/spec-audit/ch1/grapheme_poc-*.md` に追記。
- [ ] 表生成アプローチのフォーマットを `tools/unicode-table-gen/README.md` と同期。
