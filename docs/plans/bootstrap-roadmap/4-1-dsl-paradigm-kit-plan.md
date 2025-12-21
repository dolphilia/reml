# Phase4: DSL パラダイムキット計画（Core.Dsl.*）

## 背景と決定事項
- `docs/notes/dsl-paradigm-support-research.md` の「2. 提案するパラダイムキット」を Phase 4 の計画へ落とし込む。
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
- Rust 実装: `compiler/rust/runtime/src/dsl/` 配下の最小実装と API 公開。
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
1. [ ] `docs/spec/3-16-core-dsl-paradigm-kits.md` を新規作成し、4 キットの最小 API を整理する。
2. [ ] `docs/spec/3-0-core-library-overview.md` にパラダイムキットの概要と到達目標を追記する。
3. [ ] `README.md` と `docs/spec/README.md` の章構成へ新規仕様を登録する。

### フェーズB: Rust ランタイム設計
1. [ ] `compiler/rust/runtime/src/dsl/` を新設し、`object`/`gc`/`actor`/`vm` のサブモジュールを作る。
2. [ ] `compiler/rust/runtime/src/lib.rs` から `Core.Dsl` 名前空間として公開する。
3. [ ] `Core.Async` 連携に必要な最小ブリッジ API を `compiler/rust/runtime/src/runtime/` または `compiler/rust/runtime/src/ffi/` に整理する。

### フェーズC: Rust 実装（最小バックエンド）
1. [ ] `Core.Dsl.Object` の `DispatchTable` と `ObjectHandle` を実装し、メソッドディスパッチの最短経路を提供する。
2. [ ] `Core.Dsl.Gc` の `Arena`/`RefCount` を実装し、`RootScope` でルート登録できるようにする。
3. [ ] `Core.Dsl.Actor` の `MailboxBridge` を `Core.Async` の既存 API へ接続する。
4. [ ] `Core.Dsl.Vm` の `BytecodeBuilder` と最小 `VMCore`（Fetch-Decode-Execute）を実装する。

### フェーズD: 参照 DSL と回帰
1. [ ] `examples/` に Mini-Ruby / Mini-Erlang / Mini-VM の最小実装を追加する。
2. [ ] `expected/` に参照 DSL の出力スナップショットを追加し、差分確認の基盤を整える。
3. [ ] `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` に検証シナリオを登録する。

### フェーズE: 監査とリスク整理
1. [ ] `docs/spec/3-6-core-diagnostics-audit.md` に `Core.Dsl.*` の監査イベント項目を追記する。
2. [ ] `docs/notes/` に課題メモ（GC 性能・ブリッジ安全性など）を残し、TODO を明記する。

## Rust 実装の想定モジュール構成
- `compiler/rust/runtime/src/dsl/mod.rs`: 公開 API と共通型。
- `compiler/rust/runtime/src/dsl/object.rs`: `DispatchTable` / `ObjectHandle` / `MethodCache`。
- `compiler/rust/runtime/src/dsl/gc.rs`: `GcHeap` / `GcRef` / `RootScope`。
- `compiler/rust/runtime/src/dsl/actor.rs`: `ActorDefinition` / `MailboxBridge` / `SupervisionBridge`。
- `compiler/rust/runtime/src/dsl/vm.rs`: `BytecodeBuilder` / `VMCore` / `OperandStack`。

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
- `docs/notes/dsl-paradigm-support-research.md`
- `docs/spec/0-1-project-purpose.md`
- `docs/spec/3-0-core-library-overview.md`
- `docs/spec/3-6-core-diagnostics-audit.md`
- `docs/spec/3-8-core-runtime-capability.md`
- `docs/spec/3-9-core-async-ffi-unsafe.md`
- `docs/plans/bootstrap-roadmap/4-1-core-plugin-implementation-status.md`
- `docs/plans/bootstrap-roadmap/4-1-ffi-improvement-implementation-plan.md`
