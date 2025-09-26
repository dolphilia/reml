# 4.5 Hardware Capability プラグイン — CPU & Platform Introspection

> 位置付け: 公式プラグイン（オプション）。ハードウェア情報の取得はプラットフォーム依存であり、環境によっては特権操作を伴うため、標準APIから分離して運用審査を経る。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | ドラフト（公式プラグイン） |
| プラグインID | `core.hardware` |
| 効果タグ | `effect {hardware}`, `effect {unsafe}`, `effect {security}`, `effect {thread}`, `effect {audit}` |
| 依存モジュール | `Core.Runtime`, [4-2 Process Capability プラグイン](4-2-process-plugin.md), [4-1 System Capability プラグイン](4-1-system-plugin.md), `Core.Diagnostics`, `Core.Numeric & Time` |
| 相互参照 | [3.8 Core Runtime & Capability Registry](3-8-core-runtime-capability.md), [3-6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md) |

## 1. HardwareCapability API

```reml
pub type HardwareCapability = {
  read_cpu_id: fn() -> CpuId,                                 // effect {hardware}
  cpu_features: fn() -> CpuFeatures,                          // effect {hardware}
  rdtsc: fn() -> u64,                                         // effect {hardware, timing}
  rdtscp: fn() -> (u64, u32),                                 // effect {hardware, timing}
  prefetch: fn<T>(Ptr<T>, PrefetchLocality) -> (),            // effect {hardware}
  numa_nodes: fn() -> List<NumaNode>,                         // effect {hardware}
  bind_numa: fn(NumaNode) -> Result<(), HardwareError>,       // effect {hardware, thread}
}
```

- `rdtsc` / `rdtscp` は高精度タイムスタンプ。使用時は `effect {hardware}` に加え `timing` サブ効果を検討する。
- `prefetch` は CPU キャッシュへのプリフェッチヒント。
- NUMA 関連は [4-2 Process Capability プラグイン](4-2-process-plugin.md) の `set_thread_affinity` と連携。

## 2. 型定義

```reml
pub type CpuId = {
  vendor: Str,
  model: Str,
  family: u32,
  stepping: u32,
}

pub type CpuFeatures = {
  sse: Bool,
  sse2: Bool,
  sse3: Bool,
  sse4_1: Bool,
  sse4_2: Bool,
  avx: Bool,
  avx2: Bool,
  avx512: Bool,
  aes: Bool,
  sha: Bool,
  neon: Bool,
  custom: Set<Str>,
}

pub type NumaNode = {
  id: u32,
  cpus: Set<u32>,
  memory_bytes: usize,
}
```

- `custom` はベンダ独自機能を列挙。`sysfs` / `cpuid` 情報を保持する。

## 3. エラーと監査

```reml
pub type HardwareError = {
  kind: HardwareErrorKind,
  message: Str,
}

pub enum HardwareErrorKind = Unsupported | PermissionDenied | InvalidNode | OperationFailed
```

- 管理者権限が必要な操作は `PermissionDenied` を返し、監査ログに記録する。

## 4. 使用例ドラフト

- SIMD 最適化: `cpu_features()` を参照し、`RuntimeCapability::SIMD` と整合。
- NUMA アフィニティ: `bind_numa` を `ProcessCapability::set_thread_affinity` と組み合わせ、スレッドをノードに固定する例を追加予定。

## 5. 今後の拡張

- GPU・FPGA 情報の取得 API。
- 温度/電力のテレメトリ。
- ハードウェア乱数 (`RDSEED`, `RDRAND`) のラッパ。

---

*本章はドラフトであり、公式プラグインとしての配布・審査プロセスは `Chapter 4` のエコシステム仕様と連携して今後更新される。*
