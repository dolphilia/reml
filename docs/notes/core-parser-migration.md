# Core Parser Migration メモ

## P1 W1: Rust Parser 戦略と状態管理（2025-11-28）
- パーサ生成は `logos`（字句）と `chumsky`（構文）を組み合わせたコンビネータ方式を第一候補とし、`pomelo` ベースの LR(1) 生成をフォールバック案として保持する。`lalrpop` はエラー回復の柔軟性と生成物サイズの理由で除外した[^frontend-eval]。
- Rust 側の `ParserSession`（仮称）は `lexer::SourceBuffer` と `streaming::StreamingState` を束ね、`Core_parse.State`/`Core_parse_streaming.Session` が担っていた `diagnostics`・`packrat_cache`・`span_trace` を `Arc<SessionShared>` へ集約する。`SessionShared` は `AtomicUsize` で `parser.stream.*` メトリクスを更新し、`indexmap::IndexMap` で `PackratEntry`（キー: `(ParserId, ByteRange)`）を管理する。
- `ParserDriver` は `RunConfig` と `ParserFlags` を受け取り、`Driver::parse` 内で `ParserSession::enter(rule_id)`→`ParserSession::commit(rule_id)`→`ParserSession::reply()` のシーケンスを提供する。OCaml 版 `Core_parse.Reply.{consumed,committed}` は Rust 側で `ReplyFlags`（`bitflags!`）に正規化し、JSON 連携時に `Diagnostic.Builder` 相当の API へ橋渡しする。
- W1 の成果として `docs/plans/rust-migration/appendix/frontend-crate-evaluation.md` に加え、`ParserSession`/`StreamingState` の責務分割と PoC のストーリーポイントを本メモに記録し、W2 以降の AST/IR 対応表作業へ引き渡す。
- `scripts/poc_dualwrite_compare.sh` を追加し、`reports/dual-write/front-end/poc/2025-11-28-logos-chumsky/summary.md` に AST/診断比較結果（OCaml vs Rust）を保存。ケース `missing_paren` も含め、診断件数が一致することを確認済み。
- `parser::driver::tests::basic_roundtrip` と `tests/streaming_metrics.rs` を実装し、`cargo test` で AST ラウンドトリップと Packrat メトリクス操作を自動検証。`scripts/poc_dualwrite_compare.sh` の再実行で 4 ケースの AST/診断件数が OCaml ベースラインと同一であることを確認した。

### Parser 生成サブシステム
- `chumsky` は規則ごとに `fn module_decl(input) -> Parser<'a, Token, Ast::ModuleDecl>` のような静的関数を生成するテンプレートを採用し、Menhir の `parser.mly` を機械的に写経できるよう `parser/templates/` にスクリプトを配置する。
- `Recover` 系 API は `chumsky::Parser::recover_with` をベースにしつつ、`RecoverBudget` を `RunConfig.recover_budget` から供給する拡張ラッパを作成する。これにより OCaml 実装の `Parser_expectation`→`Diagnostic.Builder` の流れと同じ `expected_tokens`/`message` を生成できる。
- フォールバックとして `pomelo` を利用する場合は、`parser/build.rs` で `.pom` ファイルからテーブルを生成し、`SessionShared` の `packrat_cache` を共有する構成にする。PoC の段階で `parser/tests/pomelo_roundtrip.rs` を用意し、`dual-write` 比較に参加できるようにする。

### StreamingState と Packrat 設計
- `StreamingState` は `packrat_cache` と `span_trace` の両方を `RwLock` で管理し、読取パスと書込パスを明示する。Packrat のキーには `ParserId` と `byte_range` を採用し、`ParserId` は `u16`、`byte_range` は `ops::Range<u32>` に正規化する。`span_trace` は `VecDeque<TraceFrame>` として保存し、`RunConfig.trace_limit` を超えた場合は古いフレームから削除する。
- OCaml 実装から移植する重要メソッドは以下の対応でラップする：`Core_stream.register_diagnostic` → `StreamingState::push_diagnostic`、`Core_stream.commit` → `StreamingState::mark_committed(rule_id)`、`Core_stream.packrat_cache` → `StreamingState::packrat_snapshot()`。
- `parser_expectation` に相当する `expectation::Collector` モジュールを用意し、`RecoverExtension` を生成する際に `StreamingState` へ格納する。`Collector` は `HashMap<RuleId, ExpectationSummary>` を持ち、Rust 版 `Diagnostic` で JSON 化する時に `serde_json::Value` へ変換する。

## P1 W1: Packrat / span_trace キャッシュ再現設計（2025-12-05）
- **PackratCache の型定義**  
  - `type PackratKey = (ParserId, Range<u32>)`。`ParserId` は `u16` に収め、`Range<u32>` はバイトオフセットで管理する。  
  - `struct PackratEntry` は次のフィールドを保持する：`smallvec::SmallVec<[TokenSample; 8]> sample_tokens`、`Vec<Expectation>`、`Option<Summary>`、`usize approx_bytes`。`Summary` は OCaml 版 `Diagnostic.expectation_summary` と 1:1 対応する構造体を `serde` 互換で定義する。  
  - 実装は `indexmap::IndexMap<PackratKey, PackratEntry>` を `StreamingStateShared` 内に保持し、キー順序を維持しつつ `shift_remove` で古い要素を安価に破棄できるようにする。`IndexMap` は `RwLock` で包み、読み手のホットパス（キャッシュヒット）は `try_read`、書き込みは `write` 経由で行う。
- **API とメトリクス更新ポイント**  
  - `StreamingState::lookup_packrat(key) -> Option<PackratEntryRef>`：`parser.stream.packrat_queries` を `AtomicU64` でインクリメントし、ヒット時は `parser.stream.packrat_hits` を増やす。`CollectWarmCache` 用に連続アクセスする場合は、同じロックを再利用できる `lookup_packrat_with_filter` を提供する。  
  - `StreamingState::store_packrat(key, entry)`：既存値を置き換えた場合は `parser.stream.packrat_evictions` を増やし、新規挿入時は `parser.stream.packrat_entries` を増やす。`approx_bytes` は `entry.approx_bytes` を利用し、総和を `AtomicU64` でキャッシュして `parser.stream.packrat_bytes` に報告する。  
  - `StreamingState::prune_before(offset)`：`IndexMap::retain` を用いて `range.start < offset` を削除する。削除件数とバイト数を差し引き、`parser.stream.packrat_pruned` と `parser.stream.packrat_bytes` を更新する。`collect-iterator-audit-metrics.py` が参照するキーと整合するよう、メトリクス命名は OCaml 実装の `Core_state.packrat_*` に合わせる。
- **span_trace 設計**  
  - `SpanTrace` は `VecDeque<TraceFrame>` を `RwLock` で包んだ構造体。`TraceFrame` は `{ label: Option<SmolStr>, span: Span }`。`Span` は `start: Position` / `end: Position` を保持し、`Position` は `line`・`column`・`offset` を `u32` で格納する。  
  - `StreamingState::push_span_trace(label, span)` は `trace_enabled` が `false` の場合は早期に return し、挿入後に `RunConfig.trace_limit` を超えたら `pop_front`。操作結果に応じて `parser.stream.span_trace_retained` / `parser.stream.span_trace_dropped` を更新する。  
  - `StreamingState::drain_span_trace()` は診断生成フローでのみ使用し、`VecDeque` を `SmallVec<[TraceFrame; 16]>` へ複製して `Diagnostic::with_span_trace` に渡す。複数診断で共有する場合は `Arc<[TraceFrame]>` を利用してコピーを抑制する。
- **Core_parse_streaming との対応**  
  - `Core_parse_streaming.expectation_summary_for_checkpoint` 相当のロジックは `lookup_packrat`→`store_packrat`→`collect_warm_cache` の 1 サイクルでヒット状況を測定する。結果は `ParserMetrics` に格納し、`parser.stream.packrat_hits` と `parser.stream.packrat_queries` を更新してから `SessionShared` へ返す。  
  - `span_trace_pairs` は `StreamingState::drain_span_trace` を呼び出し、`(Option<String>, Span)` の配列として受け取った `TraceFrame` を `Diagnostic` の `extensions["parse"]["span_trace"]` に JSON で埋め込む。  
  - `StreamingState::packrat_snapshot()` は `PackratSnapshot { entries, approx_bytes }` を返し、`remlc --frontend rust --emit parse-debug` が OCaml 版と同形式の統計を出力できるようにする。
- **負荷制御と落とし穴対策**  
  - キャッシュサイズは `RunConfig.packrat_budget_bytes`（デフォルト 4 MiB）を超えた段階で `prune_before` を強制し、溢れた際は `parser.stream.packrat_budget_drops` をインクリメントする。  
  - `span_trace` についても `RunConfig.trace_limit` のハード上限を守り、上限到達時は `Diagnostic` に `trace_truncated: true` を付加する。Packrat と同様にトレース破棄が発生した場合は `SessionShared` で警告ログを残し、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` にフォローアップを記録する。

## P1 W1: Packrat / span_trace 呼び出し統合（2025-12-06）
- `compiler/rust/frontend/src/parser/mod.rs` に `record_streaming_error` を追加し、`Simple<TokenKind>` から得た期待トークン情報を `StreamingState::store_packrat` / `push_span_trace` へ連携。`ParserDriver::parse` は処理終了時に `metrics_snapshot` と `drain_span_trace` を取り出し、`ParsedModule.packrat_stats` / `span_trace` として CLI へ返す。
- CLI PoC (`compiler/rust/frontend/src/bin/poc_frontend.rs`) の JSON に `parse_result.packrat_stats`・`parse_result.span_trace`・`stream_meta` を追加し、`tooling/ci/collect-iterator-audit-metrics.py` が Rust 出力の Packrat 統計を検出できることを確認した。空入力では `queries=0` のままだが、エラー時に `span_trace` が出力される。
- `poc_frontend --emit-parse-debug <path> <input.reml>` を追加し、OCaml 側 `remlc --emit parse-debug` と同形状の `run_config` / `parse_result` / `stream_meta` JSON を生成する。CI や `scripts/poc_dualwrite_compare.sh` からこのファイルを `collect-iterator-audit-metrics.py` に渡すことで Rust 版の Packrat/SpanTrace を公式ツールチェーンへ配布できる。
- `scripts/poc_dualwrite_compare.sh` が OCaml/Rust の `packrat_stats` を比較できるようサマリ JSON/Markdown に `packrat_queries`/`packrat_hits` を追加。`reports/dual-write/front-end/poc/2025-11-28-logos-chumsky/*.summary.json` へ保存されるため、Packrat 実装差分を含むレビューが可能になった。
- フォローアップ: 現状 Rust パーサはキャッシュ問い合わせ (`lookup_packrat`) を行っていないためヒット数は 0 のまま。W2 以降で `ParserDriver` のバックトラック／回復経路と統合し、OCaml 実装と同等のクエリ回数を収集できるようにする。評価結果は `docs/plans/rust-migration/1-1-ast-and-ir-alignment.md` の Packrat チェックリストへ反映する。

### PoC マイルストーン（W1→W2 移行条件）
1. `parser::driver::tests::basic_roundtrip` で `module Main {}` 程度の AST を生成し、`dual-write` 比較スクリプト（OCaml 側スナップショットに倣った JSON）で差分ゼロを確認する。
2. `StreamingState` の `packrat_cache` をモックデータで検証し、`parser.stream.packrat_entries` と `parser.stream.packrat_hits` を増減させるユニットテストを `tests/streaming_metrics.rs` に追加する。
3. `docs/plans/rust-migration/1-3-dual-write-runbook.md` に記載された CLI フック案と互換になるよう、`remlc --frontend rust --emit parse-debug` フラグの要求仕様を `runconfig` チームへ共有する（W2 冒頭でレビュー予定）。

## Phase 2-5 RunConfig 移行サマリ（2025-11-24）
- Step1: `parser_run_config.{ml,mli}` を導入し、仕様と同じフィールド／拡張 API を実装（`compiler/ocaml/src/parser_run_config.ml`）。
- Step2: `parser_driver` と `Parser_diag_state` を `RunConfig` 受け取りへ移行し、CLI・テストが新 API を利用する準備を完了。
- Step3: CLI (`compiler/ocaml/src/main.ml`) とユニットテストで共通ビルダーを使用、監査メタデータ出力を `parser.runconfig.*` 系に統一。
- Step4: LSP の `run_config_loader` を整備し、`extensions["lex"|"recover"|"stream"]` を `run_stream` へ伝播。設定ファイル（`tooling/lsp/config/default.json`）を共有。
- Step5: `run_config_tests.ml` と `parser-runconfig-packrat.json.golden` を追加し、`collect-iterator-audit-metrics.py` で `parser.runconfig_switch_coverage` / `parser.runconfig_extension_pass_rate` を集計。
- Step6: 仕様脚注（`docs/spec/2-1-parser-type.md`、`docs/spec/2-6-execution-strategy.md`）とガイド（`docs/guides/core-parse-streaming.md`）へ RunConfig 共有手順を反映し、レビュー記録（`docs/plans/bootstrap-roadmap/2-5-review-log.md`）へ Day6 エントリを追加。

## Phase 2-5 Core コンビネーター棚卸し（2025-11-01）
- Step1: Menhir 規則と仕様コアコンビネーターの対応を整理し、欠落メタデータを洗い出した。

## Phase 2-5 Core コンビネーター Step2（2025-12-04）
- `Core_parse` 公開シグネチャ案と `ParserId` 割当戦略を `docs/notes/core-parse-api-evolution.md` にまとめ、仕様 2.1/2.2 の契約を満たすラッパーモジュール構成を定義した[^core-parse-api-note]。  
- 静的 ID は `core_parse_id_registry.ml`（自動生成予定）で `ordinal = 0-4095` を採番し、`Digestif.xxhash64` で算出した `fingerprint` を保存する。動的 ID は `ordinal >= 0x1000` を割り当て `origin = \`Dynamic` として監査ログに残す。  
- `State` ラッパーで `RunConfig`・`Parser_diag_state`・Menhir チェックポイントを共有し、`cut`/`recover` が `committed` フラグ更新と同期トークン参照を行えるようにした。Step3 で `parser_driver` ブリッジを差し替える際の依存関係と TODO を整理。

## Phase 2-5 Core コンビネーター Step4（2025-12-12）
- Packrat キャッシュ導入に向けて `Cache_key = (Id.fingerprint, byte_offset)` の設計を確定し、`Core_parse.State` へ `Packrat_cache` を保持する案を `docs/notes/core-parse-api-evolution.md` に追記。PoC 時点でキャッシュが存在しないことを `compiler/ocaml/src/core_parse.ml:5` で確認し、Step5 で `find`/`store`/`invalidate` を実装する TODO を設定した。  
- `RunConfig.packrat` と `Extensions` 名称空間の連携を整理し、`parser_run_config.ml:25` のブール値でキャッシュを切り替えるフローを決定。左再帰や trace と同じレイヤーで扱うことで CLI/LSP 関連フラグとの衝突が無いことを確認した。  
- `Recover` 設定の同期トークンを `Core_parse.recover` へ渡す経路を明文化し、`parser_diag_state.ml:8` の `record_recovery` 呼び出しと `parser_expectation.collect` の再利用条件を検証。`recover.notes` の CLI/LSP 表示は Phase 2-7 で実装する旨を TODO として残した。  
- 複数 Capability を保持するために `RunConfig.Effects`（compiler/ocaml/src/parser_run_config.ml:320）と `Diagnostic` 拡張（compiler/ocaml/src/diagnostic.ml:846-896）を突合し、Packrat キャッシュヒット時でも Stage/Capability 情報を欠落させないよう `Reply` へメタデータを埋め込む案を整理した。  
- フォローアップ: `tooling/ci/collect-iterator-audit-metrics.py` へ Packrat/回復の KPI を追加し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に `parser.packrat_cache_hit_ratio` / `parser.recover_sync_success_rate` を仮登録。Step5 で実装と検証を行う。

[^frontend-eval]: `docs/plans/rust-migration/appendix/frontend-crate-evaluation.md` の評価結果を参照。
### コアコンビネーター現況
- **`ok`**  
  - 仕様: 非消費で成功し値を返す（`docs/spec/2-2-core-combinator.md:15`）。  
  - Menhir現状: 空産出を使って `None` や `[]` を返す規則が多用される（例: `module_header_opt`、`use_decl_list`、`arg_list_opt`。`compiler/ocaml/src/parser.mly:727`, `compiler/ocaml/src/parser.mly:735`, `compiler/ocaml/src/parser.mly:1255`）。  
  - ギャップ: 成功時に `ParserId` やトレースを残す仕組みがなく、`ok` の適用箇所を走行時に識別できない。
- **`fail`**  
  - 仕様: 非消費で即時失敗し、期待集合は空（`docs/spec/2-2-core-combinator.md:17`）。  
  - Menhir現状: `Parser.MenhirInterpreter` の `HandlingError`／`Rejected` 分岐で一律に診断を生成しており、任意メッセージ付きの失敗を仕込む API が存在しない（`compiler/ocaml/src/parser_driver.ml:206-237`）。  
  - ギャップ: `fail` 相当を呼び出しても診断ドメインや期待集合を細かく制御できず、仕様の `fail(msg)` をそのまま表現できない。
- **`eof`**  
  - 仕様: 入力末尾のみ成功するゼロ幅パーサ（`docs/spec/2-2-core-combinator.md:18`）。  
  - Menhir現状: `compilation_unit` 規則で `EOF` トークンを要求し（`compiler/ocaml/src/parser.mly:719-724`）、解析後に `RunConfig.require_eof` を別途チェックしている（`compiler/ocaml/src/parser_driver.ml:241-258`）。  
  - ギャップ: `eof` の判定とエラーメッセージ生成が分散しており、コンビネーター層に集約するためのフックがない。
- **`rule`**  
  - 仕様: 規則に名前と `ParserId` を付与して Packrat と診断で利用（`docs/spec/2-2-core-combinator.md:19`）。  
  - Menhir現状: 非終端名は存在するが安定 ID として公開されず、`record_span_trace` も `compilation_unit` 固定のラベルしか持たない（`compiler/ocaml/src/parser.mly:1174`、`compiler/ocaml/src/parser_driver.ml:219-223`）。  
  - ギャップ: `rule` が要求する ID 付与・トレース・Packrat キー生成のいずれも未実装。
- **`label`**  
  - 仕様: 失敗時に期待名を差し替える（`docs/spec/2-2-core-combinator.md:20`）。  
  - Menhir現状: 期待集合はトークン／非終端名から自動生成され、`label` に相当する上書きメカニズムがない（`compiler/ocaml/src/parser_expectation.ml:139-175`）。  
  - ギャップ: ヒューマンリーダブルな期待名を与える手段がなく、仕様の `label`/`expect` 系 API を再現できない。
- **`then`**  
  - 仕様: 2 つのパーサを直列結合しタプルで返す（`docs/spec/2-2-core-combinator.md:29`）。  
  - Menhir現状: 逐次的な規則展開で同様の効果を得ている（例: `compilation_unit`。`compiler/ocaml/src/parser.mly:719-724`）。  
  - ギャップ: 直列結合の結果を明示的な `Reply` に変換する層がなく、`then` が要求する `consumed/committed` の規約を露出できていない。
- **`andThen`**  
  - 仕様: 成功結果に基づき次のパーサを決めるモナディック結合（`docs/spec/2-2-core-combinator.md:30`）。  
  - Menhir現状: 後続規則は静的に決まり、動的にパーサを差し替える手段がない。必要に応じて新しい非終端を作るしかなく、`andThen` の抽象性を欠いている（`compiler/ocaml/src/parser.mly:1174-1191`）。  
  - ギャップ: 実行時にパーサを合成する API が無いため、`andThen` 導入にはシム層でのラッパー実装が必須。
- **`skipL`**  
  - 仕様: 左側の結果を捨て右側を返す（`docs/spec/2-2-core-combinator.md:31`）。  
  - Menhir現状: アクションで先行トークンを無視する形で実現（例: `LPAREN; e = expr; RPAREN { e }`。`compiler/ocaml/src/parser.mly:1204-1211`）。  
  - ギャップ: 捨てたトークンを診断やトレースに残す仕組みが無いため、`skipL` を導入してもメタデータが欠落する。
- **`skipR`**  
  - 仕様: 右側の結果を捨て左側を返す（`docs/spec/2-2-core-combinator.md:32`）。  
  - Menhir現状: 右項を無視する場合は自前で値を返しているが、どこでも同じパターンを再利用できる抽象がない（例: `module_header_opt` で `MODULE` を破棄し `Some` を返す。`compiler/ocaml/src/parser.mly:727-732`）。  
  - ギャップ: `skipR` の呼び出し位置を追跡するメタデータや共通実装が存在せず、仕様の糖衣と差がある。
- **`or`**  
  - 仕様: 左優先の選択で消費済みなら分岐打ち切り（`docs/spec/2-2-core-combinator.md:33-35`）。  
  - Menhir現状: 代替規則として記述され、LR 解析の特性上自動的に左優先になる（`compiler/ocaml/src/parser.mly:1174-1191`）。  
  - ギャップ: `consumed`/`committed` フラグを露出する API が無いため、`or` の失敗統合ルールをコンビネーター層で評価できない。
- **`choice`**  
  - 仕様: `or` をリストに拡張した総合集合（`docs/spec/2-2-core-combinator.md:33-35`）。  
  - Menhir現状: 多分岐規則で近似されるが、分岐集合をデータとして扱う箇所は存在しない（`compiler/ocaml/src/parser.mly:1266-1320`）。  
  - ギャップ: 分岐ごとのメタデータや `choice` のソース識別を保持する仕組みがなく、Packrat/診断に必要な識別子を提供できない。
- **`map`**  
  - 仕様: 解析結果を純粋関数で変換（`docs/spec/2-2-core-combinator.md:45`）。  
  - Menhir現状: 各規則で `make_expr` などのアクションを実行しており実質的に `map` と同等（`compiler/ocaml/src/parser.mly:1194-1231`）。  
  - ギャップ: `map` が要求する純粋性・効果タグ検証が実装されておらず、`map` 呼出しを特定できない。
- **`cut`**  
  - 仕様: 以降の失敗を `committed=true` に変換（`docs/spec/2-2-core-combinator.md:46`）。  
  - Menhir現状: `parser_driver` で `committed` フラグを管理しているが、どこでも `true` に更新されていない（`compiler/ocaml/src/parser_driver.ml:185-223`）。  
  - ギャップ: `cut` を差し込むフックが皆無で、`ParserId` と組み合わせた診断強化ができない。
- **`attempt`**  
  - 仕様: 失敗時に消費を巻き戻し空失敗化（`docs/spec/2-2-core-combinator.md:48`）。  
  - Menhir現状: LR 解析のためバックトラックが無く、`attempt` 相当の挙動を制御する手段が提供されていない。  
  - ギャップ: `attempt` を実装するにはシム層でメモ化と `consumed` フラグの書き換えを導入する必要がある。
- **`recover`**  
  - 仕様: 指定トークンまで読み飛ばし診断を残して継続（`docs/spec/2-2-core-combinator.md:49-50`）。  
  - Menhir現状: `Parser_diag_state` に `recover_config` と `record_recovery` が用意されているが、呼び出し元が存在しない（`compiler/ocaml/src/parser_diag_state.ml:24-63`、`compiler/ocaml/src/parser_driver.ml:187-205`）。  
  - ギャップ: 同期トークンの指定や診断拡張を行うエントリポイントが欠如しており、RunConfig の `recover` 拡張も実装未着手。

### 欠落メタデータとリスク
- `ParserId`／`rule` 名称が生成されないため Packrat・トレース・監査が不可。  
- `committed` フラグが常に `false` のため、`cut` や `attempt` の契約を検証できない。  
- `recover` シナリオを記録する仕組みが未着手で、`parser.recover` Capability を名乗れない。  
- `label` や `choice` に紐付く期待名が静的文字列のみで、仕様要求の人間可読メッセージを提供できていない。

[^core-parse-api-note]: `docs/notes/core-parse-api-evolution.md` Phase 2-5 Step2 Core_parse シグネチャ草案。`Id`/`State`/`Reply` 構成と採番戦略を記載。


## TODO: RunConfig フラグの未実装点
- [ ] `--packrat` / `RunConfig.packrat` は警告のみで実装待ち。`PARSER-003` のメモ化シム導入後に CLI/LSP 両方で挙動を検証する。
- [ ] `--left-recursion=<mode>` の `on/auto` はシムが未着手のため警告を伴う。左再帰テーブル構築手順を `docs/plans/bootstrap-roadmap/2-5-proposals/PARSER-003-proposal.md` と同期する。
- [ ] LSP 側の設定ファイル読み込み処理はドラフト段階。`tooling/lsp/config/default.json` を更新した場合は CLI 側の `extensions["config"]` と突合し、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` にフォローアップを残す。

## 参考リンク
- `docs/plans/bootstrap-roadmap/2-5-proposals/PARSER-002-proposal.md`
- `docs/plans/bootstrap-roadmap/2-5-proposals/LEXER-002-proposal.md`
- `docs/plans/bootstrap-roadmap/2-5-review-log.md`
