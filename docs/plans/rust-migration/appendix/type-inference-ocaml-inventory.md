# OCaml 型推論スタック棚卸し（Rust 移植 P1 / W3）

**更新日**: 2027-01-05  
**担当**: Rust 移植 P1 W3 / 型推論コア移植タスク  
**関係文書**: `docs/plans/rust-migration/1-0-front-end-transition.md` W3 節、`docs/plans/rust-migration/appendix/typed_ast_schema_draft.md`、`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md`、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md`

本メモは OCaml 実装の型推論スタック（Type Inference / Constraint / Impl Registry / Effect 行）の現状を棚卸しし、Rust フロントエンド移植に必要な API・データ構造・テスト資産・仕様乖離を可視化する。W3 の後続手順（Rust 設計スケルトン、dual-write 準備）の入力資料として利用する。

## 1. モジュール構成と責務

| モジュール | 主責務 | 主要 API / 型 | グローバル状態・例外 | 備考 |
| --- | --- | --- | --- | --- |
| `type_inference.ml` | Hindley-Milner ベースの推論器本体。構文木から制約生成し `Constraint_solver` へ送る。 | `make_config`, `infer_expr`, `infer_pattern`, `infer_decl`, `infer_compilation_unit`, `generalize`, `instantiate`（`compiler/ocaml/src/type_inference.ml:22-120`, `2923-2980`） | `current_config`, `global_impl_registry`, `ffi_bridge_snapshots`, `typeclass_stage_registry` など `ref` ベースの共有状態（同:44-120）。例外は `Type_error.*` を返し、CLI では `Result` 化される。 | Rust では `TypecheckConfig`（effect context / type_row_mode）を `OnceCell` で共有し、impl/FFI/Typeclass 参照は `DashMap`/`RwLock<IndexMap<..>>` で determinism を確保する必要あり。 |
| `constraint.ml` | 単一化制約と代入のユーティリティ。 | `constraint_kind`, `constraint_set`, `unify_constraint`, `apply_subst`, `compose_subst`, `ftv_*`, `occurs_check`, `unify`（`compiler/ocaml/src/constraint.ml:15-190`） | 例外は `Type_error` を利用。`Result` で単一化失敗を返す。 | Rust では `Constraint` 構造体と `Substitution` を `SmallVec` 等で実装し、`TyVarId` を `u32` 化する前提。 |
| `constraint_solver.ml` | 型クラス制約と効果行制約の解決。Effect KPI 収集も担当。 | `EffectConstraintTable`, `record_effect_profile`, `solve_trait_constraints`, `solve_iterator_constraint`, `ensure_assignable`, `constraint_error`（`compiler/ocaml/src/constraint_solver.ml:1-220` 付近） | `effect_constraints`（mutable table）と Stage 監査メタデータを保持。例外は `Type_error.constraint_error_to_type_error` 経由。 | Rust 版では `HashMap<String, EffectEntry>` + `IndexMap` で determinism を確保し、`collect-iterator-audit-metrics.py` が参照する `effects.*` メトリクスを dual-write で計測する必要がある。 |
| `type_inference_effect.ml` | AST 上の `effect_profile_node` を `Effect_profile.profile` へ正規化し Stage 判定を行う。 | `runtime_stage`, `create_runtime_stage`, `stage_for_capability`, `resolve_function_profile`（`compiler/ocaml/src/type_inference_effect.ml:1-120`） | `runtime_stage_default` と CLI オプション由来の Stage 情報を保持。`Type_error.effect_*` を返す。 | Rust では CLI → Typer への Stage 伝播を `TypecheckConfig` に同居させ、`StageTrace` を `Vec<StageTraceStep>` で保持する。 |
| `impl_registry.ml` | impl 宣言の登録と型照合。 | `impl_info`, `impl_registry`, `register`, `lookup`, `find_matching_impls`（`compiler/ocaml/src/impl_registry.ml:1-160`） | グローバル状態は `type_inference.ml` 側で `ref` として保持。 | Rust では deterministic な探索順序（`IndexMap`）と `DashMap` ベースの共有を想定。where 句は常時成功の暫定実装であり、今後の課題。 |

### 1.1 依存関係

- `Type_inference` ⇄ `Constraint_solver`: 制約収集／解決 API (`Constraint_solver.solve_trait_constraints`) を Result でやりとり。Rust でも `Result<TypeckSuccess, TypeError>` を維持。  
- `Type_inference` ⇄ `Type_inference_effect`: `config.effect_context` を通じて Stage 判定を注入。`--type-row-mode` フラグが effect_row dual-write を制御する。  
- `Impl_registry` は `Type_inference` 内に `global_impl_registry` として保持され、`Constraint_solver` から `lookup` される。Rust 移植では `TypeContext` 構造体に束縛する案を採用予定。  
- CLI/テスト (`scripts/poc_dualwrite_compare.sh`, `compiler/ocaml/tests/*`) は `Type_inference.infer_compilation_unit` を直接呼び出し、JSON ダンプ（AST/Typed AST/Constraints）を取得する。

## 2. データ構造とログ

| 項目 | OCaml 定義 | Rust 側での対応方針 | ログ／メトリクス |
| --- | --- | --- | --- |
| `config` / `type_row_mode` | effect context + type row mode (`type_inference.ml:22-42`) | `TypecheckConfig { effect_stage: StageContext, type_row_mode: TypeRowMode }` を `OnceCell` で共有。 | CLI フラグ `--type-row-mode {metadata-only,dual-write,ty-integrated}` を JSON に書き出し、dual-write 差分を追跡。 |
| `Constraint.constraint_` | `Unify(ty, ty)` + `span` (`constraint.ml:15-52`) | `struct Constraint { kind: ConstraintKind, span: Span }` として serde 化。 | `reports/dual-write/front-end/w3-type-inference/constraints.{ocaml,rust}.json` を生成予定。 |
| `substitution` | `(type_var * ty) list` (`constraint.ml:60-110`) | `IndexMap<TyVarId, Ty>` + `SmallVec` で差分適用。 | 逸脱時は `type_effect_row_equivalence` KPI が 1.0 未満となる。 |
| `effect_row` | `TArrow` の第2要素、`Type_inference_effect` で `declared/residual` を算出 (`2-7-deferred-remediation.md#type-002-effect-row-integration`) | Rust では `Ty::Fn(Box<Ty>, EffectRow, Box<Ty>)`。`EffectRow` には `declared`, `residual`, `canonical`, `row_var`。 | `collect-iterator-audit-metrics.py --section effects` が `diagnostics.effect_row_stage_consistency`, `type_effect_row_equivalence`, `effect_row_guard_regressions` を集計。 |
| Impl Registry | `impl_info list` と `type_subst` (`impl_registry.ml:1-120`) | `Vec<ImplInfo>` + `DashMap`/`IndexMap` で deterministic 検索。`where` 句処理は TODO。 | `reports/dual-write/front-end/w3-type-inference/impl-registry.{ocaml,rust}.json` でエントリ比較予定。 |
| Effect Trace | `Effect_analysis` が `effect_profile` を生成 (`type_inference.ml:80-170`) | Rust では `EffectTrace` を `Vec<Tag>` + `StageTrace`. | `collect-iterator-audit-metrics.py --section effects` の `effects.unify.*`, `effects.impl_resolve.*`, `effects.residual_leak_rate`. |

## 3. テスト資産とシナリオ分類

| 分類 | テスト／スクリプト | カバーするシナリオ | 備考 |
| --- | --- | --- | --- |
| パターン推論 | `compiler/ocaml/tests/test_type_inference.ml`（`1-160`） | 基本／タプル／コンストラクタ／ネスト／ガード／レコードの `infer_pattern`、`infer_expr`。 | Rust へ移植する際は `cargo test type_inference::patterns` 相当のスナップショットに変換。 |
| CallConv / Core IR 連携 | `compiler/ocaml/tests/test_cli_callconv_snapshot.ml`（`1-120`） | `remlc` CLI 経由で AST→Typed AST→Core IR→LLVM まで通し、型推論失敗で診断を生成。`ffi_bridge_snapshots` を合わせて監査ログ化。 | dual-write モードでは `scripts/poc_dualwrite_compare.sh --mode typeck` で CLI を 2 回呼び出す予定。 |
| FFI 契約診断 | `compiler/ocaml/tests/test_ffi_contract.ml`（`1-160`） | `Type_inference.infer_compilation_unit` が FFI 宣言を解析し、`Type_error.to_diagnostic_with_source` で JSON/Audit ゴールデン照合。 | Rust でも同 JSON/Audit を比較。`bridge.audit_pass_rate` KPI が 1.0 であることを確認。 |
| CLI 診断整形 | `compiler/ocaml/tests/test_cli_diagnostics.ml`（`1-150`） | CLI JSON/カラー出力、`Cli.Stats` 連携、`Diagnostic.Builder` から JSON への変換。型推論エラー `E7001` を固定化。 | Rust では `diagnostics` crate へ移植予定。Typed AST 比較に紐付け。 |
| 追加参照 | `compiler/ocaml/tests/test_cli_callconv_snapshot.ml`, `test_cli_diagnostics.ml`, `scripts/poc_dualwrite_compare.sh`, `scripts/validate-diagnostic-json.sh` | dual-write 差分の収集とスキーマ検証。 | W3 Step3 以降で Rust CLI 版にも同じ CLI オプションを提供する。 |

## 4. 既知の仕様乖離との突合（W3 観点）

| ID | 差分概要（出典） | OCaml 現状（本棚卸しの観測） | Rust 移植時のアクション |
| --- | --- | --- | --- |
| `TYPE-001`（`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md:217-225`） | 値制限未実装で効果安全性が崩壊。`infer_decl` 一般化条件のドラフトが必要。 | `Type_inference.generalize` は `effect_row`／`residual` を考慮せず全束縛を一般化。`type_row_mode` が `metadata-only` の場合に発火する残課題を保持。 | Rust では `Generalizer` に `EffectRow` と `StageRequirement` を組み込む。W3 Step2 で設定する `TypecheckConfig` に `value_restriction=Strict` を追加し、dual-write テストを `reports/dual-write/front-end/w3-type-inference/cases/value-restriction` で検証。 |
| `TYPE-002`（`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md:303-333`） | 効果行統合ロードマップ。`ty-integrated` 既定化時の KPI 監視が必要。 | OCaml 実装は `type_row_mode` を CLI で切替可能だが、`Impl_registry`／`Constraint_solver` では where 句や Stage 実測が未実装のまま。 | Rust 版は最初から `EffectRow` 必須の設計とし、`collect-iterator-audit-metrics.py --section effects` を dual-write 実行に組み込む。KPI を 0.5pt 以内で一致させる。 |
| `EFFECT-001`（`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md:217-225`） | `Type_inference.Effect_analysis` で `mut`/`io` タグが未捕捉。 | `Effect_analysis.call_tag_prefixes`（`type_inference.ml:80-142`）は手動一覧で、CI のタグ増減と同期されていない。 | Rust ではタグ一覧を YAML（`runtime/capabilities/*.json`）から生成し、`effects.impl_resolve.*` KPI を `collect-iterator-audit-metrics.py` と比較。 |
| `TYPE-003` | Core IR との辞書渡し不整合。 | `impl_registry.ml` は辞書参照 `DictImplicit/DictParam/DictLocal` を定義しているが、`Type_inference` 側は `record_monomorph_instances` ログのみ。 | Rust 版は `ImplRegistry` を deterministic order で serialize し、dual-write JSON（`impl-registry.*.json`）を比較。 |

上記 ID の補足説明を `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` と `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に追記済み（2027-01-05 追記）。今後の差分は本メモの「次ステップ」で参照する。

## 5. ギャップ一覧とフォローアップ

1. **値制限と効果残余**  
   - 現状: `generalize` が効果行と Stage 情報を参照しない。  
   - 影響: `TYPE-001` の再発リスク。  
   - 対応: Rust 版で `ValueRestriction::Strict` を既定にし、OCaml 側にも `Type_inference.make_config ~type_row_mode:Type_row_dual_write` を強制する CLI フラグを W3 Step3 で追加する。

2. **Impl Registry determinism**  
   - 現状: `impl_registry.ml` は `impl_info list` をリスト順で探索するのみ。登録順によって診断が変わる可能性あり。  
   - 影響: dual-write での JSON diff が発生。  
   - 対応: Rust では `IndexMap` を採用し determinism を確保。OCaml 側でも `List.sort` を導入する案を `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` へ追記。

3. **Effect KPI と CLI フラグの乖離**  
   - 現状: `collect-iterator-audit-metrics.py --section effects` を型推論タスクではまだ必須フローに組み込んでいない。  
   - 対応: `1-3-dual-write-runbook.md` Step4〜6 へ W3 用の命名規約（`reports/dual-write/front-end/w3-type-inference/` 以下）を追加予定。Rust 版では `--emit typeck-debug <dir>` を CLI に追加し、`effects.unify.*` を JSON 化する。

4. **テスト分類のギャップ**  
   - `test_type_inference.ml` は CLI 経由での `Type_inference.infer_compilation_unit` 呼び出しを含まない。Rust ではユニットテスト + CLI ゴールデンの両方を dual-write する必要がある。  
   - Action: `p1-front-end-checklists.csv` の「制約ソルバ」行に、パターン推論／CLI callconv／FFI 診断／診断 JSON の 4 系列を受入基準として追記（別途更新済み）。

## 6. 次ステップ（W3 後半以降の入力）

- `docs/plans/rust-migration/appendix/type-inference-architecture.md`（新規）に Rust 側 `typeck` crate のモジュール図と API 草案を落とし込み、`TypecheckConfig`／`ImplRegistry`／`EffectRow` の責務を切り分ける。  
- `scripts/poc_dualwrite_compare.sh --mode typeck` へ CLI 拡張（`--emit typed-ast`, `--emit constraints`, `--emit impl-registry`, `--emit effects`) を登録し、`reports/dual-write/front-end/w3-type-inference/` のサマリテンプレートを作成する。  
- `docs/plans/rust-migration/1-0-front-end-transition.md` W3 セクションに本棚卸しの結果リンクと TODO を反映済み。後続タスクでは Rust 設計と dual-write 実装に着手する。

