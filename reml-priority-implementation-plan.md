# Reml 優先仕様導入計画

## 1. 目的と対象範囲
- 本計画は `reml-async-io-investigation.md`、`reml-stream-reactive-extension.md`、`reml-data-modeling-roadmap.md`、`reml-ffi-abi-investigation.md` 等で整理された提案のうち、Reml の設計哲学と最上位ゴール（`0-1-overview.md` / `0-2-project-purpose.md`）に直結する優先仕様を体系化する。
- 対象領域は (1) 非同期実行基盤、(2) 効果タグ拡張とストリーム診断、(3) データ品質 DSL と統計機構、(4) FFI/ABI 仕様整備、(5) 実行時メモリ管理 Capability の 5 つとし、段階的な導入ロードマップを示す。

## 2. 優先領域の整理
1. **非同期実行基盤 (`reml-async-io-investigation.md`)**
   - `Core.Async`、`Future/Task` 二層構造、`run_stream_async` の導入でゲーム／IDE／Web／DB シナリオを直接支援。
   - 効果システムと整合した `async` DSL により Reml らしい宣言的操作性を維持。
2. **ストリーム診断拡張 (`reml-stream-reactive-extension.md`)**
   - `StreamDriver`、`FlowController`、`DemandHint`、`ContinuationMeta` 拡張で高品質エラーとホットリロード運用を強化。
   - 既存 `run_stream` 基盤の互換性を保ちつつバックプレッシャ契約を明文化。
3. **データ品質 DSL (`reml-data-modeling-roadmap.md`)**
   - `QualityRule` / `QualityProfile` / `StatsProvider` により Core.Data/Core.Config の価値を拡張し、「宣言で終わる」データ検証を提供。
   - 監査ログ・CLI 連携を前提に、エンタープライズ/クラウド/分析シナリオを強化。
4. **FFI/ABI 仕様整備 (`reml-ffi-abi-investigation.md`)**
   - `a-jit.md` の ABI 明文化、`guides/` 配下の運用ガイド整備で LLVM/実サービス連携の前提を固める。
   - `unsafe` + 効果タグ運用の安全境界を明確化。
5. **GC Capability 抽象 (`reml-gc-investigation.md`)**
   - RC 基盤と両立する世代別/インクリメンタル/リージョン GC の差し替えポイントを標準 API に定義。
   - `RunConfig` から停止時間・ヒープ制御を宣言できるようにし、ストリーム/async と整合。

## 3. 導入ロードマップ
### フェーズ0: 仕様基盤整備 (Weeks 1-3)
- **狙い**: 以降フェーズの前提となる効果タグ・ABI 情報・ドキュメント体裁を確立。
- **主タスク**
  - `1-3-effects-safety.md` へ `io.async` / `io.blocking` / `io.timer` サブフラグ案と `@async_free` / `@no_blocking` / `@must_await` 属性をドラフト挿入。
  - `a-jit.md` にターゲット ABI、データレイアウト、構造体パッキング規約、所有権モデルの明示セクションを追加。
  - `guides/runtime-bridges.md` から FFI 運用ノウハウを抽出し、`guides/reml-ffi-handbook.md`（新規）草案を作成。
- **成果物**: ドラフト差分、レビュー用コメントリスト、ABI/効果タグに関するチェックリスト。

### フェーズ1: 非同期/ストリーム統合 (Weeks 4-7)
- **狙い**: `run_stream` と効果システムを拡張し、`Core.Async` API の仕様ドラフトを確定。
- **主タスク**
  - `run_stream` 戻り値を `DemandHint` 付き `StreamOutcome` に拡張する案を `2-6-execution-strategy.md` に反映。
  - `StreamDriver` / `FlowController` / `StreamDiagnosticHook` の型定義と利用例を仕様化。
  - `Core.Async` の `Future<T>` / `Task<T>` / `AsyncFeeder` / `run_stream_async` を `2-1-parser-type.md` 下書きに追記。
  - 代表シナリオ向けサンプル（ゲームホットリロード、IDE 増分解析、Web SSE）を `guides/runtime-bridges.md` に追加。
- **成果物**: 仕様ドラフト、PoC 仕様（擬似コード）、バックプレッシャ契約テーブル、監査ログスキーマ更新案。

### フェーズ2: データ品質 DSL 拡張 (Weeks 8-11)
- **狙い**: Core.Data/Core.Config を中心に品質検証と統計拡張の仕様を固める。
- **主タスク**
  - `QualityRule` / `QualityProfile` / `run_quality` API を `2-8-data.md` に追加、CLI 連携案を `guides/data-model-reference.md` へ反映。
  - `RunConfig.data_profile` セクションと `Config.schema.data_source` 属性を `2-7-config.md` に追記。
  - 統計型 (`StatType`, `HistogramBucket`) と JSON スキーマ草案を作成し、監査ログとの整合テストケースを列挙。
- **成果物**: DSL 構文サンプル、API シグネチャ、JSON スキーマドラフト、検証ベンチマーク計画。

### フェーズ3: 実行時管理と Capability 整備 (Weeks 12-14)
- **狙い**: GC Capability 抽象と監査メトリクスを整備し、async/stream/データ機能の下支えを確立。
- **主タスク**
  - `RunConfig` に `gc` セクション（ポリシー種別、停止時間ターゲット、ヒープ上限）を追加。`2-6-execution-strategy.md` に管理フローを記述。
  - `Core.Runtime`（名称未定）へ GC Capability インターフェイス案（Root 列挙、バリア、メトリクス取得）を定義。
  - GC プロファイルテンプレート（ゲーム/IDE/Web/データ）と監査ログ (`gc.stats`) 連携案を `guides/runtime-bridges.md` に追記。
- **成果物**: GC API 草案、RunConfig 拡張案、監査ログ項目一覧、PoC 設計メモ。

### フェーズ4: 統合検証とドキュメント確定 (Weeks 15-18)
- **狙い**: 各仕様ドラフトをシナリオ別にレビューし、横断ドキュメントと相互参照を更新。
- **主タスク**
  - `scenario-requirements.md` / `scenario-priorities.md` を更新し、新仕様の適用ポイントとベストプラクティスを明記。
  - LSP/CLI/監査ログの JSON スキーマ更新を `guides/lsp-integration.md` / `guides/runtime-bridges.md` に反映し、互換性チェックリストを作成。
  - PoC/擬似コードの結果をレビューし、残リスク・未解決課題を Backlog として整理 (`reml-backlog.md` 新規)。
- **成果物**: 最終仕様ドラフト、相互参照更新、リスクログ、レビュー記録。

## 4. 横断テーマと運用ルール
- **監査・可観測性**: async/stream/gc/data 各領域で発生するメトリクスを `audit.log` に統一フォーマットで送出（`domain`: `async.op` / `parser.stream` / `data.quality` / `gc.stats`）。
- **テスト/検証方針**: PoC 単位でスループット・停止時間・エラー品質の測定計画を用意し、指標を `scenario-requirements.md` へ反映。計測テンプレートは `guides/testing-matrix.md` にまとめる。
- **ツール連携**: LSP/CLI/CI 向けの JSON/構造化ログ変更は互換性テーブルを付与し、破壊的変更にはマイグレーション手順を添付。
- **ドキュメント運用**: 各フェーズで対象ファイルとレビュー担当を明示したチェックリストを作成し、最終的に `README.md` の目次と `0-1-overview.md` の要約を更新する。

## 5. リスクと緩和策
- **仕様肥大化**: 各機能を Capability/モジュール化し、`Core` 本体を最小プリミティブに留める。導入判定の際は PoC 成果と利用シナリオをセットでレビュー。
- **互換性破壊**: 効果タグ・RunConfig 拡張は既存フィールドを保持しつつ Optional/後方互換な既定値を設定。変更点はマイグレーションノートに記録。
- **工数増大**: フェーズごとにゴール判定チェックリストを定義し、次フェーズへの着手条件（PoC 完了、レビュー承認等）を明文化。
- **知識ギャップ**: MLIR/async/GC 等専門領域は調査メモを共有し、レビュー前にミニワークショップを実施。外部参考資料を `research/` に整理。

## 6. 直近アクション
1. フェーズ0 のドラフト対象者と締切を決定（効果タグ: Type System チーム、ABI: Runtime チーム等）。
2. `guides/reml-ffi-handbook.md` のアウトラインを作成し、`a-jit.md` 追記項目とリンクさせる。
3. `1-3-effects-safety.md` への差分草稿（効果タグ細分化・属性）を下書きし、レビューコメントを収集。
4. 非同期ストリーム領域の PoC 計測指標（レイテンシ、スループット、エラー回収時間）を定義して共有ボードを作る。
5. フェーズ毎のレビュー会議体を設計（参加者、頻度、成果物）し、プロジェクトタイムラインに追加。
