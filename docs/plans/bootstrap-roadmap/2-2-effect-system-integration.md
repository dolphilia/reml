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

3.1. **効果注釈の解析**
- AST から効果情報を抽出
- 関数シグネチャへの効果型の添付
- 効果の伝播ルール実装（呼び出し先→呼び出し元）
- 効果の合成（複数効果の統合）

3.2. **Stage 要件の検証**
- 関数の要求 Stage と実行環境の照合
- Stage 不一致のエラー検出
- Stage 推論（注釈がない場合のデフォルト）
- 効果型の単一化ルール

3.3. **型クラスとの整合**
- 型クラス制約と効果制約の同時解決
- 辞書引数と効果情報の独立性確保
- Typer パイプラインの責務分離
- Phase 2 型クラスタスクとの統合テスト

**成果物**: 効果解析ロジック、Stage 検証、統合 Typer

### 4. RuntimeCapability チェック実装（26-27週目）
**担当領域**: ランタイム検証

4.1. **Capability テーブル埋め込み**
- [3-8-core-runtime-capability.md](../../spec/3-8-core-runtime-capability.md) の Stage テーブルを OCaml に写像
- プラットフォーム別の Capability 定義
- Stage 判定の共通モジュール実装
- 動的 Stage 変更の検討（Phase 3 以降）

4.2. **Stage チェックロジック**
- コンパイル時の Stage 検証
- ランタイム Capability の照合（将来拡張用）
- Stage ミスマッチの詳細レポート（`Exact` / `AtLeast` の比較）
- テスト用の Capability モック機構

4.3. **プラットフォーム対応**
- Linux/Windows の Capability 差異の吸収
- Phase 2 Windows タスクとの連携
- Capability 定義の外部化検討（JSON 等）
- クロスコンパイル時の Stage 検証

**成果物**: Capability モジュール、Stage チェック、プラットフォーム対応

### 5. 診断システム強化（27週目）
**担当領域**: エラー報告

5.1. **効果診断の実装**
- `Diagnostic.extensions` に `effect.stage.*` を追加
- Stage ミスマッチの詳細メッセージ
- 効果タグの不一致エラー
- 候補 Stage の提示（`Exact`/`AtLeast` の結果を列挙）

5.2. **CLI 出力統合**
- 効果情報の CLI 表示
- `--emit-effects` フラグの実装
- カラー出力対応（効果タグごとの色分け）
- [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) との整合

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
- `tests/effects/` ディレクトリの新設

6.2. **Stage 検証テスト**
- `Exact`, `AtLeast` の各要件テスト
- プラットフォーム別の Capability テスト
- ランタイム Stage の境界値テスト
- ゴールデンテスト（診断出力）

6.3. **CI/CD 統合**
- GitHub Actions に効果テストジョブ追加
- テストカバレッジの計測（>80%）
- Phase 1/2 他タスクとの統合テスト
- ビルド時間の監視

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
