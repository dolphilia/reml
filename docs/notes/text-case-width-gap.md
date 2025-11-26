# ケース変換・文字幅ギャップ記録

## 目的
ICU / Unicode 標準との挙動差を記録し、`text-locale-support.csv` と `unicode-error-mapping.md` で参照できるようにする。

## ギャップ一覧
| 日付 | ロケール / スクリプト | 機能 | 現状挙動 | 期待挙動 | 対応状況 | 備考 |
| --- | --- | --- | --- | --- | --- | --- |
| 2027-03-29 | tr-TR | `to_upper("i")` | LocaleId 経由で `İ`/`ı` を再現済み | double-ended casing（`LocaleId::parse(tr-TR)`）が `to_upper`/`to_lower` で安定 | Closed | `compiler/rust/runtime/src/text/case.rs`・`tests/unicode_case_width.rs` 参照 |
| 2027-03-29 | az-Latn | `to_upper("i")` | `UnsupportedLocale` を返す（fallback=tr-TR） | トルコ語系の追加検証後に tr-TR と同じ分岐を有効化 | Planned | Parser 側で `unicode.locale.requested=az-Latn` を検証 |
| 2027-03-29 | ja-JP | `width_map("ｱ")` | 半角/全角ともに `width_map` で相互変換・統計収集済み | emoji/ZWS補正との整合 | Closed | `compiler/rust/runtime/src/text/width.rs` の `KANA_TABLE` 実装 |
| 2025-11-25 | emoji, ja-JP | `width_map("👨‍👩‍👧‍👦")` | `unicode-width` では幅 2 固定。CLI では 4+ に見えるケースあり。 | `WidthMode::EmojiCompat` で補正テーブルを適用し、絵文字シーケンスは `grapheme_stats` の計測幅と一致させる | Planned | `unicode-segmentation` PoC (`compiler/rust/runtime/src/text/grapheme.rs`) の統計値と比較 |
| 2025-11-25 | emoji, narrow | `width_map("🇯🇵")` | Regional Indicator ペアが幅 2 だが、LSP 表示では 4 カラム消費 | `width_map(mode = Wide)` で East Asian Width (W/A) に従いつつ、`EmojiZw` テーブルで 4 カラムを許可 | Planned | `unicode-width` では East Asian Ambiguous を 1 として扱うため、`width_corrections.csv` を追加予定 |

## TODO
- [ ] East Asian Width 表を取り込み、`width_map` API で `effect {mem}` を計測する。
- [ ] ケース変換例外を `docs/guides/ai-integration.md` の FAQ に追加。
- [ ] `width_corrections.csv`（emoji / ZWJ / regional indicator 向け補正）を `docs/plans/bootstrap-roadmap/assets/` に追加し、`width_map` の実装で参照する。
