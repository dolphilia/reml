# EFFECT-001 効果タグ検出の拡張計画

## 1. 背景と症状
- コア仕様は `mut` / `io` / `ffi` / `panic` / `unsafe` などの効果タグを常時解析し、契約違反を診断すると定義する（docs/spec/1-3-effects-safety.md:11-20）。  
- 現行実装の効果解析は `panic` 呼び出しのみタグ付けしており、`var` 再代入・`unsafe` ブロック・FFI 呼び出しなどを検出できない（compiler/ocaml/src/type_inference.ml:37-144）。  
- `Type_inference_effect.resolve_function_profile` も Capability 一覧の先頭 1 件のみを解決しており（compiler/ocaml/src/type_inference_effect.ml:53-107）、複数 Capability を要求する仕様と乖離している。

## 2. Before / After
### Before
- `Effect_analysis.collect_expr` が `panic` 名称を検査するだけで、`mut` などのタグを追加しない。  
- `Effect_analysis.collect_decl` で `TVarDecl` を再帰的に解析しないため、再代入やミュータブル更新が残余効果に反映されない。  
- Capability 情報はリスト先頭の 1 件（`cap :: _`）を小文字化して返し、残りを破棄。

### After
- 式・宣言の種類ごとにタグを付与するルールを追加し、AST から `mut` / `io` / `ffi` / `unsafe` / `panic` / `syscall` などを検出する。  
- `Type_inference_effect.resolve_function_profile` で Capability 配列全体を保持し、`Effect_analysis.merge_usage_into_profile` に複数タグを渡す。  
- 効果タグの検出結果を `typed_fn_decl.tfn_effect_profile` と `Diagnostic.extensions["effects"]` に反映し、`effect.stage.*` や `effects.residual` が仕様通りに出力される。

#### 追加するタグ検出例
```ocaml
| TAssign (lhs, rhs) -> add_tag (collect_expr tags lhs) "mut" lhs.texpr_span
| TUnsafe body -> add_tag (collect_expr tags body) "unsafe" body.texpr_span
| TCall (fn, args) when is_ffi_call fn -> add_tag tags "ffi" fn.texpr_span
```

## 3. 影響範囲と検証
- **テスト**: 効果タグ検知の単体テストを `compiler/ocaml/tests/effect_analysis_tests.ml`（新設）に追加し、`mut`・`io`・`ffi` ケースを網羅。  
- **診断**: `reports/diagnostic-format-regression.md` に効果タグ付き診断のフィクスチャを追加し、`scripts/validate-diagnostic-json.sh` で `extensions["effects"]`/`audit.metadata` が埋まることを確認。  
- **Stage 整合**: `0-3-audit-and-metrics.md` へ `effect_analysis.missing_tag` を追加し、CI でタグ漏れがゼロであることを監視。
- **型推論**: `compiler/ocaml/tests/type_inference_effect_tests.ml` を追加し、`resolve_function_profile` が複数タグと Stage 条件を維持したまま `typed_fn_decl.tfn_effect_profile` に反映されるかプロパティテストで保証する。

## 4. 実施ステップ
1. **タグ語彙と既存実装の棚卸し（Week31 Day1）**  
   - `docs/spec/1-3-effects-safety.md` §A〜C と `compiler/ocaml/docs/effect-system-design-note.md` を参照し、Phase 2-5 で扱うタグ（`mut`/`io`/`ffi`/`panic`/`unsafe`/`syscall` など）の一覧表を作成して `docs/plans/bootstrap-roadmap/2-5-review-log.md` へ記録する。  
   - `compiler/ocaml/src/type_inference.ml:37-190` の `Effect_analysis` 実装を精査し、式・宣言ごとの検出漏れ（`TAssign`/`TVarDecl`/`TUnsafe`/`TCall` など）を棚卸しする。スパン情報の有無を整理し、タグ追加時に利用する。  
   - `compiler/ocaml/src/effect_profile.ml` と `compiler/ocaml/src/type_inference_effect.ml` の Stage 判定ロジックを確認し、後続工程で更新する関数の入出力・副作用をメモ化する。

2. **AST レベルのタグ検出拡張（Week31 Day2-3）**  
   - `collect_expr`/`collect_decl`/`collect_stmt` に `mut`・`unsafe`・`panic`・`syscall` などの分岐を追加し、`TUnsafe` ブロック内部で検出したタグへ自動的に `unsafe` を付与する。`TAssign`/`TVarDecl` では再代入・ミュータブル更新時に `mut` を追加する。  
   - `TCall` 判定では `compiler/ocaml/src/ffi_contract.ml` の `classify_symbol` や `Effect_profile.normalize_effect_name` を使って `ffi`/`io`/`syscall` 対象を識別し、`docs/spec/3-0-core-library-overview.md` に定義された標準ライブラリ API と突合する。暫定ホワイトリストは `docs/plans/bootstrap-roadmap/2-5-review-log.md` に併記する。  
   - 変更による副作用を `compiler/ocaml/tests/test_type_inference.ml` など既存テストで確認し、必要なら `Effect_analysis` 用の補助関数を切り出す。

3. **効果プロファイルへの統合と Capability 連携（Week31 Day3-4）**  
   - `Type_inference_effect.resolve_function_profile` を改修し、`effect_node.effect_capabilities` の複数要素を保持したまま Stage 突合を行う。`resolved_capability` に加えて `resolved_capabilities`（内部用）を導入し、EFFECT-003 で予定している配列出力へ引き継げる構造を整える。  
   - `Effect_analysis.merge_usage_into_profile` が新タグを `Effect_profile.add_residual` 経由で重複なく取り扱えるか確認し、必要に応じて `normalize_effect_name` を共通化する。  
   - 残余効果が `typed_fn_decl.tfn_effect_profile.effect_set.residual` と `Type_error.effect_residual_leak_error` の双方へ反映されるか挙動を確認し、検証ログを `docs/plans/bootstrap-roadmap/2-5-review-log.md` に残す。

4. **診断・監査出力の更新（Week31 Day4）**  
   - `compiler/ocaml/src/diagnostic.ml` および `compiler/ocaml/src/diagnostic_serialization.ml` を更新し、`Diagnostic.extensions["effects"]` に `effect_profile.effect_set.residual` と `resolved_stage` / `resolved_capabilities` を含める。キー名は `docs/spec/3-6-core-diagnostics-audit.md` §4.2 の `effects.residual` / `effect.stage.*` に合わせる。  
   - `compiler/ocaml/src/audit_envelope.ml` で `metadata["effects.required"]` と `metadata["effects.detected"]` を補完し、`tooling/ci/collect-iterator-audit-metrics.py --require-success` が 1.0 を返すことを CI（`.github/workflows/bootstrap-*.yml`）で確認する。  
   - CLI/LSP のゴールデン JSON（`reports/diagnostic-format-regression.md`）を更新し、新タグが全経路でシリアライズされているか手動検証。変更点は `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` に脚注として追記する。

5. **テストとメトリクスの整備（Week31 Day5〜Week32 Day1）**  
   - `compiler/ocaml/tests/effect_analysis_tests.ml`（新設）で `mut`・`io`・`ffi`・`unsafe` の検出ケースを網羅し、Stage 要件失敗時の診断をスナップショット化する。既存の `test_type_inference.ml` へ残余効果が存在するサンプルを追加する。  
   - `scripts/validate-diagnostic-json.sh` を実行し、効果タグ関連フィールドが欠落しないことを確認。必要に応じて `tooling/ci/collect-iterator-audit-metrics.py` の複数タグ対応を拡張する。  
   - 新メトリクス（`effect_analysis.missing_tag`、`effect_analysis.residual_leak_pass_rate` など）を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に追加し、`.github/workflows/bootstrap-linux.yml` などの CI で収集されるよう設定する。

## 5. フォローアップ
- Capability 配列を `AuditEnvelope.metadata["required_capabilities"]` にシリアライズする仕様脚注を Chapter 1/3 に追加する。  
- Phase 2-7 `execution-config` 側で `RunConfig.extensions["effects"]` に `max_handler_depth` 等を設定した場合、タグ検出結果を連携するハンドシェイクを設計する。  
- `docs/spec/1-3-effects-safety.md` にタグ検出アルゴリズムの抜粋を掲載し、Reml 実装移植時の参照資料とする。
- `docs/spec/0-2-glossary.md` と `docs/notes/core-library-outline.md` にタグ語彙の定義と履歴を追記し、Phase 3 でのセルフホスト検証に備えた参照ポイントを整備する。
- **タイミング**: TYPE-001 など後続タスクの前提となるため、Phase 2-5 の前半で実装を完了し、再帰的な効果解析を 2-5 後半のレビューに間に合わせる。

## 6. 残課題
- `io` 判定の対象 API（`Core.IO`, `Core.Time` など）をどの階層で列挙するかをライブラリチームと合意する必要がある。  
- FFI 呼び出しのタグ付けで `extern "C"` 以外のブリッジ（Plugin, Capability Bridge）をどのように扱うか、Phase 2-7 で検討したい。
