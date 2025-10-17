# 効果システム型定義ドラフト

**作成日**: 2025-10-17  
**参照計画**: [docs/plans/bootstrap-roadmap/2-2-effect-system-integration.md](../../docs/plans/bootstrap-roadmap/2-2-effect-system-integration.md)

## 1. 目的

- Phase 2-2 で実装する効果タグ解析と StageRequirement 検証のコア型を事前に整理する。
- Parser/Typer/Core IR/Runtime 間で共有する型とヘルパ API の責務分担を明確化する。
- 実装タスク（OCaml 側）を紐付け、各モジュールで着手すべき項目を可視化する。

## 2. 型定義ドラフト

```ocaml
(* compiler/ocaml/src/effect_profile.ml *)
type stage_id =
  | Experimental
  | Beta
  | Stable
  | Custom of string

let compare_stage_id lhs rhs =
  let rank = function
    | Experimental -> 0
    | Beta -> 1
    | Stable -> 2
    | Custom _ -> 3
  in
  match (lhs, rhs) with
  | Custom a, Custom b ->
      String.compare (String.lowercase_ascii a) (String.lowercase_ascii b)
  | Custom _, _ -> 1
  | _, Custom _ -> -1
  | a, b -> compare (rank a) (rank b)

type stage_requirement =
  | StageExact of stage_id
  | StageAtLeast of stage_id

let satisfies_stage requirement actual =
  match requirement with
  | StageExact expected -> compare_stage_id expected actual = 0
  | StageAtLeast lower -> compare_stage_id lower actual <= 0

type effect_tag = {
  effect_name : string;
  effect_span : Ast.span;
}

type effect_set = {
  declared : effect_tag list;
  residual : effect_tag list;
}

type effect_profile = {
  effect_set : effect_set;
  stage_requirement : stage_requirement;
  source_span : Ast.span;
  source_name : string option;
  resolved_stage : stage_id option;
  resolved_capability : string option;
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
1. 属性値のバリデーション（未知タグ／未宣言キー）を実装し、`effects.syntax.invalid_attribute` 診断を追加する。
2. `Constraint_solver.EffectConstraintTable` を用いた残余効果検出 (`effects.contract.residual_leak`) と型クラス辞書経路との整合テストを整備する。
3. Core IR/Runtime 間の Stage 照合と監査ログ出力（`AuditEnvelope`）を実装し、Capability JSON との突合フローを確立する。
4. 効果診断の CLI ゴールデンテストを追加し、`effects.contract.*` キーを JSON で固定する。
5. `docs/spec/1-3-effects-safety.md` および `3-8-core-runtime-capability.md` の対応表を更新し、`0-3-audit-and-metrics.md` §0.3.7 に運用記録を追記する。

### 進行状況サマリー（2025-10-17）

| 領域 | 状態 | 完了内容 | 次のステップ |
| --- | --- | --- | --- |
| Parser | ✅ 完了 | 効果属性解析・`effect_profile_node` 導入・ゴールデン更新済み | 行多相拡張検討（Phase 3） |
| Typer | 🚧 進行中 | `type_inference_effect.ml` による Stage 判定・効果テーブル登録完了 | 残余効果診断／効果ゴールデン／辞書経路との統合テスト |
| Core IR | ✅ 第1段階 | `desugar` が効果セットと Stage を IR メタデータへ反映 | 複数 Capability 対応・EffectMarker 連携 |
| Runtime | ⏳ 未着手 | RuntimeCapabilityResolver で Stage コンテキスト取得 | Stage 検証・監査ログ出力・Capability JSON 突合 |
| Tooling / CI | 🚧 進行中 | RuntimeCapability JSON 雛形・検証スクリプト、`tests/typeclass_effects` 追加 | 効果診断ゴールデン整備・CI 指標 (`iterator.stage.audit_pass_rate`) 拡張 |

## 3. モジュール別タスク（Phase 2-2）

- **Parser (`parser.mly` / `ast.ml`)**
  - 効果注釈から `EffectTag.t` を構築し `EffectSet` へ格納。
  - `allows_effects` 属性を検出し `stage_requirement` の初期値を推定（`Exact Experimental` など）。
- **Typer (`type_inference.ml`, `type_inference_effect.ml`)**
  - 効果プロファイル正規化と Stage 判定 (`Type_inference_effect.resolve_function_profile`) は完了。
  - `Constraint_solver.EffectConstraintTable` を利用した残余効果診断・辞書経路独立性テストと CLI ゴールデンの整備が未完。
- **Core IR (`core_ir/desugar.ml`, `core_ir/ir.ml`)**
  - 関数メタデータへの効果・Stage 反映を確認済み。複数 Capability や `EffectMarker` への拡張を追加予定。
- **Runtime (`runtime/native/...`)**
  - RuntimeCapabilityResolver で Stage コンテキストは取得可能。ランタイム側で Stage 検証・監査ログ出力を実装し、IR メタデータと突合する。
- **Tooling/CI**
  - RuntimeCapability JSON テンプレート (`tooling/runtime/capabilities/default.json`) と検証スクリプトは整備済み。効果診断ゴールデンと CI 集計 (`iterator.stage.audit_pass_rate`) の拡張を進める。

## 4. フォローアップ

- Stage / 効果タグの更新は `docs/plans/bootstrap-roadmap/2-2-effect-system-integration.md` §1.1 を同期し、`0-3-audit-and-metrics.md` §0.3.7 に運用記録を残す。
- 行多相（Effect Polymorphism）拡張は Phase 3 以降に再検討。現在の文字列タグ表現を前提に、プラグイン拡張向けの柔軟な型設計を評価する。
- CLI ポリシー (`--effect-stage`, `--runtime-capabilities`) を CI の Stage 制御（例: `--deny experimental`）と連携させる設計を `RuntimeCapabilityResolver` の設定として定義する。
- 効果診断ゴールデン・Capability JSON を更新する際は `scripts/validate-runtime-capabilities.sh` で検証し、`0-3-audit-and-metrics.md` に変更履歴を追記する。
