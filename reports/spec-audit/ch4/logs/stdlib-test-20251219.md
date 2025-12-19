# Phase 4 stdlib Core.Test ログ

- 生成時刻: 2025-12-19 07:33:56Z
- 対象: CH3-TEST-401

## 実行詳細

### CH3-TEST-401

- ファイル: `examples/practical/core_test/snapshot/basic_ok.reml`
- 期待 Diagnostics: `[]`
- 実際 Diagnostics: `[]`
- Exit code: 0
- CLI: `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin reml_frontend -- --output json examples/practical/core_test/snapshot/basic_ok.reml`
- run_id: `3c6942ed-af7c-4808-8100-057c1102968d`
- stdout 先頭行:

```
{"command":"Check","phase":"Reporting","run_id":"3c6942ed-af7c-4808-8100-057c1102968d","diagnostics":[],"summary":{"inputs":["examples/practical/core_test/snapshot/basic_ok.reml"],"started_at":"-4657-11-12T07:33:49Z","finished_at":"-4657-11-12T07:33:49Z","artifact":null,"stats":{"cli_command":"compiler/rust/frontend/target/debug/reml_frontend --output json examples/practical/core_test/snapshot/basic_ok.reml","diagnostic_count":0,"filtering":{"audit_policy_anonymized":0,"audit_policy_dropped":0,"suppressed":0},"parse_result":{"farthest_error_offset":null,"packrat_cache":[{"entry":{"approx_bytes":118,"expectations":[],"sample_tokens":[{"kind":"Success","lexeme":"function"}],"summary":{"alternatives":[],"humanized":"success"}},"parser_id":2,"range_end":236,"range_start":68}],"packrat_snapshot":{"approx_bytes":118,"entries":1},"packrat_stats":{"approx_bytes":118,"budget_drops":0,"entries":1,"evictions":0,"hits":2,"pruned":0,"queries":3},"recovered":false,"span_trace":[],"trace_events":[{"event_kind":"module_header_accepted","label":"Examples.Practical.CoreTest.Snapshot.BasicOk","span":{"end":51,"start":0},"trace_id":"syntax:module-header"},{"event_kind":"use_decl_accepted","label":"Core.Test","span":{"end":66,"start":53},"trace_id":"syntax:use"},{"event_kind":"expr_enter","label":"block","span":{"end":236,"start":85},"trace_id":"syntax:expr::block"},{"event_kind":"expr_enter","label":"let","span":{"end":151,"start":89},"trace_id":"syntax:expr::let"},{"event_kind":"expr_enter","label":"call","span":{"end":151,"start":103},"trace_id":"syntax:expr::call"},{"event_kind":"expr_enter","label":"field-access","span":{"end":123,"start":103},"trace_id":"syntax:expr::field-access"},{"event_kind":"expr_enter","label":"identifier","span":{"end":107,"start":103},"trace_id":"syntax:expr::identifier"},{"event_kind":"expr_leave","label":"identifier","span":{"end":107,"start":103},"trace_id":"syntax:expr::identifier"},{"event_kind":"expr_leave","label":"field-access","span":{"end":123,"start":103},"trace_id":"syntax:expr::field-access"},{"event_kind":"expr_enter","label":"literal","span":{"end":141,"start":124},"trace_id":"syntax:expr::literal"},{"event_kind":"expr_leave","label":"literal","span":{"end":141,"start":124},"trace_id":"syntax:expr::literal"},{"event_kind":"expr_enter","label":"literal","span":{"end":150,"start":143},"trace_id":"syntax:expr::literal"},{"event_kind":"expr_leave","label":"literal","span":{"end":150,"start":143},"trace_id":"syntax:expr::literal"},{"event_kind":"expr_leave","label":"call","span":{"end":151,"start":103},"trace_id":"syntax:expr::call"},{"event_kind":"expr_leave","label":"let","span":{"end":151,"start":89},"trace_id":"syntax:expr::let"},{"event_kind":"expr_enter","label":"match","span":{"end":234,"start":154},"trace_id":"syntax:expr::match"},{"event_kind":"expr_enter","label":"identifier","span":{"end":167,"start":160},"trace_id":"syntax:expr::identifier"},{"event_kind":"expr_leave","label":"identifier","span":{"end":167,"start":160},"trace_id":"syntax:expr::identifier"},{"event_kind":"expr_enter","label":"literal","span":{"end":201,"start":188},"trace_id":"syntax:expr::literal"},{"event_kind":"expr_leave","label":"literal","span":{"end":201,"start":188},"trace_id":"syntax:expr::literal"},{"event_kind":"expr_enter","label":"literal","span":{"end":234,"start":218},"trace_id":"syntax:expr::literal"},{"event_kind":"expr_leave","label":"literal","span":{"end":234,"start":218},"trace_id":"syntax:expr::literal"},{"event_kind":"expr_leave","label":"match","span":{"end":234,"start":154},"trace_id":"syntax:expr::match"},{"event_kind":"expr_leave","label":"block","span":{"end":236,"start":85},"trace_id":"syntax:expr::block"}]},"run_config":{"extensions":{"config":{"ack_experimental_diagnostics":false,"compatibility":{"duplicate_key":"error","feature_guard":[],"number":"strict","trailing_comma":"arrays_and_objects","trivia":{"block":[],"doc_comment":null,"hash_inline":true,"line":["#","//"],"shebang":false},"unquoted_key":"allow_alpha_numeric"},"compatibility_profile":"toml-relaxed","compatibility_source":"default","experimental_effects":false,"left_recursion":"off","legacy_result":true,"merge_warnings":true,"packrat":true,"require_eof":false,"source":"cli","trace":false},"effects":{"type_row_mode":"ty-integrated"},"lex":{"identifier_profile":"unicode","profile":"strict_json"},"recover":{"notes":false,"sync_tokens":[]},"stream":{"checkpoint":"unspecified","chunk_size":0,"demand_min_bytes":0,"demand_preferred_bytes":0,"enabled":false,"flow":{"await_count":0,"backpressure":{"max_lag_bytes":0},"backpressure_count":0,"checkpoints_closed":1,"policy":"auto","resume_count":0},"flow_max_lag":0,"flow_policy":"auto","packrat_enabled":true,"resume_hint":"unspecified"},"target":{"capabilities":[],"detected":{"arch":"aarch64","family":"unix","os":"macos","runtime_revision":null,"stdlib_version":null},"diagnostics":true,"extra":{},"features":[],"profile_id":"macos-aarch64","requested":{"arch":"aarch64","capabilities":[],"diagnostics":true,"family":"unix","features":[],"os":"macos"},"runtime_revision":null,"stdlib_version":null,"triple":null}},"runtime_capabilities":[],"switches":{"ack_experimental_diagnostics":false,"experimental_effects":false,"left_recursion":"off","legacy_result":true,"merge_warnings":true,"packrat":true,"require_eof":false,"trace":false}},"stream_meta":{"bridge":null,"flow":{"await_count":0,"backpressure_count":0,"checkpoints_closed":1,"resume_count":0},"last_reason":null,"packrat":{"approx_bytes":118,"budget_drops":0,"entries":1,"evictions":0,"hits":2,"pruned":0,"queries":3},"packrat_enabled":true,"span_trace":{"dropped":0,"retained":0}}}},"exit_code":{"label":"success","value":0}}
```

- expected: `expected/practical/core_test/snapshot/basic_ok.stdout` は CLI JSON 出力に合わせて暫定更新。
- 備考: CLI は JSON 出力のため stdout は診断サマリ。Runtime 実行による `snapshot:ok` 出力は未実装。

## 監査イベント確認（snapshot.updated）

- CLI: `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin reml_frontend -- --emit-audit --output json examples/practical/core_test/snapshot/basic_ok.reml`
- event.kind: `snapshot.updated`

```
{"timestamp":"2025-12-19T08:05:28.715929Z","envelope":{"capability":"core.test","metadata":{"event.domain":"test","event.kind":"snapshot.updated","snapshot.hash":"4361722783805985690","snapshot.name":"core_test_basic"}}}
```

## 診断確認（test.failed）

- CLI: `REML_CORE_TEST_FORCE_FAIL=1 cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin reml_frontend -- --output json examples/practical/core_test/snapshot/basic_ok.reml`
- diagnostics: `["test.failed"]`（exit_code=failure）
