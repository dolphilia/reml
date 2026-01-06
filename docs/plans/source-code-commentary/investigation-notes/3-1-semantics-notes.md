# 第3部 第8章: 意味解析 (Semantic Analysis) 調査メモ

## 参照した資料
- `compiler/frontend/src/semantics/mod.rs:1-2`（semantics モジュールの公開範囲）
- `compiler/frontend/src/semantics/typed.rs:12-365`（TypedModule/TypedExpr/TypedPattern など型付き AST の定義）
- `compiler/frontend/src/semantics/mir.rs:16-1299`（MirModule と Typed -> MIR の変換）
- `compiler/frontend/src/typeck/driver.rs:133-712`（型推論後に TypedModule/MIR を組み立てる流れ）
- `compiler/frontend/src/typeck/driver.rs:5575-5660`（Pattern -> TypedPattern の降ろし込み）
- `compiler/frontend/src/typeck/driver.rs:7812-8374`（TypedExprDraft から TypedExpr への確定化、dict_ref/scheme の構築）
- `compiler/frontend/src/typeck/driver.rs:2400-2512`（impl 仕様と qualified call 候補の付与）
- `docs/spec/1-0-language-core-overview.md`
- `docs/spec/1-1-syntax.md`

## 調査メモ

### semantics モジュールの構成と役割
- `semantics` は `typed` と `mir` の 2 サブモジュールのみを再エクスポートする軽量な構成。(`compiler/frontend/src/semantics/mod.rs:1-2`)
- 現状の「意味解析」は独立フェーズではなく、型推論 (`typeck`) が `TypedModule` を生成し、その結果を `mir` に降ろすことで意味情報を固定する設計になっている。(`compiler/frontend/src/typeck/driver.rs:133-712`, `compiler/frontend/src/semantics/mir.rs:31-74`)

### Typed AST の主要構造
- `TypedModule` は関数/Active Pattern/Conductor/ActorSpec/Extern/DictRef/Scheme を束ねる統合コンテナ。(`compiler/frontend/src/semantics/typed.rs:12-26`)
- `TypedFunction` は引数/戻り値/属性/本体 (`TypedExpr`) と、型スキーム参照・辞書参照 (`dict_ref_ids`) を持つ。(`compiler/frontend/src/semantics/typed.rs:28-46`)
- `TypedExpr` は `ty: String` と `dict_ref_ids` を持つため、型付けと trait 解決（辞書参照）が同一ノードに同居する。(`compiler/frontend/src/semantics/typed.rs:167-173`)
- `QualifiedCall` は型メソッド/トレイトメソッド/関連関数の解決ヒントを保持し、MIR での `qualified_calls` テーブル構築に利用される。(`compiler/frontend/src/semantics/typed.rs:175-193`)
- `TypedPattern` は AST の Pattern を同型の `TypedPatternKind` に写像して保持する。(`compiler/frontend/src/semantics/typed.rs:373-443`, `compiler/frontend/src/typeck/driver.rs:5575-5660`)

### Typed AST を組み立てる流れ
- `TypecheckDriver::infer_module_from_ast` が型推論の結果として `TypedModule` を構築する。(`compiler/frontend/src/typeck/driver.rs:133-712`)
- Active Pattern/関数/impl メソッド/actor spec/Conductor の各ルートは、推論後に `TypedFunction`/`TypedActivePattern`/`TypedActorSpec`/`TypedConductor` に正規化される。(`compiler/frontend/src/typeck/driver.rs:194-637`)
- `TypedExprDraft` と `TypedStmtDraft` を `finalize_typed_expr` / `finalize_typed_stmt` で確定化し、`Substitution` を適用した型ラベルに更新する。(`compiler/frontend/src/typeck/driver.rs:7812-8326`)
- `TypedPattern` は `lower_typed_pattern` でパーサー AST から生成する（Guard は内側パターンへ畳み込み）。(`compiler/frontend/src/typeck/driver.rs:5575-5660`)
- 型スキームは `build_scheme_info` で `TypedModule.schemes` に格納され、各 `TypedFunction` は `scheme_id` で参照する。(`compiler/frontend/src/typeck/driver.rs:358-406`, `compiler/frontend/src/typeck/driver.rs:8357-8374`)
- 辞書参照（impl 参照）は `DictRefDraft` を型確定後に `DictRef` へ変換し、`TypedModule.dict_refs` に集約する。(`compiler/frontend/src/typeck/driver.rs:684-695`, `compiler/frontend/src/typeck/driver.rs:8328-8341`)

### MIR の設計と Typed -> MIR 変換
- MIR は `MirExprId` によるフラットな式配列 (`exprs`) と `body` 参照を持つ構造で、式木を ID 参照で表現する。(`compiler/frontend/src/semantics/mir.rs:16-255`)
- `MirModule::from_typed_module` が TypedModule を走査し、関数/Active Pattern/Conductor/Externs を MIR に写像する。(`compiler/frontend/src/semantics/mir.rs:31-74`)
- 型ラベルは `normalize_mir_type_label` で `Int -> i64`, `Unit -> ()` の正規化を行う。(`compiler/frontend/src/semantics/mir.rs:526-552`)
- `MirExprBuilder` が `TypedExpr` を MIR 化し、`panic` 呼び出しを `MirExprKind::Panic` として特別扱いする。(`compiler/frontend/src/semantics/mir.rs:671-965`)
- `MirExprKind::Block` は `defers` と `defer_lifo` を持ち、LIFO 実行順を明示的に保持する。(`compiler/frontend/src/semantics/mir.rs:305-313`, `compiler/frontend/src/semantics/mir.rs:742-761`)
- `QualifiedCall` は MIR 生成時に `qualified_calls` テーブルへ昇格し、`owner#expr_id` で索引される。(`compiler/frontend/src/semantics/mir.rs:867-949`)

### パターン/マッチの意味情報
- `lower_pattern` が `TypedPattern` を `MirPattern` へ変換し、Slice の head/rest/tail 分割を MIR 構造に埋め込む。(`compiler/frontend/src/semantics/mir.rs:966-1066`)
- `build_match_lowering` は `MatchLoweringPlan` を生成し、パターンの「失敗パス」と `always_matches` をラベル化する。(`compiler/frontend/src/semantics/mir.rs:1068-1241`)
- `build_match_lowerings` は TypedModule 全体から MatchLoweringPlan を収集する補助 API であり、後続フェーズのデバッグ・可視化用情報として利用できる。(`compiler/frontend/src/semantics/mir.rs:1244-1299`)

### MIR への impl/qualified call 情報の補完
- `collect_impl_specs` が impl ブロックを `MirImplSpec` へ集約し、`impl_id` と重複/未解決を検出する。(`compiler/frontend/src/typeck/driver.rs:2400-2451`)
- `populate_qualified_call_candidates` は trait method 呼び出しの候補 impl を探索し、候補が 1 件なら `impl_id` を確定する。(`compiler/frontend/src/typeck/driver.rs:2453-2512`)

### 仕様との照合メモ
- Typed/MIR の構文要素は `docs/spec/1-1-syntax.md` の式・パターン定義（match/active pattern など）と一対一対応している。
- 意味解析の具体的な仕様は 1.0/1.1 の構文セクションを起点に、型推論/効果検査にまたがって実装されているため、章では「typeck が意味解析を兼ねている」ことを明示する必要がある。

### 未確認事項 / TODO
- `QualifiedCall` の生成ロジック（`infer_expr` 周辺）で決まる `owner/name/impl_id` の解決規則を追跡する。
- `TypedModule` に格納される `dict_ref_ids` と `MirModule.impls` の対応関係を具体例で整理する。
