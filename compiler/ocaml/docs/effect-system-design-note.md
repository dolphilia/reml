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

## 5. 進捗状況（2025-10-18 更新）

### 完了した項目
- `effect_profile_node` を AST に導入し、関数宣言・トレイトシグネチャ・extern 宣言で保持するように更新。
- パーサで `@requires_capability`（`Exact`/`AtLeast`）と `@dsl_export` / `@allows_effects` 属性を解析し、`effect_stage` を補完。
- パーサテストを拡充し、`Exact:experimental` と `AtLeast:stable` の既定挙動をゴールデン化。
- 設計ノートに属性→Stage 変換ルールと AST レベルのデータ構造を追記。
- `@allows_effects(...)` / `@handles(...)` 属性から効果タグ集合を抽出し、`!{ ... }` と併用した場合も順序を維持してマージするロジックを実装。
- Stage トレースの経路（CLI/環境変数 → `RuntimeCapabilityResolver` → Typer → 診断/監査）を整備し、`effect_profile.stage_trace`・`Diagnostic.extensions.effect.stage_trace`・監査メタデータで同一配列を保持するように実装。
- `compiler/ocaml/tests/golden/diagnostics/effects/*.json.golden` / `.../audit/effects-stage.json.golden` を更新し、Stage 解釈結果とトレース差分をゴールデンとして固定。
- `scripts/validate-runtime-capabilities.sh` と `tooling/ci/sync-iterator-audit.sh` を再設計し、`reports/runtime-capabilities-validation.json`・`reports/iterator-stage-summary.md` に検証サマリーを出力する運用を確立。
- GitHub Actions（bootstrap-linux / bootstrap-macos）に iterator audit サマリー生成ジョブを追加し、`iterator.stage.audit_pass_rate` < 1.0 を即座に失敗させるゲートを常設化。
- 属性値バリデーションを Parser/Typer に組み込み、未知タグ・未宣言キー・不正 Stage 値を `effect_diagnostic_payload.invalid_attributes` に集約。Typer から `effects.syntax.invalid_attribute` を発行し、CLI JSON で `effects.diagnostic_payload` / 監査メタデータを出力するゴールデンを整備。
- `Type_error.effect_invalid_attribute_error` を追加し、`Diagnostic.extensions.effect.*` / `effect.invalid_attributes` / 監査キーを一貫させた。
- 残余効果検出 (`effects.contract.residual_leak`) を Typer で実装し、`Constraint_solver.EffectConstraintTable` に診断ペイロードを保持させることで宣言集合と残余集合の差分を追跡。`test_effect_residual.ml` と `tests/typeclass_effects/effectful_sum.reml` を用いた統合テストにより、辞書モード・モノモルフィゼーションモード双方で同一診断が得られることをゴールデン化した。

### 残タスク / 次ステップ
1. Runtime 側（`runtime/native`）で Stage 照合イベントを `AuditEnvelope` として発火し、Typer が記録した `stage_trace` と照合結果を JSON Lines で永続化する。
2. Parser で `@dsl_export(allows_effects=...)` / `@handles(effect=...)` といった named 引数を正式に受理できるよう構文規則を拡張し、現在 `expect_fail` として残しているテストを成功ケースへ更新する。
3. `docs/spec/1-3-effects-safety.md` および `3-8-core-runtime-capability.md` の Stage 表・監査キー記述を、今回の実装内容（stage_trace と CI 連携）に合わせて改訂する。
4. GitHub Actions（bootstrap-linux / bootstrap-macos）で生成する `iterator-stage-summary.md` をレビュー手順に組み込み、Windows override 向け Stage テストと `0-3-audit-and-metrics.md` への転記フローを整備する。
5. `effects.syntax.invalid_attribute` のケーススタディを増やし、`unknown_key` / `unknown_effect_tag` / `unsupported_stage_value` それぞれの補助メッセージを `docs/spec/3-6-core-diagnostics-audit.md` に反映する。
6. 残余効果診断で採取したメトリクス（欠落タグ数・追加された `stage_trace` ステップ数）を `0-3-audit-and-metrics.md` §0.3.7 に追記し、辞書モード／モノモルフィゼーション経路ごとの差分を継続監視する。

### 進行状況サマリー（2025-10-24）

| 領域 | 状態 | 完了内容 | 次のステップ |
| --- | --- | --- | --- |
| Parser | ✅ 完了 | 効果属性解析・`effect_profile_node` 導入・ゴールデン更新済み | 行多相拡張検討（Phase 3）、named 引数対応 |
| Typer | ✅ 第3段階 | Stage トレース付き `effect_profile` 正規化、効果属性から Capability ID を抽出して `Effect_profile.resolved_capability` と Stage トレースへ反映、`effects.syntax.invalid_attribute`・`effects.contract.residual_leak`・`effects.contract.stage_mismatch` を CLI / 監査両経路で固定化し、辞書/モノモルフィゼーション一致をテストで保証 | Stage メトリクス (`iterator.stage.audit_pass_rate`) への自動登録と Core IR / Runtime との突合 |
| Core IR | ✅ 第1段階 | `desugar` が効果セットと Stage を IR メタデータへ反映 | 複数 Capability 対応・EffectMarker 連携 |
| Runtime | 🚧 進行中 | `RuntimeCapabilityResolver` で CLI/環境変数/JSON を統合し Stage トレースを構築、`main.ml` から `runtime_stage_event` を監査へ出力。Core IR 側では `core_ir/iterator_audit.ml` で `DictMethodCall` の `iterator_audit` メタデータを集計し、ランタイム Stage 判定と結合した `effect.stage` 監査イベントを `main.ml` から永続化する経路を追加済み | IR メタデータとの突合結果を Windows/他プラットフォームの Capability に展開し、Stage 実走チェックや JSON Lines 監査ログの拡張を進める |
| Tooling / CI | 🚧 進行中 | Stage トレース検証スクリプト整備、`reports/runtime-capabilities-validation.json`・`reports/iterator-stage-summary.md` を生成 | CI への組み込みと自動ゲート化 (`iterator.stage.audit_pass_rate`)、Windows override テストの追加 |

## 3. モジュール別タスク（Phase 2-2）

- **Parser (`parser.mly` / `ast.ml`)**
  - 効果注釈から `EffectTag.t` を構築し `EffectSet` へ格納。
  - `allows_effects` 属性を検出し `stage_requirement` の初期値を推定（`Exact Experimental` など）。
- **Typer (`type_inference.ml`, `type_inference_effect.ml`)**
  - 効果プロファイル正規化と Stage 判定 (`Type_inference_effect.resolve_function_profile`) は完了し、Typer 起点の `stage_trace` を診断・監査の両経路へ反映済み。
  - `Constraint_solver.EffectConstraintTable` に診断ペイロードを保持し、残余効果診断 (`effects.contract.residual_leak`) と Stage ミスマッチ (`effects.contract.stage_mismatch`) を生成。辞書/モノモルフィゼーション両経路の CLI ゴールデンを共有化。
  - 今後は Stage 集計メトリクスへの送出を自動化し、抽出した Capability ID を Core IR / Runtime の検証フローへ連携する。
- **Core IR (`core_ir/desugar.ml`, `core_ir/ir.ml`)**
  - 関数メタデータへの効果・Stage 反映を確認済み。複数 Capability や `EffectMarker` への拡張を追加予定。
- **Runtime (`runtime/native/...`)**
  - RuntimeCapabilityResolver で Stage コンテキストを取得し、`main.ml` から `runtime_stage_event` を発火して監査ログへ記録。
  - 今後は IR メタデータとの突合・Stage 実走チェック・プラットフォーム別 Capability 拡張を実装し、`AuditEnvelope.metadata.stage_trace` に Runtime 由来の情報を追記する。
  - `core_ir/iterator_audit.ml` で収集した `iterator_audit` メタデータを `main.ml` が `effect.stage` 監査イベントに変換し、`effects-residual.jsonl` 監査ログへ書き出すフローを整備する。
- **Tooling/CI**
  - RuntimeCapability JSON テンプレート (`tooling/runtime/capabilities/default.json`) と検証スクリプトを整備し、`reports/runtime-capabilities-validation.json`・`reports/iterator-stage-summary.md` を生成。
  - 今後は GitHub Actions へスクリプトを組み込み、`iterator.stage.audit_pass_rate` を CI ゲート化するとともに Windows override テストを追加する。
  - `effects-residual.jsonl` のスナップショット（`compiler/ocaml/tests/golden/audit/effects-residual.jsonl.golden`）と CI 指標の照合を自動化し、RuntimeCapability JSON の更新が Stage トレースと一致するかを常時検証する。

## 4. フォローアップ

- Core IR の `iterator_audit` メタデータと Runtime Capability JSON を突合し、`RuntimeCapabilityResolver` → `AuditEnvelope` → CI 指標（`iterator.stage.audit_pass_rate`）までを一連の監査パイプラインとして固定化する。`core_ir/iterator_audit.ml` で収集した Stage 情報を `main.ml` が `effect.stage` 監査イベントへ変換し、`compiler/ocaml/tests/golden/audit/effects-residual.jsonl.golden` を更新対象として扱う。
- Stage / 効果タグの更新は `docs/plans/bootstrap-roadmap/2-2-effect-system-integration.md` §1.1 を同期し、`0-3-audit-and-metrics.md` §0.3.7 に運用記録を残す。
- 行多相（Effect Polymorphism）拡張は Phase 3 以降に再検討。現在の文字列タグ表現を前提に、プラグイン拡張向けの柔軟な型設計を評価する。
- CLI ポリシー (`--effect-stage`, `--runtime-capabilities`) を CI の Stage 制御（例: `--deny experimental`）と連携させる設計を `RuntimeCapabilityResolver` の設定として定義する。
- 効果診断ゴールデン・Capability JSON を更新する際は `scripts/validate-runtime-capabilities.sh` で検証し、`0-3-audit-and-metrics.md` に変更履歴を追記する。
- Phase 2-3 FFI 契約拡張（[docs/plans/bootstrap-roadmap/2-3-ffi-contract-extension.md](../../docs/plans/bootstrap-roadmap/2-3-ffi-contract-extension.md)）と連携し、`AuditEnvelope.metadata.bridge.*` の導入時に `stage_trace` と `RuntimeCapabilityResolver` の結果を共有する。特に Apple Silicon (arm64-apple-darwin) の Capability 定義は `tooling/runtime/capabilities/default.json` と `reports/runtime-capabilities-validation.json` を参照し、`bridge.platform` / `bridge.abi` / `bridge.stage` を効果診断の残余解析と突合できるようにする。macOS 向け追加メトリクスは `reports/ffi-macos-summary.md`（予定）に記録し、Stage ゲートと同一のレビューサイクルで確認する。
- Parser/AST で導入した `extern_metadata`（`@ffi_target`, `@ffi_ownership` 等）を Typer 側で解釈し、診断 (`ffi.contract.missing`) と 監査 (`bridge.*`) の一貫性を確保する。重複・型不整合は `extern_invalid_attributes` に集約されるため、Phase 2-3 で診断コードへ接続する。

## 5. 検証結果と成果物（2025-10-24 更新）

- `Constraint_solver.EffectConstraintTable` に `diagnostic_payload` を保持し、残余効果 (`effects.contract.residual_leak`) と Stage ミスマッチ (`effects.contract.stage_mismatch`) の診断を CLI / 監査の両経路で同期。Stage トレースは `append_runtime_stage_trace` により Typer → Runtime の経路を一体化。
- 統合テスト `compiler/ocaml/tests/test_effect_residual.ml` を追加し、**辞書モード** (`--typeclass-mode=dictionary`) と **モノモルフィゼーションモード** (`--typeclass-mode=monomorph`) 双方で同一診断になることを固定化。ゴールデン `compiler/ocaml/tests/golden/diagnostics/effects/residual-leak.json.golden` を更新。
- Stage 差分のスナップショットとして `compiler/ocaml/tests/golden/diagnostics/effects/stage-resolution.json.golden` と 監査ログ `compiler/ocaml/tests/golden/audit/effects-stage.json.golden` を整備し、Typer が抽出した Capability ID と Runtime 判定が同一になることをトレースで検証。
- `dune runtest`（`compiler/ocaml/`）を実行し、効果診断・監査テストを含むスイート全体が成功することを確認。
- `scripts/validate-runtime-capabilities.sh tooling/runtime/capabilities/default.json` を実行し、Stage 集約結果を `reports/runtime-capabilities-validation.json` に保存。
- `tooling/ci/sync-iterator-audit.sh --metrics tooling/ci/iterator-audit-metrics.json --verify-log tooling/ci/llvm-verify.log --audit compiler/ocaml/tests/golden/audit/effects-stage.json.golden` を実行し、`reports/iterator-stage-summary.md` に Stage トレース検証サマリーを生成。
- 監査ログ `compiler/ocaml/tests/golden/audit/effects-residual.jsonl.golden` と CI 指標 `iterator.stage.audit_pass_rate` の算出結果を突合し、Core IR メタデータと RuntimeCapability JSON が一致する場合のみ合格（100%）となることを確認。
