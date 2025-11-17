# 3.1 Core Prelude & Iteration 実装計画

## 目的
- 標準仕様 [3-1-core-prelude-iteration.md](../../spec/3-1-core-prelude-iteration.md) に準拠した `Core.Prelude` / `Core.Iter` モジュール群を Reml 実装へ落とし込み、章内 API の完全性と効果タグ精度を確保する。
- Option/Result/Iter を中心とした失敗制御モデルを安定化し、Chapter 3 の他モジュール (Collections/Text/Numeric) と同一インターフェイスで連携できる状態へ引き上げる。
- 仕様と実装・ドキュメントの差分を可視化し、Phase 3 以降のセルフホスト工程で再利用できるベンチマークとテスト資産を準備する。

## スコープ
- **含む**: `Option`/`Result`/`Never`/`Iter` の型・演算、`Collector` 契約、`Iter` アダプタ/終端操作、効果タグの検証、章内サンプルコードの実装検証、仕様リンクの更新。
- **含まない**: DSL / プラグイン固有拡張、1.3 章の効果システムそのものの仕様変更、未来の並列イテレータ拡張案（Phase 4 以降）。
- **前提**: Phase 2 で確定した診断/効果仕様が `Core.Diagnostics` 側に実装されており、Option/Result/Iter を利用する既存コードの回帰テストが実行可能であること。

## 作業ブレークダウン

### 1. 仕様精査と API インベントリ化（35週目）
**担当領域**: 設計調整

1.1. [3-1-core-prelude-iteration.md](../../spec/3-1-core-prelude-iteration.md) の API 一覧を機械可読な表に整理し、既存実装との差分 (新規/変更/廃止) を抽出する。
1.2. 効果タグ・属性 (`@must_use`, `effect {debug}` 等) の整合表を作成し、Phase 2 の Diagnostic 実装で要求されるメタデータ列と突き合わせる。
1.3. Option/Result の内部実装スタイル (enum vs struct, インライン最適化) を評価し、性能/サイズベンチマークの計測指標を確定する。

### 2. Option/Result 系 API 実装（35-36週目）
**担当領域**: 失敗制御プリミティブ

2.1. `Option`/`Result`/`Never` 型と付随メソッド (`map`/`and_then`/`expect` など) を Reml で実装し、`@must_use` と効果タグを正しく付与する。
2.2. `ensure`/`ensure_not_null` 等のユーティリティを組み込み、診断 (`Diagnostic`) への変換ヘルパと一緒に単体テストを整備する。
2.3. 例外排除ポリシーを検証するため、Rust 実装で `panic`/`abort` を伴う経路を禁止するテストを作成し、期待差分を `0-3-audit-and-metrics.md` へ記録する。必要に応じて OCaml 実装の挙動を参考情報として添付するが、自動比較対象には含めない。

### 3. Iter コア構造と Collectors（36-37週目）
**担当領域**: 遅延列基盤

3.1. `Iter<T>` の内部表現・所有権モデルを実装し、`IntoIter`/`FromIterator` の変換を整える。
3.2. `Collector` トレイトと標準コレクタ (`ListCollector`/`VecCollector`/`MapCollector` 等) を実装し、失敗時エラー型と効果タグの伝播をテストする。
3.3. `Iter::from_fn`/`Iter::once` など生成系ヘルパを実装し、`Iterator` 互換 API の命名・挙動差分を仕様と揃える。

### 4. Iter アダプタと終端操作（37-38週目）
**担当領域**: 宣言的データフロー

4.1. `map`/`filter`/`flat_map`/`zip`/`buffered` 等のアダプタを実装し、`effect {mem}` や `effect {mut}` の発生箇所を網羅的にテストする。
4.2. `collect_list`/`collect_vec`/`fold`/`reduce`/`try_fold` など終端操作の実装を行い、`Collector` との連携とエラー伝播経路を検証する。
4.3. パフォーマンス計測ベンチマークを作成し、Rust 実装の Phase 2 ベースライン（`docs/plans/rust-migration/3-2-benchmark-baseline.md`）と比較して ±10% 以内に収束するかを測定し、`0-3-audit-and-metrics.md` に反映する。

### 5. Diagnostics/Unicode 連携（38週目）
**担当領域**: 他章との統合

5.1. `Iter`/`Collector` が `Core.Text` の `GraphemeSeq` や `Core.Collections` の永続構造と相互運用できることを確認し、必要な補助関数を追加する。
5.2. Option/Result と `Diagnostic`/`AuditEnvelope` の相互変換ヘルパを整備し、失敗制御が監査ログに正しく反映されるか統合テストを実施する。
5.3. `effect` タグと `CapabilityStage` の境界を検証し、`effect {debug}` の利用箇所にデバッグビルド限定ステップを組み込む。

### 6. サンプルコード検証とドキュメント更新（38-39週目）
**担当領域**: 情報整備

6.1. 仕様書内サンプル (`reml` コードブロック) を Reml 実装で実行し、必要に応じて修正または `NOTE` 追記を行う。
6.2. `README.md` および `3-0-phase3-self-host.md` に Prelude/Iter 移行ステータスを追記し、利用者向けハイライトを作成する。
6.3. 新規 API の使用例を `examples/` ディレクトリに追加し、`docs/guides/core-parse-streaming.md` 等関連ガイドへのリンクを更新する。

### 7. テスト・ベンチマーク統合とリリース準備（39週目）
**担当領域**: 品質保証

7.1. 単体/統合テストを CI に追加し、`--features core-prelude` など機能ゲートを導入する。
7.2. ベンチマーク結果と API 完了状況を `0-3-audit-and-metrics.md`/`0-4-risk-handling.md` に記録し、リスク項目を更新する。
7.3. レビュー資料 (API 差分一覧、ベンチマーク、リリースノート草案) を準備し、Phase 3-2 以降へ引き継ぐ。

## 成果物と検証
- `Core.Prelude`/`Core.Iter` 実装および Collector 群が CI テストを通過し、効果タグ/属性が仕様と一致していること。
- Rust 実装のベースライン（Phase 2 ベンチマーク）と比較した性能が ±10% 以内に収まり、差分が存在する場合はメトリクスに記録されていること。OCaml 実装のデータは参考値として付録に残す。
- ドキュメント (仕様引用、ガイド、サンプル) が更新され、仕様と実装の相互参照が解決していること。

## リスクとフォローアップ
- 効果タグ伝播に不備がある場合、Phase 2 の診断タスクへエスカレートする。
- `Iter` の所有権モデルが `Core.Collections` と競合した場合は、一時的に `unsafe` ブロックの導入を避け、代替設計を `docs/notes/core-library-outline.md` に記録する。
- ベンチマーク遅延が解消しない場合、RC 最適化や並列イテレータの検討を Phase 4 の改善項目に追加する。

## 参考資料
- [3-1-core-prelude-iteration.md](../../spec/3-1-core-prelude-iteration.md)
- [3-2-core-collections.md](../../spec/3-2-core-collections.md)
- [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- [0-4-risk-handling.md](0-4-risk-handling.md)
