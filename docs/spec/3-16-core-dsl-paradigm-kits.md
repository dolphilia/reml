# 3.16 Core DSL パラダイムキット

> 目的：DSL 実装者がディスパッチ・GC・アクター・VM の最小基盤を再実装せずに済むよう、標準ライブラリとランタイムブリッジの最小 API を揃える。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | ドラフト（Phase 4 対象） |
| 効果タグ | `effect {runtime}`, `effect {memory}`, `effect {io.async}`, `effect {audit}` |
| 依存モジュール | `Core.Runtime`, `Core.Diagnostics`, `Core.Async` |
| 相互参照 | [3-0 Core Library Overview](3-0-core-library-overview.md), [3-6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md), [3-8 Core Runtime Capability](3-8-core-runtime-capability.md), [3-9 Core Async/FFI/Unsafe](3-9-core-async-ffi-unsafe.md), Notes: [dsl-paradigm-support-research](../notes/dsl-paradigm-support-research.md) |

## 1. 位置付け

`Core.Dsl.*` は DSL の意味論実装で再発しがちな「ディスパッチ・メモリ管理・アクター並行・VM 実行ループ」の最小 API を提供し、Reml の DSL ファースト方針を補強する。高度な最適化（多相インラインキャッシュ、世代別 GC、JIT）や外部プラグイン配布は Phase 5 以降に分離し、ここでは段階的導入と安全性を優先する。

## 2. 共通設計原則

- **段階的習得**: まず最小 API で DSL 実装が成立することを最優先とし、拡張は追加キットや拡張 API として切り出す。
- **Capability/Stage の整合**: `Core.Runtime` の検証を介して Stage 要件を統一し、`verify_capability_stage` に失敗した場合は `Core.Diagnostics` へ直結させる。
- **監査可能性**: 重要操作（GC 実行・アクター生成・VM 実行ループ）は監査イベントの発火点を明示し、`AuditEvent` とメタデータを統一する。
- **オブジェクトモデル非拘束**: クラスベース/プロトタイプ双方を支援し、DSL 作者が独自ルールを持てるよう `DispatchTable` と `MethodCache` を疎結合で提供する。

## 3. Core.Dsl.Object

### 3.1 主要型

```reml
pub type MethodId = Str

pub enum DispatchKind =
  | ClassBased
  | PrototypeBased

pub type DispatchTable<Value> = {
  kind: DispatchKind,
  name: Str,
  parent: Option<DispatchTable<Value>>,
  methods: Map<MethodId, MethodEntry<Value>>,
}

pub type MethodEntry<Value> =
  fn(ObjectHandle<Value>, List<Value>) -> Result<Value, DispatchError> // `effect {runtime}`

pub type ObjectHandle<Value> = {
  table: DispatchTable<Value>,
  payload: Value,
  shape_id: Int,
}

pub type MethodCacheKey = { shape_id: Int, name: MethodId }

pub type MethodCache<Value> = {
  lookup: fn(&MethodCache<Value>, MethodCacheKey) -> Option<MethodEntry<Value>>,
  record: fn(&mut MethodCache<Value>, MethodCacheKey, MethodEntry<Value>) -> (),
  invalidate: fn(&mut MethodCache<Value>, DispatchTable<Value>) -> (),
}

pub type DispatchError = { kind: DispatchErrorKind, message: Str }

pub enum DispatchErrorKind =
  | MethodNotFound
  | ArityMismatch
  | RuntimeFailure
```

### 3.2 最小 API

```reml
fn Object.call<Value>(
  obj: ObjectHandle<Value>,
  name: MethodId,
  args: List<Value>,
  cache: Option<&mut MethodCache<Value>>
) -> Result<Value, DispatchError> // `effect {runtime}`

fn Object.lookup<Value>(
  table: DispatchTable<Value>,
  name: MethodId
) -> Option<MethodEntry<Value>>

fn Object.class_builder<Value>(name: Str) -> ClassBuilder<Value>
fn Object.prototype_builder<Value>(name: Str) -> PrototypeBuilder<Value>

pub type ClassBuilder<Value> = {
  method: fn(ClassBuilder<Value>, MethodId, MethodEntry<Value>) -> ClassBuilder<Value>,
  extend: fn(ClassBuilder<Value>, DispatchTable<Value>) -> ClassBuilder<Value>,
  build: fn(ClassBuilder<Value>) -> DispatchTable<Value>,
}

pub type PrototypeBuilder<Value> = {
  method: fn(PrototypeBuilder<Value>, MethodId, MethodEntry<Value>) -> PrototypeBuilder<Value>,
  delegate: fn(PrototypeBuilder<Value>, DispatchTable<Value>) -> PrototypeBuilder<Value>,
  build: fn(PrototypeBuilder<Value>) -> DispatchTable<Value>,
}
```

### 3.3 例

```reml
use Core.Dsl.Object

let animal = Object.class_builder("Animal")
  .method("speak", |this, _| { "..." })
  .build()

let dog = { table: animal, payload: "Dog", shape_id: 1 }
let result = Object.call(dog, "speak", [], None)
```

## 4. Core.Dsl.Gc

### 4.1 主要型

```reml
pub enum GcStrategy =
  | Arena
  | RefCount
  | MarkAndSweep

pub type GcHeap = { strategy: GcStrategy }

pub type GcRef<T> = { heap: GcHeap, ptr: Int }

pub type RootScope = { heap: GcHeap, roots: List<Int> }

pub type GcError = { kind: GcErrorKind, message: Str }

pub enum GcErrorKind =
  | AllocationFailed
  | CollectFailed
```

### 4.2 最小 API

```reml
fn Gc.new(strategy: GcStrategy) -> GcHeap

fn Gc.with_scope<T>(heap: GcHeap, f: fn(RootScope) -> T) -> T

fn Gc.alloc<T>(scope: RootScope, value: T) -> Result<GcRef<T>, GcError> // `effect {memory}`
fn Gc.pin<T>(scope: RootScope, value: GcRef<T>) -> RootScope            // `effect {memory}`

fn Gc.collect(heap: GcHeap) -> Result<(), GcError>                       // `effect {memory}`
fn Gc.collect_if_needed(heap: GcHeap) -> Result<(), GcError>             // `effect {memory}`
```

### 4.3 例

```reml
use Core.Dsl.Gc

let heap = Gc.new(GcStrategy::Arena)
Gc.with_scope(heap, |scope| {
  let value = Gc.alloc(scope, "dsl:node")
  value
})
```

## 5. Core.Dsl.Actor

### 5.1 主要型

```reml
pub type ActorDefinition<Message> = {
  name: Str,
  on_message: fn(Message) -> Result<(), ActorError>,
}

pub type MailboxBridge<Message> = {
  send: fn(Message) -> Result<(), ActorError>, // `effect {io.async}`
  receive: fn() -> Result<Message, ActorError>, // `effect {io.async}`
}

pub type SupervisionBridge = {
  spec: SupervisorSpec,
}

pub type ActorError = { kind: ActorErrorKind, message: Str }

pub enum ActorErrorKind =
  | SpawnFailed
  | MailboxUnavailable
  | RuntimeFailure
```

### 5.2 最小 API

```reml
fn Actor.spawn<Message>(
  system: ActorSystem,
  def: ActorDefinition<Message>,
  supervision: Option<SupervisionBridge>
) -> Result<MailboxBridge<Message>, ActorError> // `effect {io.async}`
```

### 5.3 例

```reml
use Core.Dsl.Actor

let def = { name: "Echo", on_message: |msg| { log(msg); Ok(()) } }
let mailbox = Actor.spawn(system, def, None)
```

## 6. Core.Dsl.Vm

### 6.1 主要型

```reml
pub type Bytecode<Op> = {
  ops: List<Op>,
}

pub type VmState<Value> = {
  stack: List<Value>,
  frames: List<CallFrame>,
}

pub type CallFrame = { ip: Int }

pub type VmError = { kind: VmErrorKind, message: Str }

pub enum VmErrorKind =
  | Halted
  | InvalidOpcode
  | StackUnderflow
  | RuntimeFailure
```

### 6.2 最小 API

```reml
fn Vm.bytecode_builder<Op>() -> BytecodeBuilder<Op>

pub type BytecodeBuilder<Op> = {
  emit: fn(BytecodeBuilder<Op>, Op) -> BytecodeBuilder<Op>,
  build: fn(BytecodeBuilder<Op>) -> Bytecode<Op>,
}

fn Vm.run<Op, Value>(
  code: Bytecode<Op>,
  state: VmState<Value>,
  exec: fn(VmState<Value>, Op) -> Result<VmState<Value>, VmError>
) -> Result<VmState<Value>, VmError> // `effect {runtime}`
```

### 6.3 例

```reml
use Core.Dsl.Vm

enum Op = | Push(Int) | Add | Halt

let code = Vm.bytecode_builder()
  .emit(Op.Push(1))
  .emit(Op.Push(2))
  .emit(Op.Add)
  .emit(Op.Halt)
  .build()

let state = { stack: [], frames: [{ ip: 0 }] }
let result = Vm.run(code, state, |state, op| { exec_op(state, op) })
```

## 7. 監査と診断の扱い

| 操作 | 診断コード | 監査キー例 |
| --- | --- | --- |
| ディスパッチ失敗 | `dsl.object.dispatch_failed` | `dsl.object.method`, `dsl.object.shape_id` |
| GC 実行 | `dsl.gc.collect` | `dsl.gc.strategy`, `dsl.gc.heap_id` |
| アクター生成 | `dsl.actor.spawn_failed` | `dsl.actor.name`, `dsl.actor.stage` |
| VM 実行 | `dsl.vm.runtime_error` | `dsl.vm.opcode`, `dsl.vm.ip` |

監査イベントの詳細スキーマは [3-6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md) の `AuditEvent` に準拠し、Stage 要件は [3-8 Core Runtime Capability](3-8-core-runtime-capability.md) と一致させる。
