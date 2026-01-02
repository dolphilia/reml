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
  - `docs/spec/1-2-types-Inference.md:120-188` と `docs/spec/1-3-effects-safety.md` の「確定的な値」定義をチェックリストに落とし込み、`docs/notes/types/type-inference-roadmap.md` で値制限復元の前提を整理する。
  - **2025-10-31 更新**: 上記棚卸しを完了し、`docs/plans/bootstrap-roadmap/2-5-review-log.md` に「TYPE-001 Day1 値制限棚卸し」を追加。`dune exec remlc -- tmp/value_restriction_var.reml --emit-tast` で `var` 束縛が多相化される再現ログを取得し、差分リストに脚注 `[^type001-step0-review]` を登録。`docs/notes/types/type-inference-roadmap.md` を新設して確定値・効果タグのチェックリストを共有済み。
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
- **Step4 — ドキュメント整備とフォローアップ連携（Week33 Day1／2025-11-08 完了）**  
  - `docs/spec/1-2-types-Inference.md` §C.3 と `docs/spec/1-3-effects-safety.md` に OCaml 実装の判定手順と RunConfig 連携を脚注で補足し、`docs/plans/bootstrap-roadmap/2-5-proposals/README.md` の TYPE-001 項を更新する。  
  - `docs/notes/types/type-inference-roadmap.md` に Stage・Capability 依存の値制限方針と Phase 2-7 への残課題を追記。  
  - `docs/plans/bootstrap-roadmap/2-5-review-log.md` に最終レビュー記録を追加し、Phase 2-7 `execution-config` / `effect-metrics` サブチームへ移管する TODO を登録する。

### Step1 実施記録（2025-11-01）

#### 1. Typed_ast ノード分類と値判定カバレッジ

| 判定カテゴリ | 対象 `typed_expr_kind` | 判定方針 | 備考 |
| --- | --- | --- | --- |
| 即値 (`ImmediateValue`) | `TLiteral`, `TLambda`, `TVar`, `TModulePath` | `Typed_ast.Value_form.is_immediate`（新設予定）で即値判定。識別子参照は束縛済みスキームを照会しつつ一般化候補として扱う。 | 仕様で列挙される「確定的な値」（リテラル／ラムダ／構造リテラル）が該当[^type001-spec-value]。`docs/spec/1-5-formal-grammar-bnf.md` で示される Primary 群のうち演算・制御を含まない要素に一致[^type001-bnf]. |
| 構造値 (`AggregateValue`) | `TUnary`, `TBinary`, `TTupleAccess`, `TFieldAccess`, `TIndex`, `TPropagate`, `TPipe` | `Value_form.is_aggregate` で子ノードを再帰判定。全子が `ImmediateValue` または `AggregateValue` の場合のみ値として扱う。 | 算術／添字／パイプは純粋演算であれば即値に畳み込めるが、効果タグが付与された場合は下位分類で除外。 |
| 制御式 (`ControlFlowValue`) | `TIf`, `TMatch`, `TBlock` | ガード・分岐・末尾式を再帰的にチェックし、いずれかで `Effectful` が発生した場合は単相固定。`TBlock` は宣言文が全て `let` か単発式であることを確認。 | `if`/`match` はいずれの分岐でも値に収束することが条件。`Block` は OCaml `let... in` の糖衣に対応するため最終式のみ値判定を参照。 |
| 副作用が確定する式 (`EffectfulSyntax`) | `TCall`, `TAssign`, `TWhile`, `TFor`, `TLoop`, `TReturn`, `TDefer`, `TUnsafe`, `TContinue` | 構文レベルで値とはみなさない。`Effect_analysis.collect_expr` の結果を `effect_summary` に取り込み、その場で一般化を拒否。 | 仕様上 `mut` / `io` / `ffi` / `panic` / `unsafe` を含む場合は一般化不可[^type001-effects]. `TCall` は関数呼び出し固有のタグ付け（`ffi`/`io` 等）で判定。 |
| 未確定（再帰判定依存） | 上記以外 | 親ノードへ分類を委譲。`Effect_analysis` が検出したタグを優先し、タグが空かつ子ノードが値の場合に限り一般化候補とする。 | `Effect_analysis.collect_expr` と連携して「副作用なし」かつ「値形状」の両立を確認。 |

表に合わせて `Typed_ast` 内へ値形状判定を集約するヘルパ（`Value_form.is_immediate` / `Value_form.is_aggregate` / `Value_form.is_control_flow`）を追加する。これにより `infer_decl` 側では式ごとの網羅的なパターンマッチを避け、テストで値分類を直接検証できる。

#### 2. 判定ユーティリティの API 案

- `type effect_evidence = { tag : string; span : Ast.span; capability : string option; stage : Effect_profile.stage_id option }` を導入し、`mut` などのタグと Capability/Stage 付加情報を一体で扱う。  
- `type decision = { status : value_status; value_kind : value_kind option; effects : effect_evidence list; syntax_reasons : syntax_reason list }` を定義。`value_status = Generalizable | Monomorphic`、`value_kind = Immediate | Aggregate | Alias`、`syntax_reason` は `NonValueNode of typed_expr_kind` などで構成する。  
- `Value_restriction.evaluate : Type_inference_effect.runtime_stage -> typed_expr -> decision` を追加し、Step2 で `infer_decl` から呼び出す。判定結果は後続の診断生成 (`effects.contract.value_restriction`) とメトリクス更新に共用する。  
- `decision.effects` を `Effect_analysis.collect_expr` の結果と統合し、`Type_inference_effect.resolve_function_profile` が返す Capability/Stage 情報で補強して RunConfig との整合を確認する。

#### 3. Stage / Capability 連携の整理

- `Effect_analysis.add_call_effect_tags` で識別している `core.io.*` などの接頭辞に対応する Capability 名をマップ化し、`effect_evidence.capability` として格納する。  
- `Type_inference_effect.resolve_function_profile` の `resolved_capabilities : capability_resolution list` を `evaluate` に渡し、タグに紐づく Capability が RunConfig で禁止されている場合は常に `Monomorphic` を返す。  
- Stage 情報は `capability_resolution.capability_stage` と `profile.resolved_stage` の双方を反映し、`stage_requirement_satisfied` を満たさない場合に `syntax_reasons` へ `StageMismatch` を記録。  
- 以上の整理を踏まえ、同日に更新したレビュー記録[^type001-step1-log] と `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` の差分項目へリンクを追加し、Phase 2-5 の RunConfig/lex シム計画全体で矛盾が無いことを確認済み。

### Step2 実施記録（2025-11-03）

#### 1. Typer への導入方針

- `Value_restriction.evaluate` が `typed_expr` と `Type_inference_effect.runtime_stage` を受け取り、`Generalizable` / `Monomorphic` と判定根拠（値形状・効果タグ・Stage 差異）を返す API を確定した。判定結果は `effects.contract.value_restriction` 診断・メトリクスの両方で共有する。  
- `Type_inference.config` に `value_restriction_mode : (\`Strict\` / \`Legacy\`)` を追加し、`let` 束縛は `evaluate` の結果に従い、`var` 束縛は常に `Monomorphic` を優先する流れを定義した。`scheme_to_constrained (mono_scheme ty)` を利用して単相スキームへ即時変換する設計も整理済み[^type001-infer-decl]。  
- `Typed_ast.Value_form` で値形状（即値・集約値・制御式）を一元化し、`evaluate` が `infer_decl` から受け取る `typed_expr` を再帰解析する際にコード重複が生じないようヘルパ構成を決めた。`is_immediate` / `is_aggregate` / `is_control_flow` を公開 API とし、Step3 のテスト追加時にも再利用できる。

#### 2. RunConfig 伝播と CLI ブリッジ

- `parser_run_config.ml` の `Effects.t` に `value_restriction_mode : string option` を追加し、`set_value_restriction` / `decode_value_restriction` で `strict` / `legacy` を正規化して保持する。未指定時は `strict` を既定とし、Legacy モードは CLI の `--legacy-value-restriction`（仮称）経由で限定的に利用する前提を明示[^type001-runconfig-effects]。  
- `Type_inference.make_config` は RunConfig 拡張の値を受け取り、`value_restriction_mode` を `config` レコードへ統合する。`compiler/ocaml/src/main.ml` では Parser RunConfig の組み立て後に Effects 拡張を読み取り、Typer 設定へ橋渡しする手順を整理した[^type001-main-bridge]。  
- 既存テストは Legacy 振る舞いに依存している箇所があるため、Step3 でフィクスチャを全面更新するまでの暫定措置として `legacy` モードを保持し、CI メトリクスでは `Strict` 経路のみを対象にカウントする方針とした。

#### 3. 効果証跡と Stage 整合の統合

- `Value_restriction` から `effect_evidence = { tag; capability; stage; span } list` を返し、`Effect_analysis.collect_expr` で抽出したタグに Capability 名（`collect_from_fn_body` で得られる解決情報）と Stage 判定結果を付与する。  
- 判定過程で `Type_inference_effect.resolve_function_profile` が返す `resolved_capabilities` を参照し、RunConfig で未許可の Capability が含まれていた場合は自動的に `Monomorphic` へフォールバックする仕様を固めた。  
- `collect-iterator-audit-metrics.py` と同じフィールドレイアウト（`effect.stage.required` / `effect.stage.actual` / `effect.capability` 等）に揃えた JSON 断片を生成する案をまとめ、Step3 のメトリクス整備で二重実装を避ける方針を共有した[^type001-metrics-script]。

#### 4. Step3 以降への引き継ぎ

- `compiler/ocaml/tests/test_type_inference.ml` に Step2 で定義した `Value_form` 判定テーブルを用いたゴールデンを追加し、`strict` / `legacy` の両モードで期待値を収集する。  
- `Value_restriction.effect_evidence` を JSON シリアライザへ接続し、`type_inference.value_restriction_violation`（0 固定）と `type_inference.value_restriction_legacy_usage`（Legacy 経路発生数）を `collect-iterator-audit-metrics.py` の新セクションとして登録する。  
- RunConfig CLI オプションを文書化し、`docs/spec/2-1-parser-type.md` / `docs/spec/2-6-execution-strategy.md` に値制限切替の脚注を追加する作業を Step4 文書整備と合わせて実施する。

### Step3 実施記録（2025-11-05）

#### 1. テスト設計とフィクスチャ整備

- `compiler/ocaml/tests/test_type_inference.ml` の末尾へ `(* TODO(TYPE-001/Step3) ... *)` 形式のテスト雛形コメントを追加し、以下 3 ケースを洗い出して実装方針と期待値を明文化した:  
  1. `let` + 純粋ラムダ（`strict` モードで量化変数 ≥1）  
  2. `var` + 純粋ラムダ（常に単相）  
  3. `let` + `unsafe`（`mut`/`ffi` タグを含む式、単相固定）  
  コメント内で `Value_form` 判定ヘルパと `value_restriction_mode` 切替の利用方法を整理し、`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` の差分リストへ再現スニペットを脚注として登録した[^type001-step3-cases]。  
- テンプレートゴールデン `compiler/ocaml/tests/golden/type_inference_value_restriction.strict.json.golden` / `...legacy.json.golden` を追加し、`mode`, `status`, `evidence[]`（`tag` / `capability` / `stage.required` / `stage.actual`）を保持する JSON 雛形を用意。生成・更新手順は `reports/diagnostic-format-regression.md` §1 に追記した[^type001-step3-golden]。

#### 2. CI メトリクスと RunConfig 連携

- `tooling/ci/collect-iterator-audit-metrics.py` に `type_inference.value_restriction_violation` 指標を追加し、`--require-success` 時に「Strict 経路で違反 0」をゲートに設定。Legacy 経路の利用回数は `type_inference.value_restriction_legacy_usage` として関連メトリクスへ集約し、`0-3-audit-and-metrics.md` §0.3.1 に登録した[^type001-step3-metrics]。  
- `RunConfig.effects.value_restriction_mode` の CLI 取り回しを `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` の差分表へ反映し、`strict/legacy` の期待挙動と監査ログ整合をチェックリスト化した。

#### 3. 診断検証とレビュー手順

- `scripts/validate-diagnostic-json.sh` に `type_inference.value_restriction_violation` 診断の必須フィールド検証（`extensions.value_restriction.mode/status/evidence`, `audit_metadata.value_restriction.*`, `audit.metadata.value_restriction.*`）を追加。欠落時は CI ログで具体的なキーを出力するようにした[^type001-step3-validator]。  
- `reports/diagnostic-format-regression.md` のレビュー手順に「値制限違反ダンプの確認」「`collect-iterator-audit-metrics.py --require-success` で 0 件を確認」を加え、Step4 の仕様更新作業と連動させる TODO を登録した。

#### 4. 次工程への共有事項

- `docs/plans/bootstrap-roadmap/2-5-review-log.md` に Day4 エントリを追記し、テスト雛形・メトリクス実装・診断バリデータ更新の確認手順を記録した。  
- `docs/spec/1-2-types-Inference.md` / `docs/spec/1-3-effects-safety.md` へ追加する脚注案（値制限と効果タグの橋渡し）を Step4 文書整備項に紐付け、`docs/notes/types/type-inference-roadmap.md` へ残課題（Legacy モード削減計画、Stage 差分監査）を転記した。

### Step4 実施記録（2025-11-08）

#### 1. 仕様更新と脚注整理
- `docs/spec/1-2-types-Inference.md` §C.3 に値制限の判定根拠・`Value_restriction.evaluate`・`RunConfig.extensions["effects"].value_restriction_mode` の関係をまとめた実装メモを追加し、Strict/Legacy トグルの暫定運用を明記した。【S:docs/spec/1-2-types-Inference.md†L142-L161】
- `docs/spec/1-3-effects-safety.md` §B に値制限と効果タグ・Capability/Stage 判定の連携手順を追記し、`effects.contract.value_restriction` 診断で監査キーを共有することを明文化した。【S:docs/spec/1-3-effects-safety.md†L70-L99】
- `docs/spec/2-1-parser-type.md` の RunConfig セクションへ `extensions["effects"]` の予約キーと CLI スイッチ（`--value-restriction={strict|legacy}`／`--legacy-value-restriction` の互換経路）を掲載し、`docs/spec/2-6-execution-strategy.md` ではパーサーと Typer の橋渡し要件を脚注として整理した。【S:docs/spec/2-1-parser-type.md†L118-L166】【S:docs/spec/2-6-execution-strategy.md†L38-L116】

#### 2. ノートとレビュー記録
- `docs/notes/types/type-inference-roadmap.md` に Stage/Capability 依存ルールと Phase 2-7 で縮退予定の Legacy モード整理を追記し、TODO リストを更新した。【N:docs/notes/types/type-inference-roadmap.md†L33-L74】
- `docs/plans/bootstrap-roadmap/2-5-review-log.md` に Step4 エントリを追加し、仕様反映・CLI スイッチ・Phase 2-7 への移管タスクを記録。`execution-config`（RunConfig CLI）と `effect-metrics`（CI 指標）へそれぞれフォローアップを割り当てた。【R:docs/plans/bootstrap-roadmap/2-5-review-log.md†L22-L38】

#### 3. カタログ・差分計画の更新
- `docs/plans/bootstrap-roadmap/2-5-proposals/README.md` の TYPE-001 節を更新し、値制限の脚注追加と RunConfig CLI 整備が完了したことを明示した。【C:docs/plans/bootstrap-roadmap/2-5-proposals/README.md†L34-L54】
- `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` の差分リストに Step4 実施結果（仕様脚注反映と CLI スイッチ連携）を追記し、残課題を Phase 2-7 へ移送した。【D:docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md†L94-L123】

#### 4. フォローアップと引き継ぎ
- Phase 2-7 `execution-config` へ RunConfig CLI 周辺の統合テスト（`--value-restriction` トグルと Legacy モード縮退の告知）を依頼。
- Phase 2-7 `effect-metrics` へ `type_inference.value_restriction_violation` 指標の運用監視と Legacy モード検出アラート（0 件を逸脱した場合の通知）を引き継ぎ、Phase 3 で Reml 実装へ適用するロードマップを共有。

## 5. フォローアップ
- EFFECT-001 で追加する効果タグ検出ロジックと同時レビューとし、タグ不足による誤判定を避ける。  
- Phase 2-7 `execution-config` タスクへ「値制限メトリクス収集」の連携を追加し、`RunConfig` 差分や CLI 表示と同期する。  
- Phase 3 で予定されている Reml 実装移植時に、同じ値制限ロジックを導入するため `docs/notes/parser/core-parser-migration.md`（予定）にも計画の要点を共有する。
- `docs/notes/types/type-inference-roadmap.md` に値制限再導入の段階計画と既知の互換性リスクを記録し、PoC から正式導入までのレビュー履歴を残す。
- **タイミング**: EFFECT-001 のタグ拡張完了直後に Phase 2-5 中盤で実装へ着手し、Phase 2-5 終盤までに値制限違反ゼロを確認する。

## 6. 残課題
- 値制限判定に利用する「純粋式」判定の粒度（例: `const fn` 呼び出しを許容するか）について、Phase 2-1 型クラス戦略チームと調整が必要。  
- 効果タグ解析の段階的適用（`-Zalgebraic-effects` 未使用時でも強制するか）を決定したい。

[^type001-step0-review]: `docs/plans/bootstrap-roadmap/2-5-review-log.md` の「TYPE-001 Day1 値制限棚卸し（2025-10-31）」を参照。
[^type001-spec-value]: `docs/spec/1-2-types-Inference.md` §C.3 および `docs/spec/1-3-effects-safety.md` §B で定義される「確定的な値」と純粋性の条件。
[^type001-bnf]: `docs/spec/1-5-formal-grammar-bnf.md` §4 Primary の構成要素（Literal / Lambda / RecordLiteral / TupleLiteral 等）。
[^type001-effects]: `docs/spec/1-3-effects-safety.md` §A 表（`mut` / `io` / `ffi` / `panic` / `unsafe`）と §A.1 補助タグ定義。
[^type001-step1-log]: `docs/plans/bootstrap-roadmap/2-5-review-log.md` の「TYPE-001 Step1 値制限判定ユーティリティ設計（2025-11-01）」を参照。
[^type001-infer-decl]: `compiler/ocaml/src/type_inference.ml:2353` 付近。`LetDecl` / `VarDecl` で `scheme_to_constrained (mono_scheme ty)` を用いた単相化処理を挿入する計画を明記。
[^type001-runconfig-effects]: `compiler/ocaml/src/parser_run_config.ml:319-428`。Effects 拡張に値制限モードを保持するアクセサを追加する。
[^type001-main-bridge]: `compiler/ocaml/src/main.ml:600-642`。Parser RunConfig を Typer 設定へ橋渡しする経路を確認。
[^type001-metrics-script]: `tooling/ci/collect-iterator-audit-metrics.py:1-154`。効果 Stage 監査メトリクスの必須フィールド一覧と整合させる。
[^type001-step3-cases]: `compiler/ocaml/tests/test_type_inference.ml` 末尾。Step3 用 TODO コメントに試験ケースの想定と `value_restriction_mode` 切替の注意点を追記。
[^type001-step3-golden]: `compiler/ocaml/tests/golden/type_inference_value_restriction.strict.json.golden` / `compiler/ocaml/tests/golden/type_inference_value_restriction.legacy.json.golden`。Strict/Legacy それぞれの診断出力テンプレート。
[^type001-step3-metrics]: `tooling/ci/collect-iterator-audit-metrics.py`と `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` §0.3.1。値制限制約の監視指標と CI ゲート条件を登録。
[^type001-step3-validator]: `scripts/validate-diagnostic-json.sh`。値制限違反診断向け必須フィールド検証ロジックを追加。
