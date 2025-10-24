# TYPE-003 型クラス辞書渡し復元計画

## 1. 背景と症状
- 仕様では制約解決で得た辞書を Core IR へ渡し、Stage や Capability 情報を監査ログに残すと定義されている（docs/spec/1-2-types-Inference.md:115-119）。  
- 現行実装は算術制約解決前に型変数を強制的に `i64` へ単一化し（compiler/ocaml/src/type_inference.ml:1877-1937）、`solve_trait_constraints` の戻り値を `_dict_refs` として握り潰している（compiler/ocaml/src/type_inference.ml:2219-2376）。結果として Core IR に辞書ノードが生成されず、監査ログ（`AuditEnvelope.metadata`）へも `effect.stage.*` が記録されない。  
- 型クラス PoC の比較基盤（docs/plans/bootstrap-roadmap/2-1-typeclass-strategy.md）と `collector` 系 API の差分検証が行えず、Phase 3 のセルフホスト移行に必要な辞書情報が欠落する。

## 2. Before / After
### Before
- 制約解決直前に型変数を数値型へ強制解決し、辞書生成を迂回して演算子を単純型にマップ。  
- `_dict_refs` を破棄するため、`record_typeclass_success` 以外の辞書利用が発火せず、`Typed_ast` に辞書引数が存在しない。

### After
- `solve_trait_constraints` の結果を `typed_expr` / `typed_decl` へ反映し、Core IR へ辞書引数を追加する。  
- 算術系の簡易最適化は `RunConfig.extensions["typeclass"].mode = "dictionary"` でのみ適用し、既定は辞書渡し。  
- 辞書情報（Stage 要件・Capability ID）を `Diagnostic.extensions["typeclass"]` と `AuditEnvelope.metadata` に転記し、`0-3-audit-and-metrics.md` の `typeclass.dictionary_pass_rate` を 1.0 に引き上げる。

#### 擬似コード案
```ocaml
let* dict_refs =
  match scheme.constraints with
  | [] -> Ok []
  | constraints -> solve_trait_constraints constraints
in
let texpr_with_dicts = Typed_ast.attach_dict_args texpr dict_refs
```

## 3. 影響範囲と検証
- **IR 検証**: Core IR 生成テストで `DictCall` ノードが生成されることを確認し、LLVMI R 差分を `scripts/compare-ir.sh` でレビュー。  
- **診断**: `reports/diagnostic-format-regression.md` に辞書関連診断のゴールデンを追加し、`scripts/validate-diagnostic-json.sh` で新フィールドが出力されることを CI で保証。  
- **性能**: Phase 2-1 の `benchmark_typeclass.sh` で辞書渡しモードを既定とし、モノモルフィゼーションとの比較を継続。
- **単体テスト**: `compiler/ocaml/tests/typeclass_dictionary_tests.ml` を追加し、辞書引数が Core IR と監査ログの両方に出力されるかスナップショットで確認する。

## 4. フォローアップ
- Core IR / CodeGen で辞書引数を受け取るパスが未実装のため、`compiler/ocaml/src/core_ir_builder.ml` への追記と LLVM 側のレビュー（Phase 2-3）を依頼。  
- `docs/spec/1-2-types-Inference.md` に Dickens-style の辞書例を追加し、仕様に沿った実装を確認。  
- `typeclass.metadata` の監査連携を Phase 2-7 の `collect-iterator-audit-metrics.py` 更新と同時に実施。
- `docs/plans/bootstrap-roadmap/2-1-typeclass-strategy.md` の進捗欄へ辞書復元タスクを追記し、Phase 2 全体の型クラスロードマップと整合させる。
- **タイミング**: Phase 2-5 の前半から中盤にかけて辞書渡し実装を最優先で進め、Phase 2-6 開始前までに Core IR・監査ログと合わせて復元を完了する。

## 残課題
- 算術演算のデフォルト型選択を辞書渡しと共存させる際の互換ポリシー（既存 CLI ゴールデンとの差分許容範囲）を確認。  
- Stage 情報をどのレイヤで取得するか（Constraint solver で付与 vs. Typer 側で補完）を Phase 2-7 チームと調整したい。
