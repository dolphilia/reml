# TYPE-001 値制限の再導入計画

## 1. 背景と症状
- 仕様では「一般化は確定的な値のみ」と定義されており、副作用を持つ束縛は単相に制限する（docs/spec/1-2-types-Inference.md:136）。  
- 現行 OCaml 実装では `let` / `var` いずれも効果に関係なく `generalize` を適用しており（compiler/ocaml/src/type_inference.ml:2172-2235, compiler/ocaml/src/type_inference.ml:2236-2283）、`var` 再代入や `ffi` 呼び出しを含む束縛も多相化される。  
- 効果解析が `panic` しか検出していないため（TYPE-001 と連動する EFFECT-001）、残余効果に基づく制限が機能せず、`@pure` 契約や Stage 要件の検証が破綻する可能性がある。

## 2. Before / After
### Before
- `infer_decl` が束縛種別に関わらず `generalize` を呼び出し、`scheme.constraints` が空であれば辞書解決なしで環境へ登録する。
- 効果情報は `typed_fn_decl.tfn_effect_profile` にのみ保持され、束縛の型スキームには反映されない。
- `0-3-audit-and-metrics.md` の値制限関連メトリクスは未計測。

### After
- 束縛右辺が「確定的な値」かを判定する `is_generalizable`（純粋式 + 効果集合が空/安全タグのみ）を導入し、`let` では条件付き一般化、`var` では常に単相化する。
- `Effect_analysis.collect_from_fn_body` の結果を束縛評価へ渡し、`mut` / `io` / `ffi` / `unsafe` / `panic` のタグを持つ場合は単相に固定する。
- 一般化可否を `0-3-audit-and-metrics.md` の診断指標へ記録し、値制限違反が排除されたことを CI で確認する。
- `parser_run_config` 経由で Typer 設定へ値制限スイッチを渡し、移行期間中は `RunConfig.extensions["effects"]`（仮称）で旧挙動を再現できるようにする。

#### 擬似コード案
```ocaml
let is_generalizable ~effects expr_ty =
  Effect_tags.is_pure effects
    && Expr_utils.is_value expr_ty
```
`Effect_tags.is_pure` は EFFECT-001 の修正で導入するタグ集合判定を再利用する想定。

## 3. 影響範囲と検証
- **テスト**: 既存の型推論テストへ値制限ケースを追加し、`mut` / `ffi` / `unsafe` を含む束縛が単相に推論されることを確認。  
- **メトリクス**: `0-3-audit-and-metrics.md` に `type_inference.value_restriction_violation` を新設し、CI で 0 件を保証。  
- **互換性**: 多相化に依存していたサンプル（存在する場合）は `let` への変更や効果抑制で復元する。
- **監査ログ**: `collect-iterator-audit-metrics.py` に値制限違反検知イベントの集計を追加し、診断とメトリクスが同時に更新されるようにする。

## 4. 実施ステップ（Week32〜Week33 想定）
- **Step0 — 現状棚卸しと再現ケース整理（Week32 Day1）**  
  - `compiler/ocaml/src/type_inference.ml:596-663` の `generalize` 実装と `infer_decl`（compiler/ocaml/src/type_inference.ml:2236, compiler/ocaml/src/type_inference.ml:2284）で `let`／`var` が常時一般化されている経路を洗い出し、`docs/plans/bootstrap-roadmap/2-5-review-log.md` に再現ログを追加する。  
  - `compiler/ocaml/tests/test_type_inference.ml`・`compiler/ocaml/tests/test_cli_diagnostics.ml` の多相化依存ケースを抽出し、現行出力と仕様差分を比較。再現用スニペットを `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` の差分リストへ脚注として共有する。  
  - `docs/spec/1-2-types-Inference.md:120-188` と `docs/spec/1-3-effects-safety.md` の「確定的な値」定義をチェックリストに落とし込み、`docs/notes/type-inference-roadmap.md` で値制限復元の前提を整理する。
- **Step1 — 値制限判定ユーティリティ設計（Week32 Day2）**  
  - `Effect_analysis.collect_expr`（compiler/ocaml/src/type_inference.ml:240-308）と `Typed_ast` ノードの構成を調査し、純粋式・値式に分類できるパターンを列挙。`Typed_ast` に補助関数が無ければ値判定用ヘルパを追加する設計案をまとめる。  
  - `docs/spec/1-5-formal-grammar-bnf.md` を参照し、λ式・構造体/列挙リテラル・定数畳み込みなど一般化対象となる式の網羅表を作成。  
  - `Effect_analysis` が `mut`/`io`/`ffi`/`unsafe`/`panic` 以外に Stage 依存タグを保持できるか確認し、複数 Capability（`Type_inference_effect.resolve_function_profile`）との整合をレビューする。
- **Step2 — Typer への導入と RunConfig 連携（Week32 Day3-4）**  
  - `infer_decl` の `let` / `var` 分岐で `should_generalize`（新設）を呼び出し、`mut` や残余効果タグが付与された束縛は単相スキーム（`scheme_to_constrained (mono_scheme ty)`）へ強制する。  
  - `Type_inference.make_config` に値制限フラグを追加し、`compiler/ocaml/src/main.ml:600-780` と `parser_run_config.ml` 経由で CLI の `RunConfig` から Typer 設定へ伝播させる。`RunConfig.extensions["effects"]`（暫定キー）に `value_restriction = strict|legacy` を格納する案を検討し、`docs/plans/bootstrap-roadmap/2-5-review-log.md` に API モデルを記載する。  
  - `Effect_analysis.collect_expr` を束縛評価でも再利用できるよう、`infer_expr` の戻り値（`typed_expr`）からタグを取得するフックを実装し、`collect-iterator-audit-metrics.py` の Stage メタデータ（複数 Capability）と齟齬がないか確認する。
- **Step3 — テスト・診断・メトリクス整備（Week32 Day4-5）**  
  - `compiler/ocaml/tests/test_type_inference.ml` に `let` 多相／`var` 単相／`ffi` 呼び出しを組み合わせたケースを追加し、`compiler/ocaml/tests/golden/type_inference_*` 系フィクスチャを更新。  
  - `tooling/ci/collect-iterator-audit-metrics.py` に `type_inference.value_restriction_violation` を追加し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` へ新指標と CI ゲート条件（常に 0.0）を追記。  
  - `scripts/validate-diagnostic-json.sh` に値制限違反診断の検証を組み込み、`reports/diagnostic-format-regression.md` と `docs/plans/bootstrap-roadmap/2-5-review-log.md`（Day4 エントリ）へ結果を記録する。
- **Step4 — ドキュメント整備とフォローアップ連携（Week33 Day1）**  
  - `docs/spec/1-2-types-Inference.md` §C.3 と `docs/spec/1-3-effects-safety.md` に OCaml 実装の判定手順と RunConfig 連携を脚注で補足し、`docs/plans/bootstrap-roadmap/2-5-proposals/README.md` の TYPE-001 項を更新する。  
  - `docs/notes/type-inference-roadmap.md` に Stage・Capability 依存の値制限方針と Phase 2-7 への残課題を追記。  
  - `docs/plans/bootstrap-roadmap/2-5-review-log.md` に最終レビュー記録を追加し、Phase 2-7 `execution-config` / `effect-metrics` サブチームへ移管する TODO を登録する。

## 5. フォローアップ
- EFFECT-001 で追加する効果タグ検出ロジックと同時レビューとし、タグ不足による誤判定を避ける。  
- Phase 2-7 `execution-config` タスクへ「値制限メトリクス収集」の連携を追加し、`RunConfig` 差分や CLI 表示と同期する。  
- Phase 3 で予定されている Reml 実装移植時に、同じ値制限ロジックを導入するため `docs/notes/core-parser-migration.md`（予定）にも計画の要点を共有する。
- `docs/notes/type-inference-roadmap.md` に値制限再導入の段階計画と既知の互換性リスクを記録し、PoC から正式導入までのレビュー履歴を残す。
- **タイミング**: EFFECT-001 のタグ拡張完了直後に Phase 2-5 中盤で実装へ着手し、Phase 2-5 終盤までに値制限違反ゼロを確認する。

## 6. 残課題
- 値制限判定に利用する「純粋式」判定の粒度（例: `const fn` 呼び出しを許容するか）について、Phase 2-1 型クラス戦略チームと調整が必要。  
- 効果タグ解析の段階的適用（`-Zalgebraic-effects` 未使用時でも強制するか）を決定したい。
