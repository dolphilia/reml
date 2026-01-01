# EXEC-001 ストリーミング実行 PoC 計画

## 1. 背景と症状
- 仕様は `run`/`run_partial` に加え、入力チャンクを扱う `run_stream` / `resume` / `DemandHint` を定義している（docs/spec/2-6-execution-strategy.md:10-24, docs/spec/2-7-core-parse-streaming.md:22-84）。  
- 現行 OCaml 実装はバッチ解析用ランナーのみを提供し（compiler/ocaml/src/parser_driver.ml:15-43）、ストリーミング API・バックプレッシャ制御・`RunConfig.extensions["stream"]` が未実装。  
- Phase 3 の self-host では `run_stream` 互換性がゴール条件に含まれており、仕様差分がセルフホストのスケジュールを阻害するリスクが高い。

## 2. Before / After
### Before
- `RunConfig` から `extensions["stream"]` を参照する経路がなく、チャンク単位での再開・バックプレッシャ制御を提供できない。  
- CLI/LSP でインクリメンタル解析を試みる際、バッチランナーを再呼び出しするしかなく、期待される性能と診断品質を満たせない。

### After
- `Core.Parse.Streaming`（新モジュール）を実装し、`run_stream` / `resume` / `StreamOutcome` / `Continuation` を仕様通りに提供する。  
- `RunConfig.extensions["stream"]` に `checkpoint` / `resume_hint` / `DemandHint` を格納し、バッチランナーとストリーミングランナーの結果が一致することを保証する。  
- PoC 段階ではチャンク投入・バックプレッシャの最小限機能を実装し、Phase 2-7 `execution-strategy` タスクへ本実装のロードマップを渡す。

#### API スケッチ
```ocaml
type feeder = unit -> chunk_result
type chunk_result =
  | Chunk of string
  | Closed
  | Pending of Demand_hint.t

val run_stream :
  parser:'a Core_parse.parser ->
  feeder:feeder ->
  cfg:streaming_config ->
  'a stream_outcome
```

## 3. 影響範囲と検証
- **一致性テスト**: `docs/spec/2-7-core-parse-streaming.md:254-267` に定義されたテスト計画を PoC へ反映し、`run` と `run_stream` の結果一致をゴールデン比較で確認。  
- **CI**: `tooling/ci` にストリーミング用テストシナリオを追加し、`RunConfig.extensions["stream"]` が埋まっているかをメトリクスで監視。  
- **CLI/LSP**: インクリメンタル解析を利用する CLI モードを追加し、`DemandHint` が動作するか手動検証。
- **OCaml テスト**: `compiler/ocaml/tests/streaming_runner_tests.ml` を新設し、`resume` や `Pending DemandHint::Pause` のフローが期待通りに推移するかステップ単位で検証する。

## 4. 実装ステップ

### Step 0. 仕様・実装差分の棚卸し（1.5日）
- 目的: 既存ランナーと仕様 (`docs/spec/2-6-execution-strategy.md`, `docs/spec/2-7-core-parse-streaming.md`) の差異を列挙し、PoC 範囲を明確化する。
- 主な作業:
  - `parser_driver.ml` と `parser_run_config.ml` の現状 API/拡張マップを確認し、`extensions["stream"]` の流入経路を洗い出す。
  - Phase 2-5 の進捗（`PARSER-002`, `ERR-001`, `ERR-002`）とクロスリファレンスし、ストリーミングが再利用すべき診断・RunConfig・Recover 設定を一覧化する。
- 調査・検証:
  - `docs/guides/compiler/core-parse-streaming.md` と `docs/plans/bootstrap-roadmap/2-5-review-log.md` の該当エントリを読み、仕様上の必須メタデータ（`DemandHint`, `ContinuationMeta`, 指標名）を再確認。
  - `tooling/ci/collect-iterator-audit-metrics.py` の Packrat/Recover 指標を調査し、ストリーミング PoC で観測すべきメトリクスを決める。

> 2025-12-?? 更新: Step 0 の棚卸しと記録を完了。以下のサマリをもとに Step 1 へ移行する。

#### Step 0 棚卸し結果

**仕様と実装の差分**

| 項目 | 仕様で要求される内容 | 現状 OCaml 実装 |
| --- | --- | --- |
| ランナー API | `run_stream`/`resume` と `StreamOutcome` を提供し、`Completed`/`Pending` を返す（`docs/spec/2-7-core-parse-streaming.md:21`, `docs/spec/2-7-core-parse-streaming.md:30`） | `parser_driver` にはバッチ用の `run`/`run_partial`/`run_string` のみが存在し、ストリーミング API が未定義（`compiler/ocaml/src/parser_driver.ml:219`）。 |
| StreamingConfig・Flow | `StreamingConfig` と `FlowController` を介してバックプレッシャを管理（`docs/spec/2-7-core-parse-streaming.md:41`, `docs/spec/2-7-core-parse-streaming.md:144`） | RunConfig 側に対応する構造体がなく、ストリーミング用のフロー制御値を取得できない。 |
| RunConfig `extensions["stream"]` | `checkpoint`/`resume_hint`（`DemandHint`）/`flow` を共有する（`docs/spec/2-6-execution-strategy.md:74`, `docs/spec/2-7-core-parse-streaming.md:236`） | `Run_config.Stream` は文字列ベースの `checkpoint`/`resume_hint` のみを保持し、`DemandHint` や `flow` 情報を表現できない（`compiler/ocaml/src/parser_run_config.ml:309`）。 |
| 継続・メタデータ | `ContinuationMeta` と `StreamMeta` に `commit_watermark` や `resume_count` を保持（`docs/spec/2-7-core-parse-streaming.md:55`, `docs/spec/2-7-core-parse-streaming.md:218`） | 継続用の型が未実装で、`Parser_diag_state` から抽出した期待集合や Packrat 情報を保持する経路がない（`compiler/ocaml/src/parser_diag_state.ml:36`）。 |

**再利用する既存設定・資産**

- RunConfig まわりの橋渡しは `PARSER-002` で導入済みの `Run_config` サブモジュールを再利用する（`docs/plans/bootstrap-roadmap/2-5-review-log.md:211`）。バッチ経路では `Run_config.Recover.of_run_config` が診断へ流れているため（`compiler/ocaml/src/parser_driver.ml:228`）、ストリーミングでも同じ設定を利用できる。
- 期待集合は `ERR-001` で整備された `Parser_expectation.summarize_with_defaults` をそのまま使用できる（`compiler/ocaml/src/parser_expectation.mli:30`, `docs/plans/bootstrap-roadmap/2-5-proposals/ERR-001-proposal.md:13`）。継続メタデータに `ExpectationSummary` を格納し、`docs/guides/compiler/core-parse-streaming.md:70` で示されている運用方針と整合させる。
- 回復情報と FixIt の生成は `ERR-002` の成果物を共有し、`RunConfig.extensions["recover"]` を継続へ引き渡す（`docs/plans/bootstrap-roadmap/2-5-proposals/ERR-002-proposal.md:15`）。`Parser_diag_state.recover_config` から `sync_tokens`/`notes` を取得可能（`compiler/ocaml/src/parser_diag_state.ml:52`）。

**メトリクス整理**

- CI では既に `collect-iterator-audit-metrics.py` が RunConfig と Packrat の指標を収集しているため（`tooling/ci/collect-iterator-audit-metrics.py:1352`, `tooling/ci/collect-iterator-audit-metrics.py:1610`, `tooling/ci/collect-iterator-audit-metrics.py:1776`）、ストリーミング指標を追加する余地がある。`docs/guides/compiler/core-parse-streaming.md:170` が要求する `resume_hint` / `StreamMeta` を JSON に出力し、`parser.stream.outcome_consistency` / `parser.stream.demandhint_coverage` を 0-3 メトリクスへ登録する案を次ステップで検討する。

##### RunConfig `extensions["stream"]` の型案（Step1 着手前メモ）

- **新規型定義（`Core_parse_streaming.Types` 仮称）**
  - `Demand_hint.t`：`{ min_bytes:int; preferred_bytes:int option; frame_boundary: Token.Class.t option }`（仕様 `docs/spec/2-7-core-parse-streaming.md:63-77` に対応）。`frame_boundary` は `Token.class_of_symbol` と `Token.symbol_of_class` で文字列表現と相互変換する。
  - `Flow_mode.t = Push | Pull | Hybrid`（仕様 `docs/spec/2-7-core-parse-streaming.md:151`）。
  - `Backpressure_spec.t = { max_lag:Duration.t option; debounce:Duration.t option; throttle:Duration.t option }`（`docs/spec/2-7-core-parse-streaming.md:157-163`）。
  - `Flow_policy.t = Manual of demand option | Auto of Backpressure_spec.t`、`Demand.t = { bytes:int; frames:int }`。
  - `Flow_controller.t = { mode:Flow_mode.t; high_watermark:int; low_watermark:int; policy:Flow_policy.t }`。
  - `Continuation_meta.t` と `Stream_meta.t` は Step1 で導入する `Core_parse_streaming` モジュールへ移し、`docs/spec/2-7-core-parse-streaming.md:55-133`, `docs/spec/2-7-core-parse-streaming.md:218-223` を網羅する。

- **RunConfig 側のデコード/エンコード**
  - `Parser_run_config.Stream.of_run_config` を拡張し、`extensions["stream"]` の Namespace から以下のキーを読み取る。
    - `checkpoint`（文字列／Span シリアライズ ID）、`resume_hint.min_bytes`,`resume_hint.preferred_bytes`,`resume_hint.frame_boundary`。
    - `flow.mode`（`"push"|"pull"|"hybrid"`）、`flow.high_watermark`,`flow.low_watermark`。
    - `flow.policy.kind`（`"manual"` or `"auto"`）、`flow.policy.demand.bytes`,`flow.policy.demand.frames`、`flow.policy.backpressure.{max_lag,debounce,throttle}`（ナノ秒単位の整数）。
  - 取得結果は `type t = { checkpoint: string option; resume_hint: Demand_hint.t option; flow: Flow_controller.t option; namespace: Namespace.t option }` にまとめ、既存の `namespace` を残して round-trip を保証する。
  - 逆方向の `with_stream_extension : t -> Run_config.t -> Run_config.t` を追加し、`Core_parse_streaming` で算出したヒントを RunConfig へ再投影できるようにする。

- **モジュール分割方針**
  - `compiler/ocaml/src/core_parse_streaming_types.{ml,mli}` を新設し、上記型と RunConfig 依存度の低い変換ヘルパ（`of_namespace`/`to_namespace`）を定義。
  - `core_parse_streaming.ml`（Step1 で新設）では実行ループと `Core_parse` ブリッジを担当し、`Types` モジュールを参照して `Run_config.Stream` との境界を管理する。
  - 既存の `Parser_run_config.Stream` からは `Types` を `open` せず、`Extensions` マップに特化した変換関数だけを置いて依存方向を `Parser_run_config` → `Types` に限定する（循環防止）。
  - CLI/LSP 側の RunConfig ビルダーでは、`Demand_hint` / `Flow_controller` の JSON 表現を `parser-runconfig-*.json` に追記し、CI の `parser.runconfig_*` 指標で欠落を検知できるようにする。

### Step 1. Core.Parse.Streaming モジュール骨格の抽出（2日）
- 目的: バッチランナーから共通処理を切り出し、新規モジュール `core_parse_streaming.{ml,mli}`（仮）に PoC 用の最小骨格を定義する。
- 主な作業:
  - `Core_parse`／`Parser_diag_state` の初期化・診断集約ロジックを再利用できる形で関数化し、ストリーミング側から呼べるようにする。
  - Packrat 状態 (`Core_parse.State`) と `RunConfig` を引き回すインターフェイスを整理し、`Core_parse.Streaming` の公開シグネチャを設計する。
- 調査・検証:
  - `parser_driver.ml:120-320` 辺りの初期化順序を確認し、必要な引数（`lex_pack`, `config`, `diag_state`）を抜け漏れなくモジュールへ伝達する。
  - `docs/spec/2-6-execution-strategy.md` §B のトランポリン要件と整合するかチェックする。

> 2026-??-?? 更新: Step 1 完了。`compiler/ocaml/src/core_parse_streaming.{ml,mli}` を新設し、`Parser_diag_state`/`Core_parse.State` の初期化・Packrat 指標記録・診断登録・`require_eof` 判定・コアルール監査メタデータ付与を共通化するセッション API を定義。`parser_driver.ml` は新セッション (`Core_parse_streaming.create_session`) を用いるようリファクタし、既存の Packrat 集計・期待集合サマリ生成・診断最終化処理をモジュール経由に統合。`compiler/ocaml/src/dune` へ `core_parse_streaming` を追加してビルド対象を登録済み。

### Step 2. Feeder / Continuation / DemandHint モデル設計（2日）
- 目的: ストリーミング固有の型 (`Feeder`, `StreamOutcome`, `DemandHint`, `ContinuationMeta`) を PoC で表現し、診断との接点を定義する。
- 主な作業:
  - 仕様準拠の型定義を OCaml 版で表現し、`RunConfig.extensions["stream"]` から `resume_hint` 等を復元する処理を実装する。
  - `ContinuationMeta.expected_tokens` に `ERR-001` で導入した `ExpectationSummary` を流し込む接合部を作成し、CLI/LSP と同一の期待集合表示を保証する。
- 調査・検証:
  - `docs/spec/2-7-core-parse-streaming.md` §B-§C の契約を再読し、最低限 PoC で必要なフィールド（`min_bytes`, `commit_watermark`, `last_checkpoint` 等）を確定。
  - `parser_diag_state.ml` の `farthest_snapshot`・`span_trace_pairs` を参照し、継続に載せるデータ量の制約を調査する。

> 2026-01-12 更新: Step 2 を完了。Feeder/DemandHint/Continuation 型の PoC 仕様と RunConfig 変換スキームを確定し、`ContinuationMeta.expected_tokens` に `ExpectationSummary` を取り込む導線を実装範囲に落とし込んだ。バックプレッシャ関連フィールドと監査メタデータの収容方針も整理したため、Step 3 では制御ループ実装に専念できる。

#### Step 2 実施記録

- **型定義（`core_parse_streaming_types.{ml,mli}`）**
  - `Demand_hint.t` を以下の構造で定義し、仕様の `min_bytes`／`preferred_bytes`／`frame_boundary` を OCaml へ写像。`reason` は診断ログ用の自由記述フィールドとして `string option` を追加し、PoC の監査結果をそのまま `StreamMeta` に転送できるようにした。

    ```ocaml
    module Demand_hint = struct
      type frame_boundary =
        | Token_class of Parser_token.classification
        | Span_boundary of Diagnostic.span

      type t = {
        min_bytes : int;
        preferred_bytes : int option;
        frame_boundary : frame_boundary option;
        reason : string option;
      }
    end
    ```

  - Feeder 関連は関手ではなくファーストクラス関数で扱い、PoC では同期 pull だけを前提とする。将来の `Core.Async` 連携を妨げないよう、`pull` の返り値をバリアント `Feeder.yield` として分離し、`Pending`/`Await` を用意した。

    ```ocaml
    module Feeder = struct
      type chunk = Bytes.t

      type yield =
        | Chunk of chunk
        | Await
        | Closed
        | Error of Stream_error.t

      type t = Demand_hint.t -> yield
    end
    ```

  - `Stream_outcome.t` / `Continuation.t` は Step1 で定義した `session` 抽象を再利用。`Continuation.t` には `Core_parse_streaming.Session.t` を `state` として保持し、`meta` を `Continuation_meta.t` でラップする構造とした。

- **RunConfig 拡張との橋渡し**
  - `RunConfig.extensions["stream"]` は 3 つのネームスペースを利用する方針に整理。

    | Namespace | 主キー | 説明 |
    |-----------|--------|------|
    | `"stream"` | `checkpoint`, `flow_mode`, `resume_reason` | チェックポイントとフローモード既定値、直前の `DemandHint` 理由メモを格納 |
    | `"stream.demand"` | `min_bytes`, `preferred_bytes`, `frame_boundary` | `Demand_hint.t` をそのまま保存する |
    | `"stream.flow"` | `high_watermark`, `low_watermark`, `policy` | Backpressure 設定（Step3 で利用）をシリアライズ |

  - `Demand_hint.of_namespace` / `to_namespace` を実装し、未知のキーは警告ログに集約。`preferred_bytes` は 0 以下を拒否し、仕様同様に `min_bytes <= preferred_bytes` を強制する。
  - `Continuation_meta.t` への復元では、`checkpoint` を `commit_watermark` へ、`stream.demand` を `resume_hint` へマッピング。`Continuation_meta.buffered` には `Parser_buffer.Snapshot`（Step1 で導入）を採用し、バイナリコピーを避ける。

- **診断・期待集合との接合**
  - `parser_diag_state.farthest_snapshot` から `Diagnostic.expectation_summary` を抜き出し、`Continuation_meta.expected` に格納。現状の診断状態は `alternatives` を最大 12 件に正規化しているため、継続に保持してもメモリ増大は限定的であることを確認（`normalize_expectations` の結果を再利用）。
  - `Continuation_meta.last_checkpoint` は `parser_diag_state.span_trace_pairs` の最後の要素から推定し、`trace` 無効時は `None` を保持。`trace_label` は最後に取得したラベル文字列をそのまま登録し、LSP で補完候補を提示する際のラベルとして利用する。
  - `Stream_meta.diagnostics_pending`（Step3 で導入予定）の下準備として、`Continuation_meta.resume_hint` と `expected_tokens` を `Diagnostic.Builder` へ渡す関数 `Continuation_meta.to_resume_note` を定義した。

- **調査結果と制約整理**
  - `parser_diag_state` の `farthest_snapshot.expected` はリスト長が可変だが、`ExpectationSummary` が存在する場合はそちらを優先しており重複排除済みであることを確認。継続メタデータには `ExpectationSummary` を保持し、必要に応じて `alternatives` を `Array.sub` で 16 件までに切り詰めるルールを追加。
  - `Demand_hint.frame_boundary` に `Token_class` を保持するため、`Parser_token.classification` のシリアライズ表現（`string` キー）を `Parser_token.Class.to_symbol` / `of_symbol` で往復可能であることを手動検証。未知シンボルは `Frame_boundary.Unsupported` として `Stream_error_kind.FeederBug` を返す設計とした。
  - 継続メタデータの最大サイズを計測し、`Demand_hint`（40B 前後）＋`ExpectationSummary`（約 200B）＋`Span`（48B）＋`Buffer snapshot` 参照という構成で 512B 程度に収まる見込みを確認。`Pending` の JSON 監査出力も 1.5KB 前後に維持できる。

- **フォローアップ**
  - Step3 で `Stream_outcome.Pending` を生成する際、直前に計算した `Demand_hint` を共有して GC 圧力を避ける必要がある。`Continuation_meta.resume_hint` と `Stream_outcome.Pending.demand` を同一レコードで共有する実装メモを `core_parse_streaming.ml` に TODO として記入。
  - `Feeder` が `Await` を返した場合のトレーシング（`StreamEvent::Pending`) を Step4 で CLI/LSP へ露出する。監視メトリクス名 `parser.stream.await_ratio` を `0-3-audit-and-metrics.md` のドラフト欄へ追記済み。

### Step 3. ストリーミング制御ループ PoC（3日）
- 目的: `run_stream` / `resume` を実装し、チャンク読み取り・バックプレッシャ・診断発火を最小限動作させる。
- 主な作業:
  - `Feeder.pull` → `Core_parse.Streaming.step` → `StreamOutcome` への制御フローを構築し、`Pending` と `Completed` の遷移を確認。
  - Packrat のコミットウォーターマークと入力バッファのライフサイクルを管理し、`Pending` 返却時に `DemandHint` を算出する。
  - `RunConfig extensions["stream"]` のプレースホルダ（`checkpoint`, `resume_hint`, `flow_mode` 等）を読み取るロジックを追加。
- 調査・検証:
  - `Core_parse.State.record_packrat_access` の呼び出しタイミングをバッチと比較し、Packrat 指標の整合を確認。
  - `docs/guides/compiler/core-parse-streaming.md` §5-§7 のドライバ例を PoC に当てはめ、欠落 API を洗い出す。

> 2026-01-16 更新: Step 3 の PoC 制御ループ設計を完了。Feeder からの入力を `StreamDriver` が状態遷移形式で受け取り、`Pending`/`Completed` の分岐と `DemandHint` 再計算を含む骨格を確定した。RunConfig 連携と診断橋渡しの整合も合わせて確認済み。

#### Step3 実施記録（2026-01-16）

- **制御ステートマシン**  
  `StreamDriver` を `Parsing` / `Awaiting` / `Draining` の 3 状態に整理し、`Core_parse.Streaming.step` が Menhir チェックポイントを推進するたびに現在の状態と `StreamOutcome` を同時に更新する設計にした。核心ループは次のように整理している。

  ```reml
  fn pump(state, checkpoint, input) -> StreamOutcome {
    match CoreParse.step(checkpoint, input.buffer) {
      | NeedMore(hint) ->
          StreamOutcome::Pending {
            continuation = Continuation::snapshot(state, input),
            demand = hint,
            meta = StreamMeta::await(input)
          }
      | Produced(reply) ->
          StreamOutcome::Completed {
            result = finalize(reply, state.session),
            meta = StreamMeta::from_session(state.session, input)
          }
    }
  }
  ```

- **Feeder イベントと遷移**  
  Feeder の戻り値とドライバ状態を以下の対応表に整理し、`docs/spec/2-7-core-parse-streaming.md` §A-B の契約に沿って遷移を定義した。

  | Feeder.yield | 受信時の状態 | 次状態 | `StreamOutcome` への反映 |
  |--------------|--------------|--------|--------------------------|
  | `Chunk bytes` | `Parsing` または `Awaiting` | `Parsing` | バッファへ追記し、そのまま `Core_parse` へ供給。`StreamMeta.chunks_consumed` を増分。 |
  | `Await` | `Parsing` | `Awaiting` | `DemandHint.reason = "feeder.await"` を設定し `Pending` を返却。 |
  | `Closed` | `Parsing` または `Draining` | `Draining` | バッファが空なら即座に `Token.EOF` を挿入し完了へ遷移。残バイトがある場合は消化後に `Completed`。 |
  | `Error err` | 任意 | `Draining` | `StreamOutcome::Completed` を返しつつ、診断ビルダーに `Stream_error_kind` を橋渡し。 |

- **DemandHint とバックプレッシャ**  
  `Demand_hint.min_bytes` は現在の `packrat.commit_watermark` と `input.buffered_bytes` から `max(64, watermark - buffered)` で算出するルールを暫定採用。`preferred_bytes` は `flow.high_watermark` が指定されていれば `min(high_watermark, demand_cap)`、未指定の場合は `preferred = min_bytes * 4` とし、`frame_boundary` は `Parser_token.Class` を `extensions["stream.demand"]` から復元する。`Await` を挟んだ場合は `reason = Some "feeder.await"` とし、`resume` 呼び出し時に手元のチャンク計画を再評価できるようにした。

- **診断・Packrat 連携**  
  `Parser_diag_state` から抽出した `ExpectationSummary` を `Continuation_meta.expected_tokens` に格納し、`docs/spec/2-7-core-parse-streaming.md` §C に記載された `resume_notes` オプションに備えた。Packrat 指標は `Core_parse.State.packrat_queries/hits` を `StreamMeta.packrat = Some (queries, hits)` で転記、`Pending` 時には未消化の問い合わせがあっても再度要求できることを確認した。`Stream_meta.diagnostics_pending` は `Parser_diag_state.diagnostics` の長さで算出し、`Pending` を返す際に通知する。

- **RunConfig / Continuation 同期**  
  `RunConfig.extensions["stream"]` から取得した `checkpoint` / `flow_mode` / `demand` 情報を `Continuation_meta` の `commit_watermark`・`resume_hint` に反映し、`resume` 実行後は `with_stream_extension` で次回呼び出しへ戻す手順を定義。継続には `Trace_label` と `span_trace` の末尾を保存し、IDE 補助が `docs/guides/compiler/core-parse-streaming.md` §6 で示す補完 UI と整合することを確認した。

- **測定とフォローアップ**  
  `StreamMeta` へ `bytes_consumed` / `chunks_consumed` / `await_count` を追加し、`0-3-audit-and-metrics.md` で新設する `parser.stream.await_ratio`・`parser.stream.demandhint_coverage` の計測式を定義。Step 4 では CLI/LSP 図面へイベントを配線し、`Pending` から `resume` までのレイテンシを取得する TODO を `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に追記する。

### Step 4. CLI/LSP/CI 連携と検証ケースの整備（2.5日）
- 目的: 新ランナーを CLI/LSP/テストに統合し、PoC の振る舞いを自動検証できる状態にする。
- 主な作業:
  - `compiler/ocaml/src/main.ml`, `tooling/lsp/run_config_loader.ml` へ `--streaming` 相当のハンドラを追加し、`RunConfig` 経由でストリーミング設定を配線。
  - `compiler/ocaml/tests/streaming_runner_tests.ml`（新設）で `run` と `run_stream` の結果一致、`Pending DemandHint::Pause` の分岐、`resume` の往復をテスト。
  - `tooling/ci/collect-iterator-audit-metrics.py` と `scripts/validate-diagnostic-json.sh` にストリーミング用メトリクス/サンプルを追加し、CI で欠落を検知できるようにする。
- 調査・検証:
  - 既存ゴールデン（`parser-runconfig-packrat.json.golden`）を基にストリーミング版フィクスチャを作成し、差分測定方法を決める。
  - LSP 側で既に実装済みのインクリメンタル診断フローを確認し、PoC との整合課題を洗い出す。

> 2026-01-24 更新: Step4 を実施し、ストリーミング PoC を CLI/LSP/CI へ統合した。
>
> - CLI: `compiler/ocaml/src/cli/options.ml` に `--streaming` / `--stream-chunk-size` / `--stream-checkpoint` などのフラグを追加し、`Parser_run_config.Stream` へ `enabled`・`demand_min_bytes`・`chunk_size` を書き戻す。`compiler/ocaml/src/main.ml` は `opts.parser_streaming` または RunConfig 側の `stream.enabled` が真の場合に `Parser_driver.Streaming.run_stream` を使用する。
> - LSP: `tooling/lsp/run_config_loader.ml` が `enabled` / `chunkSize` / `demandMinBytes` / `demandPreferredBytes` を復号し、CLI と同じネームスペースを共有する。
> - ランナー実装: `compiler/ocaml/src/parser_driver.ml` に `Parser_driver.Streaming` モジュールを追加。`run_stream` と `resume` が `DemandHint`（`action="pause"`, `reason="feeder.await"` など）と `stream_meta`（`bytes_consumed` / `await_count` / `resume_count`）を算出し、`RunConfig.Stream` の既定値を不足フィールドへ補完する。
> - テスト&ゴールデン: `compiler/ocaml/tests/streaming_runner_tests.ml` を追加し、`run_string` との一致と `Pending`→`resume` パスを検証。`compiler/ocaml/tests/golden/diagnostics/parser/streaming-outcome.json.golden` を新設し、ストリーミング指標を含む RunConfig/stream_meta のサンプルを共有。
> - CI/検証: `tooling/ci/collect-iterator-audit-metrics.py` に `parser.stream_extension_field_coverage` を追加し、`enabled`・`demand_*`・`chunk_size` の出現率を監視。`scripts/validate-diagnostic-json.sh` は `stream_meta` の整数フィールドと `last_reason` を検証する。
> - 既知の制限: PoC はチャンクを蓄積してから既存の `Parser_driver.run` を呼び出す方式であり、バックプレッシャ制御や Packrat キャッシュ共有は未着手。`Stream.resume` の `Error` 経路も監査イベントへ転送していないため、Step5 で Packrat/追跡強化タスクへ引き継ぐ。

### Step 5. ドキュメント・フォローアップ登録（1.5日）
- 目的: PoC の制限・今後の課題を記録し、Phase 2-7 以降のフル実装に繋げる。
- 主な作業:
  - `docs/guides/compiler/core-parse-streaming.md`、`docs/spec/2-6-execution-strategy.md`、`docs/spec/2-7-core-parse-streaming.md` に PoC 状態と既知制限を脚注追加。
  - `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` と `docs/guides/runtime/runtime-bridges.md` へ制限/連携要件を追記。
  - `0-3-audit-and-metrics.md` に新規指標（例: `parser.stream.outcome_consistency`, `parser.stream.demandhint_coverage`）を登録し、計測結果を記録する。
- 調査・検証:
  - Phase 3 計画書におけるストリーミング要件を確認し、PoC で満たせていない項目を列挙。
  - CLI/LSP チームとレビューを実施し、PoC 公開チャネル（`-Zstreaming` フラグ等）の合意形成状況をまとめる。

> 2026-01-26 更新: Step5 を実施し、PoC 状態の公開と Phase 2-7 フォローアップ登録を完了。
> - 仕様・ガイド更新: `docs/spec/2-6-execution-strategy.md` / `docs/spec/2-7-core-parse-streaming.md` に Phase 2-5 PoC の脚注を追加し、`RunConfig.extensions["stream"]` の監視指標と未解決の制限（Packrat 共有・バックプレッシャ自動化・Pending/Error 監査）を明文化。`docs/guides/compiler/core-parse-streaming.md` §10 へ PoC 状況サマリを追加し、利用者向けに既知制限を提示。
> - フォローアップ登録: `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に「ストリーミング PoC フォローアップ」セクションを新設し、Phase 2-7 で解消するタスク（Packrat キャッシュ共有、バックプレッシャ自動化、監査連携、CLI メトリクス統合、Runtime Bridge 連携）を列挙。`docs/guides/runtime/runtime-bridges.md` へストリーミング信号の橋渡し手順と監査要件を追加。
> - メトリクス登録: `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に `parser.stream.outcome_consistency` / `parser.stream.demandhint_coverage` を追加し、`tooling/ci/collect-iterator-audit-metrics.py --require-success` と `scripts/validate-diagnostic-json.sh` を通じて CI 監視する運用を確定。
> - レビュー記録: `docs/plans/bootstrap-roadmap/2-5-review-log.md` へ Step5 実施ログを追記し、CLI/LSP 連携チームとの合意内容（`-Zstreaming` の公開チャネル方針は Phase 2-7 で決定、自動化タスクは上記フォローアップへ移送）をまとめた。

## 5. フォローアップ
- PoC の仕様制限（例: チャンクサイズ固定、`Pending` の暗黙タイムアウト未実装）を `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に記録し、本実装フェーズへ引き継ぐ。  
- `docs/guides/compiler/core-parse-streaming.md` のサンプルを PoC API で動作させ、差分を脚注として追加。  
- Phase 3 の self-host 計画書で `run_stream` をクリティカルパスに含めるため、進捗を `0-3-audit-and-metrics.md` に定期記録する。
- `docs/guides/runtime/runtime-bridges.md` にストリーミング API と Runtime Bridge の連携要件を追記し、バックプレッシャ信号の橋渡し方法を明文化する。
- **タイミング**: PARSER-001/002/LEXER-002 が揃った Phase 2-5 後半に PoC 実装へ着手し、Phase 2-6 開始前までに最小機能のストリーミングランナーを完成させる。

## 6. 残課題
- Feeder API とバックプレッシャの初期値（`DemandHint::Continue` / `::Pause`) をどの程度細分化するか決定が必要。  
- PoC をどのリリースチャネルに公開するか（`-Zstreaming` フラグの導入有無）を Phase 2-7 チームと合意したい。
