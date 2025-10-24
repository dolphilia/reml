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

## 4. フォローアップ
- Capability 配列を `AuditEnvelope.metadata["required_capabilities"]` にシリアライズする仕様脚注を Chapter 1/3 に追加する。  
- Phase 2-7 `execution-config` 側で `RunConfig.extensions["effects"]` に `max_handler_depth` 等を設定した場合、タグ検出結果を連携するハンドシェイクを設計する。  
- `docs/spec/1-3-effects-safety.md` にタグ検出アルゴリズムの抜粋を掲載し、Reml 実装移植時の参照資料とする。
- `docs/spec/0-2-glossary.md` と `docs/notes/core-library-outline.md` にタグ語彙の定義と履歴を追記し、Phase 3 でのセルフホスト検証に備えた参照ポイントを整備する。
- **タイミング**: TYPE-001 など後続タスクの前提となるため、Phase 2-5 の前半で実装を完了し、再帰的な効果解析を 2-5 後半のレビューに間に合わせる。

## 残課題
- `io` 判定の対象 API（`Core.IO`, `Core.Time` など）をどの階層で列挙するかをライブラリチームと合意する必要がある。  
- FFI 呼び出しのタグ付けで `extern "C"` 以外のブリッジ（Plugin, Capability Bridge）をどのように扱うか、Phase 2-7 で検討したい。
