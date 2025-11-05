# 2.5 仕様差分補正計画

## 目的
- Phase 2 で仕様書 (Chapter 1〜3) と実装の差分を洗い出し、記述ゆれ・不足項目を補正する。
- 更新内容を `0-3-audit-and-metrics.md` および計画書に脚注として残し、将来のレビュートレイルを確保する。

## スコープ
- **含む**: 仕様レビュー、差分リストの作成、関連ドキュメントの更新 (本文・用語集・脚注)、メトリクス記録。
- **含まない**: 新機能追加、仕様の大規模刷新。必要な場合は別タスクとして起票。
- **前提**: Phase 2 の実装タスク（型クラス、効果、FFI、診断）が概ね完了し、差分が明確になっていること。
- **連携**: Phase 2-7 で診断・監査パイプラインの残課題を処理し、Phase 2-8 で最終監査を行う前提となるため、差分リストは両フェーズから参照可能な構成で記録する。
- **前提更新 (2026-12-21)**: Phase 2-4 で保留していた監査ゲート整備と Phase 2-7 の残課題解消を反映済み。`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` §5 と `docs/plans/bootstrap-roadmap/2-7-to-2-8-handover.md` §2 に記録された差分ログ、ならびに `reports/audit/dashboard/diagnostics.md` のベースラインを Phase 2-8 の仕様監査で前提とする[^phase27-handshake-spec].

## 作業ディレクトリ
- `docs/spec/` : Chapter 0〜3 の本文・図表・脚注更新
- `docs/guides/` : 仕様変更に追随するガイド修正
- `docs/notes/` : レビュー結果や TODO を記録（例: `docs/notes/guides-to-spec-integration-plan.md`）
- `docs/README.md`, `README.md` : 目次・導線の同期
- `docs/plans/repository-restructure-plan.md`, `docs/notes/llvm-spec-status-survey.md` : 作業ログとリスク管理

## 着手前の準備と初期調査
- **ハンドオーバー確認**: `docs/plans/bootstrap-roadmap/2-4-to-2-5-handover.md` を起点に、差分レビューで参照すべき成果物（`reports/diagnostic-format-regression.md`, `scripts/validate-diagnostic-json.sh` 等）を再確認し、Phase 2-7 と共有する差分リストの初期エントリを整備する。
- **完了報告の整理**: `docs/plans/bootstrap-roadmap/2-4-completion-report.md` のメトリクス欄を確認し、`ffi_bridge.audit_pass_rate`・`iterator.stage.audit_pass_rate` が 0.0 のままである理由と影響範囲を記録しておく。差分補正中に欠落フィールドを発見した場合は Phase 2-7 と即時連携する。
- **技術的負債の把握**: `compiler/ocaml/docs/technical-debt.md` の ID 22/23（Windows Stage / macOS FFI）を優先監視項目とし、差分レビューで関連フィールドが不足していないかチェックリストへ加える。
- **プロジェクト方針との整合**: `docs/spec/0-1-project-purpose.md` に定義された価値観（性能・安全性・段階的拡張）をレビュー観点に反映し、差分の優先順位付けに利用する。
- **実装ガイド更新**: `docs/plans/bootstrap-roadmap/IMPLEMENTATION-GUIDE.md` の Phase 2 重点事項を参照し、Type Class/効果/診断の整備状況を差分調査の前提条件として整理する。
- **作業ログ方針**: 差分補正で生じた判断は `0-3-audit-and-metrics.md`・`0-4-risk-handling.md` に記録し、`docs/plans/repository-restructure-plan.md` のフェーズ定義と矛盾しないようタイムラインを合わせる。

## 作業ブレークダウン

### 1. レビュー計画と体制整備（31週目）
**担当領域**: 計画策定

1.1. **レビュースコープの決定**
- 以下の範囲を対象に、セルフホスト移行へ直結する章から優先レビューする。優先順位は `High`→`Medium`→`Low` の順で実施し、各章の完了条件を `0-3-audit-and-metrics.md` に記録する。

| 領域 | 対象ドキュメント | 主な観点 | 優先度 | 完了条件 |
|------|------------------|----------|--------|----------|
| 言語コア | [1-1-syntax.md](../../spec/1-1-syntax.md), [1-2-types-Inference.md](../../spec/1-2-types-Inference.md), [1-3-effects-safety.md](../../spec/1-3-effects-safety.md) | 型クラス実装の写像、効果注釈と Stage 整合 | High | サンプルコードが OCaml 実装で再現でき、差分リストに原因・影響が記録されている |
| パーサー API | [2-0-parser-api-overview.md](../../spec/2-0-parser-api-overview.md)〜[2-6-execution-strategy.md](../../spec/2-6-execution-strategy.md) | API 呼出シグネチャ、エラー復元戦略、実行ポリシー | High | `Parser<T>` API の現行シグネチャと差異が無いことを確認し、差分があれば追補案を添付 |
| 標準ライブラリ | [3-0-core-library-overview.md](../../spec/3-0-core-library-overview.md)〜[3-10-core-env.md](../../spec/3-10-core-env.md) | Capability Stage、診断メタデータ、FFI 契約 | Medium | `AuditEnvelope`/`Diagnostic` のフィールド一覧と突合し、欠落フィールドが無いことを証明 |
| 補助資料 | `reports/diagnostic-format-regression.md`, `compiler/ocaml/src/diagnostic_serialization.ml`, `scripts/validate-diagnostic-json.sh` | JSON スキーマ、フォーマット差分レビュー手順 | Medium | Phase 2-4 の成果物と仕様の整合が `validate-diagnostic-json.sh` の出力で確認されている |
| 用語・索引用補 | [0-2-glossary.md](../../spec/0-2-glossary.md), `docs/README.md`, `docs/plans/repository-restructure-plan.md` | 用語統一、導線更新 | Low | Glossary の更新差分がリンク整合チェック（手動）で確認済み |

- Phase 2-4 で整備した診断ログ資産をレビュー対象に組み込み、仕様に未記載のフィールドや命名ゆれを差分リストへ記録する。`compiler/ocaml/docs/technical-debt.md` の ID 22/23 はレビュースコープに含め、Windows/macOS 監査ゲートの整備状況を確認する。

1.2. **レビュー観点チェックリスト作成**
- レビュー時に必ず確認する観点をカテゴリ別に整理し、チェックリスト形式で `docs/plans/bootstrap-roadmap/checklists/` 配下へ保存する。初版では以下の項目を Must チェックとする。
  - **用語整合**: [0-2-glossary.md](../../spec/0-2-glossary.md) に定義済みの表記を参照し、差異がある場合は Glossary 更新案と一緒に記録。
  - **コードサンプル検証**: `reml` タグ付きコードブロックを収集し、`compiler/ocaml` のサンプルランナーで構文・型検証を実施。失敗時は差分リストに再現手順を記載。
  - **データ構造対照**: 仕様に記載されたレコード/enum と OCaml 実装（例: `diagnostic_serialization.ml`, `runtime/native/capability_stage.ml`）のフィールドを比較し、差異を表形式で整理。
  - **リンク・参照**: 相互参照や脚注が `README.md`・`docs/README.md` と一致しているか確認。リンク切れは URL と原因を記録。
  - **診断・監査フィールド**: `schema.version`, `audit.timestamp`, `bridge.stage.*`, `effect.stage.*`, `ffi_bridge.audit_pass_rate` 等が仕様・実装双方で一致しているか検証し、`scripts/validate-diagnostic-json.sh` の結果ログを差分リストに添付。
  - **技術的負債トラッキング**: `compiler/ocaml/docs/technical-debt.md` の該当 ID（特に 22/23）に紐づく観点がレビュー時に抜けていないか確認。

1.3. **スケジュールと担当割当**
- 31週目を 3 つのチェックポイントに分割し、各領域の担当とアウトプットを固定する。マイルストーンは `0-3-audit-and-metrics.md` の `phase2.week31` エントリとして記録し、遅延時は `0-4-risk-handling.md` に登録する。

| マイルストーン | 期限 | 担当（ロール） | 成果物 | 依存関係 |
|----------------|------|----------------|--------|----------|
| Kick-off レビュー会議 | 31週目 Day1 午前 | 仕様差分補正チームリード、Phase 2-7 代表 | レビュースコープ承認メモ、連絡窓口一覧 | `2-4-to-2-5-handover.md`、技術的負債 ID 22/23 の最新状況 |
| Chapter/領域別レビュー | 31週目 Day3 終了 | Chapter 1/2/3 担当、診断ログ担当 | 差分リスト初版（章別）、チェックリスト記入結果 | Kick-off のスコープ承認、`scripts/validate-diagnostic-json.sh` 実行ログ |
| スケジュール確定報告 | 31週目 Day5 終了 | 仕様差分補正チーム PM、Phase 2-7 調整役 | 週次レビュー計画（Week32-34）、`0-3-audit-and-metrics.md` 更新 | Chapter レビュー成果、Phase 2-7 タスク進行状況 |

- Phase 2-7 の未完了タスク（Windows/macOS 監査ゲート等）と相談する窓口を Kick-off で明示し、レビュー中に診断ログの欠落を発見した場合は即時フィードバックできる体制を整える。

**成果物**: レビュー計画書、チェックリスト、スケジュール

### 2. Chapter 1 差分抽出（31-32週目）
**担当領域**: 言語コア仕様レビュー

2.1. **構文仕様のレビュー（[1-1-syntax.md](../../spec/1-1-syntax.md)）**
- Phase 1 Parser 実装との差分抽出
- 効果注釈構文の追加反映（Phase 2）
- FFI 宣言構文の追加反映（Phase 2）
- サンプルコードの検証（実際にパース可能か）

**差分リスト（2025-10-28 初版）**
- `SYNTAX-001` Unicode 識別子: 仕様（docs/spec/1-1-syntax.md:27-43）では `XID_Start + XID_Continue*` を前提としているが、実装（compiler/ocaml/src/lexer.mll:46-52）では ASCII + `_` の暫定実装が残存している。非 ASCII 識別子を含むサンプルが Phase 2 OCaml 実装で拒否され、Chapter 3 の多言語サンプル検証が行えない。修正案ドラフト: 仕様側に Phase 2 制約の脚注を追加しつつ、Phase 2-7 技術的負債（Unicode Lexer 拡張）と統合した実装計画を立てる。完了後に `docs/spec/1-1-syntax.md` と `docs/spec/1-5-formal-grammar-bnf.md` を Unicode 仕様へ再同期。
- `SYNTAX-002` `use` の多段ネスト: 仕様（docs/spec/1-1-syntax.md:65-84）では `use Core.Parse.{Lex, Op.{Infix, Prefix}}` のような再帰的ネストを許容するが、実装（compiler/ocaml/src/parser.mly:780-792）では `item_nested = None` 固定で 1 段のみ受理する。ネストした再エクスポート手順がドキュメントと乖離し、Phase 3 セルフホスト案のサンプルが OCaml 実装で失敗する。修正案ドラフト: Parser にネスト対応を実装する案と、仕様側で「Phase 2 時点は 1 段まで」に制限を追記する案を比較。実装する場合は `use_item` の再帰展開を追加し、`parser.conflicts` 影響をレビュー。
- `SYNTAX-003` 効果構文: 仕様（docs/spec/1-1-syntax.md:200-259）と Formal BNF で `effect`/`perform`/`handle` 構文が定義されているが、実装では `PERFORM`/`HANDLE` 以降の構文規則が未実装（compiler/ocaml/src/parser.mly 内に該当生成規則なし）。Phase 2 OCaml 実装では効果構文が受理されず、Chapter 1 のサンプルコードが再現できない。修正案ドラフト: Phase 2-5 では仕様に「効果構文は `-Zalgebraic-effects` + PoC のみ」と明示し、構文実装タスクを Phase 2-2/Phase 2-7 に再割当。効果構文を実装する場合は `parser.mly` に `PerformExpr` と `HandleExpr` 規則を追加し、型推論・効果解析と同時に導入する必要あり。Phase 2-5 `SYNTAX-003 S0` で `docs/spec/1-1-syntax.md`・`docs/spec/1-5-formal-grammar-bnf.md`・`docs/spec/3-8-core-runtime-capability.md` に脚注 [^effects-syntax-poc-phase25] を追加し、PoC ステージの前提を明示した。2026-03-19 には `SYNTAX-003 S2` で `Type_inference_effect` が扱う最小構成（単一タグ捕捉・`resume` 1 回・未宣言タグ禁止）と `Σ_before`/`Σ_after` 記録方針を設計し、PoC 指標 `syntax.effect_construct_acceptance` / `effects.syntax_poison_rate` の JSON 形式を `docs/notes/effect-system-tracking.md` に添付した。2026-03-27 追記: `SYNTAX-003 S3` で診断・CI 計測フローを策定し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に両指標の KPI を追加、`reports/diagnostic-format-regression.md` へ効果構文ゴールデン更新手順を追記。2026-04-03 追記: `SYNTAX-003 S4` で Phase 2-7 への引き継ぎチェックリストとフラグ運用メモを作成し、`2-7-deferred-remediation.md` に PoC 実装タスクとメトリクス更新条件を追加、`docs/notes/effect-system-tracking.md` にハンドオーバー表を掲載した。2026-04-18 追記: `EFFECT-002 Step4` で `extensions.effects.sigma.*` と `audit.metadata["effect.syntax.constructs.*"]` の出力仕様、および CI 指標算出ロジックを確定し、Phase 2-7 での実装タスク（診断拡張・ゴールデン更新・ダッシュボード接続）を明示した。PoC サンプル JSON は計画書と `docs/notes/effect-system-tracking.md` に収録済みで、Phase 2-7 ではスクリプト実装とダッシュボード更新を担当する。2026-12-12 追記: Phase 2-7 Week42 のレビューで H-O1/H-O2 が完了したこと、`syntax.effect_construct_acceptance = 1.0` / `effects.syntax_poison_rate = 0.0` を維持していることを確認し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` と `docs/notes/effect-system-tracking.md#2026-12-12-h-o1〜h-o5-進捗レビュー` に記録した。H-O3（フラグ伝播）、H-O4（Stage 監査連携）、H-O5（脚注撤去条件）は未達のため `2-7-deferred-remediation.md` §8.2〜§8.3 で継続管理する。

**修正案ドラフト対応方針**
- `SYNTAX-001` → 仕様脚注追加 + Phase 2-7 への実装依頼（Unicode Lexer タスク ID を再利用）。CI では ASCII 限定テストを維持し、Unicode 対応時にゴールデンテストを拡張。
- `SYNTAX-002` → 週次レビューで Parser 拡張の工数を確認。追加実装が難しい場合は仕様側を一時的に制限し `docs/notes/` に TODO を記録。
- `SYNTAX-003` → 効果構文の扱いを Chapter 1/3 両方で「実験段階（未実装）」と明記し、実装ロードマップは `docs/plans/bootstrap-roadmap/2-2-effect-system-integration.md` と整合させる。

2.2. **型システムのレビュー（[1-2-types-Inference.md](../../spec/1-2-types-Inference.md)）**
- Phase 2 型クラス実装との差分抽出
- 辞書渡しの仕様追記
- 制約解決アルゴリズムの擬似コード検証
- サンプルコードの型推論結果検証

**差分リスト（2025-10-28 初版）**
- `TYPE-001` 値制限の未実装: 仕様（docs/spec/1-2-types-Inference.md:136-141）は「一般化は確定値のみ」と定義するが、実装（compiler/ocaml/src/type_inference.ml:2172-2283）は `let`/`var` 共に常に `generalize` を適用し、可変参照や I/O を含む式でも多相化される。効果システムとの整合が崩れ、Phase 3 のセルフホスト計画で想定する `mut`/`io` 制約が働かない。修正案ドラフト: Phase 2-5 で一般化条件のチェックリストを作成し、`infer_decl` 内に値制限判定（純粋式判定と効果共存）を実装するオプションと、仕様側に暫定注釈を追加するオプションの両方を提示。再現ログは Step0 棚卸しで取得した CLI 実行結果を参照（`dune exec remlc -- tmp/value_restriction_var.reml --emit-tast`）。2025-11-03 更新: Step2 で Typer 設定へ `value_restriction_mode` を導入し、RunConfig の Effects 拡張からモードを伝播させる設計を確定[^type001-step2-review]。2025-11-05 更新: Step3 で Strict/Legacy 両モードのテスト雛形・診断テンプレート・CI メトリクス（`type_inference.value_restriction_violation`）を用意し、`scripts/validate-diagnostic-json.sh` と `collect-iterator-audit-metrics.py` に値制限チェックを組み込んだ[^type001-step3-cases][^type001-step3-golden][^type001-step3-metrics][^type001-step3-validator]。2025-11-08 更新: Step4 で 1-2/1-3/2-1/2-6 へ RunConfig 連携と診断共有の脚注を追加し、Phase 2-7 `execution-config` / `effect-metrics` へ Legacy モード縮退と監視タスクを移管した。
- `TYPE-002` 効果行が型表現に含まれていない: 仕様（docs/spec/1-2-types-Inference.md:155-169）は `A -> B ! {io, panic}` の効果行を型スキームへ含めると記述するが、実装の型表現（compiler/ocaml/src/types.ml:48-63）は `TArrow` のみで効果集合を保持していない。効果プロファイルは `typed_fn_decl.tfn_effect_profile` に別管理され、型比較時に効果を考慮できない。修正案ドラフト: Phase 2-5 では仕様に「効果行は診断用メタデータとして暫定運用」と補足し、Phase 2-7 で `ty` 表現へ効果情報を統合する計画を評価。2026-04-18 更新: Step2 で `effect_row` レコード案と `TArrow of ty * effect_row * ty` 拡張の影響分析を `compiler/ocaml/docs/effect-system-design-note.md` に追加し、型・診断・IR の改修 TODO とデータフローを整理済み。2026-04-22 追記: Step3 で `docs/spec/1-2-types-Inference.md`・`docs/spec/1-3-effects-safety.md`・`docs/spec/3-6-core-diagnostics-audit.md` に移行ガード脚注を追加し、`RunConfig.extensions["effects"].type_row_mode = "metadata-only"` ガードと診断 `effects.type_row.integration_blocked` / 監査キー `effect.type_row.*` を定義、解除条件を `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md#type-002-effect-row-integration` に登録した。2026-04-24 追記: Step5 で Phase 2-7 向けハンドオーバーノート (`2-5-to-2-7-type-002-handover.md`) とリスク ID `TYPE-002-ROW-INTEGRATION` を登録し、Gate 条件と KPI 追跡導線を `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` / `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に整理した。2026-12-18 更新: Phase 2-7 Sprint C で Core IR・Runtime まで効果行が伝播し、`type_row_mode` の既定値を `"ty-integrated"` へ切り替え。関連脚注を撤去し、仕様・診断・監査の整合を更新した。
- `TYPE-003` 型クラス辞書渡し記録不足: 仕様（docs/spec/1-2-types-Inference.md 全体）のサンプルは `Add`, `Eq`, `Ord` などの辞書引数が Core IR へ落ちる前提だが、実装（compiler/ocaml/src/type_inference.ml:1880-1955, 2352-2356）は型変数を `i64` に強制解決し辞書制約をドロップしている。制約解決器も `solve_trait_constraints` が未実装（常に空）。修正案ドラフト: Phase 2-5 で辞書生成の仕様差分を `0-3-audit-and-metrics.md` に記録し、Phase 2-1 タスクと連携して辞書渡しの PoC 成果を仕様に反映する。

**修正案ドラフト対応方針**
- `TYPE-001` → 値制限導入を優先。`collect_expr` の効果タグと連携させて「純粋式」判定を実装。Step1 で `Typed_ast` 値形状の分類表と `Value_restriction.evaluate` API 案を確定済みであり、実装が間に合わない場合は仕様側で制限をマークし `0-4-risk-handling.md` へリスク登録。
- `TYPE-002` → 効果行を型へ組み込む設計案を `compiler/ocaml/docs/effect-system-design-note.md` に追記し、次週レビューで採用案を決定。仕様には暫定脚注を追加して差分を明示。2026-04-18 更新: Step2 で `effect_row` 統合ドラフトを固め、Phase 2-7 では RowVar 対応と `diagnostics.effect_row_stage_consistency` KPI 導入を実装タスクとして扱う。2026-04-22 追記: Step3 で移行ガード脚注を公開し、`type_row_mode` と診断 `effects.type_row.integration_blocked` / 監査キー `effect.type_row.*` を仕様・計画書へ登録した。2026-12-18 更新: 脚注撤去後の整合性チェックを追加し、`type_row_mode` 切替手順と KPI の最新状態を反映。
- `TYPE-003` → 型クラス差分を `docs/plans/bootstrap-roadmap/2-1-typeclass-strategy.md` へフィードバックし、辞書引数の出力仕様を Chapter 1/2 の双方で補強。

2.3. **効果システムのレビュー（[1-3-effects-safety.md](../../spec/1-3-effects-safety.md)）**
- Phase 2 効果実装との差分抽出
- Stage 要件の記述精緻化
- 効果推論ルールの擬似コード追加
- [3-8-core-runtime-capability.md](../../spec/3-8-core-runtime-capability.md) との整合

**差分リスト（2025-10-28 初版）**
- `EFFECT-001` 効果タグ検出不足: 仕様（docs/spec/1-3-effects-safety.md:11-38）では `mut`/`io`/`ffi` などのコア効果を常時解析すると定義しているが、実装の効果解析（compiler/ocaml/src/type_inference.ml:40-138）で追加されるタグは `panic` のみ。`var` 再代入や `unsafe` ブロック、`ffi` 呼び出しの効果が残余集合へ反映されず、`@no_panic` 以外の契約が機能しない。修正案ドラフト: 効果解析に `mut`／`ffi`／`unsafe` 検出を追加し、`Effect_analysis` のタグ付けルールを仕様付録として Chapter 1 に反映。  
  - ✅ 2025-11-05: `Type_inference.Effect_analysis` にタグ付与ロジックを実装し、`Type_inference_effect.resolve_function_profile`・`effect_profile`・診断/監査経路が複数 Capability を扱えるよう更新済み（詳細は `docs/plans/bootstrap-roadmap/2-5-proposals/EFFECT-001-proposal.md` 参照）。
- `EFFECT-002` 効果操作未実装: 仕様（docs/spec/1-3-effects-safety.md:201-259）では `effect` 宣言と `handle` 構文を前提に `Σ_before/Σ_after` を計算するが、実装は `effect`/`handler` を AST に保持するのみで解析・型付けを未実装（parser/type_inference に対応処理なし）。効果操作が存在しないため `Σ` の差分計算が検証できず、Chapter 1 のハンドラ例が実行不能。修正案ドラフト: Phase 2-5 で仕様に PoC ステータスを注記し、Phase 2-2/Phase 2-7 のハンドラ実装計画を精査。実装を進める場合は `perform` の型付け規則と `handler` の残余効果計算を `Effect_analysis` に追加。
- 2026-04-20 追記: Step5 にて `docs/spec/1-1-syntax.md`・`docs/spec/1-3-effects-safety.md`・`docs/spec/1-5-formal-grammar-bnf.md`・索引 (`docs/spec/0-0-overview.md`, `docs/spec/README.md`) を更新し、`Σ_before`/`Σ_after` 記録および PoC KPI の参照先（`EFFECT-002 Step4`, `docs/notes/effect-system-tracking.md`, `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md`）を明示。残課題は Phase 2-7 ハンドラ実装と Stage 昇格に引き継ぎ、`0-4-risk-handling.md` に「EFFECT-POC-Stage」リスクとして登録。
- `EFFECT-003` Capability 参照の限定処理: `effect` 属性は複数 Capability を要求できる想定だが、実装の `resolve_function_profile`（compiler/ocaml/src/type_inference_effect.ml:38-86）では先頭 1 件のみ `resolved_capability` に反映。複数 Capability を列挙する仕様（docs/spec/1-1-syntax.md:255-259 および Chapter 3 の DSL 例）と不一致で、診断ログに十分なエビデンスが残らない。修正案ドラフト: Stage/Capability の突合を複数エントリ対応へ拡張し、`AuditEnvelope.metadata` へ全 Capability を記録する方針を追加。
  - ✅ 2025-12-06: Step4 で CLI/LSP/監査の `required_capabilities` / `actual_capabilities` 配列整合をテスト化し、`effect.capability_array_pass_rate` 指標と仕様脚注を更新済み（`EFFECT-003` 進捗と連携）。

**修正案ドラフト対応方針**
- `EFFECT-001` → 効果タグ検出ルールを `Type_inference.Effect_analysis` に追加し、`mut`/`io`/`ffi` を最小セットとして導入。仕様には検出アルゴリズムの擬似コードを追加。
- `EFFECT-002` → 効果操作の未実装を明示する脚注を Chapter 1 に追加し、ハンドラ PoC の範囲を `docs/plans/bootstrap-roadmap/2-2-effect-system-integration.md` と同期。実装優先度は Phase 2-2 で再評価。
- `EFFECT-003` → Capability 配列処理を Phase 2-5 の修正案に盛り込み、`runtime_capability_resolver` と連携して複数 Capability を監査ログへ出力するテストケースを追加予定。

**成果物**: Chapter 1 差分リスト、修正案ドラフト

### 3. Chapter 2 差分抽出（32週目）
**担当領域**: パーサーAPI 仕様レビュー

3.1. **コアパーサー型のレビュー（`2-0`〜`2-2`）**
- Phase 1 Parser 実装との差分抽出
- `Parser<T>` 型の OCaml 実装との対応
- コンビネーター API の網羅性確認
- サンプルコードの検証

**差分リスト（2025-10-28 初版）**
- `PARSER-001` Parser<T>/Reply/ParseResult 未整備: 仕様（docs/spec/2-1-parser-type.md:11-109）は純粋関数型パーサーと `Reply{consumed, committed}`、`ParseResult` による診断集約を前提とするが、実装は Menhir ドライバ（compiler/ocaml/src/parser_driver.ml:15-44）が `Result.t` を直接返し、`consumed`/`committed`/`DiagState` を持たない。これにより 2.2/2.5 の切断・回復規則を検証できず、最遠エラー統計も取得不可。修正案ドラフト: Phase 2-5 で Menhir 出力を `Core.Parse` インターフェイスへ包むシム設計（`State`/`Reply`/`ParseResult` 再構築）を作成し、暫定運用として仕様に「OCaml 実装は移行中」の脚注を追加、`0-3-audit-and-metrics.md` に適用範囲を記録する。
- `PARSER-002` RunConfig/MemoTable 欠落: 仕様（docs/spec/2-1-parser-type.md:90-142）は `RunConfig`・Packrat メモ・左再帰制御を必須とするが、実装は構成情報を受け取らず固定設定で解析を行う。Packrat/左再帰/locale 切替・監査拡張が無効化され、Phase 2-6 実行戦略（docs/spec/2-6-execution-strategy.md）とも整合しない。修正案ドラフト: `parser_driver` に `RunConfig` パラメータを導入するロードマップを策定し、設定項目ごとに既存 CI への影響と計測計画を `0-3-audit-and-metrics.md` へ追記、実装タスクは Phase 2-7 `execution-config` サブタスクへ連携。
- `PARSER-003` Core コンビネーター未提供: 仕様（docs/spec/2-2-core-combinator.md:1-160）は `rule`/`label`/`cut`/`recover` など 15 個の公理的コンビネーターを定義するが、実装は `parser.mly` の LR 規則（compiler/ocaml/src/parser.mly:1-24）へ直接エンコードされており、標準 API として再利用できるコンビネーター層が存在しない。Phase 3 で予定している Reml 実装（self-host）に写像できず、DSL/プラグインから `Parser<T>` を利用する前提が成り立たない。修正案ドラフト: Phase 2-5 で OCaml 実装から抽出可能な最小関数群を洗い出し、`Core.Parse`（OCaml 版）モジュール再構成案 + 代替として仕様に「Menhir ブリッジ層で提供」の注記を追加、Phase 3-1（Parser 移植）へ引き継ぐ。2025-12-12 時点で Step4（Packrat・`recover`・Capability 統合）の設計整理を完了し、キャッシュキー方針と RunConfig 連携を `docs/notes/core-parse-api-evolution.md` / `docs/notes/core-parser-migration.md` に記録済み。実装と CI 指標追加は Step5 で追跡する。

**修正案ドラフト対応方針**
- `PARSER-001` を優先度 High（セルフホスト前提）で管理し、Menhir 由来の返却値を `ParseResult` に変換する設計資料を 33 週目レビューへ提示する。
- `PARSER-002` は Phase 2-7 の `execution-config` タスクと合同で `RunConfig` API を段階導入（`require_eof`→`packrat`→`extensions` の順）し、導入ごとにメトリクス更新を行う。
- `PARSER-003` は 2-1 型クラス戦略・2-2 効果統合と足並みを揃え、`Core.Parse` 再構成で必要なサポートコード（`rule` ラベル/診断統合）を洗い出し、仕様脚注と TODO を `docs/notes/core-parser-migration.md`（新設予定）へ蓄積する。

3.2. **字句・演算子のレビュー（`2-3`〜`2-4`）**
- 字句解析実装との差分抽出
- 演算子優先度テーブルの整合
- Phase 2 で追加された構文への対応
- 用語統一（トークン名等）

**差分リスト（2025-10-28 初版）**
- `LEXER-001` Unicode/プロファイル未対応: 仕様（docs/spec/2-3-lexer.md:1-124）は UAX #31/XID ベースの識別子と Unicode ホワイトスペース処理、`lexeme`/`symbol` 等のトリビア共有を前提とするが、実装は ASCII 限定（compiler/ocaml/src/lexer.mll:41-71）の簡易版で、`string_normal` も ASCII エスケープのみ対応。結果として Chapter 2 のサンプル（`identifier(profile=DefaultId)` 等）が実行できず、`Stage`/`Capability` 名の Unicode 別名も受理されない。修正案ドラフト: Unicode 対応は Phase 2-7 `lexer-unicode` サブタスクへ移管し、仕様には `Phase 2-5` の暫定制限を明記。差分チェックリストに ASCII 制約で問題となる箇所（DSL プラグイン、FFI）を追加する。→ **2025-12-12 更新**: Step1 で ASCII 限定挙動とエラーメッセージを棚卸し（`docs/plans/bootstrap-roadmap/2-5-review-log.md` `LEXER-001 Step1`）、再現フィクスチャとテストを追加して差分検証手順を固定化した。→ **2026-02-18 更新**: Step2 で `docs/spec/2-3-lexer.md` D-1・`docs/spec/1-1-syntax.md`・`docs/spec/0-2-glossary.md`・`docs/spec/README.md` に ASCII 制約脚注と索引追記を実施し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に `lexer.identifier_profile_unicode` 指標を登録、レビュー記録を `docs/plans/bootstrap-roadmap/2-5-review-log.md` へ追加した。→ **2026-03-04 更新**: Step3 で DSL/プラグインチーム向け運用ルールを `docs/notes/dsl-plugin-roadmap.md` §7 に整理し、ASCII 限定運用と Phase 2-7 フォローアップ TODO を登録、レビュー記録へ共有事項を追記した。→ **2026-03-21 更新**: Step4 で CI 指標と `--lex-profile=ascii|unicode` スイッチ設計を確定し、`tooling/ci/collect-iterator-audit-metrics.py` にプレースホルダ集計を追加。Phase 2-7 `lexer-unicode` の受け入れ条件として `expected_pass_fraction=1.0` を差分リストに登録した。[^lexer001-phase25]
- `LEXER-002` Core.Parse.Lex API 欠落: 仕様が提供する `whitespace`/`commentBlock`/`leading`/`trim`（docs/spec/2-3-lexer.md:126-156）に相当する API が実装側に存在せず、`parser_driver` から直接 `Lexer.token` を呼び出す構成で `RunConfig.extensions["lex"]` も未使用。ガイド（docs/guides/core-parse-streaming.md）で想定する共有トリビア設定が適用できない。修正案ドラフト: Phase 2-5 で `Lexer` から `Core.Parse.Lex` 互換のヘルパーを切り出す案と、仕様脚注で「OCaml 実装は直接トークン化」を併記する案を比較し、採用案を 33 週レビューで決定。
- `OP-001` 演算子ビルダー未提供: `precedence`/`level` API（docs/spec/2-4-op-builder.md:1-139）を用いた宣言的優先度設定が実装に存在せず、Menhir の文法規則（compiler/ocaml/src/parser.mly:520-812 付近）で手動管理している。期待集合や `cut_here()` の自動挿入、FixIt 生成ができず、複数演算子の曖昧性処理（`choice_longest` 等）が仕様通りに動作しない。修正案ドラフト: Phase 2-5 で演算子宣言の再現手段（Menhir 規則→Op DSL 化）の調査タスクを起票し、仕様脚注で「2.4 は Phase 3 でのセルフホスト向け基準」と明示する。

**修正案ドラフト対応方針**
- `LEXER-001` は Phase 2-7 の Unicode 対応ロードマップを参照し、`Core.Parse` で必要となる最小要件（XID Start/Continue、Bidi 制御禁止、NFC 検証）を優先度 High として差分リストに記録する。
- `LEXER-002` は `RunConfig.extensions["lex"]` を導入する設計書を 33 週レビューで共有し、ガイド類（docs/guides/plugin-authoring.md 等）を更新するフラグを立てる。
- `OP-001` は 2-4 計画の PoC を Phase 2-7/Phase 3-1 と連携し、宣言 DSL の最小核（`infixl`/`prefix`）から順に導入する工程案を作成、`reports/diagnostic-format-regression.md` へ演算子診断のテストケースを追記する。

3.3. **エラー・実行戦略のレビュー（`2-5`〜`2-6`）**
- Phase 1 診断実装との差分抽出
- Phase 2 診断拡張の反映
- 実行戦略の記述精緻化
- `docs/guides/core-parse-streaming.md` との整合

**差分リスト（2025-10-28 初版）**
- `ERR-001` 期待集合/summary 未出力: 仕様（docs/spec/2-5-error.md:1-160）は `Expectation` 集合と `ExpectationSummary` の生成を必須とするが、実装は `Diagnostic.of_parser_error` 呼び出し時に `expected = []` を固定（compiler/ocaml/src/parser_driver.ml:10-38）。CLI/LSP での候補提示や `effects.contract` 連携が機能せず、Phase 2-4 の診断共通化成果を活かせない。修正案ドラフト: `Core.Parse` シム整備と同時に Menhir からの `expected` を収集する仕組み（`Parser.MenhirInterpreter.expected`）を設計し、仕様へ暫定脚注（「OCaml 実装は期待集合を返さない」）を追加、`reports/diagnostic-format-regression.md` で差分テストを拡充する。→ **2025-11-17 更新**: Phase 2-5 ERR-001 S5 で仕様脚注・ガイド・監査 TODO を整備し、期待集合出力は OCaml 実装／CLI／LSP／監査で完全に反映された。暫定脚注は撤去済み。
- `ERR-002` recover/notes/fixit 欠如: `recover`/`severity_hint`/`fixits` の仕様（docs/spec/2-5-error.md:161-318）に対し、実装は `Result.Error` で単一診断を返すのみで回復・FixIt を生成しない。`scripts/validate-diagnostic-json.sh` で期待されるフィールドが欠落し、Phase 2-7 で予定している CLI テキスト刷新にも影響する。修正案ドラフト: 差分補正で `recover` ポイント候補と FixIt 生成ルールを整理し、`0-4-risk-handling.md` に「回復なし」のリスクを登録、Phase 2-7 と連携して JSON スキーマ拡張を前倒しする。**2025-12-09 追記**: Step2 で FixIt テンプレートと `extensions["recover"].has_fixits` 追加案を確定し、notes/hints 文面とゴールデン更新計画を Step3 へ引き継いだ。**2025-12-12 追記**: Step3 で CLI/LSP/ストリーミング各経路のゴールデンと `parser_recover_tests.ml` を整備し、`parser.recover_fixit_coverage = 1.0` を CI で確認。Packrat 経路と notes 翻訳ルールは Phase 2-7 へ移管済み。
- `EXEC-001` 実行戦略/ストリーミング未実装: 仕様（docs/spec/2-6-execution-strategy.md, docs/guides/core-parse-streaming.md）で定義されるトランポリン・Packrat・`run_stream` API が実装に存在せず、`parser_driver` は同期一括解析のみを提供する。`RunConfig.stream` 拡張やバックプレッシャ制御（DemandHint）が未接続で、Phase 3 のセルフホスト `run_stream` 互換性検証ができない。修正案ドラフト: Phase 2-5 でストリーミング API の PoC 領域を定義し、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` へ「stream-runner PoC」の依頼を追加、仕様には暫定脚注で対応状況を記載する。

**修正案ドラフト対応方針**
- `ERR-001` は `Parser.MenhirInterpreter` の `invocation.expected` を使う取り出し手順を調査し、期待集合を `Diagnostic` へ渡すための設計書を 33 週レビューで提示、成果は `0-3-audit-and-metrics.md` の診断メトリクスへ記録する。
- `ERR-002` は Phase 2-7 診断タスクと統合し、`recover`/FixIt の優先度付け（構文エラー vs FFI 設定）を整理、CLI/LSP テストフィクスチャへの反映計画を作成する。
- `EXEC-001` は Phase 2-7 `execution-strategy` と共同で `run_stream` の要件定義（`resume`/`DemandHint`/`SpanTrace` 連携）をまとめ、Phase 3-1 に PoC を渡せるよう差分リストへ TODO を追記する。

**成果物**: Chapter 2 差分リスト、修正案ドラフト

### 4. Chapter 3 差分抽出（32-33週目）
**担当領域**: 標準ライブラリ仕様レビュー

4.1. **コアライブラリのレビュー（`3-0`〜`3-5`）**
- Phase 1 ランタイム実装との差分抽出
- コレクション型の API 整合性
- テキスト処理の Unicode モデル整合
- 数値・時間・IO・パス操作の仕様精緻化

4.2. **診断・Capability のレビュー（`3-6`〜`3-8`）**
- Phase 2 診断実装との差分抽出
- `Diagnostic`/`AuditEnvelope` の仕様更新
- Capability Stage テーブルの最新化
- メタデータキー命名規約の追記

**差分リスト（2025-10-29 初版）**
- `DIAG-001` Severity 列挙の欠落: 仕様では `Severity = Error | Warning | Info | Hint` を必須としている（`docs/spec/3-6-core-diagnostics-audit.md:20`）が、OCaml 実装は `type severity = Error | Warning | Note` のままで `Info`/`Hint` を出力できない（`compiler/ocaml/src/diagnostic.ml:39`）。CLI/LSP のインフォメーション診断が `Warning` へ丸められ、フェーズ 3 の段階的リリース条件（情報レベルの警告分離）が満たせない。→ **2025-10-27 更新**: `compiler/ocaml/src/diagnostic.ml` と `diagnostic_serialization.ml` を更新し、ネイティブに `Info`/`Hint` をハンドリングできるよう修正済み。CLI カラーリングも `Info=青` / `Hint=シアン` で整合。残差分: JSON スキーマ／ゴールデン／メトリクスの `Info`/`Hint` 追加を Step3 以降で対応。→ **2025-11-09 追記**: `diagnostic-v2-info-hint.json` フィクスチャと LSP テストを追加し、`collect-iterator-audit-metrics.py` に `diagnostics.info_hint_ratio` を実装。CLI/LSP/監査の整合確認（Step4）まで完了。→ **2025-11-10 追記**: `docs/spec/3-6-core-diagnostics-audit.md` に DIAG-001 脚注を追加し、Severity 4 値化の履歴を明文化。`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` へ `diagnostic.info_hint_ratio` / `diagnostic.hint_surface_area` を登録し、レビュー記録へ完了メモを追記して差分補正を完了。
- `DIAG-002` `Diagnostic.audit`/`timestamp` 必須化: Builder/Legacy/CLI テスト経路すべてを `phase2.5.audit.v1` テンプレートへ移行し、`audit_id = "cli/<build_id>#<sequence>"`・`change_set.policy = "phase2.5.audit.v1"` を強制する実装を完了（compiler/ocaml/src/diagnostic.ml:292-561, compiler/ocaml/src/main.ml:416-515, compiler/ocaml/tests/test_*）。`python3 tooling/ci/collect-iterator-audit-metrics.py --require-success` で `iterator.stage` / `typeclass.dictionary` / `ffi_bridge` の pass rate が 1.0 に回復した。詳細と検証ログは [`docs/plans/bootstrap-roadmap/2-5-proposals/DIAG-002-proposal.md`](./2-5-proposals/DIAG-002-proposal.md) §3 を参照。
- `DIAG-003` Domain/Metadata の語彙不足: 仕様の `DiagnosticDomain` には `Effect`/`Target`/`Plugin`/`Lsp`/`Other(Str)` が含まれる（`docs/spec/3-6-core-diagnostics-audit.md:172`）が、実装は `Parser/Type/Config/Runtime/Network/Data/Audit/Security/CLI` のみ対応（`compiler/ocaml/src/diagnostic.ml:54`）。`effect.stage.*` 拡張キーも CLI JSON では `extensions["effects"]` のみに出力され、監査ログ側の `metadata["event.kind"]` 等が空のまま（`compiler/ocaml/src/diagnostic.ml:342`）。→ Domain 列挙と `AuditEnvelope.metadata` のキー体系を仕様と揃え、`docs/spec/3-8-core-runtime-capability.md` の Stage テーブル更新とあわせて 2-7 へフィードバックする。**2025-11-27 更新**: Step3 で `Other` ドメインの正規化（`extensions["domain.other"]` への退避）、JSON スキーマ列挙、および Plugin/Lsp/Other ゴールデン追加を完了。**2025-11-28 更新**: `event.domain` / `capability.ids` / `plugin.bundle_id` を監査ログへ出力し、`diagnostics.domain_coverage` / `diagnostics.effect_stage_consistency` / `diagnostics.plugin_bundle_ratio` 指標を導入。**2025-11-30 更新**: Step5 で `docs/spec/3-6-core-diagnostics-audit.md`, `docs/spec/0-2-glossary.md`, `docs/spec/3-8-core-runtime-capability.md`, `docs/guides/runtime-bridges.md`, `docs/notes/dsl-plugin-roadmap.md` に脚注・周知メモを追加し、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` へ CI ダッシュボード改修 TODO を登録。

**修正案ドラフト対応方針**
- `DIAG-001` `Severity` 拡張と JSON スキーマ改版案を Phase 2-5 差分リストにまとめ、CLI/LSP 双方のゴールデンを更新するタイムラインを 2-7 チームへ共有。Step5 で仕様・指標ドキュメントの更新とレビュー記録追記まで完了しているため、Phase 2-7 では CLI テキスト出力刷新と `diagnostic.hint_surface_area` 集計実装を引き受ける。
- `DIAG-002` `phase2.5.audit.v1` テンプレートを前提に、監査ダッシュボードで `diagnostic.audit_presence_rate` / `cli.change_set.origin` / `cli.audit_id.sequence` を KPI として可視化する実装、および `reports/audit/` のインデックス更新フローを Phase 2-7 (`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md`) へ引き継ぐ。
- `DIAG-003` Domain/Metadata の語彙差分を `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` の Capability/Stage 再検証タスクへリンクさせ、仕様脚注に現状の出力制限を仮明記する（2025-11-30 時点で TODO 登録済み、ダッシュボード改修で監査指標を可視化予定）。

4.3. **非同期・FFI・環境のレビュー（`3-9`〜`3-10`）**
- Phase 2 FFI 実装との差分抽出
- ABI 仕様テーブルの精緻化
- 所有権契約の擬似コード追加
- 環境変数 API の整合性確認

**成果物**: Chapter 3 差分リスト、修正案ドラフト

### 5. 修正案の作成とレビュー（33週目）
**担当領域**: 修正案策定

5.1. **差分の分類と優先順位付け**
- Critical（セルフホスト阻害または監査継続不能）:

  | ID | 章/領域 | 差分概要 | 推奨初動 |
  |----|---------|----------|----------|
  | `TYPE-001` | Chapter 1／型推論 | 値制限が未実装で効果安全性が崩壊 | `infer_decl` の一般化条件ドラフトを 33 週レビューへ提出し、`0-3-audit-and-metrics.md` に想定エラー率を記録 |
  | `TYPE-003` | Chapter 1／型クラス | 辞書渡しがドロップされ Core IR と不整合 | 型クラス PoC 結果を反映した辞書生成フロー案を `docs/plans/bootstrap-roadmap/2-1-typeclass-strategy.md` と同期 |
  | `EFFECT-001` | Chapter 1／効果タグ | `mut`/`io` が検出されず契約が機能しない | `Type_inference.Effect_analysis` のタグ拡張案を起案し、CI で測定するメトリクスを定義 |
  | `PARSER-001` | Chapter 2／Parser 型 | `ParseResult` が欠落し診断統計が取れない | Menhir 包装シムの設計メモをまとめ、`Core.Parse` インターフェイス試案を添付 |
  | `PARSER-003` | Chapter 2／コンビネーター | 標準コンビネーター層が存在しない | Reml self-host で利用する最小 API 抽出案と脚注追加方針をセットで提示 |
  | `EXEC-001` | Chapter 2／実行戦略 | `run_stream` 等が未実装でストリーミング検証不可 | `stream-runner PoC` の責任分担とタイムラインを Phase 2-7 と合意 |
  | `DIAG-002` | Chapter 3／診断監査 | `AuditEnvelope`/`timestamp` が任意扱いで監査欠落 | 最低限の自動補完案とリスク登録フローを決定し、監査メトリクスの閾値を再設定 |

- High（Phase 3 着手前に解消したい差分）:

  | ID | 章/領域 | 差分概要 | 推奨初動 |
  |----|---------|----------|----------|
  | `SYNTAX-002` | Chapter 1／モジュール | `use` の多段ネストが未対応 | Parser 拡張と暫定脚注の二案を比較し、工数見積りを記録 |
  | `SYNTAX-003` | Chapter 1／効果構文 | `perform`/`handle` が未実装 | 効果 PoC の扱いを仕様脚注へ反映し、実装ロードマップとリンク |
  | `TYPE-002` | Chapter 1／効果行 | 効果集合が型へ反映されない | 効果行組込み案と暫定メタデータ運用案を比較検討 |
  | `EFFECT-002` | Chapter 1／効果操作 | `effect`/`handle` の残余計算が欠落 | ハンドラ PoC の受け入れ条件を整理し、Phase 2-2 へ戻し条件を通知 |
  | `EFFECT-003` | Chapter 1／Capability | `effect` 属性が複数 Capability を扱えない | `AuditEnvelope.metadata` への多値出力方針とテスト計画を作成 |
  | `PARSER-002` | Chapter 2／RunConfig | Packrat/左再帰制御が欠落 | `RunConfig` 段階導入表と CI 影響を `0-3-audit-and-metrics.md` に追記 |
  | `LEXER-002` | Chapter 2／Lex API | トリビア共有 API が未実装 | `Core.Parse.Lex` ラッパー抽出案を提示し、仕様脚注の案文を用意 |
  | `ERR-001` | Chapter 2／期待集合 | エラー期待候補が空集合 | Menhir 期待集合取り出し手順の PoC を作成し、`reports/diagnostic-format-regression.md` で検証ケースを追加 |
  | `ERR-002` | Chapter 2／Recover | `recover`/`fixit` 情報が欠落 | CLI/LSP ゴールデン更新計画と優先度付けを策定 |
  | `DIAG-001` | Chapter 3／Severity | OCaml 実装は `Info`/`Hint` 出力へ更新済み（2025-10-27 対応）。JSON スキーマとゴールデンが未追随 | 列挙型拡張を反映したスキーマ改版スケジュールを整理 |

- Medium（Phase 3 並走で対応可だが追跡継続）:

  | ID | 章/領域 | 差分概要 | 推奨初動 |
  |----|---------|----------|----------|
| `SYNTAX-001` | Chapter 1／識別子 | Unicode 識別子未対応 | Step1 で脚注範囲と拒否メッセージを棚卸済み。Step2（2026-02-24）で BNF・索引・用語集へ ASCII 暫定脚注を追補し、技術的負債 ID 22/23 と Phase 2-7 `lexer-unicode` タスクへ橋渡し済み。Step3 で Unicode 受理テスト雛形とサンプル (`unicode_ident_tests.ml` / `samples/unicode_identifiers.reml`) を作成し、実装後に即時検証できる土台を整備。Step4（2026-03-31）で `lexer.identifier_profile_unicode = 0.0` の測定値を記録し、リスク台帳と Phase 2-7 ロードマップを更新 |
  | `LEXER-001` | Chapter 2／Unicode | `identifier(profile)` が使えない | プロファイル差分と運用ルールを `docs/notes/dsl-plugin-roadmap.md` §7 に記録 |
  | `DIAG-003` | Chapter 3／Domain | Domain/metadata が不足 | 語彙拡張案と Capability Stage テーブル更新予定を紐付け |

- Low（記述改善・冗長性の整理）:
  - 小見出しの文言統一、脚注整備、誤字脱字。差分リストに専用 ID を設け、ドキュメント更新時にまとめて反映する。

5.2. **修正案の作成**
- アウトプット形式: `docs/plans/bootstrap-roadmap/2-5-proposals/<ID>-proposal.md` に Markdown で整備し、以下の節を必須とする。
  1. **背景と症状**（関連仕様・実装ファイル・メトリクスへの参照を脚注で付与）
  2. **Before / After**（必要に応じて `reml` や `json` タグ付きコードブロックで提示）
  3. **影響範囲と検証**（テスト・CI・`0-3-audit-and-metrics.md` 更新項目）
  4. **フォローアップ**（Phase 2-7/2-8 への連携、未解決リスクの記録先）
- 差分を仕様側で折り返す場合は、該当章へ脚注か TODO を追加し、根拠 URL を明示する。
- 提案内で引用した実装断片は行番号付きで記し、Patch 適用時に確認できるよう `compiler/ocaml/...:line` 形式を徹底する。
- レビュアへの問いや留意点は末尾に「確認事項」節を設け、決定を待つ項目が埋もれないようにする。

5.3. **レビュープロセス**
- 提案受付: 週次レビュー (33 週 Day2) までに `Critical` と `High` の提案ドラフトを提出し、Phase 2-7 代表と合同レビュー枠を確保する。
- レビュー記録: フィードバックは `docs/plans/bootstrap-roadmap/2-5-review-log.md` に記入し、承認/保留/却下を明示。却下の理由は Phase 3 への引き継ぎ資料として保管する。
- 再調整フロー: 修正案が差し戻された場合、更新差分を明示して再提出し、`0-3-audit-and-metrics.md` の進捗欄を更新する。
- 承認後: 実装チームへタスク化する際は Issue/チケット ID を記載し、仕様更新スケジュール（セクション 6）と連携。監査項目は `0-4-risk-handling.md` の該当エントリをクローズまたは更新する。

**成果物**: 承認済み修正案、レビュー記録

### 6. 修正計画の実施（33-34週目）
**担当領域**: 修正計画実行

6.1. **計画準備の同期**
- `docs/plans/bootstrap-roadmap/2-5-proposals/README.md` の着手順序ガイドを元に Phase 2-5 内で実行する計画を確定し、週次計画に反映する。
- 各計画の担当者・依存関係・完了条件を `docs/plans/bootstrap-roadmap/2-5-review-log.md` に記載し、レビュー時に参照できるようにする。
- `0-3-audit-and-metrics.md` に計画 ID ごとのメトリクスキー（例: `diagnostic.info_hint_ratio`, `parser.runconfig_coverage`）を追記し、進捗を定量把握できる状態を整える。

6.2. **Critical/High 計画の実施**
- **PARSER-001**: `compiler/ocaml/src/parser_driver.ml` を `State`/`Reply`/`ParseResult` シムで包み、`DiagState` と `consumed`/`committed` を Phase 2-5 Week31 内に復元する。
  - Week31 Day1-2: `docs/plans/bootstrap-roadmap/2-5-proposals/PARSER-001-proposal.md` を実装仕様レベルへ拡張し、`State`/`Reply` の OCaml 型定義、`DiagState` のフィールド、`reports/diagnostic-format-regression.md` で検証する JSON 例を追記する。
  - Week31 Day3-4: Menhir ドライバ呼び出しを `Core.Parse` 互換シムに差し替え、`ParseResult.diagnostics` へ複数診断を束ねる PoC を `parser_driver_tests.ml`（新規）で検証する。`parser.parse_result_consistency` と `parser.farthest_error_offset` を `0-3-audit-and-metrics.md` に登録し、CI で `run`/`run_partial` 一致率を測定する。
  - Week31 Day5: `docs/spec/2-1-parser-type.md` と `docs/spec/2-6-execution-strategy.md` に移行脚注を追加し、CLI/LSP で `ParseResult.recovered` を JSON 出力へ載せるための更新作業を Phase 2-5 Week32 のドキュメント反映ステップへ連携する。完了条件は `scripts/validate-diagnostic-json.sh` で `ParseResult` 拡張フィールドが検証されること。
- **PARSER-003 Step6**: 2025-12-24 時点でコアコンビネーター抽出計画のドキュメント同期を実施し、`Core_parse` PoC の状況を仕様・ガイド・索引へ反映する。
  - `docs/spec/2-2-core-combinator.md` に OCaml 実装進捗の脚注を追加し、`rule`/`label`/`cut` と Packrat 指標が仕様側で追跡できるよう整備。`docs/guides/plugin-authoring.md` / `docs/guides/core-parse-streaming.md` へ RunConfig 共有手順とコンビネーター利用例を追記した。
  - `README.md` と `docs/plans/bootstrap-roadmap/2-5-proposals/README.md` の索引を更新し、PARSER-003 の Step6 完了を記録。`docs/notes/core-parse-api-evolution.md` に Step6 セクション、`docs/plans/bootstrap-roadmap/2-5-review-log.md` に 2025-12-24 ログを追加した。
  - テレメトリ統合・Menhir 置換判断は未完了のため `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` へ TODO を移送し、Phase 2-7 で Packrat 指標 (`parser.core_comb_rule_coverage` など) の監査強化を検討する。
- **TYPE-003 / DIAG-002**: Phase 2-5 開始直後に実装を進め、`ParseResult` シム導入後でも型クラス辞書復元・監査必須化が阻害されないよう依存関係を整理する。完了後は各仕様章へ脚注・脚注解除の予定を反映する。TYPE-003 については以下の 5 ステップを Week31 Day1-5 で順に進める。
  1. **Typer での辞書復元（Day1）**: `compiler/ocaml/src/type_inference.ml:2213-2414` の `_dict_refs` ダミー変数を `Typed_ast` へ伝播できるよう書き換え、`solve_trait_constraints` の結果を `typed_expr`/`typed_decl` に添付する。`typed_ast.ml` に辞書引数スロット（`typed_dict_arg list`）を追加し、`generalize` 後も制約が脱落しないことを `type_inference_tests.ml` のゴールデンで確認する。
  2. **Core IR への接続（Day2-3）**: `core_ir/desugar.ml:110-210` の辞書生成パスと `core_ir/monomorphize_poc.ml` を更新し、Typer から渡された `dict_ref` を `DictConstruct`/`DictLookup`/`DictMethodCall` に還元する。`core_ir/ir.ml` と `core_ir/llvm_backend` へ辞書引数の ABI 情報を渡し、`scripts/compare-ir.sh` で辞書ノードが生成されていることを検証する。
  3. **監査・診断の貫通（Day3-4）**: `typeclass_metadata.ml` と `type_error.ml` の `Typeclass_metadata.make_summary` 呼び出しに辞書情報を必須化し、`Diagnostic.extensions["typeclass"]` および `AuditEnvelope.metadata["typeclass.*"]` を `docs/spec/3-6-core-diagnostics-audit.md` のキーセットに合わせて更新する。DIAG-002 が要求する `Diagnostic.audit`/`timestamp` 必須化と同じ枠で `typeclass.dictionary_pass_rate`（`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` へ追記）を 1.0 に引き上げる。
  4. **Capability / Stage 連携（Day4）**: `core_ir/desugar.ml` と `core_ir/iterator_audit.ml` に `dict_ref` から Stage 情報を逆引きするヘルパーを追加し、`effects.stage.*` 診断と `runtime_capability` 監査が Chapter 3（`docs/spec/3-8-core-runtime-capability.md`）と一致するよう `collector` 系の PoC を再実行する。
  5. **検証と文書化（Day5）**: `compiler/ocaml/tests/typeclass_dictionary_tests.ml`（新設）と `reports/diagnostic-format-regression.md` に辞書付き診断ケースを追加し、`scripts/validate-diagnostic-json.sh` / `scripts/benchmark_typeclass.sh` を更新する。`docs/spec/1-2-types-Inference.md` §B.1 と `docs/spec/2-1-parser-type.md` に辞書引数の復元状況を脚注で記録し、`docs/plans/bootstrap-roadmap/2-1-typeclass-strategy.md` の進捗欄にも差分を反映する。
  - DIAG-002 については以下の順で対応する。
    1. **診断生成経路の棚卸し（Week31 Day1）**: `Diagnostic.Builder` を経由しない生成箇所（`diagnostic_of_legacy`, CLI/LSP 補助など）と `audit_metadata` を空にしているユーティリティを列挙し、`docs/plans/bootstrap-roadmap/2-5-review-log.md` へ記録する。
    2. **型定義とビルダー更新（Week31 Day2）**: `Diagnostic.t` の `audit` / `timestamp` を非 `option` 化し、`Diagnostic.Builder.build`・`merge_audit_metadata`・`diagnostic_of_legacy` で `Audit_envelope.empty_envelope` と `iso8601_timestamp` を常に設定する（compiler/ocaml/src/diagnostic.ml:120-273, 818-900）。  
    3. **スキーマ・シリアライズ整備（Week31 Day2-3）**: `tooling/json-schema/diagnostic-v2.schema.json` に `audit` / `timestamp` を追加必須化し、AJV テストと `scripts/validate-diagnostic-json.sh` のゴールデンを更新する。`compiler/ocaml/src/diagnostic_serialization.ml:59-146` へ欠落検知のアサーションを導入する。
    4. **CI メトリクス連携（Week31 Day3-4）**: `tooling/ci/collect-iterator-audit-metrics.py:61-174` の必須キー検証で欠落時に詳細ログを出すよう拡張し、`diagnostic.audit_presence_rate` を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に追記。Linux/Windows/macOS CI で当該スクリプトを必須化する。
    5. **ドキュメントとノート反映（Week31 Day4-5）**: `docs/spec/3-6-core-diagnostics-audit.md` に必須化脚注を追加し、`reports/diagnostic-format-regression.md` と `docs/notes/diagnostic-audit-gap.md`（新設）へ移行チェックリストを記録する。`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` へ監査ダッシュボード更新タスクを連携する。
- **EFFECT-001 / DIAG-001 / SYNTAX-002 / ERR-001**: Phase 2-5 前半で実施し、効果タグ拡張・Severity 拡張・`use` ネスト対応・期待集合出力を確立する。関連する JSON スキーマや CLI/LSP ゴールデンを更新し、`scripts/validate-diagnostic-json.sh` で回帰確認する。
- 実装完了時は `docs/plans/bootstrap-roadmap/2-5-proposals/<ID>-proposal.md` 内のメトリクス更新項目を実行し、`0-3-audit-and-metrics.md` の対象欄を更新する。

**進捗ログ（2025-10-25 時点）**
- `PARSER-001` は `compiler/ocaml/src/parser_driver.ml` を `State`/`Reply` シムに置き換え、`parser_diag_state.ml` と `run_string`/`run_partial` を実装。`test_parser_driver.ml` / `test_parse_result_state.ml` を追加して `dune runtest tests` で回帰確認済み。
- `parser.parse_result_consistency` / `parser.farthest_error_offset` を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に登録し、`docs/spec/2-1-parser-type.md` / `docs/spec/2-6-execution-strategy.md` へ Phase 2-5 の移行脚注を追記済み。`scripts/validate-diagnostic-json.sh` でも `parse_result.recovered` の欠落を検知する自動化を追加した。
- `TYPE-003` は Day1〜Day5 を完了。`solve_trait_constraints` の結果を `typed_expr`/`typed_decl` に伝播し（compiler/ocaml/src/type_inference.ml:2219、compiler/ocaml/src/typed_ast.ml:19）、Core IR が辞書パラメータを自動挿入して `DictConstruct`/`DictMethodCall` を生成できる状態になった（compiler/ocaml/src/core_ir/desugar.ml:393、compiler/ocaml/src/core_ir/monomorphize_poc.ml:23）。`Typeclass_metadata` と `type_error.ml` 経由で辞書メタデータを `Diagnostic` / `AuditEnvelope` へ反映し、CI メトリクス（tooling/ci/collect-iterator-audit-metrics.py:1464、docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md:14）から `typeclass.dictionary_pass_rate` を監視できる。Day4 で Stage 逆引きを復旧し、Day5 で辞書付き診断ゴールデン・仕様脚注・レビュー手順（`reports/diagnostic-format-regression.md`）まで更新済み。
- `typeclass.dictionary_pass_rate` を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に追加し、CI 集計 (`tooling/ci/collect-iterator-audit-metrics.py`) で `extensions.typeclass.dictionary.*` と `AuditEnvelope.metadata["typeclass.dictionary.*"]` の実データを検証するメトリクスを導入。`typeclass_metrics.related_metrics` として辞書検証結果を `iterator-audit-metrics.json` に出力する。
- `SYNTAX-002` は S2（AST/型付き AST 整合確認）を完了。`compiler/ocaml/src/typed_ast.ml:150-163` と `compiler/ocaml/src/type_inference.ml:2796-2833` で `tcu_use_decls = cu.uses` が維持されることを確認し、`compiler/ocaml/docs/parser_design.md` へ脚注を追加。`use_item.item_nested` を Menhir が埋めれば下流がそのまま受け取れる見通しを [`docs/plans/bootstrap-roadmap/2-5-review-log.md`](docs/plans/bootstrap-roadmap/2-5-review-log.md#syntax-002-day1-2-ast型付きast整合確認2025-10-27) に記録した。2025-10-29 追記: `Module_env.flatten_use_decls` を実装して `tcu_use_bindings` を生成するよう拡張し、束縛情報を Typer／診断に渡す準備が整った（[`docs/plans/bootstrap-roadmap/2-5-review-log.md`](docs/plans/bootstrap-roadmap/2-5-review-log.md#syntax-002-day3-4-束縛診断連携2025-10-29) を参照）。
- 2025-10-28 追記: S3（Menhir ルール実装）で `compiler/ocaml/src/parser.mly` の `use_item` を再帰対応へ更新し、`menhir --list-errors parser.mly` の再生成を完了。`parser.conflicts` に追加コンフリクトが発生しないことを確認し、残作業は S4（Typer/診断連携）と S5（テスト・ドキュメント更新）へ移行。
- 2025-11-12 追記: S5（検証とドキュメント更新）を完了。`compiler/ocaml/tests/test_parser.ml` に多段ネスト `use` のユニットテストを追加し、`test_module_env.ml` と併せて `dune runtest` で成功を確認。`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に `parser.use_nested_support` 指標を登録し、`docs/spec/1-5-formal-grammar-bnf.md`／`docs/spec/3-0-core-library-overview.md` へ脚注と概要を追記して仕様側の記述を最新化した（[`docs/plans/bootstrap-roadmap/2-5-review-log.md`](docs/plans/bootstrap-roadmap/2-5-review-log.md#syntax-002-day4-5-検証ドキュメント更新2025-11-12) を参照）。

6.3. **High 計画の連続実行**
- **PARSER-002 / LEXER-002 / DIAG-003 / EFFECT-003 / TYPE-001**: Phase 2-5 中盤で着手し、RunConfig 導入・Lex API 抽出・診断ドメイン拡張・複数 Capability 対応・値制限復元を進める。`PARSER-002` は Week32 Day1-2 で RunConfig 基本型と拡張 API を `compiler/ocaml/src/parser_run_config.ml` に実装済みであり、Week32 Day4-5 のテスト整備で `parser.runconfig_switch_coverage` / `parser.runconfig_extension_pass_rate` を `0-3-audit-and-metrics.md` に登録し、`collect-iterator-audit-metrics.py --require-success` による監視を有効化した[^runconfig-step1-phase25][^runconfig-step5-phase25].
  2025-11-20 時点で Step 3 を完了し、`extensions["lex"]`/`["recover"]`/`["stream"]` のデコードシムと `Parser_diag_state` の回復設定連携を実装済み。`RunConfig` に未指定の `lex.profile` は `strict_json` へフォールバックし、`recover.notes` は `Parser_diag_state.recover_notes_enabled` で確認できるようになった[^runconfig-step3-phase25].
 2025-11-26 追記: LEXER-002 Step1 で `core_parse_lex` の公開境界（`Trivia_profile` alias／`Pack` record／`Api`／`Bridge`）と `RunConfig` round-trip 方針（`effective_profile`・`attach_space`）を合意し、`Pack.space_id` を `Extensions.Parser_id` へ格納する設計を計画書・レビュー記録に反映した。`core_parse_lex_tests.ml` と `lexer.shared_profile_pass_rate` 指標による検証計画も整理済み。
 2025-11-27 追記: LEXER-002 Step2 で `core_parse_lex` ブリッジ実装と `parser_run_config` 拡張（`Lex.set_profile` / `set_space_id`、`Config.trivia_profile` / `with_trivia_profile`）を追加し、`extensions["config"].trivia` ↔ `extensions["lex"]` の同期経路を確立。`Bridge.with_space_id` により `ParserId` を `Extensions.Parser_id` 型で保持できることを確認。
 2025-11-28 追記: LEXER-002 Step3 で `Core.Parse.Lex.Api` の `lexeme`/`symbol` 等を公開し、`lexer.mll` に `set_trivia_profile`・`read_token` を追加して `hash_inline`・shebang・ネストコメントのプロフィール切替を有効化。`doc_comment` と `ParserId` の戻し込みは Step4 へ残課題として送付。
 2025-11-29 追記: LEXER-002 Step4 で `parser_driver` が `Core.Parse.Lex` ブリッジ経由で `RunConfig.extensions["lex"]` を適用し、CLI/LSP の RunConfig も `lex.profile=strict_json` を既定反映するよう更新。`lexer.mll` へ `Core_parse_lex.Record.consume` フックを追加し、空白・コメント削除を統一的に記録できる足場を整備した。`space_id` 警告と実測集計は Step5 での TODO として残す。
 2025-11-30 追記: LEXER-002 Step5 で `compiler/ocaml/tests/core_parse_lex_tests.ml` を追加し、プロフィール切替と `Api.symbol` の挙動を検証。`tooling/ci/collect-iterator-audit-metrics.py` に `lexer.shared_profile_pass_rate` を実装し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` へ指標を追記。`docs/notes/lexer-performance-study.md` を新設してベンチマーク手順と `remlc` 未構築による計測保留を記録した。実測と `Record.consume` 集計は Step6 へ継続。
 2025-12-02 追記: LEXER-002 Step6 で 2.3/2.6 仕様書と `core-parse-streaming` ガイドに RunConfig ↔ Lex API の進捗脚注を追加し、`docs/plans/bootstrap-roadmap/2-5-review-log.md` にドキュメント反映ログを記録。残課題（`Record.consume` 集計と `space_id` 警告、doc_comment 仕様反映）は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` へ移送予定。
- 各計画で追加した単体テスト（`runconfig_tests.ml`, `core_parse_lex_tests.ml`, `capability_profile_tests.ml`, `type_inference_effect_tests.ml` 等）を CI に組み込み、`0-3-audit-and-metrics.md` の新メトリクスが反映されることを確認する。
- 計画進行中に検出したリスク・課題は `0-4-risk-handling.md` へ即時登録し、必要に応じて Phase 2-7 へエスカレーションする。

[^type001-step0-review]:
    2025-10-31 に `TYPE-001` Step0 で実行した CLI 再現ログ。`fn main() -> i64 !{ mut } = { var poly = |x| x; ... }` を含むサンプルを `dune exec remlc -- tmp/value_restriction_var.reml --emit-tast` で型チェックすると、値制限違反は報告されず `=== Typed AST ===` のみが出力される。詳細は `docs/plans/bootstrap-roadmap/2-5-review-log.md` の「TYPE-001 Day1 値制限棚卸し」を参照。

[^type001-step2-review]:
    2025-11-03 に `TYPE-001` Step2 で確定した Typer 設定・RunConfig 連携の設計整理。`docs/plans/bootstrap-roadmap/2-5-review-log.md` の「TYPE-001 Step2 Typer 連携と RunConfig 導入」を参照。

[^runconfig-step1-phase25]:
    2025-11-18 時点。PARSER-002 Step 1（RunConfig 型設計とドキュメント同期）で `Parser_run_config` モジュールを追加し、`with_extension` / `find_extension` / `Legacy.bridge` など仕様準拠の API を整備。後続ステップでメトリクス登録・ドライバ連携を行う前提条件を満たした。

[^runconfig-step3-phase25]:
    2025-11-20 時点。`parser_run_config` に `Config`/`Lex`/`Recover`/`Stream` サブモジュールを追加し、`extensions` ネームスペースを型安全に読み取るシムを実装。`lex.profile` 非指定時の `ConfigTriviaProfile::strict_json` フォールバック、`recover.sync_tokens`/`notes` の `Parser_diag_state` 連携、`stream` プレースホルダの導入、および `dune build` による検証を完了。

[^runconfig-step5-phase25]:

[^phase27-handshake-spec]: Phase 2-7 残課題クローズ記録と Phase 2-8 監査準備の導線。`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` §5、`docs/plans/bootstrap-roadmap/2-7-to-2-8-handover.md` §2、`reports/audit/dashboard/diagnostics.md` を参照し、Phase 2-8 で使用する監査ベースラインと差分ログを集約した。
    2025-11-22 時点。PARSER-002 Step 5 で `compiler/ocaml/tests/run_config_tests.ml` を追加、RunConfig ゴールデン JSON（`parser-runconfig-packrat.json.golden`）を整備し、`tooling/ci/collect-iterator-audit-metrics.py` に RunConfig 用指標を実装。`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` へ `parser.runconfig_switch_coverage` / `parser.runconfig_extension_pass_rate` を登録し、CI ゲートで値を監視できる状態に移行した。
6.4. **後半フェーズの仕上げ**
- **PARSER-003 / EXEC-001 / ERR-002**: Phase 2-5 後半でコアコンビネーター抽出とストリーミング PoC、`recover` FixIt 拡張を実装し、ランナ―整合性を最終確認する。
- 2025-12-15 更新: ERR-002 Step4 で `docs/spec/2-5-error.md`・`docs/spec/3-6-core-diagnostics-audit.md` の脚注と共有ログ（`docs/plans/bootstrap-roadmap/2-5-review-log.md`）を整備し、Packrat／notes／ストリーミング重複の残課題を `docs/notes/core-parse-streaming-todo.md` と Phase 2-7 計画 §3.4 へ移管した。
- 2026-01-16 更新: EXEC-001 Step3 でストリーミング制御ループ PoC（`StreamDriver` ステートマシン、`DemandHint` 再計算、`StreamMeta` 指標集計）の設計を完了し、`Pending`/`Completed` 遷移表と計測方針を `EXEC-001` 計画書に記録した。Step4 では CLI/LSP/CI への配線とレイテンシ測定を実装する。
- 2026-01-24 更新: Step4 で CLI/LSP/CI へストリーミング PoC を接続。`--streaming` 系フラグ、`Parser_driver.Streaming`、`streaming_runner_tests.ml`、`streaming-outcome.json.golden`、`parser.stream_extension_field_coverage` を実装し、Pending→`resume`→Completed の往復を確認済み。残課題（Packrat 共有・バックプレッシャ通知・CLI メトリクス転送）は Phase 2-7 へ引き継ぎ。
- 完了後は `docs/guides/core-parse-streaming.md` や `docs/spec/2-2-core-combinator.md` に脚注・更新を反映させ、関連サンプルが動作することを確認する。

6.5. **Phase 2-7 以降に向けた準備**
- **LEXER-001 / SYNTAX-001 / SYNTAX-003 / EFFECT-002 / TYPE-002** など Phase 2-7 移行案件は、Phase 2-5 期間中に脚注整備・ロードマップ作成・必要なノート作成（`docs/notes/effect-system-tracking.md` 等）を完了する。`SYNTAX-003 S4` で整理したハンドオーバー チェックリスト H-O1〜H-O5 を Phase 2-7 計画へ転記済みであるため、進捗確認は同チェックリストに沿って実施する。
- Phase 2-7 の着手条件（XID テーブル生成、効果 PoC、効果行統合など）を `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` へ追記し、次フェーズでのエントリポイントを明確にする。

**成果物**: 更新済み修正計画、実施記録、更新されたメトリクス

### 7. ドキュメント更新の実施（33-34週目）
**担当領域**: 仕様書更新

7.1. **主文書の更新**
- 承認された修正案の反映
- サンプルコードの更新
- 図表の更新（必要に応じて）
- 脚注・TODO の追加

**更新チェックリスト（優先度順）**
- [x] `docs/spec/3-6-core-diagnostics-audit.md`・`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md`・`docs/guides/runtime-bridges.md` 間で Severity 4 値化と `DiagnosticDomain` 拡張（DIAG-001〜003）の脚注・メトリクス・ガイド記述が一致しているか再確認し、不整合がある場合は `docs/spec/3-0-core-library-overview.md` と `docs/notes/dsl-plugin-roadmap.md` も同時に更新する。
- [x] `docs/spec/1-1-syntax.md`・`docs/spec/1-3-effects-safety.md`・`docs/spec/1-5-formal-grammar-bnf.md`・`docs/notes/effect-system-tracking.md` に掲載された `Σ_before`/`Σ_after` PoC 脚注（EFFECT-002 Step4/5）と KPI (`syntax.effect_construct_acceptance`, `effects.syntax_poison_rate`) の説明が揃っているか確認し、差分があれば Phase 2-7 向けの撤去条件を明示する。
- [x] `docs/spec/2-1-parser-type.md`・`docs/spec/2-6-execution-strategy.md`・`docs/guides/core-parse-streaming.md`・`docs/notes/core-parse-api-evolution.md` で RunConfig（PARSER-002/003 Step6）に関する表記と参照リンクが一致しているか確認し、`parser-runconfig-packrat.json.golden` の更新履歴を脚注に追記する。
- [x] `docs/spec/1-2-types-Inference.md`・`docs/spec/3-6-core-diagnostics-audit.md` の `type_row_mode = "metadata-only"` 脚注（TYPE-002 Step3）と `effects.type_row.integration_blocked` KPI の説明を同一語彙に統一し、解除条件を `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` と同期させる。

7.2. **用語集・索引の更新**
- [0-2-glossary.md](../../spec/0-2-glossary.md) の用語追加・更新
- 新規概念の定義追加
- 廃止された用語の非推奨マーク
- 用語の統一チェック

**用語棚卸しタスクリスト**
- [x] `DiagnosticDomain::{Target, Plugin, Lsp, Other}` の語彙説明を用語集へ追記し、`docs/spec/3-6-core-diagnostics-audit.md` の表と参照ラベルを揃える。
- [x] `RunConfig.extensions[...]` の名称と略語（`packrat`, `recover`, `stream` 等）を索引用語へ登録し、`docs/spec/2-7-core-parse-streaming.md` から逆参照できるようにする。
- [x] 効果構文 PoC に関連する用語（`Σ_before`, `Σ_after`, `syntax.effect_construct_acceptance` など）を `docs/spec/0-2-glossary.md` と `docs/spec/0-0-overview.md` の索引用語に追加し、Phase 2-7 での昇格条件を脚注に明示する。

7.3. **サンプルコードの検証**
- 更新されたサンプルのパース検証
- 型推論結果の確認
- エラーケースの検証
- `examples/` ディレクトリとの整合

**検証観点メモ**
- RunConfig・効果 PoC・診断 JSON など Phase 2-5 で追加したサンプルは `compiler/ocaml/tests/` および `compiler/ocaml/tests/golden/diagnostics/` のゴールデンと照合し、`scripts/validate-diagnostic-json.sh`・`tooling/ci/collect-iterator-audit-metrics.py --require-success` の出力が仕様記述と一致しているか確認する。
- `examples/algebraic-effects/` など PoC サンプルが仕様の脚注と同じ条件で動作するかをチェックし、差異があれば `docs/plans/bootstrap-roadmap/2-5-review-log.md` に記録して Phase 2-7 のフォローアップへ渡す。

**成果物**: 更新された仕様書、用語集

### 8. クロス参照とリンク整備（34週目）
**担当領域**: ドキュメント整合

8.1. **索引系ドキュメントの更新**
- `README.md` の目次更新
- [0-0-overview.md](../../spec/0-0-overview.md) の概要更新
- [0-1-project-purpose.md](../../spec/0-1-project-purpose.md) の目的・方針の見直し
- [0-3-code-style-guide.md](../../spec/0-3-code-style-guide.md) のコード例更新

**整合確認ポイント**
- [x] ルート `README.md` と `docs/README.md` に Phase 2-5 で追加した KPI・脚注（DIAG-001〜003, EFFECT-002, PARSER-002/003, TYPE-002）がリンク付きで掲載されているか確認し、欠落している場合は追補する。
- [x] `docs/spec/README.md` と Chapter 0 系目次（`0-0`〜`0-3`）の脚注が最新の提案 ID・レビュー記録と一致しているか点検する。
- [x] `docs/plans/bootstrap-roadmap/README.md` の Phase 2-5 セクションに Step7-9 の進捗と参照リンクが反映されているかを確認し、更新が必要な場合は同期する。

8.2. **相互参照リンクの検証**
- 全 Markdown ファイルのリンク抽出
- リンク切れの検出と修正
- セクション参照の正確性確認
- 相対パスの統一

**リンク検証ノート**
- `scripts/validate-diagnostic-json.sh` や `tooling/ci/collect-iterator-audit-metrics.py --summary` の出力で参照する Markdown が 404 にならないか手動確認し、必要に応じてパスを更新する。
- Phase 2-5 で新設した脚注（例: `[^effects-syntax-poc-phase25]` など）が全ファイルで整合しているか確認し、撤去が完了したものは差分ログに完了理由を記録する。
- `docs/notes/`・`docs/guides/` に追加した参照（`docs/notes/effect-system-tracking.md`, `docs/guides/core-parse-streaming.md` 等）から仕様書への逆リンクが成立しているか確認する。

8.3. **ガイド・ノートの整合**
- `docs/guides/` 以下のガイド更新
- `docs/notes/` 以下の調査ノート整理
- Phase 2 実装との整合確認
- 廃止されたドキュメントの削除/非推奨化

**整合アクション**
- [x] `docs/guides/runtime-bridges.md`・`docs/guides/core-parse-streaming.md`・`docs/guides/plugin-authoring.md` で DIAG／RunConfig／効果 PoC に関する脚注と仕様参照が最新化されているか確認し、未反映の場合は脚注を更新する。
- [x] `docs/notes/effect-system-tracking.md`・`docs/notes/core-parse-api-evolution.md`・`docs/notes/dsl-plugin-roadmap.md` に記録した Phase 2-5 の進捗と `docs/plans/bootstrap-roadmap/2-5-review-log.md` のエントリが相互参照できるかを確認し、欠落していればメモを追加する。
- [x] 廃止・移設予定のノートが残っている場合は `docs/plans/repository-restructure-plan.md` と `docs-migrations.log` に補足を追記し、読者が移行経路を把握できるようにする。

**成果物**: 整合された索引、検証済みリンク

### 9. 記録と Phase 3 準備（34週目）
**担当領域**: 記録と引き継ぎ

9.1. **差分処理結果の記録**
- `0-3-audit-and-metrics.md` への記録
- 処理した差分の統計（件数、分類別）
- レビュー工数の記録
- 残存課題の明示

**記録作業の詳細**
- `diagnostic.info_hint_ratio`, `diagnostic.hint_surface_area`, `parser.runconfig_switch_coverage`, `parser.runconfig_extension_pass_rate`, `syntax.effect_construct_acceptance`, `effects.syntax_poison_rate`, `effects.type_row.integration_blocked` など Phase 2-5 で導入した KPI が `0-3-audit-and-metrics.md` に登録され、最新値・レビュー日・根拠ファイルが更新されているかを確認する。
- 差分統計とレビュー工数は `docs/plans/bootstrap-roadmap/2-5-review-log.md` に追記し、レビュアの確認印跡（担当者・日付・確認済みフラグ）が残っているか点検する。
- 集計に用いたスクリプトやゴールデン（`tooling/ci/collect-iterator-audit-metrics.py`, `scripts/validate-diagnostic-json.sh`, `compiler/ocaml/tests/golden/diagnostics/parser-runconfig-packrat.json.golden` 等）のバージョンとコマンド例を併記し、再実行時に差分比較できる状態を保つ。

9.2. **リスク管理への登録**
- 未解決の差分を `0-4-risk-handling.md` に登録
- Phase 3 で対応すべき事項の明示
- 仕様変更提案の記録
- 将来の仕様拡張検討事項

**リスク・課題メモ**
- [x] `0-4-risk-handling.md` に登録済みの `EFFECT-POC-Stage`, `TYPE-002-ROW-INTEGRATION`, `LEXER-Unicode-Stage`, `STREAMING-Capability-Sync` などの ID が最新状態（Open/Blocked/Resolved）になっているか確認し、更新が必要な場合は Phase 2-7 / Phase 3 の着手条件を追記する。
- [x] 新たに発生した差分（RunConfig 拡張、診断メタデータ語彙追加など）がリスク台帳へ登録されているか確認し、未登録の場合は根拠ファイルと暫定対応策を添えて記録する。
- [x] リスク項目と `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` のハンドオーバーチェックリスト（H-O1〜H-O5）が相互参照しているか点検し、重複管理を避ける。

9.3. **Phase 3 引き継ぎ**
- セルフホスト時の仕様参照ポイント整理
- OCaml 実装から Reml 実装への写像ガイド
- 仕様の曖昧な箇所のリスト
- レビュープロセスの改善提案

**引き継ぎ準備メモ**
- Phase 3 で参照する資料（`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md`, `docs/plans/bootstrap-roadmap/3-0-phase3-transition.md`, `docs/notes/effect-system-tracking.md`, `docs/notes/core-parse-api-evolution.md` 等）への導線を `README.md` と `docs/spec/0-0-overview.md` に明示する。
- `docs/plans/bootstrap-roadmap/2-5-review-log.md` の各エントリに Phase 3 でのフォローアップタグ（例: `Phase3:effects`, `Phase3:parser-streaming`, `Phase3:diagnostics`) を付与し、セルフホスト移行時に抽出しやすくする。
- Phase 2-5 で整備した CLI/CI ゴールデンや測定スクリプトの保管先を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` と `reports/audit/index.json` の双方に記載し、再計測手順を追記しておく。

**成果物**: 差分処理記録、リスク登録、引き継ぎ文書

## 成果物と検証
- 差分リストが公開され、レビュー記録が残っていること。
- 更新されたドキュメントが CI のリンクチェック（存在する場合）や手動確認で問題ないこと。
- 索引類が最新のリンクを指し、リンク切れがゼロであること。

## リスクとフォローアップ
- レビュー負荷が高い場合はフェーズ内で優先順位を付け、セルフホスト移行に影響する項目を先行対応。
- 新たな仕様変更案が発生した場合、Phase 3 のドキュメントフィードバックタスクと連携し調整。
- 差分が大きい場合は補足ノートを `docs/notes/` 以下に作成し、計画的に反映する。

## 参考資料
- [2-0-phase2-stabilization.md](2-0-phase2-stabilization.md)
- [0-0-overview.md](../../spec/0-0-overview.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- [0-4-risk-handling.md](0-4-risk-handling.md)

[^lexer001-phase25]: `docs/notes/dsl-plugin-roadmap.md` §7「Unicode プロファイル導入準備」および `docs/plans/bootstrap-roadmap/2-5-review-log.md` 2026-03-04 / 2026-03-21 記録を参照。Phase 2-5 Step3 で ASCII 限定ポリシーと Phase 2-7 フォローアップ TODO を確定し、Step4 で `--lex-profile=ascii|unicode` スイッチ・CI 指標の運用方針を定めた。

[^effects-syntax-poc-phase25]:
    Phase 2-5 `SYNTAX-003 S0` の結論として、効果構文は `-Zalgebraic-effects` フラグ配下の PoC ステージに据え置き、正式実装は Phase 2-7 以降に移管した。脚注は `docs/spec/1-1-syntax.md`・`docs/spec/1-5-formal-grammar-bnf.md`・`docs/spec/3-8-core-runtime-capability.md` に同期済み。
