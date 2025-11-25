# ケース変換・文字幅ギャップ記録

## 目的
ICU / Unicode 標準との挙動差を記録し、`text-locale-support.csv` と `unicode-error-mapping.md` で参照できるようにする。

## ギャップ一覧
| 日付 | ロケール / スクリプト | 機能 | 現状挙動 | 期待挙動 | 対応状況 | 備考 |
| --- | --- | --- | --- | --- | --- | --- |
| 2025-11-21 | tr-TR | `to_upper("i")` | `I`（ドット無し）を返せない → `UnsupportedLocale` | ドット付き I へ変換 | Pending | Parser で禁止し Diagnostics で提案文を出す |
| 2025-11-21 | ja-JP | `width_map("ｱ")` | 半角→全角 OK、全角→半角は未実装 | 双方向対応 | Planned | assets/text-locale-support.csv 行 1 を参照 |
| 2025-11-25 | emoji, ja-JP | `width_map("👨‍👩‍👧‍👦")` | `unicode-width` では幅 2 固定。CLI では 4+ に見えるケースあり。 | `WidthMode::EmojiCompat` で補正テーブルを適用し、絵文字シーケンスは `grapheme_stats` の計測幅と一致させる | Planned | `unicode-segmentation` PoC (`compiler/rust/runtime/src/text/grapheme.rs`) の統計値と比較 |
| 2025-11-25 | emoji, narrow | `width_map("🇯🇵")` | Regional Indicator ペアが幅 2 だが、LSP 表示では 4 カラム消費 | `width_map(mode = Wide)` で East Asian Width (W/A) に従いつつ、`EmojiZw` テーブルで 4 カラムを許可 | Planned | `unicode-width` では East Asian Ambiguous を 1 として扱うため、`width_corrections.csv` を追加予定 |

## TODO
- [ ] East Asian Width 表を取り込み、`width_map` API で `effect {mem}` を計測する。
- [ ] ケース変換例外を `docs/guides/ai-integration.md` の FAQ に追加。
- [ ] `width_corrections.csv`（emoji / ZWJ / regional indicator 向け補正）を `docs/plans/bootstrap-roadmap/assets/` に追加し、`width_map` の実装で参照する。
