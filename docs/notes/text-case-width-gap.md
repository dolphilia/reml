# ケース変換・文字幅ギャップ記録

## 目的
ICU / Unicode 標準との挙動差を記録し、`text-locale-support.csv` と `unicode-error-mapping.md` で参照できるようにする。

## ギャップ一覧
| 日付 | ロケール / スクリプト | 機能 | 現状挙動 | 期待挙動 | 対応状況 | 備考 |
| --- | --- | --- | --- | --- | --- | --- |
| 2025-11-21 | tr-TR | `to_upper("i")` | `I`（ドット無し）を返せない → `UnsupportedLocale` | ドット付き I へ変換 | Pending | Parser で禁止し Diagnostics で提案文を出す |
| 2025-11-21 | ja-JP | `width_map("ｱ")` | 半角→全角 OK、全角→半角は未実装 | 双方向対応 | Planned | assets/text-locale-support.csv 行 1 を参照 |

## TODO
- [ ] East Asian Width 表を取り込み、`width_map` API で `effect {mem}` を計測する。
- [ ] ケース変換例外を `docs/guides/ai-integration.md` の FAQ に追加。
