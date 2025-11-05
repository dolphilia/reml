# 2-5 → 2-7 フェーズ引き継ぎノート

## 1. 目的と適用範囲
- Phase 2-5「仕様差分是正」で整理した成果物・未完事項を Phase 2-7 実装チームへ確実に引き継ぐ。
- 既存ハンドオーバー（`2-5-to-2-7-type-002-handover.md`）で扱っていない計画書を中心に、実装ステップ・検証条件・リスクを再掲する。
- 参照元: `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md`, `docs/plans/bootstrap-roadmap/2-5-review-log.md`, `docs/plans/bootstrap-roadmap/2-5-proposals/`.

## 2. フェーズ総括
- Phase 2-5 では差分可視化（脚注・索引更新）と KPI 定義を完了し、Unicode/効果関連は実装を Phase 2-7 に移管する運用ガードを配置済み。
- `collect-iterator-audit-metrics.py` / `scripts/validate-diagnostic-json.sh` には各計画向けのプレースホルダ指標が追加済みだが、多くが監視モード（`pass_rate = null`）で止まっている。
- 仕様側の脚注は Phase 2-7 成果を前提に撤去する条件を明示しており、解除には KPI 1.0 達成とレビュー記録更新が必要。

## 3. 優先引き継ぎ案件

### 3.1 LEXER-001 Unicode プロファイル（`docs/plans/bootstrap-roadmap/2-5-proposals/LEXER-001-proposal.md`）
- **現状**: ASCII 限定挙動の棚卸し（Step1）と仕様脚注の整備（Step2〜4）を完了。CI には `--lex-profile=ascii|unicode` スイッチが設計され、`lexer.identifier_profile_unicode` 指標が監視中。
- **Phase 2-7 タスク**:
  - `Core.Parse.Lex` API と `IdentifierProfile` 構造体の確定（LEXER-002 と共同、Step5）。
  - Unicode データ生成パイプライン（UAX #31/#39, NFC 検査）の実装と CI 組み込み（Step6）。
  - `RunConfig.lex.identifier_profile` フラグを実装し、CLI/LSP/ストリーミングでの互換モードと診断検証を整備（Step7）。
  - `collect-iterator-audit-metrics.py` の `lexer.identifier_profile_unicode` を PASS 運用へ昇格し、CI で 1.0 を達成させる。
- **ゲート／リスク**: `0-4-risk-handling.md` 登録「Unicode XID 識別子実装未完了」（期限: 2026-08-31）。`docs/spec/1-1-syntax.md`/`2-3-lexer.md` 脚注 `[^lexer-ascii-phase25]` 撤去は KPI 達成後にレビュー合意が必要。

### 3.2 SYNTAX-001 Unicode 識別子脚注（`docs/plans/bootstrap-roadmap/2-5-proposals/SYNTAX-001-proposal.md`）
- **現状**: Chapter 1/1-5/索引/用語集の脚注同期とリスク登録を完了。ASCII 限定テストはゴールデン化済み。
- **Phase 2-7 タスク**:
  - XID テーブル生成スクリプトとビルドフローの決定（Step5）を LEXER-001 と共通化。
  - CLI/LSP/診断での Unicode 互換表示（Span, ハイライト）の再検証、およびサンプル更新（Step6）。
  - `lexer.identifier_profile_unicode` が 1.0 になるタイミングで脚注撤去と索引更新をセット運用。
- **ゲート／リスク**: `0-4-risk-handling.md` の同一リスクに統合管理。`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に登録済みの「Unicode 識別子プロファイル移行」セクションで進捗を追跡。

### 3.3 SYNTAX-003 効果構文 PoC（`docs/plans/bootstrap-roadmap/2-5-proposals/SYNTAX-003-proposal.md`）
- **現状**: S0〜S4 で PoC ステージ整理・パーサ挿入位置設計・診断/KPI 設計を完了。脚注 `[^effects-syntax-poc-phase25]` を Chapter 1/1-5/3-8 へ適用済み。
- **Phase 2-7 タスク**:
  - `parser.mly` への `perform_expr`/`handle_expr` 実装と `parser_run_config` `experimental_effects` フラグ導入（S1/S2 実装フェーズ）。
  - `Type_inference_effect` と `effect_analysis` の PoC 実装（単一タグ捕捉・`Σ_before/after` 記録）および `syntax.effect_construct_acceptance`/`effects.syntax_poison_rate` KPI 算出ロジックの実装。
  - ゴールデン・CI・監査フロー（`collect-iterator-audit-metrics.py --section effects`, `scripts/validate-diagnostic-json.sh`）を PoC 仕様に合わせて更新。
  - `-Zalgebraic-effects` フラグ命名確定と CLI/LSP/ビルド経路の統一。
- **ゲート／リスク**: `0-4-risk-handling.md` 登録「効果構文 Stage 昇格遅延」（期限: 2026-09-30）。脚注撤去は KPI 達成と `docs/notes/effect-system-tracking.md` H-O1〜H-O5 完了が条件。

### 3.4 EFFECT-002 効果操作 PoC（`docs/plans/bootstrap-roadmap/2-5-proposals/EFFECT-002-proposal.md`）
- **現状**: PoC スコープ棚卸・診断/KPI 設計（Step1〜4）と脚注連携を完了。`extensions.effects.sigma.*` 出力仕様と JSON サンプルを定義。
- **Phase 2-7 タスク**:
  - AST/Typed AST/IR で `perform`/`handle` ノードを実装し、`Σ_before`→`Σ_after` の残余効果計算を `Type_inference` へ統合。
  - `diagnostic.ml` / `diagnostic_serialization.ml` に `effects.syntax.constructs` と `sigma` 系フィールドの実出力を追加。
  - `compiler/ocaml/tests/effect_handler_poc_tests.ml`（新設）で PoC ケースを固定し、`collect-iterator-audit-metrics.py` の KPI を 1.0 / 0.0 運用に移行。
  - `EFFECT-003`（複数 Capability）、`TYPE-002`（効果行統合）との連携スケジュールを `2-7-deferred-remediation.md` で同期。
- **ゲート／リスク**: `0-4-risk-handling.md` の「効果構文 Stage 昇格遅延」と連動。`syntax.effect_construct_acceptance` ≥ 1.0、`effects.syntax_poison_rate` = 0.0 が脚注解除条件。

### 3.5 TYPE-002 効果行統合（参考）
- 専用ハンドオーバー（`docs/plans/bootstrap-roadmap/2-5-to-2-7-type-002-handover.md`）を優先参照。EFFECT-002/SYNTAX-003 の完了条件に `type_row_mode` の切替と KPI 3 種（`diagnostics.effect_row_stage_consistency`, `type_effect_row_equivalence`, `effect_row_guard_regressions`）が含まれるため、スプリント計画の整合を随時確認すること。

## 4. 追加フォローアップ項目（抜粋）
- **TYPE-001 値制限モード**（`docs/plans/bootstrap-roadmap/2-5-proposals/TYPE-001-proposal.md`）: RunConfig の `value_restriction_mode` は導入済み。Phase 2-7 `execution-config` で Strict 既定化と Legacy 通知の実装、`type_inference.value_restriction_violation` 指標の 0 件維持を監視。
- **DIAG-002/003 ダッシュボード更新**（`docs/plans/bootstrap-roadmap/2-5-proposals/DIAG-002-proposal.md`, `.../DIAG-003-proposal.md`）: `phase2.5.audit.v1` テンプレートと Domain 拡張は完了済み。Phase 2-7 `diagnostics` チームはダッシュボード改修とメタデータ整合テストを引き継ぎ、`collect-iterator-audit-metrics.py --require-success` で 1.0 を維持する。
- **ERR-002 回復系 FixIt**（`docs/plans/bootstrap-roadmap/2-5-proposals/ERR-002-proposal.md`）: CLI/LSP の自動修正インターフェイス刷新を Phase 2-7 `diagnostics` と連携して実装し、Packrat 経路のカバレッジと翻訳整備を `docs/notes/core-parse-streaming-todo.md` に従って完了させる。
- **監査メトリクス未達成項目**: `docs/plans/bootstrap-roadmap/2-4-completion-report.md` で 0.0 のまま残る `ffi_bridge.audit_pass_rate` / `iterator.stage.audit_pass_rate` は Phase 2-7 `deferred-remediation` セクションで追跡。差分検出時は `0-3-audit-and-metrics.md` と連携して指標定義を更新する。

## 5. リスクと KPI 監視
- `Unicode XID 識別子実装未完了`（期限 2026-08-31）: KPI `lexer.identifier_profile_unicode = 1.0` 達成と脚注撤去が完了条件。
- `効果構文 Stage 昇格遅延`（期限 2026-09-30）: KPI `syntax.effect_construct_acceptance = 1.0` / `effects.syntax_poison_rate = 0.0`、`-Zalgebraic-effects` の公開名確定が必須。
- `効果行統合遅延`（期限 2026-10-31）: `type_row_mode` を `ty-integrated` へ移行し、KPI 3 種の達成と脚注撤去を確認（2026-12-18 に完了、リスククローズ済み）。
- 進捗は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` と `docs/notes/effect-system-tracking.md` のチェックリストで月次レビューを実施。

## 6. Phase 2-7 初期アクションチェックリスト
1. キックオフ週に `LEXER-001`/`SYNTAX-001`/`SYNTAX-003`/`EFFECT-002`/`TYPE-002` 共通レビュー会（`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` 参照）を開催し、API 境界とスプリント順序を確定。
2. `collect-iterator-audit-metrics.py` と `scripts/validate-diagnostic-json.sh` の Phase 2-7 ブランチを準備し、各 KPI を `--require-success` で実行可能な状態に引き上げる。
3. 仕様脚注・索引の撤去条件と照合ポイントを週次レビューで共有し、KPI 達成時には `docs/spec/README.md` 更新とリスク台帳のステータス変更を同時に行う運用を定義。
4. 既存ハンドオーバー（`2-5-to-2-7-type-002-handover.md`）と本ノートの参照リンクを `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` から辿れることを確認し、更新が発生した場合は差分を `docs/plans/bootstrap-roadmap/2-5-review-log.md` に追記する。
