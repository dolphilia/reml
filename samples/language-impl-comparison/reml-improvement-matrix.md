# Reml 改善注目マトリクス

この資料は `samples/language-impl-comparison/` に含まれる既存サンプルの観点から、Reml 仕様で重点的に補強すべき領域を整理したものです。比較対象言語や実装予定リストを踏まえ、仕様書の該当章を参照しながら改善タスクの優先度付けに利用できます。

| 観点 | 参考実装/根拠 | Reml 現状から読み取れるポイント | 仕様改善に向けた着眼点 | 関連章 |
| - | - | - | - | - |
| 字句解析とトークナイズ | `samples/language-impl-comparison/reml/json_parser.reml:56` | 手続き型トークナイザーが `Text.char_at` と `List.push_back` を逐一呼び出し、`read_*` 系は TODO なダミー実装のまま。 | `Core.Parse.Lex` を既定値として組み込む手順と、Unicode 正規化/数値解析のエラーを `Diagnostic` に変換するガイドラインを 2-3/3-3 へ追記する。 | 1-1, 2-3, 3-3, 3-5 |
| パーサーコンビネーター運用 | `samples/language-impl-comparison/reml/json_extended.reml:27`, `samples/language-impl-comparison/reml/yaml_parser.reml:120` | コメントスキップやトレーリングカンマ、インデント検証など高度な前処理を `RunConfig` 設定なしに都度書いている。 | `RunConfig` の Packrat/左再帰/コメント扱いを公式スイッチとして整理し、ストリーミング・復旧戦略を Chapter 2 と `guides/core-parse-streaming.md` で体系化する。 | 2-0, 2-2, 2-6, guides/core-parse-streaming.md |
| 効果ハンドリング戦略 | `samples/language-impl-comparison/reml/algebraic_effects.reml:32` | `with State/Except/Choose` 合成と部分ハンドル例が揃うが、効果順序やハンドラー入れ替え時の規則は注釈止まり。 | 効果行の整列基準、`perform` の評価順序、Capability 連携のステージ管理を 1-3 と 3-8 に明文化し、他言語比較の表を `notes/dsl-plugin-roadmap.md` と同期する。 | 1-3, 3-8, notes/dsl-plugin-roadmap.md |
| 診断とエラー報告 | `samples/language-impl-comparison/reml/yaml_parser.reml:58`, `samples/language-impl-comparison/reml/yaml_parser.reml:139` | インデント不一致やネスト判定で `Parse.fail` に素朴な文字列を渡しており、スパン・期待集合・監査メタが欠落。 | `Parse.fail` や `Parse.recover` から `Diagnostic` を生成する標準フローと、監査ログへの橋渡し API を 2-5/3-6/3-7 へ定義する。 | 2-5, 3-6, 3-7 |
| 標準コレクション操作 | `samples/language-impl-comparison/reml/yaml_parser.reml:156`, `samples/language-impl-comparison/reml/template_engine.reml:343` | `Map.from_list`・`List.fold`・`Map.insert` を多用するが、順序保証や差分更新の契約がドキュメント化されていない。 | `List`/`Map` の安定順序・イミュータブル更新コスト・`Iter` との相互変換を 3-1/3-2 に追記し、DSL からの利用ガイドを用意する。 | 3-1, 3-2 |
| Unicode/Grapheme 操作 | `samples/language-impl-comparison/reml/markdown_parser.reml:42`, `samples/language-impl-comparison/reml/markdown_parser.reml:63` | カーソル管理で `String.grapheme_at` と `Grapheme.display_width` を直呼びし、列位置と診断位置を手計算している。 | `ParseState` と `Diagnostic` 間で Grapheme 単位の列情報を共有する規約、`Core.Text` の幅計算 API の利用手順を 1-4/3-3/2-5 に明示する。 | 1-4, 2-5, 3-3 |
| 設定ファイル拡張耐性 | `samples/language-impl-comparison/reml/json_extended.reml:27`, `samples/language-impl-comparison/reml/json_extended.reml:74` | コメント許容・トレーリングカンマ・期待集合活用が手動実装で、互換性スイッチや診断メタは未統一。 | `Lex.commentLine`/`commentBlock` の互換モード、拡張 JSON/TOML の互換性フラグ、`RunConfig` の feature ガードを 2-3/3-7/3-10 に整理する。 | 2-3, 3-7, 3-10 |
| テンプレート DSL とセキュリティ | `samples/language-impl-comparison/reml/template_engine.reml:64`, `samples/language-impl-comparison/reml/template_engine.reml:242`, `samples/language-impl-comparison/reml/template_engine.reml:329` | フィルター登録・HTML エスケープ・`Map` ベースの実行環境が独自実装で、効果タグや Capability 要件が未整理。 | DSL 向けテンプレート API、フィルター登録と `CapabilityRegistry` 連携、`Diagnostic` によるテンプレート実行エラー報告を 1-1/3-3/3-6/3-8 へ拡充する。 | 1-1, 3-3, 3-6, 3-8 |
| Regex/Parser 連携と性能 | `samples/language-impl-comparison/reml/regex_engine.reml:10`, `samples/language-impl-comparison/reml/regex_engine.reml:175` | `Core.Parse.Op` で正規表現を構築しつつ Packrat 活用はコメントに留まり、`feature {regex}` の挙動が暗黙。 | `Core.Parse` と `Core.Regex` の責務境界、Packrat/memo オプションの既定値、Unicode クラスの互換保証を 2-2/2-6/3-3/3-8 に取りまとめる。 | 2-2, 2-6, 3-3, 3-8 |
| 並行・分散モデル比較 | `samples/language-impl-comparison/README.md:12` | Elixir・Scala 3・Nim など並行モデルを持つ比較対象が列挙される一方、Reml の並行 API サンプルは未整備。 | `Core.Async`・`Core.FFI` の仕様ドラフトを進め、Actor/プロセス指向 DSL の最小例と Capability 検証手順を公開する。 | 3-9, guides/runtime-bridges.md, 3-8 |

> 今後、比較サンプルが追加された際は、本マトリクスの「参考実装/根拠」列を更新し、`README.md` の新項目と整合させてください。
