# Rust 型推論モジュール設計スケルトン（P1 / W3）

**更新日**: 2027-01-08  
**担当**: Rust 移植 P1 W3 / 型推論コア移植チーム  
**関係文書**: `docs/plans/rust-migration/1-0-front-end-transition.md` W3 節、`docs/plans/rust-migration/appendix/type-inference-ocaml-inventory.md`、`docs/plans/rust-migration/appendix/typed_ast_schema_draft.md`、`docs/plans/rust-migration/1-1-ast-and-ir-alignment.md#1-1-4-typed-ast--型情報の整合`、`docs/plans/rust-migration/1-3-dual-write-runbook.md`

本メモは Rust フロントエンド用の型推論モジュール（`compiler/rust/frontend/src/typeck/`）を構築する際の設計骨子を示す。W3 で確立した方針を記録し、以降の実装・dual-write 検証の指針とする。

## 1. 範囲と前提

- 対象は OCaml 実装における `type_inference.ml` / `constraint*.ml` / `type_inference_effect.ml` / `impl_registry.ml` の Rust 化スコープ。  
- AST/Typed AST の ID・Span 規約は `docs/plans/rust-migration/1-1-ast-and-ir-alignment.md#1-1-4-typed-ast--型情報の整合` に従い、`TyId`/`SpanId` は共通の 32bit 空間を利用する。  
- 診断 JSON / 監査メトリクスは `docs/plans/rust-migration/1-2-diagnostic-compatibility.md` と `reports/diagnostic-format-regression.md` の順序・省略規則を継承する。  
- Windows / Unix 双方で determinism を確保するため、乱数や時刻に依存した ID 生成は行わず、`TypeVarGen` で単調増加する `u32` を配布する。

## 2. モジュール構成（`compiler/rust/frontend/src/typeck/`）

| ファイル | 役割 | 主な型/関数 | 備考 |
| --- | --- | --- | --- |
| `mod.rs` | 公開 API の集約。`TypecheckDriver` のエントリポイントを提供。 | `pub struct TypecheckDriver;` `pub fn infer_unit(...) -> Result<TypeckOutput, TypeError>` | CLI から参照され、`TypecheckConfig` の注入とログ発火を統括。 |
| `types.rs` | 型表現・ID・変数生成。 | `TyId`, `TyVar`, `TypeRow`, `Scheme`, `Subst`, `TypeVarGen` | `TyVarGen` は `AtomicU32` + `ThinVec<Option<TyKind>>` で実装し、`OnceCell<TypeVarGen>` に収める。 |
| `constraint.rs` | 制約モデルとシリアライズ。 | `Constraint`, `ConstraintSet`, `ConstraintKind`, `impl serialize(&self) -> ConstraintSnapshot` | dual-write で JSON 比較できるよう `IndexMap` で順序固定。 |
| `solver.rs` | 単一化・効果行・impl 解決。 | `fn unify(lhs, rhs, ctx)`, `fn occurs_check`, `fn solve_trait_constraints` | `TypeError` を `thiserror` ベースで表現。監査メトリクスは `effects.*` キーで集計。 |
| `effect.rs` | Stage/Capability 情報を扱う。 | `StageRequirement`, `StageTrace`, `EffectProfile`, `fn resolve_effect_profile(...)` | CLI 由来の Stage コンテキストと AST の `effect_profile_node` を接続。 |
| `impl_registry.rs` | impl 登録と探索。 | `ImplRegistry`, `ImplKey`, `ImplSpec`, `fn lookup(...)` | 内部は `RwLock<IndexMap<ImplKey, ImplSpec>>`。determinism 目的で `IndexMap` を採用。 |
| `env.rs` | グローバル共有とハンドル。 | `TypecheckConfig`, `TypeContext`, `DualWriteGuards` | `OnceCell<TypecheckConfig>` に CLI 設定を格納し、`DualWriteGuards` が JSON/監査ログの保存先を決定。 |

## 3. 基本データ型とフィールド方針

| 項目 | OCaml 由来 | Rust 定義方針 | 参照 |
| --- | --- | --- | --- |
| `TyId` | `type_id : int` | `NonZeroU32`（0 は未割当）。`Typed_ast` と共用。 | `docs/plans/rust-migration/appendix/typed_ast_schema_draft.md` |
| `TyVar` | `Type_variable.t` | `u32` + `TyKind`（enum）。`TypeVarGen` が `AtomicU32` で管理。 | 同上 |
| `Scheme` | `forall` + 制約セット | `struct Scheme { vars: SmallVec<[TyVar; 4]>, body: TyId, where_: ConstraintSet }` | `docs/spec/1-2-types-Inference.md` |
| `Constraint` | `Constraint.t` | `enum ConstraintKind { Equals(TyId, TyId), Impl(ImplKey), EffectRow(EffectConstraint) ... }` | `compiler/ocaml/src/constraint.ml` |
| `EffectRow` | `effect_row` | `IndexMap<EffectLabel, EffectEntry>` + `SmallVec<EffectLabel, 4>` で順序固定。 | `docs/spec/1-3-effects-safety.md` |
| `StageRequirement` | `StageRequirement.t` | `enum StageRequirement { Exact(StageId), AtLeast(StageId) }`。`StageId` は `NonZeroU16`. | `docs/spec/3-8-core-runtime-capability.md` |
| `TypecheckConfig` | `Type_inference.make_config` | `struct TypecheckConfig { effect_context: StageContext, type_row_mode: TypeRowMode, recover: RecoverConfig }`。`OnceCell<TypecheckConfig>` で共有。 | `docs/plans/rust-migration/1-0-front-end-transition.md#1-0-6-ワークストリームと主要論点` |

## 4. 設定と依存注入

- CLI からの設定値（`--type-row-mode`, `--effect-stage`, `--recover-strategy` 等）は `TypecheckConfigBuilder` によって収集し、`env::install_config(builder.build())` で `OnceCell` に注入する。  
- `TypeContext` は `&'static TypecheckConfig` と `SharedStateHandles`（impl registry, effect tables, type var gen, metrics sink）を束ね、`TypecheckDriver` から各フェーズへ渡される。  
- `TypecheckDriver::infer_unit` 内では以下の順序で初期化する：  
  1. `DualWriteGuards::new(input_digest)` を作成し、`reports/dual-write/front-end/w3-type-inference/` への書き込み先パスを確定。  
  2. `TypeVarGen::bootstrap()` で `AtomicU32` カウンタと `ThinVec` を確保。  
  3. `ImplRegistry::load_from(snapshot)` を呼び出し、OCaml 側 JSON を読み込み比較できるようにする（dual-write 中限定）。  
  4. `ConstraintRecorder` を solver に注入し、Rust/OCaml 双方の JSON ログを `IndexMap` 順序で整列。  
- `TypecheckConfig` は AST 側が期待する ID/Span 空間を共有するため、`typed_ast_schema_draft.md` で列挙した `SpanTraceId` を引用して構造体コメントに記載済みとする。

## 5. 共有状態と決定性

- **Impl Registry**: `RwLock<IndexMap<ImplKey, ImplSpec>>` を採用。書き込み順序は登録順で固定し、dual-write 比較用に `impl-registry.rust.json` を生成。  
- **Effect Profile Table**: `DashMap<EffectLabel, EffectEntry>` + `StageTrace`。集計結果は `collect-iterator-audit-metrics.py --section effects` で利用されるため、`records.sort_by_key(|entry.stage_id, effect_label|)` 後に JSON 出力。  
- **Constraint Logging**: `ConstraintSet` の内部表現を `IndexMap<ConstraintId, ConstraintKind>` とし、`ConstraintId` を `u32` 単調増加で割り当てることで determinism を担保。  
- **Error Propagation**: 例外は `TypeError` enum (`codes::TYPE_*`) に集約し、`diagnostic::Builder` へ変換する際に `extensions.effect_stage_consistency` を埋める。  
- **Dual-write Hooks**: `DualWriteGuards` が AST/Typed AST/Constraint/Impl Registry の JSON ファイル名を管理し、`1-3-dual-write-runbook.md` に定義した CLI 共有オプション（`--dualwrite-dir`, `--dualwrite-label`）と一致させる。

## 6. テストおよびハーネス連携

- Rust 側テストは `compiler/rust/frontend/tests/type_inference.rs`（ユニット）と `tests/streaming_metrics.rs` 由来のストリーミング指標を共有する形で追加。  
- `p1-front-end-checklists.csv` の「制約ソルバ」各行を満たすため、以下の JSON を dual-write で生成する：  
  - `typed-ast.{ocaml,rust}.json`（TypedExpr/Pattern + TyId 整合）  
  - `constraint.{ocaml,rust}.json`（ConstraintSet / Effect 行）  
  - `impl-registry.{ocaml,rust}.json`（登録順序）  
  - `effects-metrics.{ocaml,rust}.json`（`collect-iterator-audit-metrics.py` 入力）  
- `poc_frontend` CLI へ `--type-row-mode` / `--effect-stage-*(runtime|capability)` / `--recover-*` を追加済みで、実行時に `TypecheckConfig::builder` へ注入し `typeck::install_config` を呼び出す。`scripts/poc_dualwrite_compare.sh` からは `--dualwrite-root` `--dualwrite-run-label` `--dualwrite-case-label` が渡され、`DualWriteGuards` が `typeck/config.json` `typeck/metrics.json` を生成する。
- CLI 検証は `scripts/poc_dualwrite_compare.sh --mode typeck --cases @p1-front-end-checklists.csv` で統一し、`docs/plans/rust-migration/1-3-dual-write-runbook.md#1-3-2-w3-type-inference-モード` に沿って運用する。  
- 失敗時は `reports/dual-write/front-end/w3-type-inference/case-*/` 以下へ差分を保存し、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に再掲する。

## 7. 未決事項 / TODO

1. **impl where 句の厳格化**: 現状は恒真扱い。`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md#type-impl` の `TYPE-003` 対応後に Rust 側にも制約を実装する。  
2. **Effect Row 圧縮**: `SmallVec` ベースの差分圧縮は PoC 未着手。W4 以降の最適化タスクへ委譲し、現段階では OCaml と同じ O(n) マージで実装する。  
3. **FFI Bridge Snapshot**: `Type_inference` が保持する `ffi_bridge_snapshots` を Rust で再現するには、`runtime` チームの P2 設計（`docs/plans/rust-migration/2-1-runtime-integration.md`）との整合が必要。暫定的に JSON ログの対照のみ提供。  
4. **並列型推論**: `rayon` 等による並列化は determinism を損なうため、P1 期間はシングルスレッドを維持。Stage 以降での導入可否を `4-0-risk-register.md` で追跡。

以上の骨子に基づき、Rust 型推論モジュールの実装を開始する。
