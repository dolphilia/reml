# 第3部 第9章: 実行パイプライン 調査メモ

## 参照した資料
- `compiler/frontend/src/pipeline/mod.rs:1-319`
- `compiler/frontend/src/streaming/mod.rs:1-600`
- `compiler/frontend/src/streaming/flow.rs:1-178`
- `compiler/frontend/src/parser/streaming_runner.rs:1-165`
- `compiler/frontend/src/parser/mod.rs:315-424`
- `compiler/frontend/src/parser/mod.rs:6801-7037`
- `compiler/frontend/src/bin/reml_frontend.rs:545-720`
- `compiler/frontend/src/bin/reml_frontend.rs:1878-1934`
- `docs/spec/2-6-execution-strategy.md`
- `docs/spec/2-7-core-parse-streaming.md`

## 調査メモ

### CLI パイプライン監査の基本構造
- `PipelineDescriptor` は CLI 実行に紐づく識別子（input/path/run_id/phase 等）とスキーマバージョンを束ね、監査イベントの共通メタデータを生成する。(`compiler/frontend/src/pipeline/mod.rs:16-107`)
- `AuditEmitter` は `pipeline_started` / `pipeline_completed` / `pipeline_failed` / `config_compat_changed` を生成し、`AuditEnvelope` を JSON ラインで出力する。(`compiler/frontend/src/pipeline/mod.rs:153-303`)
- `pipeline_identifiers` は入力パスから `dsl://` の識別子を生成し、パイプライン ID とノード名を決める。(`compiler/frontend/src/pipeline/mod.rs:306-318`)

### ストリーミング状態（Packrat + span_trace）
- `StreamingStateConfig` は Packrat/SpanTrace の有効化とバジェット・上限を保持する。(`compiler/frontend/src/streaming/mod.rs:30-47`)
- Packrat は `IndexMap<(parser_id, range)>` を `RwLock` で保持し、統計値 (`PackratStats`) を原子的に集計する。(`compiler/frontend/src/streaming/mod.rs:168-257`, `283-421`)
- `SpanTrace` は `VecDeque<TraceFrame>` に保存され、`trace_limit` を超えると古いフレームを落とす。(`compiler/frontend/src/streaming/mod.rs:459-487`)
- `StreamMetrics` は Packrat と SpanTrace の統計を束ね、CLI の `--emit parse-debug` などで参照される。(`compiler/frontend/src/streaming/mod.rs:69-81`, `494-499`)

### ストリームフロー制御とブリッジシグナル
- `StreamFlowConfig` は CLI 由来のストリーム設定（resume_hint/flow_policy/demand_* など）を保持する。(`compiler/frontend/src/streaming/flow.rs:11-22`)
- `StreamFlowState` は checkpoint の終了回数や backpressure/resume/await のカウントを保持し、最新の `RuntimeBridgeSignal` を履歴として保存する。(`compiler/frontend/src/streaming/flow.rs:99-177`)
- `RuntimeBridgeSignal` は runtime との橋渡し用で、`StageTraceStep` を付随させる設計になっている。(`compiler/frontend/src/streaming/flow.rs:52-75`)

### StreamingRunner の実装
- `StreamingRunner` は `Continuation` を保持し、`run_stream` / `resume` を `ParserDriver` への薄いラッパとして提供する。(`compiler/frontend/src/parser/streaming_runner.rs:57-128`)
- 現在の実装では `chunk_size` を `RunConfig.extensions["stream"]` から読み、`buffer` を一定サイズで進めるだけの簡易ストリーミングになっている。(`compiler/frontend/src/parser/streaming_runner.rs:93-156`)
- `StreamMeta` は `ParseResult` が持つ `stream_metrics` と `StreamFlowState` の統計をまとめる。(`compiler/frontend/src/parser/streaming_runner.rs:6-21`)

### パーサ側でのストリーミング連携
- `ParserDriver::parse_with_options` は `StreamingState` を作成し、`stream_flow_state` の有無で `streaming_enabled` を決める。(`compiler/frontend/src/parser/mod.rs:344-360`)
- パース中のエラーは `StreamingRecoverController` を通じて診断に集約し、checkpoint 単位で merge される。(`compiler/frontend/src/parser/mod.rs:373-384`, `6908-7037`)
- `record_streaming_error` / `record_streaming_success` は span_trace と Packrat エントリを生成し、CLI/監査側が参照する統計を暖気する。(`compiler/frontend/src/parser/mod.rs:6801-6905`)

### CLI での実行パイプライン
- `main` は `PipelineDescriptor` を生成し、`pipeline_started`/`config_compat_changed` の監査イベントを先に送る。(`compiler/frontend/src/bin/reml_frontend.rs:545-592`)
- `run_frontend` では `StreamFlowState` を生成し、`RunConfig` に `stream` 拡張（chunk_size, resume_hint 等）を付与してパーサを起動する。(`compiler/frontend/src/bin/reml_frontend.rs:656-678`)
- `StreamSettings` は CLI や workspace 設定からのストリーム設定を保持し、`StreamFlowConfig` に変換する。(`compiler/frontend/src/bin/reml_frontend.rs:1878-1902`)
- `CliArgs::streaming_state_config` は Packrat/trace の有効化を CLI 設定に合わせて調整する。(`compiler/frontend/src/bin/reml_frontend.rs:1905-1911`)

### 仕様との対応
- 実行パイプラインは `docs/spec/2-6-execution-strategy.md` の RunConfig 拡張と、`docs/spec/2-7-core-parse-streaming.md` の streaming runner / DemandHint 仕様に対応する。
- 現実装の `StreamingRunner` は chunk_size ベースで、spec の Feeder/FlowController による backpressure とは異なる簡易版になっているため、差分を章で明記する必要がある。

### RuntimeBridgeSignal の生成箇所
- `RuntimeBridgeSignal` 自体の定義は `compiler/frontend/src/streaming/flow.rs` にあるが、生成して `record_bridge_signal` を呼ぶ実装は現状見当たらない。(`compiler/frontend/src/streaming/flow.rs:52-177`)\n- `StreamingRunner::record_bridge_signal` は用意されているが、呼び出し側が存在しないため、`bridge_signal` は通常 `None` のままになる。(`compiler/frontend/src/parser/streaming_runner.rs:57-85`, `compiler/frontend/src/bin/reml_frontend.rs:693-733`)\n- `diagnostic/effects.rs` では監査メタデータへの埋め込みを実装しているが、実運用ではテスト以外で payload が生成されていない。(`compiler/frontend/src/diagnostic/effects.rs:445-525`)\n+
### merge_stream_extension の実装
- `merge_stream_extension` は既存の `RunConfig.extensions["stream"]` をオブジェクトとして読み取り、`stream_config_payload` のキーで上書きする。(`compiler/frontend/src/bin/reml_frontend.rs:3328-3342`)\n- `stream_config_payload` は `enabled`/`checkpoint`/`resume_hint`/`demand_*`/`chunk_size`/`flow_*`/`packrat_enabled` を必ず挿入し、未指定値は `"unspecified"` または `0` に正規化する。(`compiler/frontend/src/bin/reml_frontend.rs:3272-3324`)\n+
### 未確認事項 / TODO
- `stream_flow_state` の `flow_policy` / `demand_*` が実際にどの箇所で利用されるかを確認する。
