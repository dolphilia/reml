# Reml 標準API向け非同期I/O検討ノート

## 1. Remlの前提と非同期I/Oの位置付け
- Remlはパーサーコンビネーター処理系を最短距離で構築することを目的とし、実用性能・宣言的操作性・高品質診断・Unicode対応を核に据えている `0-1-overview.md:3` `0-2-project-purpose.md:19`
- 効果システムは `mut/io/ffi/panic/unsafe` の5分類で、副作用を静的に管理しつつ将来的な `async` 効果を `io` のサブ分類として導入できる余地を示唆している `1-3-effects-safety.md:8` `1-3-effects-safety.md:125`
- パーサ実行系は `run_stream`/`resume` など継続ベースのストリーミング API を備え、リングバッファ入力とゼロコピーを標準化している `2-1-parser-type.md:154` `2-6-execution-strategy.md:12`
- FFI/ランタイム連携ガイドは外界I/Oを効果タグ付きで扱い、監査ログや安全境界を重視する設計を提示している `guides/runtime-bridges.md:5`

## 2. 主な利用シナリオから見た非同期I/Oニーズ
- ゲームエンジン向けスクリプト言語ではホットリロードやシグナル駆動処理が必須で、リアルタイムI/Oやイベント駆動ロジックを短時間で組み込みたい要求が明示されている `scenario-requirements.md:21`
- IDE/ツール向け拡張では `run_stream` を活かしたインクリメンタル解析、外部デバッガとのIPC、非同期ログ処理などが必須機能として列挙されている `scenario-requirements.md:52`
- Webフレームワーク向けDSL群はHTTPルーティング、テンプレート再描画、ホットリロード、外部サービスとの通信など非同期I/O前提のワークロードを想定している `scenario-requirements.md:84`
- データベース／クラウド系シナリオでは長時間I/Oと監査ログの両立、バックグラウンドジョブ制御などが求められ、効果タグによる安全境界と宣言的制御が必要となる `scenario-requirements.md:116`

## 3. 非同期I/O技術・研究の俯瞰
- **イベントループ（Reactor型）**: libuv/libevent/Boost.Asioなどが代表。クロスプラットフォームのソケット・ファイルI/O、タイマー、スレッドプール統合を提供し、Node.jsやRust Tokioの基盤となっている。
- **プロアクタ/カーネル駆動型**: Windows IOCP、Linux `io_uring` などカーネル非同期APIを直接活用し、高スループットと低レイテンシを目指す。Jens Axboeによる io_uring 設計資料が参考。
- **Future/Promise & async/await**: Rust Tokio、C#/.NET、Kotlin、Swiftなどが採用するモデルで、トランポリン実行・構造化並行性（Structured Concurrency）によるキャンセル伝播が研究されている。
- **Actor/CSP/FRP系**: Erlang/OTP、Akka、GoのCSPなどメッセージパッシング中心。RemlでDSLを構築する際の抽象層として活用可能。
- **学術的知見**: Reactor/Proactorパターン（POSA2）、SEDAアーキテクチャ（Welsh et al., SOSP 2001）、Promiseスケジューリング（Celik et al., OOPSLA 2016）、OCamlのAsync Effects研究などが設計指針として有用。

## 4. Reml標準APIへ取り込む際の評価
- 効果システムが `io` を中心に副作用を明示できるため、`async`/`await` 相当の構文やFuture型を導入しても静的契約で抑制しやすい `1-3-effects-safety.md:8` `1-3-effects-safety.md:125`
- 既存の `run_stream`/`Continuation` はpull型ストリーミングを標準化しており、イベントループと連携するブリッジを追加すれば非同期入力を自然に統合できる `2-1-parser-type.md:154` `2-6-execution-strategy.md:141`
- FFIガイドが効果タグと監査ログを前提にしているため、非同期I/Oバックエンド（libuv, io_uring, OSイベントループ）をCapabilityとして差し替える設計がRemlの「小さく強いコア」と整合する `guides/runtime-bridges.md:5`
- 標準APIでプラットフォーム実装を抱え込むのではなく、イベントループ抽象とFuture/Task型を提供し、バックエンド差し替えを可能にする方針が肥大化リスクを抑えつつ最短距離開発を支援する。

## 5. API設計に向けた検討課題
- **Core.Async（仮）レイヤ**: イベントループ抽象、タスク/Future、キャンセル、タイマー、チャネルAPIの最小セットを策定。
- **効果タグ拡張**: `io` を細分化して `io.async`・`io.blocking` など区別し、属性（例: `@async_free`）で静的制御する案を評価 `1-3-effects-safety.md:39`。
- **RunConfig連携**: 非同期実行のスケジューラ設定、同時接続上限、バックプレッシャ制御を `RunConfig` に宣言的追加する計画を検討 `2-1-parser-type.md:82`。
- **パーサとI/Oの橋渡し**: `Feeder` をFutureベース供給に拡張し、バックプレッシャと継続再開を統一する設計を模索 `2-6-execution-strategy.md:148`。
- **監査・計測**: ランタイム連携ガイドの監査ログ/メトリクスAPIと親和性を確保し、非同期オペレーションのトレース/統計を標準化 `guides/runtime-bridges.md:40`。

## 6. 推奨アクション
1. Core.Async(仮)のインターフェイス案を起草し、イベントループ抽象・Future/Task・タイマー・チャネル等の仕様ドラフトを共有する。
2. 効果タグ拡張と属性制約（`@async_free`, `@no_blocking` など）を設計メモにまとめ、効果推論との整合性を評価する。
3. libuv型・io_uring型バックエンドを想定したCapability境界の比較表を作成し、FFI安全性と監査要件を整理する。
4. 代表シナリオ（ゲーム、IDE、Web、DB/クラウド）でのAPI利用例を短い擬似コードで試作し、Packrat/継続機構との相互作用を確認する。
5. 仕様書で更新が必要な節（2-1, 2-6, 1-3, guides/runtime-bridges.md 等）を洗い出し、追記計画とレビュー体制を提案する。

## 7. 参考資料
- `0-1-overview.md`
- `0-2-project-purpose.md`
- `1-3-effects-safety.md`
- `2-1-parser-type.md`
- `2-6-execution-strategy.md`
- `guides/runtime-bridges.md`
- `scenario-requirements.md`

## 8. 効果統合ガイド（ドラフト）
### 8.1 効果フラグ拡張の提案
- `io` を分解し、`io.async`（ノンブロッキングI/O）、`io.blocking`（ブロッキング呼び出し）、`io.timer`（タイマー/スケジューラ）をサブフラグとして管理。
- 効果推論では `io` を上位集合とし、`io.async` を含む関数は自動的に `io` も保持する（逆は成立しない）。
- `1-3-effects-safety.md:39` の属性検査を拡張し、`@async_free` で `io.async` を禁止、`@no_blocking` で `io.blocking` を禁止。
- `async fn` は宣言時に `io.async` を暗黙付与し、ブロッキング操作は `await blocking { ... }` のような隔離構文へ押し込む方針を提案。

### 8.2 属性検査と静的保証
- `@pure` は従来通り `io` 全体を禁止する。`async` 関数に `@pure` を付けた場合はエラーを報告。
- `@scheduler(bound="single"|"multi")` 属性を導入し、`FlowController` や `RunConfig.async` が要求するスレッド数制約と整合させる。
- `@must_await` 属性を `Future` 型の戻り値に付与し、未使用時に警告を出す。（`1-3-effects-safety.md:58` の `@must_use` を再利用）
- 非同期クロージャは `capture` 属性で捕捉変数の `Send`/`Sync` 相当マーカーを明示し、静的検証で失敗時に `Diagnostic` を返す。

### 8.3 `async` 導入時の API 設計メモ
1. **`Future<T>`/`Task<T>` の二層構造**
   - `Future<T>` は値生成の制御フロー、`Task<T>` はスケジューラでの所有権とキャンセル制御を担当。
   - `Task::spawn(future, cfg)` は `RunConfig` の `async` セクションを受け取り、イベントループまたはスレッドプールに登録する。
2. **`AsyncFeeder` の導入**
   - `run_stream_async(parser, feeder_async, cfg)` を追加し、`Feeder` を `Future<InputChunk>` として扱う。
   - `DemandHint`（`reml-stream-reactive-extension.md`）を `Future` 側へ反映してバックプレッシャと同期。
3. **キャンセルと構造化並行性**
   - `scope async { ... }` 構文を想定し、スコープ終了時に子タスクへキャンセルシグナルを送る。
   - `CancelToken` は `Result<T, Cancelled>` を返し、`ContinuationMeta.trace_id` と紐付けてトレース性を高める。
4. **診断と監査**
   - `guides/runtime-bridges.md:40` の監査ログに `async.op` ドメインを追加し、`TaskId`, `Scheduler`, `Latency` を送出。
   - `2-5-error.md` の `Diagnostic` に `async_context` フィールドを追加する検討。

### 8.4 既存仕様への反映ポイント
- `1-3-effects-safety.md` に `io.async` サブフラグ、`@async_free`/`@no_blocking` 属性の記述を追加。
- `2-1-parser-type.md` に `AsyncFeeder` と `run_stream_async` のシグネチャを追記。
- `2-6-execution-strategy.md` の `RunConfig` 節に `async` セクション（スケジューラ選択、同時実行上限、バックプレッシャ統合）を追加。
- `guides/runtime-bridges.md` に構造化並行性とキャンセル報告ルールを記載。

### 8.5 今後の検証タスク
1. 効果推論実装のスケッチ：`io.async` を持つラムダが一般化を阻害するケースのテストを書く。
2. 属性検査の診断文例：`@no_blocking` を破った場合のエラーメッセージテンプレートを `2-5-error.md` に合わせて作成。
3. `run_stream_async` のフェイルファスト条件：`FlowController` からのキャンセルと `CancellationToken` の整合性を確認。
4. 代表シナリオごとの `async` ベストプラクティスを `guides` ディレクトリへ配布する計画を立案。
