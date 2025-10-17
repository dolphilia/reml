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

### 2.1 AST レベルの EffectProfileNode

Parser ではタグ名を `Ast.ident` のまま保持し、解析段階の効果情報を以下の構造体に格納する：

```ocaml
(* compiler/ocaml/src/ast.ml *)
type effect_profile_node = {
  effect_declared : Ast.ident list;      (* !{ io, panic } など明示された集合 *)
  effect_residual : Ast.ident list;      (* AST では declared と同一で初期化 *)
  effect_stage : stage_requirement_annot option;  (* @requires_capability 等で解析 *)
  effect_span : Ast.span;                (* 属性／注釈の位置情報 *)
}
```

- `effect_declared` は宣言上のタグ順を保持し、後続フェーズで `EffectSet` へ変換する。
- `effect_residual` は Typer での解析結果を反映しやすくするために確保しておき、Parser では `effect_declared` のコピーで初期化する。
- `effect_stage` は `StageExact`/`StageAtLeast` 判定を `@requires_capability(stage=...)` などの属性から設定する。未注釈の場合は `None`。
- Typer では `effect_profile_node` から `effect_profile` へ正規化し、`declared`/`residual` を `EffectSet` に変換する。
- 属性からの推論ルール（第1段階）:
  - `@requires_capability(...)` / `@requires_capability_exact(...)` は第1引数（文字列リテラルまたは識別子）を `StageExact` として解釈。
  - `@requires_capability_at_least(...)` は `StageAtLeast` として扱う。
  - `@dsl_export` や `@allows_effects` だけが存在し Stage 未指定の場合は、暗黙に `StageAtLeast("stable")` を設定する（将来的に属性引数で上書き可）。

## 5. 進捗状況（2025-10-17 時点）

### 完了した項目
- `effect_profile_node` を AST に導入し、関数宣言・トレイトシグネチャ・extern 宣言で保持するように更新。
- パーサで `@requires_capability`（`Exact`/`AtLeast`）と `@dsl_export` / `@allows_effects` 属性を解析し、`effect_stage` を補完。
- パーサテストを拡充し、`Exact:experimental` と `AtLeast:stable` の既定挙動をゴールデン化。
- 設計ノートに属性→Stage 変換ルールと AST レベルのデータ構造を追記。
- `@allows_effects(...)` / `@handles(...)` 属性から効果タグ集合を抽出し、`!{ ... }` と併用した場合も順序を維持してマージするロジックを実装。

### 残タスク / 次ステップ
1. 属性値のバリデーション（未知タグ／未宣言キー）と診断出力を追加し、`effects.syntax.invalid_attribute` へ接続する。
2. Typer 側で `effect_profile_node` を `effect_profile` に正規化し、`type_inference_effect.ml`（新規ヘルパ）経由で Stage 検証と診断出力（`effects.contract.stage_mismatch` / `effects.contract.stage_escalation_required` / `effects.contract.residual_leak`）へ接続する。
3. `EffectSet` 化した情報を Core IR / Runtime Capability 検査へ伝搬し、CI メトリクス（`collect-iterator-audit-metrics.py` 等）と連動させる。
4. 追加仕様変更が発生した場合は `docs/spec/1-3-effects-safety.md`・`docs/spec/3-8-core-runtime-capability.md` の対応表を更新し、監査ログのキー整合を確認する。

## 3. モジュール別タスク（Phase 2-2）

- **Parser (`parser.mly` / `ast.ml`)**
  - 効果注釈から `EffectTag.t` を構築し `EffectSet` へ格納。
  - `allows_effects` 属性を検出し `stage_requirement` の初期値を推定（`Exact Experimental` など）。
- **Typer (`type_inference.ml`, `type_env.ml`, `type_inference_effect.ml`, `type_inference/typeclass_pipeline.ml`)**
  - `effect_profile_node` を正規化して `Type_env.function_entry` に `effect_profile` を保持するフィールドを追加。
  - `core_ir/effect.ml` で定義する `stage_requirement` 判定ヘルパを介して Capability Stage と比較し、`effects.contract.stage_mismatch` / `effects.contract.stage_escalation_required` / `effects.contract.residual_leak` を `Diagnostic.extensions["effect.stage.*"]` に出力。
  - 型クラス辞書生成との独立性を維持しつつ、`typeclass_pipeline` へ効果情報を受け渡さないインターフェースを設計し、回帰テスト（`tests/typeclass_effects/`）で確認。
- **Core IR (`core_ir/desugar.ml`, `core_ir/function.ml`, `core_ir/effect.ml`)**
  - `EffectSet` を IR ノードに伝播。
  - StageRequirement を `RuntimeCapability` チェックのメタデータへ埋め込み。
- **Runtime (`runtime/native/...`)**
  - StageRequirement を Capability Registry へ受け渡し、`verify_capability_stage` 結果を診断へ反映。
- **Tooling/CI (`tooling/ci/collect-iterator-audit-metrics.py`, `tooling/ci/sync-iterator-audit.sh`, 新規効果テスト)**
  - 監査メトリクスに `effect_profile` の Stage 評価結果を追加し、`iterator.stage.audit_pass_rate` に Typer 側の判定結果を突合。
  - 効果診断のゴールデンテスト（CLI/JSON）を追加し、`effects.contract.*` のフィールド内容をスナップショットで固定。

## 4. フォローアップ

- Stage と効果タグの見直しは `docs/plans/bootstrap-roadmap/2-2-effect-system-integration.md` §1.1 のドラフト表を確定させた後、`0-3-audit-and-metrics.md` に測定結果を転記する。
- 行多相（Effect Polymorphism）の拡張は Phase 3 以降に再検討。`EffectTag` の列挙はプラグイン拡張を想定して `Other of string` を追加する案も検証する。
- CI での Stage フラグ (`--deny experimental` など) を CLI オプションへつなぐため、`compiler/ocaml/src/cli/options.ml` に `stage_policy` パラメータを追加するタスクを別途登録する。
