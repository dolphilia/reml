# 調査メモ: 第16章 解析・DSL・診断

## 対象モジュール

- `compiler/runtime/src/parse/mod.rs`
- `compiler/runtime/src/parse/combinator.rs`
- `compiler/runtime/src/parse/cst.rs`
- `compiler/runtime/src/parse/embedded.rs`
- `compiler/runtime/src/parse/meta.rs`
- `compiler/runtime/src/parse/op_builder.rs`
- `compiler/runtime/src/dsl/mod.rs`
- `compiler/runtime/src/dsl/object.rs`
- `compiler/runtime/src/dsl/gc.rs`
- `compiler/runtime/src/dsl/actor.rs`
- `compiler/runtime/src/dsl/vm.rs`
- `compiler/runtime/src/diagnostics/mod.rs`
- `compiler/runtime/src/diagnostics/metric_point.rs`
- `compiler/runtime/src/diagnostics/audit_bridge.rs`
- `compiler/runtime/src/diagnostics/dsl.rs`
- `compiler/runtime/src/diagnostics/stage_guard.rs`
- `compiler/runtime/src/config/mod.rs`
- `compiler/runtime/src/config/manifest.rs`
- `compiler/runtime/src/config/compat.rs`
- `compiler/runtime/src/config/collection_diff.rs`
- `compiler/runtime/src/data/mod.rs`
- `compiler/runtime/src/data/change_set.rs`
- `compiler/runtime/src/data/schema.rs`

## 入口と全体像

- `parse` は DSL 向けのパーサーコンビネータと OpBuilder DSL を実行時に構築するための基盤を提供する。公開 API は `parse/mod.rs` で一括再エクスポートされる。
  - `compiler/runtime/src/parse/mod.rs:1-30`
- `dsl` は最小の DSL ランタイム API を揃える。監査フックの登録と event 名定義を含む。
  - `compiler/runtime/src/dsl/mod.rs:1-118`
- `diagnostics` はメトリクス送出の最小実装で、Capability/Stage 検証を通して監査メタデータを生成する。
  - `compiler/runtime/src/diagnostics/mod.rs:1-15`
- `config` は `reml.toml` と互換モード、および ChangeSet/SchemaDiff を含むデータ差分ユーティリティを束ねる。
  - `compiler/runtime/src/config/mod.rs:1-156`

## データ構造

### Parse

- `Input` と `InputPosition` は UTF-8 境界を保証しつつ位置情報を進める入力ビュー。
  - `compiler/runtime/src/parse/combinator.rs:120-247`
- `Span` は `InputPosition` の範囲を表す。
  - `compiler/runtime/src/parse/combinator.rs:250-259`
- `Parser` / `Reply` / `ParseResult` がランナーの戻り値と診断情報を管理するコア構造。
  - `compiler/runtime/src/parse/combinator.rs:1387-1518`
- `ParseError` は期待トークンや復旧メタデータ、FixIt を保持し、`GuardDiagnostic` へ変換できる。
  - `compiler/runtime/src/parse/combinator.rs:1171-1313`
- `ParserProfile` は packrat ヒット数や回復回数などのメトリクスを蓄積する。
  - `compiler/runtime/src/parse/combinator.rs:399-475`
- `CstNode` と `CstBuilder` がトリビアを保持した CST を組み立てる。
  - `compiler/runtime/src/parse/cst.rs:5-126`
- `EmbeddedDslSpec` / `EmbeddedBoundary` / `EmbeddedNode` が埋め込み DSL の境界と結果を保持する。
  - `compiler/runtime/src/parse/embedded.rs:9-79`
- `ParseMetaRegistry` は `ParserId` とルールメタデータを関連付ける。
  - `compiler/runtime/src/parse/meta.rs:5-78`
- `OpBuilder` は `FixitySymbol` を用いて優先順位テーブル `OpTable` を構築する。
  - `compiler/runtime/src/parse/op_builder.rs:6-104`

### DSL

- `AuditPayload` と `DslAuditHook` が DSL 監査イベントの共通フォーマットを提供する。
  - `compiler/runtime/src/dsl/mod.rs:56-99`
- `DispatchTable` / `ObjectHandle` / `MethodCache` がメソッドディスパッチの最小構成。
  - `compiler/runtime/src/dsl/object.rs:12-102`
- `GcHeap` / `GcRef` / `RootScope` が GC の状態と参照を表す。
  - `compiler/runtime/src/dsl/gc.rs:16-99`
- `ActorDefinition` / `MailboxBridge` / `SupervisionBridge` がアクター実行の足場を表す。
  - `compiler/runtime/src/dsl/actor.rs:11-108`
- `Bytecode` / `VmState` / `CallFrame` が VM の最小表現。
  - `compiler/runtime/src/dsl/vm.rs:10-55`

### Diagnostics

- `MetricPoint` と `MetricValue` が `Core.Diagnostics` のメトリクス表現を担う。
  - `compiler/runtime/src/diagnostics/metric_point.rs:23-178`
- `MetricsStageGuard` は `metrics.emit` Capability の Stage 要件を検証する。
  - `compiler/runtime/src/diagnostics/stage_guard.rs:6-61`
- `apply_dsl_metadata` は DSL 埋め込み情報を `GuardDiagnostic` に付与する。
  - `compiler/runtime/src/diagnostics/dsl.rs:1-53`

### Config/Data

- `Manifest` と `ConfigRoot` が `reml.toml` のトップレベル構造を表す。
  - `compiler/runtime/src/config/manifest.rs:40-182`
- `RunCapabilityEntry` が実行時 Capability 要件を `StageRequirement` に変換する。
  - `compiler/runtime/src/config/manifest.rs:185-223`
- `ConfigCompatibility` と `CompatibilityProfile` が互換モードの定義を持つ。
  - `compiler/runtime/src/config/compat.rs:21-179`
- `SchemaDiff` / `ConfigChange` は ChangeSet を Config/Data の差分表現へ変換する。
  - `compiler/runtime/src/config/collection_diff.rs:14-249`
- `ChangeEntry` / `ChangeSet` / `Schema` が Config/Data の最小データモデル。
  - `compiler/runtime/src/data/change_set.rs:4-141`
  - `compiler/runtime/src/data/schema.rs:7-259`

## コアロジック

### Parse

- `Parser::parse` は packrat メモ化と左再帰ガードを内蔵し、`ParseState` に memo を保存する。
  - `compiler/runtime/src/parse/combinator.rs:1492-1517`
- `ParseError::to_guard_diagnostic` は expected tokens / recover / fixits を extensions と audit_metadata に反映する。
  - `compiler/runtime/src/parse/combinator.rs:1225-1311`
- `run_with_recovery_config` が `RunConfig.extensions["recover"]` を書き換え、sync token を補う。
  - `compiler/runtime/src/parse/combinator.rs:4365-4397`
- `run_with_state_cst` は CST 収集モード時の ParseResult を構築する。
  - `compiler/runtime/src/parse/combinator.rs:4316-4354`
- `OpBuilder::build` が優先度順に `OpTable` を生成する。
  - `compiler/runtime/src/parse/op_builder.rs:86-103`

### DSL

- `Object::call` は監査イベントを送出し、メソッドキャッシュのヒット時は再検索を回避する。
  - `compiler/runtime/src/dsl/object.rs:133-172`
- `Gc::alloc` / `Gc::pin` / `Gc::collect` は GC 操作ごとに監査イベントを発火する。
  - `compiler/runtime/src/dsl/gc.rs:123-192`
- `VmCore::step` は `VmTraceEvent` と監査イベントを生成し、`catch_unwind` で panic を吸収する。
  - `compiler/runtime/src/dsl/vm.rs:90-125`
- `Actor::spawn` は `ActorSystem` を通じて Mailbox を生成する薄いラッパ。
  - `compiler/runtime/src/dsl/actor.rs:114-121`

### Diagnostics

- `emit_metric` は `MetricsStageGuard` を通じて Capability を検証し、監査メタデータ付きでシンクへ送出する。
  - `compiler/runtime/src/diagnostics/metric_point.rs:194-212`
- `attach_audit` は `metric_point.*` と `effect.*` を `AuditEnvelope.metadata` へ展開する。
  - `compiler/runtime/src/diagnostics/audit_bridge.rs:16-80`

### Config/Data

- `merge_maps_with_audit` が `PersistentMap::merge_with_change_set` を包み、Config/Data 用に結果を整形する。
  - `compiler/runtime/src/config/mod.rs:70-84`
- `set_collections_change_set_env` は ChangeSet を一時ファイル化し、環境変数を設定する。
  - `compiler/runtime/src/config/mod.rs:114-155`
- `load_manifest` が TOML 解析と DSL エントリの存在検証を行う。
  - `compiler/runtime/src/config/manifest.rs:1016-1025`
- `validate_manifest` が project/build/dsl の妥当性を検証する。
  - `compiler/runtime/src/config/manifest.rs:1028-1035`
- `resolve_compat` は CLI > Env > Manifest > Default の優先順位で互換モードを確定する。
  - `compiler/runtime/src/config/compat.rs:309-338`

## エラー処理

- `ParseError` は `GuardDiagnostic` へ変換され、DSL 由来の場合は `apply_dsl_metadata` で埋め込み情報が付与される。
  - `compiler/runtime/src/parse/combinator.rs:1286-1311`
- `DslError` / `DispatchError` / `GcError` / `ActorError` / `VmError` は `IntoDiagnostic` を実装し、`dsl.*` 系コードを付与する。
  - `compiler/runtime/src/dsl/mod.rs:102-118`
  - `compiler/runtime/src/dsl/object.rs:200-210`
  - `compiler/runtime/src/dsl/gc.rs:224-239`
  - `compiler/runtime/src/dsl/actor.rs:124-139`
  - `compiler/runtime/src/dsl/vm.rs:161-178`
- `compatibility_violation_diagnostic` が互換違反を GuardDiagnostic にまとめる。
  - `compiler/runtime/src/config/compat.rs:394-420`

## 仕様との対応メモ

- DSL ランタイムの API は `docs/spec/3-16-core-dsl-paradigm-kits.md` と対応。
- 診断の監査メタデータは `docs/spec/3-6-core-diagnostics-audit.md` と対応。
- Config/Data のマニフェストと互換モードは `docs/spec/3-7-core-config-data.md` と対応。

## TODO / 不明点

- `apply_dsl_metadata` は `dsl.embedding.mode` など spec で要求されるキーを付与していないため差分整理が必要。
  - `compiler/runtime/src/diagnostics/dsl.rs:20-53`
- `Core.Dsl` の監査イベントは `dsl.id` / `dsl.node` などの必須メタデータが未付与で、仕様のキーセットとのギャップがある。
  - `compiler/runtime/src/dsl/mod.rs:56-82`
  - `compiler/runtime/src/dsl/object.rs:133-172`
- `parse` の CST 生成と DSL 埋め込みの連携は `EmbeddedNode` の `cst` に留まり、上位構造との統合箇所は未確認。
  - `compiler/runtime/src/parse/embedded.rs:72-79`
