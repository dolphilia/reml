# Chapter 4 準備: Remlエコシステム取り込み計画

## 目的
- `reml-ecosystem-analysis.md` で示されたエコシステム要件を、Reml仕様書へ段階的に組み込む準備と分類を行う。
- `0-2-project-purpose.md` の価値観（性能・安全性・段階的習得・DSLファースト）に沿った取り込み順序を整理する。
- Chapter 4（エコシステム）起草に先立ち、関連する既存章・ガイドとの役割分担を明確にする。

## 分類結果

### 1. 言語仕様 (Chapter 1) で扱うべき事項
| 項目 | 内容と理由 | 想定反映先 |
| --- | --- | --- |
| DSLエントリーポイント契約<br>（出典: reml-ecosystem-analysis.md:386-387） | `reml.toml` の `[dsl]` セクションが `entry` / `exports` を要求しており、モジュール公開単位と導出型安全性を明文化する必要がある。これにより DSL 間連携時の型安全性（0-2 指針の安全性・段階的習得）を保証。 | `1-1-syntax.md` に「DSLエントリーポイント定義」の節を追記し、エクスポート対象のシグネチャ要件と名前解決規則を定義。 |
| DSL互換性メタデータ | DSLカテゴリ分類と互換性チェック要求（reml-ecosystem-analysis.md:452-455）を型・効果システムに結びつけ、`effect` アノテーションや能力タグを仕様レベルで規定。 | `1-2-types-Inference.md` と `1-3-effects-safety.md` に、DSL互換ラベルの型判定と `effect` ベースの安全境界を追記。 |
| Conductor Pattern と複合DSL安全性 | 既存文書 `5-1-dsl-first-development.md` の Conductor Pattern を中核仕様に吸収し、複合DSLの評価順序・性能保証を Chapter 1 側に要約（0-2 の性能原則に合致）。 | `1-3-effects-safety.md` に「Conductor制御による複合DSL安全性」のリファレンス節を追加し、Chapter5 への相互参照を整備。 |

### 2. 標準API (Chapter 3) で扱うべき事項
| 項目 | 内容と理由 | 想定反映先 |
| --- | --- | --- |
| Manifest/Config パーサ | `reml.toml` 仕様策定（reml-ecosystem-analysis.md:368-387, 553）に対応し、耐エラー性と差分診断（0-2 の安全性・分かりやすいエラー）を備えた API を定義。 | `3-7-core-config-data.md` に `Core.Config.Manifest` 章を追加し、`Result` 型ベースの検証APIと警告統合 (`Core.Diagnostics`) を規定。 |
| CLI 出力の診断統合 | `reml build/test/fmt/check` などのCLI（reml-ecosystem-analysis.md:372-377）と `Core.Diagnostics` 連携を標準化し、 JSON 構造化ログ要件を明示（0-2 のエコシステム統合）。 | `3-6-core-diagnostics-audit.md` に CLI 統合サブセクションを追加し、構造化イベントと監査ログのスキーマを規定。 |
| DSLツール支援ユーティリティ | DSL性能最適化・互換性チェック（reml-ecosystem-analysis.md:334-335, 452-455）を実現する補助ライブラリ。性能計測は 0-2 の性能指標に直結。 | `3-8-core-runtime-capability.md` に DSL Capability Utility 節を追加し、互換性評価API・性能計測フックを定義。 |
| 基本テストフレームワーク支援 | Phase 1 のテスト基盤要求（reml-ecosystem-analysis.md:343, 570）に合わせ、`Core.Async` と連動するテストランナー抽象化を標準API化。 | 新規 `3-10-core-test-support.md`（章番号要調整）として最低限のテスト DSL とアサーション API を定義。 |

### 3. Chapter 4（エコシステム）で新規定義する事項
| セクション案 | 収容する主題 | 元情報 |
| --- | --- | --- |
| 4-1 Package Manager & CLI | `reml new/add/build/test/fmt/check` のワークフロー、`reml.toml` フォーマット、分散取得戦略。 | reml-ecosystem-analysis.md:368-403, 553-575 |
| 4-2 Registry & Distribution | レジストリAPI、署名検証、品質指標、DSLカテゴリ分類。 | reml-ecosystem-analysis.md:431-456, 482-515 |
| 4-3 Developer Toolchain | フォーマッタ、リンタ、テスト、デバッガー、プロファイラ、LSP 実装計画。 | reml-ecosystem-analysis.md:322-339, 431-447, 501-515 |
| 4-4 Community & Content | 公式サイト、チュートリアル、イベント、コンテンツマーケティング。 | reml-ecosystem-analysis.md:401-468, 602-643 |
| 4-5 Roadmap & Metrics | Phase 1-3 ロードマップ、成功指標、リスク管理。 | reml-ecosystem-analysis.md:480-666 |
| 4-6 Risk & Governance | 技術/市場/リソースリスクと軽減策、運営モデル。 | reml-ecosystem-analysis.md:647-676 |

### 4. guides/ に配置すべき文書
| ガイド案 | 目的 | 元情報 |
| --- | --- | --- |
| guides/manifest-authoring.md | `reml.toml` の記述例とベストプラクティス。段階的習得支援（0-2 指針）。 | reml-ecosystem-analysis.md:368-387 |
| guides/cli-workflow.md | `reml new/build/test/fmt/check` のタスク駆動手順と CI 連携。 | reml-ecosystem-analysis.md:368-377, 497-505 |
| guides/dsl-gallery.md | DSLギャラリー整備、テンプレート作成、互換性チェック運用。 | reml-ecosystem-analysis.md:332-335, 452-455, 568-602 |
| guides/community-handbook.md | 公式サイト・イベント運営・コンテンツ計画。 | reml-ecosystem-analysis.md:401-468, 606-643 |
| guides/ai-integration.md | AI支援コマンドの利用方針と安全ガードライン。 | reml-ecosystem-analysis.md:504-515, 625-631 |

## 取り込み作業進捗サマリ
| トラック | 主要タスク | 状況 | 次のアクション |
| --- | --- | --- | --- |
| 言語仕様整備 | DSLエントリーポイント定義、互換性メタデータ仕様化 | **完了**（1-1/1-2/1-3 に関連節を追加済） | 図版・参照リンクの整備と最終レビュー。 |
| 標準API拡張 | Manifest API、CLI診断、DSLユーティリティ、テスト支援 | **進行中**（3-6/3-7/3-8 を更新済、テスト支援のみ未着手） | `3-10-core-test-support.md` のドラフトを起草し、CLI ベンチマーク要件を反映。 |
| Chapter 4 起草 | 4-1〜4-6 の章構成とアウトライン化 | **完了（ドラフト）**（各章の骨子ファイルを追加） | 節ごとの詳細執筆スケジュールと図表/参考文献リストの作成。 |
| ガイド整備 | manifest/CLI/DSL/コミュニティ/AI ガイド | **完了（ドラフト）**（5 本のガイド草案を追加） | 各ガイドにサンプル/テンプレートを追記し、公開フローを定義。 |

## メモ
- 0-2-project-purpose.md の性能・安全性重視により、CLI/ツール仕様では構造化エラー・線形性能基準を必ず章内で明記すること。
- DSLギャラリーとテンプレートは、標準APIよりもガイドでナレッジ共有し、Chapter 4 では公開基準とレビュープロセスを示す二段構成が望ましい。
- AI統合機能は長期目標（Phase 3）に位置付けられているため、Chapter 4 本文では将来計画として扱い、guide で安全策・利用条件を詳細化する。
