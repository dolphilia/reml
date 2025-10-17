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

type stage_trace_source =
  | StageFromTyper
  | StageFromRuntime
  | StageFromCapabilityJson of string  (* tooling/runtime/capabilities/*.json のファイル名 *)
  | StageFromCliOption
  | StageFromEnvVar

type stage_trace_step = {
  source : stage_trace_source;
  stage : stage_id option;
  capability : string option;
  note : string option;  (* 例: CLI フラグ値、RuntimeCapabilityResolver の分岐理由 *)
}

type stage_trace = stage_trace_step list

type effect_tag = {
  effect_name : string;
  effect_span : Ast.span;
}

type effect_set = {
  declared : effect_tag list;
  residual : effect_tag list;
}

type invalid_attribute_reason =
  | UnknownAttributeKey of string
  | UnsupportedStageValue of string
  | DuplicateAttribute

type invalid_attribute = {
  attribute_name : string;
  attribute_span : Ast.span;
  reason : invalid_attribute_reason;
}

type residual_effect_leak = {
  leaked_tag : effect_tag;
  leak_origin : Ast.span;
}

type effect_diagnostic_payload = {
  invalid_attributes : invalid_attribute list;
  residual_leaks : residual_effect_leak list;
}

type effect_profile = {
  effect_set : effect_set;
  stage_requirement : stage_requirement;
  source_span : Ast.span;
  source_name : string option;
  resolved_stage : stage_id option;
  resolved_capability : string option;
  stage_trace : stage_trace;
  diagnostic_payload : effect_diagnostic_payload;
}
```

> **メモ**: `Source_code.Span.t` は既存モジュールを流用予定。必要に応じて `EffectProfile` を AST/TAST/IR で分割する（AST では `stage_requirement` を `stage_requirement option` にして解析中の未確定状態を表す）。`stage_trace` は Typer → Runtime → CI の Stage 判定経路を記録するため、AST 段階では空リスト、Typer で初期化し、Runtime で追記する。`diagnostic_payload` は `effects.syntax.invalid_attribute` / `effects.contract.residual_leak` の生成根拠を保持し、`Diagnostic.extensions.effect.*` に転写する。

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

### 2.2 Stage トレース（`stage_trace`）

- `stage_trace` は Stage 判定の由来を記録するリストであり、Typer で `StageFromTyper` を起点に、`RuntimeCapabilityResolver` が判定した結果を `StageFromRuntime`、JSON/CLI/環境変数などの入力値を `StageFromCapabilityJson` / `StageFromCliOption` / `StageFromEnvVar` として追記する。
- Typer では `effect_profile.stage_trace` に `StageFromTyper` を格納し、`resolved_stage` と同じ値をセットする。Runtime 側では `AuditEnvelope.metadata.stage_trace` と CLI 診断 (`Diagnostic.extensions.effect.stage_trace`) に同一配列をエクスポートし、CI で `iterator.stage.audit_pass_rate` を算出する際の根拠として使用する。
- JSON ゴールデンでは `stage_trace` をそのまま出力し、Typer と Runtime の判定が一致しない場合は差分が可視化されるようにする。差分が 0 件であることが CI ゲート条件になる。

### 2.3 診断ペイロード

- `invalid_attributes` は未知属性・重複属性・Stage 値の記法エラーを保持し、Typer が `effects.syntax.invalid_attribute` を生成する際に `attribute_name` / `reason` / `span` を `Diagnostic.extensions.effect.attribute` へコピーする。
- `residual_leaks` は宣言された効果集合と解析済み残余集合の差分を格納し、`effects.contract.residual_leak` の本文・ハイライト・ガイダンス生成に利用する。`leak_origin` には、型推論中に初めて残余集合へ追加された場所の `Ast.span` を保持し、報告時に最小限のスパンを指す。
- `effect_diagnostic_payload` は Typer 完了時点で確定させ、Runtime では参照のみ。Runtime から追加情報が必要な場合は `stage_trace` を介して補う。

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
3. `stage_trace` を `Diagnostic.extensions.effect.stage_trace` と `AuditEnvelope.metadata.stage_trace` に反映し、`iterator.stage.audit_pass_rate` で Typer/Runtime の差分を自動評価できるようにする。
4. Core IR/Runtime 間の Stage 照合と監査ログ出力（`AuditEnvelope`）を実装し、Capability JSON との突合フローを確立する。
5. 効果診断の CLI ゴールデンテストを追加し、`effects.contract.*` キーと Stage トレースを JSON で固定する。
6. `docs/spec/1-3-effects-safety.md` および `3-8-core-runtime-capability.md` の対応表を更新し、`0-3-audit-and-metrics.md` §0.3.7 に運用記録を追記する。

### 進行状況サマリー（2025-10-17）

| 領域 | 状態 | 完了内容 | 次のステップ |
| --- | --- | --- | --- |
| Parser | ✅ 完了 | 効果属性解析・`effect_profile_node` 導入・ゴールデン更新済み | 行多相拡張検討（Phase 3） |
| Typer | 🚧 進行中 | `type_inference_effect.ml` による Stage 判定・効果テーブル登録完了 | 残余効果診断／未知属性診断／Stage トレース初期化と CLI ゴールデン整備 |
| Core IR | ✅ 第1段階 | `desugar` が効果セットと Stage を IR メタデータへ反映 | 複数 Capability 対応・EffectMarker 連携 |
| Runtime | ⏳ 未着手 | RuntimeCapabilityResolver で Stage コンテキスト取得 | Stage 照合・`stage_trace` 追記・監査ログ出力・Capability JSON 突合 |
| Tooling / CI | 🚧 進行中 | RuntimeCapability JSON 雛形・検証スクリプト、`tests/typeclass_effects` 追加 | 効果診断ゴールデン整備・Stage トレースを含む CI 指標 (`iterator.stage.audit_pass_rate`) 拡張 |

## 3. モジュール別タスク（Phase 2-2）

- **Parser (`parser.mly` / `ast.ml`)**
  - 効果注釈から `EffectTag.t` を構築し `EffectSet` へ格納。
  - `allows_effects` 属性を検出し `stage_requirement` の初期値を推定（`Exact Experimental` など）。
- **Typer (`type_inference.ml`, `type_inference_effect.ml`)**
  - 効果プロファイル正規化と Stage 判定 (`Type_inference_effect.resolve_function_profile`) は完了。
  - `Constraint_solver.EffectConstraintTable` を利用した残余効果診断・辞書経路独立性テストと CLI ゴールデンの整備が未完。
  - `effect_profile.stage_trace` を初期化し、未知属性・残余効果に対応する `effect_diagnostic_payload` を埋めて `Diagnostic.extensions` へ転写する。
- **Core IR (`core_ir/desugar.ml`, `core_ir/ir.ml`)**
  - 関数メタデータへの効果・Stage 反映を確認済み。複数 Capability や `EffectMarker` への拡張を追加予定。
- **Runtime (`runtime/native/...`)**
  - RuntimeCapabilityResolver で Stage コンテキストは取得可能。ランタイム側で Stage 検証・監査ログ出力を実装し、IR メタデータと突合する。
  - Typer が付与した `stage_trace` に Runtime 判定と Capability JSON 情報を追記し、`AuditEnvelope.metadata.stage_trace` に反映する。
- **Tooling/CI**
  - RuntimeCapability JSON テンプレート (`tooling/runtime/capabilities/default.json`) と検証スクリプトは整備済み。効果診断ゴールデンと CI 集計 (`iterator.stage.audit_pass_rate`) の拡張を進める。
  - `scripts/validate-runtime-capabilities.sh` と CLI `--emit-effects --format=json` を組み合わせ、Stage トレースを含むゴールデンファイルを更新する手順を `0-3-audit-and-metrics.md` と同期する。

## 4. フォローアップ

- Stage / 効果タグの更新は `docs/plans/bootstrap-roadmap/2-2-effect-system-integration.md` §1.1 を同期し、`0-3-audit-and-metrics.md` §0.3.7 に運用記録を残す。
- 行多相（Effect Polymorphism）拡張は Phase 3 以降に再検討。現在の文字列タグ表現を前提に、プラグイン拡張向けの柔軟な型設計を評価する。
- CLI ポリシー (`--effect-stage`, `--runtime-capabilities`) を CI の Stage 制御（例: `--deny experimental`）と連携させる設計を `RuntimeCapabilityResolver` の設定として定義する。
- 効果診断ゴールデン・Capability JSON を更新する際は `scripts/validate-runtime-capabilities.sh` で検証し、`0-3-audit-and-metrics.md` に変更履歴を追記する。
