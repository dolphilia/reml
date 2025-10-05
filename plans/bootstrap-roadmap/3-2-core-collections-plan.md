# 3.2 Core Collections 実装計画

## 目的
- 標準仕様 [3-2-core-collections.md](../../3-2-core-collections.md) に従い、永続／可変コレクション API を Reml 実装へ移植し、`Iter`・`Diagnostics`・`Text` との相互運用性を確保する。
- 永続構造のパフォーマンスと効果タグの整合性を検証し、監査ログ・Config/Data 連携で要求される差分出力機能を備える。
- 仕様／実装／ドキュメントの差分を整理し、Phase 3 以降のセルフホスト環境で安定利用できるテスト資産を整備する。

## スコープ
- **含む**: `List`/`Map`/`Set`/`Vec`/`Cell`/`Ref`/`Table` の実装、Collector 連携、監査ログ向け変換、効果タグ検証、ドキュメント更新。
- **含まない**: 並列・分散コレクション、GC 導入を前提とした最適化（Phase 4 のメモリ戦略に委譲）。
- **前提**: `Core.Prelude`/`Core.Iter` 実装タスク (3-1) が完了または並行で進行しており、`Core.Diagnostics`/`Core.Text` の基盤が Phase 2 から提供されていること。

## 作業ブレークダウン

### 1. API 差分調査とモジュール設計（38週目）
**担当領域**: 設計調整

1.1. 仕様に記載された公開 API を一覧化し、既存実装（OCaml 版）との差異・未実装 API を洗い出す。
1.2. 効果タグ (`effect {mut}`, `{mem}`, `{cell}`, `{rc}`, `{audit}`) の付与規則を整理し、テスト戦略とメトリクス項目を定義する。
1.3. 永続構造と可変構造で共有する内部ユーティリティ (アロケータ、ハッシュ関数) の設計指針を決定する。

### 2. 永続コレクション実装（38-39週目）
**担当領域**: `List`/`Map`/`Set`

2.1. `List<T>` の finger tree ベース実装を移植し、`as_vec` や `of_iter` の性能評価を行う。
2.2. `PersistentMap`/`PersistentSet` を実装し、差分マージ (`merge`, `diff`, `update`) と `Collector` 連携をテストする。
2.3. 構造共有によるメモリ削減効果を測定し、`0-3-audit-and-metrics.md` にベンチマーク結果を記録する。

### 3. 可変コレクションと内部可変性（39週目）
**担当領域**: `Vec`/`Cell`/`Ref`/`Table`

3.1. 標準 `Vec` の API セットを仕様通り実装し、`effect {mut}`/`{mem}` の正確な付与を確認する。
3.2. `Cell<T>`/`Ref<T>` の内部可変性モデルを実装し、`effect {cell}`/`{rc}` を活用したテストケースを整備する。
3.3. `Table<K,V>` の挿入順序保持ロジック・CSV ローダを実装し、`Core.IO`/`Core.Text` と連携する統合テストを追加する。

### 4. Iter/Collector 相互運用（39-40週目）
**担当領域**: 遅延列との結合

4.1. `Iter` から各コレクションへ変換する API (`collect_list`, `collect_vec`, `Map.from_iter` 等) を実装し、重複キー検出やエラーハンドリングを確認する。
4.2. `IntoIter` 実装を整備し、`Iter` と永続構造の往復変換で所有権が崩れないことをテストする。
4.3. `Collector` 実装と `Iter.try_collect` の統合を検証し、失敗時の `CollectError` が `Diagnostic` に落とし込まれるか確認する。

### 5. Diagnostics / Config / Audit 連携（40週目）
**担当領域**: 他章との統合

5.1. `Core.Diagnostics` の `AuditEnvelope.change_set` と連携するための JSON 差分ユーティリティを実装し、`Map`/`Table` の変換を提供する。
5.2. Config/Data 章 (3-7) で利用する差分 API (`SchemaDiff`, `Change`) との互換アダプタを用意し、双方向変換テストを実施する。
5.3. `effect {audit}` を伴う操作 (`emit_metric` 等) の前提条件を確認し、Capability チェックのフックを追加する。

### 6. ドキュメント整備とサンプル検証（40-41週目）
**担当領域**: 情報更新

6.1. 仕様書内サンプルの動作確認と更新、必要に応じて `NOTE` や脚注で制約事項を明記する。
6.2. `README.md`/`3-0-phase3-self-host.md` に Core.Collections 実装状況と API ハイライトを追記する。
6.3. `samples/` ディレクトリに永続コレクション利用例を追加し、CI で自動実行するテストを用意する。

### 7. テスト・ベンチマーク統合（41週目）
**担当領域**: 品質保証

7.1. 単体・プロパティテスト (例えば QuickCheck 相当) を導入し、構造共有や順序保持に関する不変条件を検証する。
7.2. ベンチマークスイートを追加し、OCaml 実装比 ±15% 以内を目標に性能を測定する。
7.3. テスト・ベンチマークの結果をメトリクス／リスク管理ドキュメントに反映し、未達の場合はフォローアップタスクを起票する。

## 成果物と検証
- `Core.Collections` API が仕様と一致し、効果タグ・診断連携が正しく機能すること。
- 永続／可変コレクション双方で OCaml 実装と比較した性能指標が基準内に収まっていること。
- ドキュメント・サンプルが更新され、Config/Data/Diagnostics との相互参照が成立していること。

## リスクとフォローアップ
- Finger tree 実装が性能目標を満たさない場合、代替構造 (RRB-Tree 等) の調査を `notes/core-library-outline.md` に記録し、Phase 4 で検討する。
- `Cell`/`Ref` の内部可変性が効果システムと衝突した場合、仕様更新 (1-3 章) をエスカレーションする。
- CSV ロード等 IO 連携でプラットフォーム依存差異が生じた際は `0-4-risk-handling.md` に記載し、Phase 3-5 (IO & Path) で調整する。

## 参考資料
- [3-2-core-collections.md](../../3-2-core-collections.md)
- [3-1-core-prelude-iteration.md](../../3-1-core-prelude-iteration.md)
- [3-6-core-diagnostics-audit.md](../../3-6-core-diagnostics-audit.md)
- [3-7-core-config-data.md](../../3-7-core-config-data.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- [0-4-risk-handling.md](0-4-risk-handling.md)
