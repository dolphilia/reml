# Reml 改善注目マトリクス

この資料は `samples/language-impl-comparison/` に含まれる既存サンプルの観点から、Reml 仕様で重点的に補強すべき領域を整理したものです。比較対象言語や実装予定リストを踏まえ、仕様書の該当章を参照しながら改善タスクの優先度付けに利用できます。

| 観点 | 参考実装/根拠 | Reml 現状から読み取れるポイント | 仕様改善に向けた着眼点 | 関連章 |
| - | - | - | - | - |
| 字句解析とトークナイズ | `samples/language-impl-comparison/reml/json_parser.reml:49` | 手続き的トークナイザーで `Text.char_at` と `List.push_back` を逐次併用。`read_*` 系はダミー実装で TODO が残存。 | `Core.Text` と `Core.Lex` の責務分担、Unicode・数値リテラル正規化、エンコーディングエラー時の診断指針を仕様で明文化する。 | 1-1, 1-3, 3-3, 3-5 |
| パーサーコンビネーター利用 | `samples/language-impl-comparison/reml/json_parser_combinator.reml:16` | `Core.Parse.rule/choice/attempt` による宣言的定義が揃っているが、バックトラック戦略やメモ化要否は暗黙。 | Parser API のエラー優先順位・性能特性・ストリーミング互換を Chapter 2 で規定し、手続き実装との選択基準を提示。 | 2-0, 2-2, guides/core-parse-streaming.md |
| 効果ハンドリング戦略 | `samples/language-impl-comparison/reml/algebraic_effects.reml:32` | `with State/Except/Choose` とハンドラー合成で多効果を明示。コメントで Haskell/Rust との対比が既に記載。 | 効果順序と型推論の相互作用、部分的ハンドル時の伝播規則、`perform` の評価順序をコア仕様へ反映し、他言語比較表を補完。 | 1-3, 3-8, notes/dsl-plugin-roadmap.md |
| 診断とエラー報告 | `samples/language-impl-comparison/reml/json_parser.reml:31`, `samples/language-impl-comparison/reml/algebraic_effects.reml:195` | `Result.err` に期待値を載せるが、位置情報や複数候補の提示は未定義。 | `Core.Diagnostics` との連携を規定し、位置追跡・復旧戦略・多段効果下のエラーバブルアップ方法を整理。 | 2-5, 3-6, 3-7 |
| 標準コレクション操作 | `samples/language-impl-comparison/reml/json_parser.reml:95`, `samples/language-impl-comparison/reml/algebraic_effects.reml:133` | `List.push_back/append`, `Map.insert` などが頻出。API 仕様は暗黙のまま。 | イミュータブル構造の計算量保証、構造的等価の判定規約、`List` と `Map` の相互変換を標準ライブラリ章で補強。 | 3-1, 3-2 |
| 並行・分散モデル比較 | `samples/language-impl-comparison/README.md:12` | Elixir/BEAM, Scala 3, Nim 等が比較対象として列挙されているが、Reml 側の並行記述例は未整備。 | 予定サンプルを見据え、`Core.Async` や FFI 安全領域の仕様ドラフトを先行で充実させ、プロセス指向/Actor モデル対応を検討。 | 3-9, guides/runtime-bridges.md |

> 今後、比較サンプルが追加された際は、本マトリクスの「参考実装/根拠」列を更新し、`README.md` の新項目と整合させてください。
