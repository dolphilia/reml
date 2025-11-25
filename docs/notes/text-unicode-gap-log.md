# Text & Unicode ギャップログ

Core.Text/Unicode 仕様と実装の差分、調査結果、フォローアップを記録する。3-3 計画や Phase 3 KPI 更新時に参照する。

## 記入フォーマット
| 日付 | 区分 | 概要 | 影響範囲 | 対応状況 | チケット/リンク |
| --- | --- | --- | --- | --- | --- |

### 例
| 2025-11-20 | API 差分 | `TextBuilder::push_grapheme` が未実装 | `compiler/rust/runtime/src/text/builder.rs` | Pending | docs/plans/bootstrap-roadmap/3-3-core-text-unicode-plan.md#2.3 |

## TODO
- [ ] 既存の差分調査ノート（Phase 2）から Unicode 関連の項目を移設する。
- [ ] エントリごとに `docs/plans/bootstrap-roadmap/assets/text-unicode-api-diff.csv` の該当行をリンクする。
