# 2.5 仕様差分補正計画

## 目的
- Phase 2 で仕様書 (Chapter 1〜3) と実装の差分を洗い出し、記述ゆれ・不足項目を補正する。
- 更新内容を `0-3-audit-and-metrics.md` および計画書に脚注として残し、将来のレビュートレイルを確保する。

## スコープ
- **含む**: 仕様レビュー、差分リストの作成、関連ドキュメントの更新 (本文・用語集・脚注)、メトリクス記録。
- **含まない**: 新機能追加、仕様の大規模刷新。必要な場合は別タスクとして起票。
- **前提**: Phase 2 の実装タスク（型クラス、効果、FFI、診断）が概ね完了し、差分が明確になっていること。
- **連携**: Phase 2-7 で診断・監査パイプラインの残課題を処理し、Phase 2-8 で最終監査を行う前提となるため、差分リストは両フェーズから参照可能な構成で記録する。

## 作業ディレクトリ
- `docs/spec/` : Chapter 0〜3 の本文・図表・脚注更新
- `docs/guides/` : 仕様変更に追随するガイド修正
- `docs/notes/` : レビュー結果や TODO を記録（例: `docs/notes/guides-to-spec-integration-plan.md`）
- `docs/README.md`, `README.md` : 目次・導線の同期
- `docs/plans/repository-restructure-plan.md`, `docs/notes/llvm-spec-status-survey.md` : 作業ログとリスク管理

## 着手前の準備と初期調査
- **ハンドオーバー確認**: `docs/plans/bootstrap-roadmap/2-4-to-2-5-handover.md` を起点に、差分レビューで参照すべき成果物（`reports/diagnostic-format-regression.md`, `scripts/validate-diagnostic-json.sh` 等）を再確認し、Phase 2-7 と共有する差分リストの初期エントリを整備する。
- **完了報告の整理**: `docs/plans/bootstrap-roadmap/2-4-completion-report.md` のメトリクス欄を確認し、`ffi_bridge.audit_pass_rate`・`iterator.stage.audit_pass_rate` が 0.0 のままである理由と影響範囲を記録しておく。差分補正中に欠落フィールドを発見した場合は Phase 2-7 と即時連携する。
- **技術的負債の把握**: `compiler/ocaml/docs/technical-debt.md` の ID 22/23（Windows Stage / macOS FFI）を優先監視項目とし、差分レビューで関連フィールドが不足していないかチェックリストへ加える。
- **プロジェクト方針との整合**: `docs/spec/0-1-project-purpose.md` に定義された価値観（性能・安全性・段階的拡張）をレビュー観点に反映し、差分の優先順位付けに利用する。
- **実装ガイド更新**: `docs/plans/bootstrap-roadmap/IMPLEMENTATION-GUIDE.md` の Phase 2 重点事項を参照し、Type Class/効果/診断の整備状況を差分調査の前提条件として整理する。
- **作業ログ方針**: 差分補正で生じた判断は `0-3-audit-and-metrics.md`・`0-4-risk-handling.md` に記録し、`docs/plans/repository-restructure-plan.md` のフェーズ定義と矛盾しないようタイムラインを合わせる。

## 作業ブレークダウン

### 1. レビュー計画と体制整備（31週目）
**担当領域**: 計画策定

1.1. **レビュースコープの決定**
- 以下の範囲を対象に、セルフホスト移行へ直結する章から優先レビューする。優先順位は `High`→`Medium`→`Low` の順で実施し、各章の完了条件を `0-3-audit-and-metrics.md` に記録する。

| 領域 | 対象ドキュメント | 主な観点 | 優先度 | 完了条件 |
|------|------------------|----------|--------|----------|
| 言語コア | [1-1-syntax.md](../../spec/1-1-syntax.md), [1-2-types-Inference.md](../../spec/1-2-types-Inference.md), [1-3-effects-safety.md](../../spec/1-3-effects-safety.md) | 型クラス実装の写像、効果注釈と Stage 整合 | High | サンプルコードが OCaml 実装で再現でき、差分リストに原因・影響が記録されている |
| パーサー API | [2-0-parser-api-overview.md](../../spec/2-0-parser-api-overview.md)〜[2-6-execution-strategy.md](../../spec/2-6-execution-strategy.md) | API 呼出シグネチャ、エラー復元戦略、実行ポリシー | High | `Parser<T>` API の現行シグネチャと差異が無いことを確認し、差分があれば追補案を添付 |
| 標準ライブラリ | [3-0-core-library-overview.md](../../spec/3-0-core-library-overview.md)〜[3-10-core-env.md](../../spec/3-10-core-env.md) | Capability Stage、診断メタデータ、FFI 契約 | Medium | `AuditEnvelope`/`Diagnostic` のフィールド一覧と突合し、欠落フィールドが無いことを証明 |
| 補助資料 | `reports/diagnostic-format-regression.md`, `compiler/ocaml/src/diagnostic_serialization.ml`, `scripts/validate-diagnostic-json.sh` | JSON スキーマ、フォーマット差分レビュー手順 | Medium | Phase 2-4 の成果物と仕様の整合が `validate-diagnostic-json.sh` の出力で確認されている |
| 用語・索引用補 | [0-2-glossary.md](../../spec/0-2-glossary.md), `docs/README.md`, `docs/plans/repository-restructure-plan.md` | 用語統一、導線更新 | Low | Glossary の更新差分がリンク整合チェック（手動）で確認済み |

- Phase 2-4 で整備した診断ログ資産をレビュー対象に組み込み、仕様に未記載のフィールドや命名ゆれを差分リストへ記録する。`compiler/ocaml/docs/technical-debt.md` の ID 22/23 はレビュースコープに含め、Windows/macOS 監査ゲートの整備状況を確認する。

1.2. **レビュー観点チェックリスト作成**
- レビュー時に必ず確認する観点をカテゴリ別に整理し、チェックリスト形式で `docs/plans/bootstrap-roadmap/checklists/` 配下へ保存する。初版では以下の項目を Must チェックとする。
  - **用語整合**: [0-2-glossary.md](../../spec/0-2-glossary.md) に定義済みの表記を参照し、差異がある場合は Glossary 更新案と一緒に記録。
  - **コードサンプル検証**: `reml` タグ付きコードブロックを収集し、`compiler/ocaml` のサンプルランナーで構文・型検証を実施。失敗時は差分リストに再現手順を記載。
  - **データ構造対照**: 仕様に記載されたレコード/enum と OCaml 実装（例: `diagnostic_serialization.ml`, `runtime/native/capability_stage.ml`）のフィールドを比較し、差異を表形式で整理。
  - **リンク・参照**: 相互参照や脚注が `README.md`・`docs/README.md` と一致しているか確認。リンク切れは URL と原因を記録。
  - **診断・監査フィールド**: `schema.version`, `audit.timestamp`, `bridge.stage.*`, `effect.stage.*`, `ffi_bridge.audit_pass_rate` 等が仕様・実装双方で一致しているか検証し、`scripts/validate-diagnostic-json.sh` の結果ログを差分リストに添付。
  - **技術的負債トラッキング**: `compiler/ocaml/docs/technical-debt.md` の該当 ID（特に 22/23）に紐づく観点がレビュー時に抜けていないか確認。

1.3. **スケジュールと担当割当**
- 31週目を 3 つのチェックポイントに分割し、各領域の担当とアウトプットを固定する。マイルストーンは `0-3-audit-and-metrics.md` の `phase2.week31` エントリとして記録し、遅延時は `0-4-risk-handling.md` に登録する。

| マイルストーン | 期限 | 担当（ロール） | 成果物 | 依存関係 |
|----------------|------|----------------|--------|----------|
| Kick-off レビュー会議 | 31週目 Day1 午前 | 仕様差分補正チームリード、Phase 2-7 代表 | レビュースコープ承認メモ、連絡窓口一覧 | `2-4-to-2-5-handover.md`、技術的負債 ID 22/23 の最新状況 |
| Chapter/領域別レビュー | 31週目 Day3 終了 | Chapter 1/2/3 担当、診断ログ担当 | 差分リスト初版（章別）、チェックリスト記入結果 | Kick-off のスコープ承認、`scripts/validate-diagnostic-json.sh` 実行ログ |
| スケジュール確定報告 | 31週目 Day5 終了 | 仕様差分補正チーム PM、Phase 2-7 調整役 | 週次レビュー計画（Week32-34）、`0-3-audit-and-metrics.md` 更新 | Chapter レビュー成果、Phase 2-7 タスク進行状況 |

- Phase 2-7 の未完了タスク（Windows/macOS 監査ゲート等）と相談する窓口を Kick-off で明示し、レビュー中に診断ログの欠落を発見した場合は即時フィードバックできる体制を整える。

**成果物**: レビュー計画書、チェックリスト、スケジュール

### 2. Chapter 1 差分抽出（31-32週目）
**担当領域**: 言語コア仕様レビュー

2.1. **構文仕様のレビュー（[1-1-syntax.md](../../spec/1-1-syntax.md)）**
- Phase 1 Parser 実装との差分抽出
- 効果注釈構文の追加反映（Phase 2）
- FFI 宣言構文の追加反映（Phase 2）
- サンプルコードの検証（実際にパース可能か）

2.2. **型システムのレビュー（[1-2-types-Inference.md](../../spec/1-2-types-Inference.md)）**
- Phase 2 型クラス実装との差分抽出
- 辞書渡しの仕様追記
- 制約解決アルゴリズムの擬似コード検証
- サンプルコードの型推論結果検証

2.3. **効果システムのレビュー（[1-3-effects-safety.md](../../spec/1-3-effects-safety.md)）**
- Phase 2 効果実装との差分抽出
- Stage 要件の記述精緻化
- 効果推論ルールの擬似コード追加
- [3-8-core-runtime-capability.md](../../spec/3-8-core-runtime-capability.md) との整合

**成果物**: Chapter 1 差分リスト、修正案ドラフト

### 3. Chapter 2 差分抽出（32週目）
**担当領域**: パーサーAPI 仕様レビュー

3.1. **コアパーサー型のレビュー（`2-0`〜`2-2`）**
- Phase 1 Parser 実装との差分抽出
- `Parser<T>` 型の OCaml 実装との対応
- コンビネーター API の網羅性確認
- サンプルコードの検証

3.2. **字句・演算子のレビュー（`2-3`〜`2-4`）**
- 字句解析実装との差分抽出
- 演算子優先度テーブルの整合
- Phase 2 で追加された構文への対応
- 用語統一（トークン名等）

3.3. **エラー・実行戦略のレビュー（`2-5`〜`2-6`）**
- Phase 1 診断実装との差分抽出
- Phase 2 診断拡張の反映
- 実行戦略の記述精緻化
- `docs/guides/core-parse-streaming.md` との整合

**成果物**: Chapter 2 差分リスト、修正案ドラフト

### 4. Chapter 3 差分抽出（32-33週目）
**担当領域**: 標準ライブラリ仕様レビュー

4.1. **コアライブラリのレビュー（`3-0`〜`3-5`）**
- Phase 1 ランタイム実装との差分抽出
- コレクション型の API 整合性
- テキスト処理の Unicode モデル整合
- 数値・時間・IO・パス操作の仕様精緻化

4.2. **診断・Capability のレビュー（`3-6`〜`3-8`）**
- Phase 2 診断実装との差分抽出
- `Diagnostic`/`AuditEnvelope` の仕様更新
- Capability Stage テーブルの最新化
- メタデータキー命名規約の追記

4.3. **非同期・FFI・環境のレビュー（`3-9`〜`3-10`）**
- Phase 2 FFI 実装との差分抽出
- ABI 仕様テーブルの精緻化
- 所有権契約の擬似コード追加
- 環境変数 API の整合性確認

**成果物**: Chapter 3 差分リスト、修正案ドラフト

### 5. 修正案の作成とレビュー（33週目）
**担当領域**: 修正案策定

5.1. **差分の分類と優先順位付け**
- Critical: セルフホストに影響する差分
- High: Phase 3 で必要な差分
- Medium: 将来的に必要な差分
- Low: 記述改善・誤字脱字

5.2. **修正案の作成**
- 各差分について Markdown で修正案作成
- Before/After の明示
- 根拠の明記（実装コード、仕様意図）
- レビュアへの質問事項の整理

5.3. **レビュープロセス**
- 修正案のレビュー依頼
- フィードバックの収集
- 修正案の調整・合意形成
- 却下された修正案の記録

**成果物**: 承認済み修正案、レビュー記録

### 6. ドキュメント更新の実施（33-34週目）
**担当領域**: 仕様書更新

6.1. **主文書の更新**
- 承認された修正案の反映
- サンプルコードの更新
- 図表の更新（必要に応じて）
- 脚注・TODO の追加

6.2. **用語集・索引の更新**
- [0-2-glossary.md](../../spec/0-2-glossary.md) の用語追加・更新
- 新規概念の定義追加
- 廃止された用語の非推奨マーク
- 用語の統一チェック

6.3. **サンプルコードの検証**
- 更新されたサンプルのパース検証
- 型推論結果の確認
- エラーケースの検証
- `examples/` ディレクトリとの整合

**成果物**: 更新された仕様書、用語集

### 7. クロス参照とリンク整備（34週目）
**担当領域**: ドキュメント整合

7.1. **索引系ドキュメントの更新**
- `README.md` の目次更新
- [0-0-overview.md](../../spec/0-0-overview.md) の概要更新
- [0-1-project-purpose.md](../../spec/0-1-project-purpose.md) の目的・方針の見直し
- [0-3-code-style-guide.md](../../spec/0-3-code-style-guide.md) のコード例更新

7.2. **相互参照リンクの検証**
- 全 Markdown ファイルのリンク抽出
- リンク切れの検出と修正
- セクション参照の正確性確認
- 相対パスの統一

7.3. **ガイド・ノートの整合**
- `docs/guides/` 以下のガイド更新
- `docs/notes/` 以下の調査ノート整理
- Phase 2 実装との整合確認
- 廃止されたドキュメントの削除/非推奨化

**成果物**: 整合された索引、検証済みリンク

### 8. 記録と Phase 3 準備（34週目）
**担当領域**: 記録と引き継ぎ

8.1. **差分処理結果の記録**
- `0-3-audit-and-metrics.md` への記録
- 処理した差分の統計（件数、分類別）
- レビュー工数の記録
- 残存課題の明示

8.2. **リスク管理への登録**
- 未解決の差分を `0-4-risk-handling.md` に登録
- Phase 3 で対応すべき事項の明示
- 仕様変更提案の記録
- 将来の仕様拡張検討事項

8.3. **Phase 3 引き継ぎ**
- セルフホスト時の仕様参照ポイント整理
- OCaml 実装から Reml 実装への写像ガイド
- 仕様の曖昧な箇所のリスト
- レビュープロセスの改善提案

**成果物**: 差分処理記録、リスク登録、引き継ぎ文書

## 成果物と検証
- 差分リストが公開され、レビュー記録が残っていること。
- 更新されたドキュメントが CI のリンクチェック（存在する場合）や手動確認で問題ないこと。
- 索引類が最新のリンクを指し、リンク切れがゼロであること。

## リスクとフォローアップ
- レビュー負荷が高い場合はフェーズ内で優先順位を付け、セルフホスト移行に影響する項目を先行対応。
- 新たな仕様変更案が発生した場合、Phase 3 のドキュメントフィードバックタスクと連携し調整。
- 差分が大きい場合は補足ノートを `docs/notes/` 以下に作成し、計画的に反映する。

## 参考資料
- [2-0-phase2-stabilization.md](2-0-phase2-stabilization.md)
- [0-0-overview.md](../../spec/0-0-overview.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- [0-4-risk-handling.md](0-4-risk-handling.md)
