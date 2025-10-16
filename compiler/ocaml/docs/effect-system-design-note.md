# 効果システム型定義ドラフト

**作成日**: 2025-10-17  
**参照計画**: [docs/plans/bootstrap-roadmap/2-2-effect-system-integration.md](../../docs/plans/bootstrap-roadmap/2-2-effect-system-integration.md)

## 1. 目的

- Phase 2-2 で実装する効果タグ解析と StageRequirement 検証のコア型を事前に整理する。
- Parser/Typer/Core IR/Runtime 間で共有する型とヘルパ API の責務分担を明確化する。
- 実装タスク（OCaml 側）を紐付け、各モジュールで着手すべき項目を可視化する。

## 2. 型定義ドラフト

```ocaml
(* compiler/ocaml/src/core_ir/effect.ml （新規予定） *)
type stage_id =
  | Experimental
  | Beta
  | Stable

let compare_stage_id lhs rhs =
  match (lhs, rhs) with
  | Experimental, Experimental -> 0
  | Experimental, _ -> -1
  | Beta, Experimental -> 1
  | Beta, Beta -> 0
  | Beta, Stable -> -1
  | Stable, Stable -> 0
  | Stable, _ -> 1

type stage_requirement =
  | Exact of stage_id
  | AtLeast of stage_id

let satisfies_stage requirement actual =
  match requirement with
  | Exact expected -> expected = actual
  | AtLeast lower -> compare_stage_id lower actual <= 0

module EffectTag = struct
  type t =
    | Mut
    | Io
    | Panic
    | Unsafe
    | Ffi
    | Syscall
    | Process
    | Thread
    | Memory
    | Signal
    | Hardware
    | Realtime
    | Audit
    | Security
    | Mem
    | Debug
    | Trace
    | Unicode
    | Time
    | Runtime

  let to_string = function
    | Mut -> "mut"
    | Io -> "io"
    | Panic -> "panic"
    | Unsafe -> "unsafe"
    | Ffi -> "ffi"
    | Syscall -> "syscall"
    | Process -> "process"
    | Thread -> "thread"
    | Memory -> "memory"
    | Signal -> "signal"
    | Hardware -> "hardware"
    | Realtime -> "realtime"
    | Audit -> "audit"
    | Security -> "security"
    | Mem -> "mem"
    | Debug -> "debug"
    | Trace -> "trace"
    | Unicode -> "unicode"
    | Time -> "time"
    | Runtime -> "runtime"
end

module EffectSet = Set.Make (EffectTag)

type effect_profile = {
  declared : EffectSet.t;
  residual : EffectSet.t;
  stage_requirement : stage_requirement;
  source_span : Source_code.Span.t;
}
```

> **メモ**: `Source_code.Span.t` は既存モジュールを流用予定。必要に応じて `EffectProfile` を AST/TAST/IR で分割する（AST では `stage_requirement` を `stage_requirement option` にして解析中の未確定状態を表す）。

## 3. モジュール別タスク（Phase 2-2）

- **Parser (`parser.mly` / `ast.ml`)**
  - 効果注釈から `EffectTag.t` を構築し `EffectSet` へ格納。
  - `allows_effects` 属性を検出し `stage_requirement` の初期値を推定（`Exact Experimental` など）。
- **Typer (`type_inference.ml`, `type_env.ml`)**
  - `effect_profile` を関数シグネチャへ添付し、型推論中に `declared`/`residual` の包含チェックを実装。
  - `satisfies_stage` を用いて Capability Stage と比較し、`Diagnostic.extensions["effect.stage.*"]` を生成。
- **Core IR (`core_ir/desugar.ml`, `core_ir/function.ml`)**
  - `EffectSet` を IR ノードに伝播。
  - StageRequirement を `RuntimeCapability` チェックのメタデータへ埋め込み。
- **Runtime (`runtime/native/...`)**
  - StageRequirement を Capability Registry へ受け渡し、`verify_capability_stage` 結果を診断へ反映。
- **Tooling/CI (`tooling/ci/collect-iterator-audit-metrics.py`, 新規効果テスト)**
  - 監査メトリクスに `effect_profile` の Stage 評価結果を追加。

## 4. フォローアップ

- Stage と効果タグの見直しは `docs/plans/bootstrap-roadmap/2-2-effect-system-integration.md` §1.1 のドラフト表を確定させた後、`0-3-audit-and-metrics.md` に測定結果を転記する。
- 行多相（Effect Polymorphism）の拡張は Phase 3 以降に再検討。`EffectTag` の列挙はプラグイン拡張を想定して `Other of string` を追加する案も検証する。
- CI での Stage フラグ (`--deny experimental` など) を CLI オプションへつなぐため、`compiler/ocaml/src/cli/options.ml` に `stage_policy` パラメータを追加するタスクを別途登録する。
