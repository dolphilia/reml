# Phase4: DSL パラダイムキット計画（Core.Dsl.*）

## 背景と決定事項
- `docs/notes/dsl/dsl-paradigm-support-research.md` の「2. 提案するパラダイムキット」を Phase 4 の計画へ落とし込む。
- DSL 作者が意味論実装（ディスパッチ、GC、アクター、VM）に注力しすぎる課題を解消し、Reml の DSL ファースト方針と整合させる。
- `docs/spec/0-1-project-purpose.md` の安全性・性能・段階的習得の原則を優先し、キットは最小構成から段階的に拡張する。

## 目的
1. `Core.Dsl.Object`/`Core.Dsl.Gc`/`Core.Dsl.Actor`/`Core.Dsl.Vm` の仕様ドラフトを整備する。
2. Rust ランタイムに最小バックエンド実装を追加し、Reml 実装に実際の手を入れる工程を確立する。
3. リファレンス DSL で動作検証し、Phase 4 の回帰計画へ接続する。

## スコープ
- **含む**: 仕様追加、Rust ランタイム実装、参照 DSL の設計・サンプル、監査ログの整理。
- **含まない**: フル LSP 対応、外部プラグイン配布、JIT 実装（Phase 5 以降）。

## 成果物
- 仕様書: `docs/spec/3-16-core-dsl-paradigm-kits.md` の新設。
- 既存概要の更新: `docs/spec/3-0-core-library-overview.md` と `README.md` の章構成更新。
- ガイド: `docs/guides/dsl-paradigm-kits.md` の追加（導入とユースケース）。
- Rust 実装: `compiler/runtime/src/dsl/` 配下の最小実装と API 公開。
- 参照 DSL: `examples/` に Mini-Ruby / Mini-Erlang / Mini-VM を配置。

## 仕様ドラフト（最小構成の案）

### Core.Dsl.Object
- `DispatchTable` / `ObjectHandle` / `MethodCache` を中心に、クラス/プロトタイプの両モデルを定義。
- `MethodCache` はモノモーフィックの最小キャッシュから開始し、拡張は Phase 5 へ委譲。

### Core.Dsl.Gc
- `GcHeap`（`Arena`/`RefCount`）の最小実装と `GcRef<T>`/`RootScope` の API を定義。
- `MarkAndSweep` は仕様のみ定義し、実装は後続フェーズへ延期可能とする。

### Core.Dsl.Actor
- `Core.Async` の `ActorSystem` と `Mailbox` を DSL へ公開するブリッジ仕様を追加。
- `ActorDefinition`/`MailboxBridge`/`SupervisionBridge` を最小セットで提供。

### Core.Dsl.Vm
- `BytecodeBuilder`/`VMCore`/`OperandStack`/`CallFrame` を定義。
- 命令セットは利用側 DSL が持つ前提で、ビルダーと実行ループの責務を分離。

## 作業ステップ

### フェーズA: 仕様ドラフトと概要更新
1. [x] `docs/spec/3-16-core-dsl-paradigm-kits.md` を新規作成し、4 キットの最小 API を整理する。
2. [x] `docs/spec/3-0-core-library-overview.md` にパラダイムキットの概要と到達目標を追記する。
3. [x] `README.md` と `docs/spec/README.md` の章構成へ新規仕様を登録する。

### フェーズB: Rust ランタイム設計
1. [x] `compiler/runtime/src/dsl/` を新設し、`mod.rs` と `object.rs`/`gc.rs`/`actor.rs`/`vm.rs` の雛形を追加する。
2. [x] `mod.rs` に公開 API の最小集合（`DispatchTable`/`GcHeap`/`ActorDefinition`/`BytecodeBuilder`）と共通型（`Result`/`Error`/`AuditPayload` など）を定義する。
3. [x] `compiler/runtime/src/lib.rs` で `Core.Dsl` 名前空間を公開し、`Core.Dsl.*` のモジュール境界を明示する。
4. [x] `Core.Async` 連携のため、`compiler/runtime/src/runtime/` か `compiler/runtime/src/ffi/` に `MailboxBridge`/`SupervisionBridge` の接続点を設計し、依存関係（`Core.Async`/`Core.Diagnostics`）を整理する。
5. [x] 監査イベントの最小スキーマ（`dsl.object.dispatch`/`dsl.gc.root`/`dsl.actor.mailbox`/`dsl.vm.execute` など）を設計し、後続の `docs/spec/3-6-core-diagnostics-audit.md` 追記に接続する。

### フェーズC: Rust 実装（最小バックエンド）
1. [x] `Core.Dsl.Object` の `DispatchTable`/`ObjectHandle` を実装し、`MethodCache` の単純キャッシュ（モノモーフィック）を組み込む。
2. [x] `ObjectHandle` の所有権・参照モデル（`GcRef`/`RootScope` と接続）を明文化し、ディスパッチ時のライフタイム規約を揃える。
3. [x] `Core.Dsl.Gc` の `Arena`/`RefCount` を実装し、`RootScope` の開始・終了でルート登録を管理できるようにする。
4. [x] `Core.Dsl.Gc` の監査フック（割り当て/解放/ルート登録）を最小で追加し、診断ログへ出力できるようにする。
5. [x] `Core.Dsl.Actor` の `MailboxBridge` を `Core.Async` の既存 API へ接続し、`ActorDefinition` から `Mailbox` を起動できる導線を作る。
6. [x] `Core.Dsl.Vm` の `BytecodeBuilder` と最小 `VMCore`（Fetch-Decode-Execute）を実装し、命令ディスパッチのトレース出力を用意する。
7. [x] 例外・失敗時の `Result`/`Error` を `Core.Diagnostics` と統合し、パニック回避の最小ルールを決める。

### フェーズD: 参照 DSL と回帰
1. [x] `examples/` に Mini-Ruby / Mini-Erlang / Mini-VM の最小実装を追加し、各 DSL で利用する `Core.Dsl.*` の範囲を明記する（`examples/dsl_paradigm/`）。
2. [x] Mini-Ruby は `Object`/`Gc`、Mini-Erlang は `Actor`/`Gc`、Mini-VM は `Vm`/`Object` の最小ルートを示し、依存関係を一覧化する（`examples/dsl_paradigm/README.md`）。
3. [x] `expected/` に参照 DSL の出力スナップショットを追加し、`examples/` との対応表（入力/期待値/監査ログの有無）を整理する（`expected/dsl_paradigm/README.md`）。
4. [x] `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` に検証シナリオを登録し、`Core.Dsl.*` の監査イベント有無・Stage 条件を列に追加する。
5. [x] `docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md` と整合する形で、回帰観点（性能・安全性・監査ログ）を参照 DSL へ紐付ける。

### フェーズE: 監査とリスク整理
1. [ ] `docs/spec/3-6-core-diagnostics-audit.md` に `Core.Dsl.*` の監査イベント（`dsl.object.dispatch`/`dsl.gc.root`/`dsl.actor.mailbox`/`dsl.vm.execute`）を追記する。
2. [ ] `docs/spec/3-8-core-runtime-capability.md` と Stage 整合を確認し、監査イベントの必須フィールド（Stage/Bridge/Effect）を揃える。
3. [ ] `docs/notes/` に課題メモ（GC 性能・ブリッジ安全性・VM 拡張）を残し、TODO と参照リンクを付与する。
4. [ ] 監査ログの最低限の運用ルール（ログ粒度、個人情報の扱い、パフォーマンス影響）を整理し、ガイドまたはノートへ整理する。

## Rust 実装の想定モジュール構成
- `compiler/runtime/src/dsl/mod.rs`: 公開 API と共通型。
- `compiler/runtime/src/dsl/object.rs`: `DispatchTable` / `ObjectHandle` / `MethodCache`。
- `compiler/runtime/src/dsl/gc.rs`: `GcHeap` / `GcRef` / `RootScope`。
- `compiler/runtime/src/dsl/actor.rs`: `ActorDefinition` / `MailboxBridge` / `SupervisionBridge`。
- `compiler/runtime/src/dsl/vm.rs`: `BytecodeBuilder` / `VMCore` / `OperandStack`。

## Rust API の最小イメージ
```reml
use Core.Dsl.Object
use Core.Dsl.Gc

let class = Object.ClassBuilder.new("Animal")
  .method("speak", |this, args| { ... })
  .build()

Gc.with_scope(|scope| {
  let dog = class.instantiate(scope)
  dog.call("speak", [])
})
```

## 依存関係
- `docs/spec/3-9-core-async-ffi-unsafe.md` の ActorSystem 仕様。
- `docs/spec/3-8-core-runtime-capability.md` の Capability Stage と監査。
- `docs/plans/bootstrap-roadmap/4-1-ffi-improvement-implementation-plan.md` の FFI 安全化方針。
- `docs/plans/bootstrap-roadmap/4-1-core-plugin-implementation-status.md` のプラグイン連携状況。

## リスクと緩和策
| リスク | 影響 | 緩和策 |
| --- | --- | --- |
| GC API が複雑化する | 学習コスト増大 | `Arena`/`RefCount` の最小構成に限定し、`MarkAndSweep` は仕様のみ定義 |
| ディスパッチが遅い | DSL 実行性能低下 | `MethodCache` を単純なインラインキャッシュとして先行実装 |
| ブリッジが安全性を損なう | ランタイム障害 | `Core.Diagnostics` と監査イベントを標準化し、監査ログに必須項目を追加 |
| VM 抽象が過剰 | 実装の肥大化 | `VMCore` を最小実装に絞り、最適化や JIT を Phase 5 以降へ延期 |

## 参照
- `docs/notes/dsl/dsl-paradigm-support-research.md`
- `docs/spec/0-1-project-purpose.md`
- `docs/spec/3-0-core-library-overview.md`
- `docs/spec/3-6-core-diagnostics-audit.md`
- `docs/spec/3-8-core-runtime-capability.md`
- `docs/spec/3-9-core-async-ffi-unsafe.md`
- `docs/plans/bootstrap-roadmap/4-1-core-plugin-implementation-status.md`
- `docs/plans/bootstrap-roadmap/4-1-ffi-improvement-implementation-plan.md`
