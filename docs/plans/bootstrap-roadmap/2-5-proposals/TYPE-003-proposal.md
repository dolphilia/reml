# TYPE-003 型クラス辞書渡し復元計画

## 1. 背景と症状
- 仕様では制約解決で得た辞書を Core IR へ渡し、Stage や Capability 情報を監査ログに残すと定義されている（docs/spec/1-2-types-Inference.md:115-119）。  
- 現行実装は算術制約解決前に型変数を強制的に `i64` へ単一化し（compiler/ocaml/src/type_inference.ml:2186-2212）、`solve_trait_constraints` の戻り値を `_dict_refs` として握り潰している（compiler/ocaml/src/type_inference.ml:2213-2376）。結果として Core IR に辞書ノードが生成されず、監査ログ（`AuditEnvelope.metadata`）へも `effect.stage.*` が記録されない。  
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
- Core IR / CodeGen で辞書引数を受け取るパスが未実装のため、`compiler/ocaml/src/core_ir/desugar.ml` と `compiler/ocaml/src/llvm_gen/codegen.ml` への追記と LLVM 側のレビュー（Phase 2-3）を依頼。  
- `docs/spec/1-2-types-Inference.md` に Dickens-style の辞書例を追加し、仕様に沿った実装を確認。  
- `typeclass.metadata` の監査連携を Phase 2-7 の `collect-iterator-audit-metrics.py` 更新と同時に実施。
- `docs/plans/bootstrap-roadmap/2-1-typeclass-strategy.md` の進捗欄へ辞書復元タスクを追記し、Phase 2 全体の型クラスロードマップと整合させる。
- **タイミング**: Phase 2-5 の前半から中盤にかけて辞書渡し実装を最優先で進め、Phase 2-6 開始前までに Core IR・監査ログと合わせて復元を完了する。

## 5. 実施ステップ（Week31）
- **Day1 — Typer で辞書を保持**: `compiler/ocaml/src/type_inference.ml:2213-2376` の `let* _dict_refs = …` ブロックを `Typed_ast.attach_dict_args`（新設）へ置き換え、`typed_decl`/`typed_expr` が `dict_ref list` を保持できるよう `typed_ast.ml` に追加フィールドを定義する。`generalize` 後も制約が失われないことを `compiler/ocaml/tests/type_inference_tests.ml`、`compiler/ocaml/tests/test_typeclass_solver.ml` のゴールデンで確認する。
- **Day2-3 — Core IR / CodeGen 連携**: `core_ir/desugar.ml:110-320` と `core_ir/monomorphize_poc.ml` を更新し、Typer が付与した `dict_ref` を `DictConstruct` / `DictMethodCall` / `DictLookup` ノードへ落とし込む。`core_ir/ir.ml`・`llvm_gen/codegen.ml` で辞書レイアウトと ABI（`docs/spec/3-8-core-runtime-capability.md` §10）を再計算し、`scripts/compare-ir.sh` と `core_ir/tests/test_dict_gen.ml` で差分を承認する。
- **Day3-4 — 診断・監査整合 (DIAG-002 連携)**: `typeclass_metadata.ml` の `dictionary_json_of_ref` を `None` 禁止にし、`type_error.ml` / `diagnostic.ml` で `Diagnostic.audit` への転写を必須化する。`tooling/ci/verify-audit-metadata.py` と `reports/diagnostic-format-regression.md` へ辞書フィールドを追加し、`scripts/validate-diagnostic-json.sh` が `extensions.typeclass.dictionary.kind != "none"` を検証できるようにする。
- **Day4 — Capability / Stage 逆引き**: `core_ir/iterator_audit.ml` と `core_ir/desugar.ml` に辞書由来の Capability ID / Stage 情報を差し込むヘルパーを追加し、`Type_inference.record_typeclass_success` が `effect.stage.*` を `AuditEnvelope.metadata` に設定できるよう `typeclass_audit_events` を拡張する。`docs/spec/3-6-core-diagnostics-audit.md` のキー一覧と照合し、`effect.stage.mismatch` 診断の再発を防ぐ。
- **Day5 — テスト・文書・メトリクス**: `compiler/ocaml/tests/typeclass_dictionary_tests.ml`（新規）や `compiler/ocaml/tests/test_core_ir_codegen.ml` を追加して辞書の生成・伝搬・監査をスナップショット化する。`docs/spec/1-2-types-Inference.md` §B.1 と `docs/spec/2-1-parser-type.md` に辞書復元の脚注を記し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` へ `typeclass.dictionary_pass_rate` を追加、CI では `record-metrics.sh` で値を集計する。

### 5.1 成果物／検証一覧
| 種別 | 内容 | エビデンス |
|------|------|------------|
| Typer | `typed_decl` / `typed_expr` が `dict_ref list` を保持 | `compiler/ocaml/tests/typeclass_dictionary_tests.ml` の `let eq_i64 = ...` ゴールデン |
| Core IR | `DictConstruct` / `DictMethodCall` が生成され LLVM まで到達 | `scripts/compare-ir.sh` / `llvm_gen/tests/test_codegen_dict.ml` |
| 診断 | `extensions.typeclass.dictionary.*` と `metadata["typeclass.*"]` が一致 | `reports/diagnostic-format-regression.md` と `tooling/ci/verify-audit-metadata.py` |
| メトリクス | `typeclass.dictionary_pass_rate` 追加・既定 1.0 | `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` と CI `record-metrics.sh` |
| ドキュメント | 仕様脚注とロードマップ更新 | `docs/spec/1-2-types-Inference.md` / `docs/plans/bootstrap-roadmap/2-1-typeclass-strategy.md` |

## 6. 残課題
- 算術演算のデフォルト型選択を辞書渡しと共存させる際の互換ポリシー（既存 CLI ゴールデンとの差分許容範囲）を確認。  
- Stage 情報をどのレイヤで取得するか（Constraint solver で付与 vs. Typer 側で補完）を Phase 2-7 チームと調整したい。

## 7. 進捗状況（2025-10-30）
- **Day1 完了（Typer 辞書復元）**: `solve_trait_constraints` の戻り値を `typed_expr` / `typed_decl` に添付し、辞書参照を型付き AST へ残す実装を反映（compiler/ocaml/src/type_inference.ml:2219、compiler/ocaml/src/typed_ast.ml:19）。テストでは `test_type_inference.ml` と `test_typeclass_mode.ml` の既存ケースが辞書モードで回ることを確認済み。
- **Day2-3 完了（Core IR／CodeGen 連携）**: 関数宣言に辞書パラメータを先頭挿入し、`DictConstruct` / `DictMethodCall` を生成する経路を整備（compiler/ocaml/src/core_ir/desugar.ml:393、compiler/ocaml/src/core_ir/monomorphize_poc.ml:23、compiler/ocaml/tests/test_dict_gen.ml:1）。`test_typeclass_execution.ml:1` では辞書版コードが LLVM IR まで到達することを E2E で検証。
- **Day3-4 完了（監査・診断メタデータ）**: `Typeclass_metadata` と `type_error.ml` が辞書情報を `Diagnostic` と `AuditEnvelope` に埋め込み（compiler/ocaml/src/typeclass_metadata.ml:60、compiler/ocaml/src/type_error.ml:1888）、ドライバから `typeclass_audit_events` を出力する新経路を追加（compiler/ocaml/src/main.ml:513）。CI では辞書メトリクスの必須キーを検証する処理を導入（tooling/ci/collect-iterator-audit-metrics.py:1464）し、`typeclass.dictionary_pass_rate` を指標表へ登録済み（docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md:14）。
- **Day4 完了（Capability／Stage 逆引き）**: 型推論と Core IR の両方で Stage メタデータを逆引きし、辞書呼び出しの監査 (`effect.stage.*`) を復元（compiler/ocaml/src/typeclass_metadata.ml:20、compiler/ocaml/src/type_inference.ml:249、compiler/ocaml/src/core_ir/desugar.ml:70、compiler/ocaml/src/core_ir/iterator_audit.ml:8）。監査イベントの `AuditEnvelope.metadata` に Stage 情報を転写し、`iterator.stage.audit_pass_rate` の必須キー欠落を解消。
- **Day5 完了（辞書ゴールデン／仕様脚注／レポート）**: `test_cli_diagnostics.ml` に辞書解決スナップショットを追加し、新ゴールデン `compiler/ocaml/tests/golden/typeclass_dictionary_resolved.json.golden` を整備。加えて `docs/spec/1-2-types-Inference.md` の脚注と `reports/diagnostic-format-regression.md` の手順へ辞書監査の追記を反映し、Stage 逆引き後のドキュメント残務を解消した。
