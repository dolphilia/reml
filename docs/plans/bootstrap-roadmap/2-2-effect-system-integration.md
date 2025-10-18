# 2.2 効果システム統合計画

## 目的
- [1-3-effects-safety.md](../../spec/1-3-effects-safety.md) と [3-8-core-runtime-capability.md](../../spec/3-8-core-runtime-capability.md) に定義される効果タグと Stage 要件を Phase 2 で OCaml 実装へ統合する。
- Parser/Typer/Lint/Runtime が同一の Stage 判定ロジックを共有し、セルフホスト前の整合を確保する。

## スコープ
- **含む**: AST/TAST への `effect` 注釈保持、Stage 要件 (`Exact`, `AtLeast`) の検証、RuntimeCapability との照合、CI テスト。
- **含まない**: ランタイム Stage の動的変更、プラグインによる Stage 拡張。これらは Phase 3 以降。
- **前提**: Parser が効果構文を取り込み、Typer が型クラス拡張と競合しない設計であること。

## 作業ディレクトリ
- `compiler/ocaml/src/parser`, `compiler/ocaml/src/typer` : 効果タグ解析と型検証
- `compiler/ocaml/src/ir`, `compiler/ocaml/src/codegen` : 効果タグの IR 伝播と Capability チェック
- `runtime/native` : Stage/Capability 監査の実装
- `tooling/ci` : 効果タグと Stage 整合性を検証する CI ジョブ
- `docs/spec/1-3-effects-safety.md`, `docs/spec/3-8-core-runtime-capability.md`, `docs/spec/3-6-core-diagnostics-audit.md` : 仕様更新の対象

## 着手前チェックリスト（Phase 2-1 からの引き継ぎ）
- [x] `compiler/ocaml/scripts/benchmark_typeclass.sh --static-only` を実行し、`benchmark_results/static_comparison.json` を生成済み（辞書渡し vs モノモルフィゼーションの静的比較を `0-3-audit-and-metrics.md` に転記）。※現フェーズでは IR/BC 未生成のため値は 0、Phase 3 で再計測予定。
- [x] `tooling/ci/collect-iterator-audit-metrics.py` → `tooling/ci/sync-iterator-audit.sh` のワークフローで `verify_llvm_ir.sh` ログと監査メトリクスを突合し、`iterator.stage.audit_pass_rate` を `0-3-audit-and-metrics.md` に記録済み。
- [x] `docs/spec/1-2-types-Inference.md`, `docs/spec/3-1-core-prelude-iteration.md`, `docs/spec/3-8-core-runtime-capability.md` に型クラス辞書と Stage 監査の連携ノートを追記し、診断キー (`effect.stage.iterator.*`) の定義が参照可能である。

## 進行状況（2025-10-24 時点）

| 作業ブレークダウン | ステータス | 完了内容 | 次アクション |
| --- | --- | --- | --- |
| 1. 効果システム設計と仕様整理 | 🚧 進行中 | `effect_profile` モデルと StageRequirement 評価ルールを設計ノートに反映し、`RuntimeCapabilityResolver` の優先度と Stage トレース形式を `reports/runtime-capabilities-validation.json` で検証 | Capability/Stage 表を `docs/spec/1-3-effects-safety.md`・`3-8-core-runtime-capability.md` に反映し、Windows override と監査キーの整合を確定 |
| 2. Parser への効果注釈統合 | ✅ 完了 | `effect_profile_node`、属性解析、効果関連の Parser/CLI ゴールデンを更新済み | 行多相（Phase 3）と属性バリデーションを Typer 側に接続 |
| 3. Typer 統合と効果解析 | ✅ 第3段階 | `type_inference_effect.ml` が Runtime Stage を継承して `stage_trace` を構築し、効果属性から Capability ID を抽出して `effect_profile.resolved_capability` / stage トレースへ反映、`effects.syntax.invalid_attribute`・`effects.contract.residual_leak`・`effects.contract.stage_mismatch` を CLI / 監査の両経路で固定化 | Stage 集計メトリクス (`iterator.stage.audit_pass_rate`) への自動登録と Core IR メタデータ突合 |
| 4. RuntimeCapability チェック | ✅ 第3段階 | `RuntimeCapabilityResolver.resolve` を CLI/環境変数/JSON の三系統で統合し、`main.ml` から `runtime_stage_event` を出力。Windows override (`tooling/runtime/capabilities/default.json`) を含む Stage トレースを `reports/` に保存 | Windows / 追加プラットフォーム向け Capability 拡張と Stage override テスト、CI サマリー（`iterator-stage-summary.md`）のレビュー手順確立 |
| 5. 診断システム強化 | ✅ 第1段階 | `Diagnostic.extensions.effect.*` と `AuditEnvelope.metadata.stage_trace` が Typer/Runtime の経路を共有し、`compiler/ocaml/tests/golden/diagnostics/effects/*.json.golden` と `tests/golden/audit/effects-*.golden` を更新 | 残余効果サマリの CI 検証と `effect.residual.*` キーの監査レポート化、Stage 差分の自動フェイルゲート |
| 6. テスト整備 | 🚧 進行中 | `test_effect_residual.ml` で辞書／モノモルフィゼーションの一致を検証し、`scripts/validate-runtime-capabilities.sh` → `reports/runtime-capabilities-validation.json`、`tooling/ci/sync-iterator-audit.sh` → `reports/iterator-stage-summary.md` を取得 | Windows 向け Stage override テストと効果診断ゴールデンの定期再生成、CI 出力サマリーの自動検証ルーチン追加 |
| 7. ドキュメント更新と仕様同期 | 🚧 進行中 | 設計ノート・本計画書・`0-3-audit-and-metrics.md` §0.3.7 に中間結果を反映 | 仕様 (`1-3`, `3-6`, `3-8`) とメトリクス表を同期し、残タスクの索引を整理 |
| 8. 統合検証と Phase 3 準備 | ⏳ 未着手 | — | Typer/Runtime 完了後の統合シナリオと Phase 3 引き継ぎ資料を設計 |

### 次のステップ（短期フォーカス）
- ✅ Core IR メタデータと RuntimeCapability JSON の突合を `main.ml` / `AuditEnvelope` / `tooling/ci/sync-iterator-audit.sh` で接続済み。`reports/iterator-stage-summary.md` は 2025-10-18 時点で pass_rate 1.0（欠落 0）を確認。
- ✅ GitHub Actions（bootstrap-linux / bootstrap-macos）へ `tooling/ci/sync-iterator-audit.sh` を常設化し、`iterator.stage.audit_pass_rate` が 1.0 未満の場合に即失敗させるゲートを有効化。
- Windows / 追加ターゲット用 Capability JSON の差分を検証し、`tooling/runtime/capabilities/*.json` 更新手順と `scripts/validate-runtime-capabilities.sh` の運用を `0-3-audit-and-metrics.md` に追記する。

> **進捗アップデート（2025-10-24 更新）**  
> - `main.ml` の `runtime_stage_event` へ `typer` / `runtime` ステップを追加し、RuntimeCapabilityResolver → AuditEnvelope → CI 指標の Stage トレースが常に揃うようにした。`compiler/ocaml/tests/golden/audit/effects-residual.jsonl.golden` を同期済み。  
> - `tooling/ci/sync-iterator-audit.sh --metrics tooling/ci/iterator-audit-metrics.json --verify-log tooling/ci/llvm-verify.log --audit compiler/ocaml/tests/golden/audit/effects-residual.jsonl.golden` を実行し、`reports/iterator-stage-summary.md` で pass_rate 1.0 / 欠落 0 を確認（exit code 0）。  
> - 次は Windows / ターゲット別 Capability override の検証と CI ジョブへの常設化を進め、`0-3-audit-and-metrics.md` に運用手順を追記する。

## 作業ブレークダウン

### 1. 効果システム設計と仕様整理（24-25週目）
**担当領域**: 効果システム基盤設計

1.1. **効果タグとStage定義の抽出**
- [1-3-effects-safety.md](../../spec/1-3-effects-safety.md) §A から `Σ_core`（`mut`/`io`/`panic`/`unsafe`/`ffi`）と `Σ_system`（`syscall`/`process`/`thread`/`memory`/`signal`/`hardware`/`realtime`/`audit`/`security`）を抽出し、表形式で共有。
- 同章 §A.1 の補助タグ（例: `mem`/`debug`/`trace`/`unicode`/`time`/`runtime`）を整理し、標準ライブラリでの利用箇所とプラットフォーム差分を Column に追加。
- [3-8-core-runtime-capability.md](../../spec/3-8-core-runtime-capability.md) §1.2 の Stage テーブルを再掲し、`StageId = Experimental < Beta < Stable` の全順序と StageRequirement（`Exact`/`AtLeast`）の評価ルールを仕様引用付きで記述。
- プラットフォーム別 Stage 設定（Linux/Windows/macOS）を Capability Registry 観点で一覧化し、`RuntimeCapability`/`TargetCapability` と `effect_scope` の突合条件を洗い出す。
- 監査キー（`effect.stage.*`、`effects.contract.stage_mismatch` 等）を列挙し、`docs/spec/3-6-core-diagnostics-audit.md` との整合確認ポイントを明示。

> **ドラフト: 効果タグ/Stage 対応表（レビュー用）**
>
> | 区分 | 効果タグ | 典型 API / Capability | 想定 Stage 下限（仕様） | Registry Stage（2025-10-17 実測） | 監査キー・診断 | 参照 |
> | --- | --- | --- | --- | --- | --- | --- |
> | `Σ_core` | `mut` | `Vec.push`, `Cell.set` | `AtLeast Stable`（純粋境界で許容） | 管理対象外（組み込み） | `effects.contract.mut_usage` | [1-3-effects-safety.md §A](../../spec/1-3-effects-safety.md#a-効果の分類コア--システム拡張) |
> |  | `io` | `Core.IO.readFile`, `print` | `AtLeast Stable`（プラットフォーム依存で段階調整） | 管理対象外（組み込み） | `effects.contract.io_policy` | 同上 |
> |  | `panic` | `panic`, `assert` | `AtLeast Stable`（`@no_panic` で制約可） | 管理対象外（組み込み） | `effects.contract.panic_violation` | 同上 |
> |  | `unsafe` | `unsafe { … }` 境界 | `Exact Stable`（`unsafe` ブロック必須） | 管理対象外（境界内で完結） | `effects.contract.unsafe_boundary` | 同上 |
> |  | `ffi` | `extern "C"`, `RuntimeBridge` | `Exact Beta`（RuntimeCapability で昇格管理） | `experimental`（`examples/algebraic-effects/audit-log.json:3`） | `effects.contract.ffi_scope`, `effects.contract.stage_mismatch` | 同上 / [3-8-core-runtime-capability.md §1.2](../../spec/3-8-core-runtime-capability.md#capability-stage-contract) |
> | `Σ_system` | `syscall` | `Core.System.raw_syscall` | `Exact Experimental`（CI では既定拒否） | 未登録（2025-10-17 時点） | `effects.contract.syscall_policy` | [1-3-effects-safety.md §A](../../spec/1-3-effects-safety.md#a-効果の分類コア--システム拡張) |
> |  | `process` | `Core.Process.spawn_process` | `AtLeast Beta`（監査ログ必須） | 未登録（2025-10-17 時点） | `effects.contract.process_policy` | 同上 |
> |  | `thread` | `Core.Process.create_thread` | `AtLeast Beta` | 未登録（2025-10-17 時点） | `effects.contract.thread_policy` | 同上 |
> |  | `memory` | `Core.Memory.mmap` | `Exact Experimental`（`unsafe` と併用） | 未登録（2025-10-17 時点） | `effects.contract.memory_scope` | 同上 |
> |  | `signal` | `Core.Signal.register_signal_handler` | `AtLeast Beta` | 未登録（2025-10-17 時点） | `effects.contract.signal_policy` | 同上 |
> |  | `hardware` | `Core.Hardware.rdtsc` | `Exact Experimental` | 未登録（2025-10-17 時点） | `effects.contract.hardware_scope` | 同上 |
> |  | `realtime` | `Core.RealTime.set_scheduler_priority` | `AtLeast Beta` | 未登録（2025-10-17 時点） | `effects.contract.realtime_policy` | 同上 |
> |  | `audit` | `Diagnostics.audit_ctx.log` | `AtLeast Stable` | 未登録（2025-10-17 時点） | `effects.contract.audit_scope`, `audit.event.*` | 同上 / [3-6-core-diagnostics-audit.md §2](../../spec/3-6-core-diagnostics-audit.md#2-監査イベント仕様) |
> |  | `security` | `Capability.enforce_security_policy` | `Exact Stable` | 未登録（2025-10-17 時点） | `effects.contract.security_policy` | 同上 |
> | 補助タグ | `mem` | `Core.Alloc.alloc` | `AtLeast Stable`（`@no_alloc` と連携） | 管理対象外（組み込み） | `effects.contract.mem_usage` | [1-3-effects-safety.md §A.1](../../spec/1-3-effects-safety.md#a1-標準ライブラリによる補助タグ) |
> |  | `debug` | `Core.Diagnostics.expect` | `Exact Experimental`（デバッグビルド限定） | 未登録（2025-10-17 時点） | `effects.contract.debug_scope` | 同上 |
> |  | `trace` | 実行トレース API | `AtLeast Beta` | 未登録（2025-10-17 時点） | `effects.contract.trace_scope` | 同上 |
> |  | `unicode` | `Core.Text.normalize` | `AtLeast Stable` | 未登録（2025-10-17 時点） | `effects.contract.unicode_scope` | 同上 |
> |  | `time` | `Core.Time.now` | `AtLeast Stable` | 未登録（2025-10-17 時点） | `effects.contract.time_policy` | 同上 |
> |  | `runtime` | Capability Registry 操作 | `AtLeast Beta`（Stage 昇格時は要監査） | `experimental`（`compiler/ocaml/tests/golden/typeclass_iterator_stage_mismatch.json.golden:6`） | `effects.contract.runtime_policy`, `effects.contract.stage_mismatch` | 同上 / [3-8-core-runtime-capability.md](../../spec/3-8-core-runtime-capability.md) |
>
> 上表の Stage 下限は実装検証前のドラフト値。StageRequirement 実装後に CI で実測し、`0-3-audit-and-metrics.md` へ確定値を記録する。
> 実測列は現行 Registry ログから確認できた値のみ反映している。`未登録` のタグは Capability 登録テストを追加し、Stage メタデータを収集後に更新する（Phase 2-2 テスト整備で追跡）。

1.2. **データモデル設計**
- `EffectTag` 列挙と `EffectSet` ヘルパを `compiler/ocaml/src/core_ir/effect.ml`（新規）または既存ユーティリティに配置し、Parser/IR/Runtime で共有するストラクチャ設計を決定。
- AST (`parser.mly` → `Ast.fn_decl`)・TAST (`Typedtree.fn_decl`)・Core IR (`core_ir/function.ml`) に `effects: EffectSet`、`residual_effects: EffectSet` を追加し、Span 情報と併せて保持する方式を設計。
- `StageRequirement` 型を `type_env.ml` に定義し、`Exact`/`AtLeast` の比較関数と `StageId` 順序マップを提供。`allows_effects` 属性から暗黙 Stage を推論する場合のルール（未指定は `AtLeast Stable`、`@experimental` は `Exact Experimental` 等）を策定。
- 既存属性 (`@pure`/`@requires_capability`/`@handles`/`allows_effects`) の解析フローを洗い出し、追加構文を導入しない前提での AST 拡張手順書を作成。CLI フラグや DSL への後方互換性を検証。

1.3. **型システムとの統合方針**
- `type_inference.ml` で効果制約を `EffectConstraint`（例: `RequiresEffect`, `SubsetOf`, `StageAtLeast`）として表現し、型クラス辞書解決との依存関係を分離。
- 効果多相性は Phase 2 では rank-1 相当の `allows_effects` 付き関数に限定し、`EffectScheme`（ベース集合 + 許可された拡張タグ）を導入するかを検討。将来の行多相拡張を Section 7 と連携して TODO 記述。
- 関数シグネチャ（`Type.Function`）に `effects: EffectSet` と `stage_requirement: StageRequirement` を埋め込み、呼び出しチェックで残余効果・Stage 違反を診断へルーティング。
- Phase 2-1 実装済みの `StageRequirement` 監査（Iterator 辞書等）と共通化する責務境界を整理し、辞書渡し PoC で追加した診断フィールドと重複しないよう調整計画をまとめる。

**成果物**:
- 効果タグおよび Stage 定義の集約表（仕様引用付き）。
- AST/TAST/IR/型環境の拡張設計ノート（責務分担と API 変更点を含む）。
- 効果制約と型クラス制約の統合ポリシー案、および `0-3-audit-and-metrics.md` へ計測観点を追記するためのドラフト。

### 2. Parser/AST 拡張（25週目）
**担当領域**: 構文解析

2.1. **効果構文の実装**
- `@requires_capability`, `@handles`, `allows_effects`, `@pure` など既存属性の解析
- 関数宣言・式への効果注釈の付与
- ネストした効果の構文解析
- エラーハンドリング（不正な効果指定）
- `parser.mly` で `EffectAnnot ::= "!" "{" EffectTags "}"` を正式対応させ、`1-1-syntax.md` §B.6（属性）および §C.11（ハンドラ構文）の BNF と一致させる。
- `lexer.mll` へ `!{` / `}` / `stage` 等のトークン扱いを追加し、`@dsl_export(allows_effects=[...])` のような属性引数が式として評価される前提でトークナイズする。
- `parser_driver.ml` と `ast_builder.ml` に効果タグリストを構築するユーティリティを追加し、`compiler/ocaml/docs/effect-system-design-note.md` で定義した `EffectTag` と合流できるようにする。
- `@cfg` と併用された場合の無効分岐スキップや、`@handles(effect = "...")` のキー解釈など、属性値→タグ変換の失敗を `effects.syntax.invalid_attribute` として診断へ伝搬する。

> **進捗（2025-10-17 更新）**
> - `@requires_capability` および `@dsl_export` / `@allows_effects` から Stage 要件を抽出し、`effect_profile` に `Exact` / `AtLeast` を設定する処理を実装済み。
> - `@allows_effects(...)` / `@handles(...)` から効果タグ集合を抽出し、`!{}` とのマージや重複排除を整備済み。
> - Parser テストに Stage／タグ解析の期待値を追加し、`Exact:experimental` / `AtLeast:stable` の既定挙動を確認済み。
> - 未着手: `allows_effects=[...]` や `@handles(effect = "...")` といった NamedArg 形式の解析、属性値バリデーション／診断連携。

2.2. **AST ノード拡張**
- `Decl::Fn` に `effects: EffectTag[]` を追加
- `Expr::*` に効果伝播用フィールド追加
- Span 情報の保持
- デバッグ用の AST pretty printer 更新
- `ast.ml` に `EffectProfileNode`（`declared: EffectTag list`, `explicit_stage: StageRequirement option`, `source: Span`）を追加し、`FnDecl`, `HandlerDecl`, `EffectDecl` で共有する。
- `core_ir/effect.ml`（新規予定）を AST から参照できるよう、`EffectTag.of_ident : Ident -> (EffectTag, Diagnostic)` を準備し、未知タグは TODO として `docs/spec/1-3-effects-safety.md` の更新対象に記録する。
- `ast_printer.ml` / `parser/print_ast.ml` に `!{ mut, io }` や `@requires_capability(stage = "beta")` のフォーマットを新設し、ゴールデンテストで差分を検出できるようにする。
- `handler` と `effect` 宣言も `EffectProfileNode` を保持し、操作宣言が暗黙に導入する効果タグをプレビューできる状態にする。

> **進捗メモ**
> - `effect_profile_node` を AST/Typed AST 両方に導入し、関数宣言・トレイトシグネチャ・extern 項目で共有する構造を実装済み。
> - `EffectProfileNode` への Stage 埋め込みと、`@allows_effects` / `@handles`（NamedArg 含む）によるタグ取り込みを完了。次は属性値バリデーションと診断連携を実装する。

2.3. **パーサテスト整備**
- 効果注釈の正常系テスト
- 構文エラーのテスト
- ゴールデンテスト（AST 出力）
- Phase 1 パーサとの統合検証
- `compiler/ocaml/tests/test_parser.ml` に `fn demo() !{ io, panic } { ... }`、`@dsl_export(allows_effects=[io, audit]) fn ...`、`handler Console { operation print -> ... }` などの AST 期待値テストを追加。
- `compiler/ocaml/tests/snapshots/` に効果注釈付き AST のゴールデンを生成し、`parse_expect_test` で `EffectProfileNode` への変換が確認できるようにする。
- 属性値の誤指定（例: `@requires_capability(stage=123)`、`@handles` で未知キー）を `test_type_errors.ml` へ追加し、診断キーが `effects.syntax.invalid_attribute` / `effects.contract.stage_mismatch` になることを固定化。
- `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` へ新規テスト ID を追記し、効果構文カバレッジのチェック項目を増やす（Phase 2-2 週次レビューで参照）。

> **テスト状況**
> - Stage 推論を検証するユニットテストを `compiler/ocaml/tests/test_parser.ml` に追加済み。
> - 今後: 効果タグ抽出や異常系（`@handles` 未対応キーなど）をテストに反映させる。

**成果物**: 拡張 Parser、効果 AST、パーサテスト

### 3. Typer 統合と効果解析（25-26週目）
**担当領域**: 型推論と効果検証


### 検証ログ（2025-10-17 実施）
- `dune runtest`（`compiler/ocaml/`）を実行し、Typer/Parser/CLI 診断テストの回帰が無いことを確認。
- `scripts/validate-runtime-capabilities.sh tooling/runtime/capabilities/default.json` を実行し、Stage 集約結果とトレースを `reports/runtime-capabilities-validation.json` に記録。
- `tooling/ci/sync-iterator-audit.sh --metrics tooling/ci/iterator-audit-metrics.json --verify-log tooling/ci/llvm-verify.log --audit compiler/ocaml/tests/golden/audit/effects-stage.json.golden` を実行し、`reports/iterator-stage-summary.md` に Stage トレース検証結果を出力。
3.1. **効果注釈の解析**
- `compiler/ocaml/src/ast.ml` の `effect_profile_node` を [effect-system-design-note.md](../../../compiler/ocaml/docs/effect-system-design-note.md) に沿って `Effect_profile.profile` へ正規化済み（`type_inference_effect.ml`）。
- `Constraint_solver.EffectConstraintTable` を介して関数シンボルと効果集合を記録済み。差分判定と未知属性検証を組み込み、`effects.contract.residual_leak`／`effects.syntax.invalid_attribute` 診断を Typer から発行するパイプラインを確立済み。
- 残余効果解析で生成する診断を CLI ゴールデン（`compiler/ocaml/tests/golden/diagnostics/effects/residual-leak.json.golden`）と `tests/typeclass_effects/effectful_sum.reml` のコンパイル結果で固定化し、辞書渡し／モノモルフィゼーション両経路で同一出力になることを `test_effect_residual.ml` で検証する。
- `core_ir/desugar.ml` へ書き出した効果メタデータを利用し、`runtime/native` まで一貫して参照できるよう差分を検証する。
- `compiler/ocaml/tests/test_type_errors.ml` と CLI ゴールデンで効果診断を固定化し、属性ケースを共有する。未知属性の入力例を追加し、`Diagnostic.extensions.effect.attribute` に解析結果を埋める。
- Stage トレースを含む効果診断ゴールデン（`compiler/ocaml/tests/golden/diagnostics/effects/*.json.golden`）と監査ゴールデン（`compiler/ocaml/tests/golden/audit/effects-stage.json.golden`）を更新し、CI で差分を監視する。
- CLI/JSON のゴールデンには Stage 解釈結果 (`effect.stage.required` / `effect.stage.actual` / `effect.stage.source`) と `Diagnostic.extensions.effect.stage_trace` を出力し、Typer での Stage 決定が RuntimeCapability 判定と一致することをスナップショットで示す。
- `reports/runtime-capabilities-validation.json` と `reports/iterator-stage-summary.md` を生成し、Stage 解釈とトレース差分の検証ログを計画書にリンクできるよう整備する。

3.2. **Stage 要件の検証**
- `type_inference_effect.resolve_function_profile` で CLI/JSON/環境変数由来の Stage を解析し、`Type_error.effect_stage_mismatch_error` で `effect.stage.*` 診断を出力済み。
- `RuntimeCapabilityResolver` の優先度（CLI > JSON > env）を仕様化し、Stage 判定結果を `Constraint_solver` の効果テーブルへ反映する。
- IR メタデータ (`core_ir/function_metadata.effects`) と Runtime Capability (`tooling/runtime/capabilities/*.json`) を突合し、`effects.contract.stage_mismatch` / `effects.contract.stage_escalation_required` を Typer 側で分類した上で Runtime に監査イベントを引き継ぐフローを確立する。
- `tooling/ci/collect-iterator-audit-metrics.py` を拡張し、Stage 判定結果を `iterator.stage.audit_pass_rate` に反映させる。CLI `--emit-effects --format=json` の出力を CI に取り込み、Typer 判定と Runtime 判定の差分が 0 であることを自動検証する。
- `tooling/ci/sync-iterator-audit.sh` を Stage トレース検証に対応させ、`reports/iterator-stage-summary.md` を生成。Typer／Runtime の Trace 差分が存在する場合は CI を失敗させる。
- Typer で生成した Stage トレースを `AuditEnvelope.metadata.stage_trace` へ転記し、CI 集計スクリプトが `RuntimeCapabilityResolver` → `AuditEnvelope` → `iterator.stage.audit_pass_rate` を一連の検証チェーンとして扱えるようにする。

3.3. **型クラスとの整合**
- `Constraint_solver.EffectConstraintTable` で型クラス辞書と効果制約を分離済み。`tests/typeclass_effects/` で辞書解決との独立性を確認する。
- 効果集合の包含チェック・残余効果診断を追加し、型クラス推論への副作用がないことを CLI ゴールデン（辞書モード／モノモルフィゼーション）で検証する。
- `0-3-audit-and-metrics.md` に辞書サイズ・IR 行数の観測値を追記し、効果解析が辞書経路へ与える影響を可視化する。

**成果物**: 効果解析ロジック、Stage 検証、統合 Typer

### 4. RuntimeCapability チェック実装（26-27週目）
**担当領域**: ランタイム検証

4.1. **Capability テーブル埋め込み**
- `RuntimeCapabilityResolver` で CLI/JSON/環境変数（`REMLC_EFFECT_STAGE`）を統合するロジックを実装済み。
- `tooling/runtime/capabilities/default.json` と検証スクリプト (`scripts/validate-runtime-capabilities.sh`) を整備し、`stage` / `capabilities` / `overrides` フォーマットを確立。
- プラットフォーム差分（Linux/Windows）の Capability 定義と Stage テーブルを `docs/spec/3-8-core-runtime-capability.md` と同期する。
- 動的 Stage 変更や `Other` Capability 拡張は Phase 3 で検討。

4.2. **Stage チェックロジック**
- Core IR の関数メタデータに埋め込まれた Stage 要件（`core_ir/desugar.ml` で生成）をランタイムで読み取り、`RuntimeCapabilityResolver` が決定した実行 Stage と照合する。
- `core_ir/iterator_audit.ml` で `DictMethodCall` に付与された `iterator_audit` 情報を収集し、Stage 判定と組み合わせて `main.ml` に渡す。`main.ml` は `RuntimeCapabilityResolver` の結果と突合し、`effect.stage` 監査イベントを `AuditEnvelope` に追加する実装を整備する。
- `tooling/runtime/capabilities/*.json` の内容をロードし、Typer が計算した `Effect_profile.profile.resolved_stage` と一致しない場合は `effects.contract.stage_mismatch` / `effects.contract.stage_escalation_required` を区別して `AuditEnvelope.metadata` に出力する。
- CLI オプション `--effect-stage`（優先度: CLI > JSON > 環境変数）を Runtime 側でも再評価し、Typer 判定と同じ優先度ルールで実行 Stage を決定する。
- Stage ミスマッチの詳細レポートに `effect.stage.expected`, `effect.stage.actual`, `effect.stage.capability_source` を含め、CI で JSON スナップショット比較ができるようにする。
- ランタイム側で Stage 判定の決定経路を `stage.interpretation` ノードとして CLI/JSON 出力に埋め込み、Typer の `effect.stage.source` と突き合わせて差分がないことをゴールデンで保証する。
- テスト用の Capability モック機構を整備し、`runtime/native/tests/effects_stage.ml`（新設予定）で IR メタデータと JSON 設定の差分検証を自動化する。

4.3. **プラットフォーム対応**
- Linux/Windows の Capability 差異の吸収
- Phase 2 Windows タスクとの連携
- Capability 定義の外部化検討（JSON 等）
- クロスコンパイル時の Stage 検証

4.4. **監査ログと CI 連動**
- `runtime/native` で生成する `AuditEnvelope` に Stage 判定結果（`audit.effect.stage.required` 等）を記録し、CLI `--emit-audit` 出力と CI サマリー（`tooling/ci/sync-iterator-audit.sh`）に統合する。
- `tooling/ci/sync-iterator-audit.sh` の出力を `compiler/ocaml/tests/golden/audit/effects-residual.jsonl.golden`（`effects-residual.jsonl.golden`）と突合し、`RuntimeCapabilityResolver` → `AuditEnvelope` → `iterator.stage.audit_pass_rate` の検証チェーンが Core IR メタデータと RuntimeCapability JSON の差分を0件に保つことを CI で確認する。
- `iterator.stage.audit_pass_rate` の算出時に Typer の診断件数と Runtime 監査ログを突合し、`AuditEnvelope.metadata.stage_trace` に記録された Typer/Runtime の Stage 判定差分が 0 件であることをもって合格判定とする。
- 監査ログの JSON スキーマを `docs/spec/3-6-core-diagnostics-audit.md` に合わせ、スナップショットテスト（`compiler/ocaml/tests/golden/audit/effects-stage.json.golden`）で回帰を防止する。

**成果物**: Capability モジュール、Stage チェック、プラットフォーム対応

### 5. 診断システム強化（27週目）
**担当領域**: エラー報告

5.1. **効果診断の実装**
- `Type_error.effect_stage_mismatch_error` を拡張し、`effect.stage.required` / `effect.stage.actual` / `effect.stage.capability` を JSON に出力済み。
- 残余効果診断・未知タグ検知 (`effects.contract.residual_leak`, `effects.syntax.invalid_attribute`) を追加し、`compiler/ocaml/tests/golden/diagnostics/effects/*.golden` で固定化する。
- 効果タグの不一致／候補 Stage の提示を `Diagnostic.extensions` に揃え、監査キーと整合させる。
- CLI/JSON の診断には Stage 解釈手順のサマリー（`diagnostic.extensions.effect.stage_trace`）と RuntimeCapability 由来の Stage 情報を含め、Typer / Runtime / CI の 3 者で同一キーを参照できるようにする。

5.2. **CLI 出力統合**
- 効果情報の CLI 表示
- `--emit-effects` フラグの実装
- カラー出力対応（効果タグごとの色分け）
- [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) との整合
- `--emit-effects --format=json` の結果に Stage 解釈テーブル（Typer 判定・Runtime 判定・Capability JSON ソース）を埋め込み、ゴールデンファイルで Stage 追跡が行えるようにする。

5.3. **AuditEnvelope 統合**
- 効果メタデータの `AuditEnvelope` への記録
- Stage 検証結果の監査ログ出力
- Phase 2 診断タスクとの連携
- JSON 出力のスキーマ定義

**成果物**: 効果診断、CLI 統合、監査ログ

### 6. テスト整備（27-28週目）
**担当領域**: 品質保証

6.1. **効果シナリオテスト**
- 正常系: 各効果タグの基本動作テスト
- 異常系: Stage ミスマッチ、不正な効果指定
- 複合系: 型クラス + 効果の組み合わせ
- `tests/effects/` ディレクトリの新設（Phase 2-3 で検討）
- 型クラス統合向けに `tests/typeclass_effects/` を併設し、`test_effect_residual.ml` で辞書モードとモノモルフィゼーション PoC の双方が同一効果診断を返すことをスナップショット比較
- CLI ゴールデン（`compiler/ocaml/tests/golden/diagnostics/effects/*.json.golden`）を追加し、`effects.contract.residual_leak` / `effects.syntax.invalid_attribute` / `effects.contract.stage_mismatch` の出力を固定化
- ゴールデンでは Stage 解釈トレースと RuntimeCapability ソースを `diagnostic.extensions.effect.stage_trace` に記録し、Typer→Runtime→CI の検証経路が一目で追えるようにする。

6.2. **Stage 検証テスト**
- `Exact`, `AtLeast` の各要件テスト
- プラットフォーム別の Capability テスト
- ランタイム Stage の境界値テスト
- ゴールデンテスト（診断出力）
- CLI `--effect-stage` および `--runtime-capabilities` の組み合わせで Stage 決定フローが変化しないことを `compiler/ocaml/tests/golden/diagnostics/effects/stage-resolution.json.golden` で検証
- `scripts/validate-runtime-capabilities.sh` を CI で実行し、Capability JSON の更新が Stage 判定ゴールデンと矛盾しないこと、ならびに `stage_trace` の Typer/Runtime 同期が壊れていないことを確認
- 監査ログの JSONL (`effects-residual.jsonl`) を `compiler/ocaml/tests/golden/audit/effects-residual.jsonl.golden` と比較し、`iterator.stage.audit_pass_rate` が Core IR メタデータと RuntimeCapability JSON の突合結果を正しく反映しているかを自動で確認する。

6.3. **CI/CD 統合**
- GitHub Actions に効果テストジョブ追加
- テストカバレッジの計測（>80%）
- Phase 1/2 他タスクとの統合テスト
- ビルド時間の監視
- CI 成果物として `iterator.stage.audit_pass_rate` を公開し、Stage トレースの差分が 0 件であることをゲート条件に設定

**成果物**: 効果テストスイート、CI 設定

### 7. ドキュメント更新と仕様同期（28週目）
**担当領域**: 仕様整合

7.1. **仕様書フィードバック**
- [1-3-effects-safety.md](../../spec/1-3-effects-safety.md) への実装差分の反映
- 効果推論ルールの擬似コードを追加
- 新規サンプルコードの追加
- 実装上の制約・TODO の明示

7.2. **Capability 仕様の更新**
- [3-8-core-runtime-capability.md](../../spec/3-8-core-runtime-capability.md) の Stage テーブル更新
- プラットフォーム別の差異を文書化
- 将来拡張（プラグイン Stage）の検討メモ
- Phase 3 への引き継ぎ事項

7.3. **メトリクス記録**
- `0-3-audit-and-metrics.md` に効果検証のオーバーヘッド記録
- Stage チェックのコンパイル時間への影響測定
- CI レポートの自動生成設定
- `0-3-audit-and-metrics.md` §0.3.7 の RuntimeCapability 運用手順とゴールデン更新フローに沿って記録を追記し、CLI オプション優先度や JSON 差分が同期されているか確認
- 効果診断ゴールデンの Stage 解釈結果と `iterator.stage.audit_pass_rate` の最新値を同ページに記録し、RuntimeCapability JSON 更新ごとに差分を明記する。

**成果物**: 更新仕様書、Capability 文書、メトリクス

### 8. 統合検証と Phase 3 準備（28-29週目）
**担当領域**: 統合と引き継ぎ

8.1. **Phase 2 タスク統合**
- 型クラス + 効果 + FFI の統合テスト
- 診断システムの一貫性検証
- Windows 対応との整合確認
- 仕様差分タスクとの調整

8.2. **セルフホスト準備**
- Phase 3 型チェッカへの効果システム移植計画
- OCaml 実装から Reml 実装への写像設計
- 責務分離の確認（Parser/Typer/Runtime）
- 残存課題の `docs/notes/` への記録

8.3. **レビューと承認**
- M2/M3 マイルストーン達成報告
- 効果システムのデモンストレーション
- レビューフィードバックの反映
- Phase 3 への引き継ぎドキュメント作成

**成果物**: 統合検証レポート、セルフホスト設計、引き継ぎ文書

## 成果物と検証
- Stage 判定の単体テストが全て通過し、Capability Stage のミスマッチ検査が CI で 0 件になる。
- CLI 診断で効果タグ・Stage 情報が表示され、`0-3-audit-and-metrics.md` にレポートされる。
- 仕様書の記述と実装が整合していることをレビューで確認し、差異があれば `0-4-risk-handling.md` に登録。

## リスクとフォローアップ
- Stage テーブルが増加した場合のメンテナンス負荷を軽減するため、外部定義ファイル（JSON 等）から読み込む設計を検討。
- 効果タグが増えると型クラス解析と競合する可能性があるため、Typer 内で責務を分離し、Phase 3 でセルフホスト型チェッカに渡す準備を整える。
- RuntimeCapability の定義がプラットフォーム依存となるため、Phase 2 の Windows 対応タスクと整合を取る。

## 参考資料
- [2-0-phase2-stabilization.md](2-0-phase2-stabilization.md)
- [1-3-effects-safety.md](../../spec/1-3-effects-safety.md)
- [3-8-core-runtime-capability.md](../../spec/3-8-core-runtime-capability.md)
- [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
