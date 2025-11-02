# 2.7 診断パイプライン残課題・技術的負債整理計画

## 目的
- Phase 2-4 で持ち越した診断・監査パイプライン関連タスクと技術的負債（ID 22/23 など）を集中して解消する。
- CLI/LSP/CI の各チャネルで `Diagnostic` / `AuditEnvelope` の新仕様を安定運用できる状態を整え、Phase 2-8 の仕様検証に備える。

## スコープ
- **含む**: Windows/macOS CI での監査ゲート導入、LSP V2 互換テスト整備、CLI フォーマッタの再統合、技術的負債リストで Phase 2 中に解消可能な項目。
- **含まない**: 仕様書の全文レビュー（Phase 2-8 で実施）、新規機能の追加、Phase 3 以降へ移送済みの低優先度負債。
- **前提**:
  - Phase 2-4 の共通シリアライズ層導入と JSON スキーマ検証が完了していること。
  - Phase 2-5 の仕様差分補正で参照する基礎データ（差分リスト草案）が揃っていること。
  - Phase 2-6 の Windows 実装で `--emit-audit` を実行できる環境が CI 上に整備済みであること。

## 作業ディレクトリ
- `compiler/ocaml/src/cli/` : `diagnostic_formatter.ml`, `json_formatter.ml`, `options.ml`
- `compiler/ocaml/src/diagnostic_*` : Builder/API 互換レイヤ
- `tooling/lsp/` : `diagnostic_transport.ml`, `compat/`, `tests/client_compat`
- `tooling/ci/` : `collect-iterator-audit-metrics.py`, `sync-iterator-audit.sh`, 新規検証スクリプト
- `scripts/` : CI 向け検証スクリプト、レビュー補助ツール
- `reports/` : 監査ログサマリ、診断フォーマット差分
- `compiler/ocaml/docs/technical-debt.md` : ID 22/23, H1〜H4 の進捗更新

## 作業ブレークダウン

### 1. 監査ゲート整備（34-35週目）
**担当領域**: Windows/macOS CI

1.1. **Windows Stage 自動検証 (ID 22)**
- `tooling/ci/sync-iterator-audit.sh` を MSYS2 Bash で動作させ、`--platform windows-msvc` 実行パスを整備。
- `tooling/ci/collect-iterator-audit-metrics.py` に Windows プラットフォーム専用プリセット (`--platform windows-msvc`) を追加し、`ffi_bridge.audit_pass_rate` / `iterator.stage.audit_pass_rate` を算出。
- `bootstrap-windows.yml` に `audit-matrix` ジョブを追加し、pass_rate < 1.0 の場合は PR を失敗させる。
- `reports/ffi-bridge-summary.md` と `docs/plans/bootstrap-roadmap/2-3-to-2-4-handover.md` の TODO 欄を更新。
- DIAG-002 で追加した `diagnostic.audit_presence_rate` をダッシュボードへ組み込み、`python3 tooling/ci/collect-iterator-audit-metrics.py --require-success` の結果を Windows 行にも掲載する（ソース: `compiler/ocaml/tests/golden/diagnostics/**/*.json.golden` / `compiler/ocaml/tests/golden/audit/**/*.json[l].golden`）。

1.2. **macOS FFI サンプル自動検証 (ID 23)**
- `ffi_dispatch_async.reml` / `ffi_malloc_arm64.reml` をビルド可能なよう修正し、`scripts/ci-local.sh --target macos-arm64 --emit-audit` に組み込む。
- `collect-iterator-audit-metrics.py` で `bridge.platform = macos-arm64` の pass_rate 集計を追加し、`ffi_bridge.audit_pass_rate` に反映。
- `bootstrap-macos.yml` に監査ゲートを追加し、成果物 (audit JSON, summary) をアーティファクト化。

**成果物**: Windows/macOS CI 監査ゲート、更新済みレポート、技術的負債リスト反映

### 2. CLI 出力統合とテキストフォーマット刷新（35週目前半）
**担当領域**: CLI フォーマッタ

2.1. **`--format` / `--json-mode` 集約**
- `compiler/ocaml/src/cli/options.ml` で `--format` と `--json-mode` の派生オプションを整理し、`SerializedDiagnostic` を利用するフォーマッタ選択ロジックを再構築。
- `docs/spec/0-0-overview.md` と `docs/guides/ai-integration.md` に新オプションを追記。

2.2. **テキストフォーマット刷新**
- `compiler/ocaml/src/cli/diagnostic_formatter.ml` を `SerializedDiagnostic` ベースへ移行し、`unicode_segment.ml`（新規）を導入して Grapheme 単位のハイライトを実装。
- `--format text --no-snippet` を追加し、CI 向けログを簡略化。
- テキストゴールデン (`compiler/ocaml/tests/golden/diagnostics/*.golden`) を更新し、差分は `reports/diagnostic-format-regression.md` に記録。

**成果物**: CLI オプション整理、テキストフォーマッタ更新、ドキュメント追記

### 3. LSP V2 互換性確立（35週目後半）
**担当領域**: LSP・フロントエンド

3.1. **フィクスチャ拡充とテスト**
- `tooling/lsp/tests/client_compat/fixtures/` に効果診断・Windows/macOS 監査ケースを追加し、AJV スキーマ検証を更新。
- `npm run ci` にフィクスチャ差分のレポート出力を追加し、PR で参照可能にする。

3.2. **`lsp-contract` CI ジョブ**
- GitHub Actions に `lsp-contract` ジョブを追加し、V1/V2 双方の JSON を `tooling/json-schema/diagnostic-v2.schema.json` で検証。
- `tooling/lsp/README.md` と `docs/guides/plugin-authoring.md` に V2 連携手順を追記。

3.3. **互換レイヤ仕上げ**
- `tooling/lsp/compat/diagnostic_v1.ml` を安定化させ、`[@deprecated]` 属性を付与。
- `tooling/lsp/jsonrpc_server.ml` で `structured_hints` の `command`/`data` 変換エラーを `extensions.lsp.compat_error` に記録。

3.4. **Recover FixIt 継続整備**
- `Parser_expectation.Packrat` に `recover` スナップショットを保持するハンドルを追加し、Packrat 経路でも `parser.recover_fixit_coverage = 1.0` を維持する。検証手順と残課題は `docs/notes/core-parse-streaming-todo.md` に追記済み。
- `Diagnostic.Builder.add_note` が生成する `recover` notes をローカライズ可能なテンプレートへ移行し、CLI/LSP のテキスト刷新と連動して多言語化を完了させる。`docs/spec/2-5-error.md`・`docs/spec/3-6-core-diagnostics-audit.md` の脚注と整合させる。
- ストリーミング Pending → resume 循環で FixIt が重複発火しないことを監査ログ (`StreamOutcome.Pending.extensions.recover`) と `collect-iterator-audit-metrics.py` の新指標で確認する。必要に応じて CI に検証ステップを追加する。

**成果物**: 拡充済み LSP テスト群、CI ジョブ、更新ドキュメント

### 4. 技術的負債の棚卸しとクローズ（36週目前半）
**担当領域**: 負債管理

4.1. **技術的負債リスト更新**
- `compiler/ocaml/docs/technical-debt.md` で ID 22 / 23 を完了扱いに更新し、H1〜H4 の進捗をレビュー。
- Phase 2 以内に解消できなかった項目を Phase 3 へ移送し、`0-4-risk-handling.md` に直結するリスクとして記録。

4.2. **レポート更新**
- `reports/diagnostic-format-regression.md` と `reports/ffi-bridge-summary.md` に完了状況を追記し、差分がないことを確認。
- 監査ログの成果物パスを `reports/audit/index.json` に登録し、`tooling/ci/create-audit-index.py` のテストを更新。

**成果物**: 最新化された技術的負債リスト、報告書更新、移送リスト

### 5. Phase 2-8 への引き継ぎ準備（36週目後半）
**担当領域**: ドキュメント整備

5.1. **差分記録**
- Phase 2-4, 2-7 で実施した変更点・残項目を `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` の前提セクションへ追記。
- 監査ログ/診断の安定化完了を `docs/plans/bootstrap-roadmap/2-8-spec-integrity-audit.md`（新規）から参照できるよう脚注を整備。

5.2. **メトリクス更新**
- `0-3-audit-and-metrics.md` に CI pass_rate の推移と LSP テスト完了状況を記録。
- `tooling/ci/collect-iterator-audit-metrics.py` の集計結果を `reports/audit/dashboard/` に反映し、Phase 2-8 のベースラインとする。
- DIAG-003 Step5 で追加された `diagnostics.domain_coverage` / `diagnostics.plugin_bundle_ratio` / `diagnostics.effect_stage_consistency` をダッシュボードへ掲載し、`Plugin` / `Lsp` / `Capability` ドメインの Stage 連携が視覚化されるようグラフとしきい値を設計する（`docs/spec/3-6-core-diagnostics-audit.md` 脚注参照）。

**成果物**: 更新済み前提資料、メトリクス記録、Phase 2-8 用脚注

### 6. ストリーミング PoC フォローアップ（Phase 2-7 序盤）
**担当領域**: Core.Parse.Streaming / Runtime Bridge

- **Packrat キャッシュ共有**: `Parser_driver.Streaming` がチャンク処理時に `Parser_driver.run` へ委譲する PoC 構造を見直し、`Core_parse.State` のメモ化領域を継続に含める。`parser.stream.outcome_consistency` が 1.0 未満の場合は差分レポートを `reports/audit/dashboard/streaming.md` へ記録する。
- **バックプレッシャ自動化**: `FlowController.policy=Auto` を CLI/LSP から選択できるよう `RunConfig.extensions["stream"].flow` を拡張し、`demand_min_bytes` / `demand_preferred_bytes` と `PendingReason::Backpressure` を同期させる。完了時に `docs/guides/core-parse-streaming.md` §10 の制限項目をクローズする。
- **Pending/Error 監査**: `StreamEvent::Pending` / `StreamEvent::Error` を `AuditEnvelope` 経由で `parser.stream.pending` / `parser.stream.error` へ転送し、`resume_hint` / `last_reason` / `continuation.meta.last_checkpoint` を必須フィールドとして検証する。`parser.stream.demandhint_coverage` を 1.0 で維持。
- **CLI メトリクス連携**: `Cli.Stats` と JSON 出力に `stream_meta`（`bytes_consumed` / `resume_count` / `await_count`）を出力し、`collect-iterator-audit-metrics.py --require-success` が値を集計できるようにする。
- **Runtime Bridge 連携**: `docs/guides/runtime-bridges.md` にストリーミング信号（`DemandHint`, backpressure hooks）を Runtime Bridge へ渡す手順と、`effects.contract.stage_mismatch` 拡張キーの同期方法を追記する。

### 7. Unicode 識別子プロファイル移行（SYNTAX-001 / LEXER-001）
**担当領域**: Lexer / Docs / Tooling

7.1. **XID テーブル整備**
- `scripts/` 配下に UnicodeData 由来の `XID_Start` / `XID_Continue` テーブル生成スクリプトを追加し、CI キャッシュとライセンス整備を実施する。生成物は `compiler/ocaml/src/lexer_tables/`（新設予定）で管理し、`dune` の `@check-unicode-tables` で再生成チェックを行う。
- `compiler/ocaml/src/lexer.mll` と `Core_parse.Lex` に新テーブルを組み込み、`--lex-profile=unicode` を既定へ移行する段階的ロードマップを作成する。ASCII プロファイルは互換モードとして残し、切り替え手順を `docs/spec/2-3-lexer.md` に記載する。

7.2. **テストとメトリクス**
- CI で `REML_ENABLE_UNICODE_TESTS=1` を常時有効化し、`compiler/ocaml/tests/unicode_ident_tests.ml` と `unicode_identifiers.reml` フィクスチャを全プラットフォームで実行する。`collect-iterator-audit-metrics.py --require-success` の `parser.runconfig.lex.profile` 集計で `unicode` が 100% となることを確認する。
- `lexer.identifier_profile_unicode` 指標が 1.0 へ遷移した日付とログを `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に追記し、値が下回った場合は `0-4-risk-handling.md` のリスクを更新する。

7.3. **ドキュメントとクライアント整備**
- `docs/spec/1-1-syntax.md`・`docs/spec/1-5-formal-grammar-bnf.md` の暫定脚注を撤去し、Unicode 識別子仕様への更新内容を `docs/spec/0-2-glossary.md` と `docs/spec/README.md` に波及させる。
- CLI/LSP のエラーメッセージから ASCII 制限文言を除去し、Unicode 識別子が正しく表示されることを `compiler/ocaml/tests/golden/diagnostics` と `tooling/lsp/tests/client_compat` で検証する。`docs/guides/plugin-authoring.md` と `docs/notes/dsl-plugin-roadmap.md` のチェックリストを更新する。
- `docs/plans/bootstrap-roadmap/2-5-proposals/SYNTAX-001-proposal.md` Step5/6 の進捗を反映し、完了後は Phase 2-8 へ脚注撤去タスクを引き継ぐ。

**成果物**: Unicode プロファイル既定の lexer/parser、更新済みテスト・CI 指標、仕様およびガイドの脚注整理

### 8. 効果構文 PoC 移行（SYNTAX-003 / EFFECT-002）
**担当領域**: 効果システム / CLI / CI

8.1. **PoC 実装の統合**
- `parser.mly` に `perform` / `do` / `handle` を受理する規則を導入し、`Type_inference_effect` へ `TEffectPerform` / `TEffectHandle`（仮称）を追加する。PoC 設計（Phase 2-5 S1/S2）を反映し、`Σ_before` / `Σ_after` の差分が残余効果診断へ渡ることを確認する。
- `compiler/ocaml/tests/effect_syntax_tests.ml` を新設し、成功ケース・未捕捉ケース・Stage ミスマッチケースをゴールデン化する。`collect-iterator-audit-metrics.py --section effects` で `syntax.effect_construct_acceptance = 1.0`、`effects.syntax_poison_rate = 0.0` を期待値としてゲート化する。
- `tooling/ci/collect-iterator-audit-metrics.py` に effect 指標の集計関数を実装し、`--require-success` 時には両指標が 1.0 でない場合に失敗するようガードを追加する。逸脱時は `0-4-risk-handling.md` へ登録。

8.2. **フラグ運用とドキュメント**
- `-Zalgebraic-effects`（仮称）を CLI/LSP/ビルドスクリプトで共通制御する。CLI オプションは `compiler/ocaml/src/cli/options.ml`、LSP は `tooling/lsp/tests/client_compat/fixtures/` で検証し、ビルドスクリプトは `scripts/validate-diagnostic-json.sh` や CI 定義に Experimental フラグを反映する。
- 仕様書 (`docs/spec/1-1-syntax.md`・`1-5-formal-grammar-bnf.md`・`3-8-core-runtime-capability.md`) と索引 (`docs/spec/README.md`) に付与した脚注 `[^effects-syntax-poc-phase25]` の撤去条件を整理し、Stage = Stable へ到達した後に Phase 2-8 へ通知する運用を確立する。
- `docs/notes/dsl-plugin-roadmap.md` に効果ハンドラと Capability Stage の整合チェックを追加し、`effects.contract.stage_mismatch` / `bridge.stage.*` 診断が PoC 実装で再現できることを検証する。

8.3. **ハンドオーバーとレビュー**
- `docs/notes/effect-system-tracking.md` の「Phase 2-5 S4 引き継ぎパッケージ」に沿って、PoC 到達条件と残課題を確認。チェックリスト H-O1〜H-O5 が完了した時点で `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` に更新メモを残す。
- 週次レビューで効果構文の Stage 遷移を報告し、`syntax.effect_construct_acceptance` / `effects.syntax_poison_rate` の推移を `0-3-audit-and-metrics.md` へ記録する。脚注撤去可否は Phase 2-7 終盤のレビューで判断する。

**成果物**: 効果構文 PoC 実装、CI メトリクス 100% 化、フラグ運用指針、脚注撤去条件の整理

## 成果物と検証
- Windows/macOS CI で `ffi_bridge.audit_pass_rate` / `iterator.stage.audit_pass_rate` が 1.0 を維持し、監査欠落時にジョブが失敗すること。
- CLI `--format` / `--json-mode` の整合が取れており、テキスト・JSON 双方のゴールデンが更新済みであること。
- LSP V2 の互換テストが `npm run ci` および GitHub Actions `lsp-contract` で成功し、フィクスチャ差分がレポートとして残ること。
- 効果構文の PoC 実装を有効化した状態で `collect-iterator-audit-metrics.py --require-success` が `syntax.effect_construct_acceptance = 1.0`、`effects.syntax_poison_rate = 0.0` を満たし、CLI/LSP/監査ログに `effects.contract.*` 診断が出力されること。
- 技術的負債リストと関連レポートに最新状況が反映され、Phase 3 へ移送する項目が明確になっていること。

## リスクとフォローアップ
- CI 監査ゲート導入によるジョブ時間増大: 実行時間を監視し、10% 超過時はサンプル数の調整や並列化を検討。
- CLI フォーマット変更による開発者体験への影響: `reports/diagnostic-format-regression.md` で差分レビューを必須化し、顧客影響を評価。
- LSP V2 導入に伴うクライアント側調整: `tooling/lsp/compat/diagnostic_v1.ml` を一定期間維持し、互換性レイヤ廃止時のスケジュールを Phase 3 で検討。
- PARSER-003 Step5 連携: Packrat キャッシュ実装後に `effect.stage.*`／`effect.capabilities[*]` が欠落しないことを CI で確認するため、`tooling/ci/collect-iterator-audit-metrics.py --require-success` に Packrat 専用チェックを追加する（Stage 監査テストケースを新設）。  
- Recover 拡張: §3.4 で定義した Packrat カバレッジ・notes ローカライズ・ストリーミング重複検証を遅延させず実施する。`RunConfig.extensions["recover"].notes` を CLI/LSP 表示へ反映し、`Diagnostic.extensions["recover"]` の多言語テンプレートを `docs/spec/2-5-error.md` 脚注と同期させる。
- PARSER-003 Step6 連携: `Core_parse` モジュールのテレメトリ統合と Menhir 完全置換の是非を評価し、`parser.core_comb_rule_coverage` / `parser.packrat_cache_hit_ratio` を利用した監査ダッシュボード拡張を決定する。仕様更新時は `docs/spec/2-2-core-combinator.md` 脚注と `docs/guides/plugin-authoring.md` / `core-parse-streaming.md` の共有手順を再検証する。
- 効果構文の Stage 遷移: `syntax.effect_construct_acceptance` が 1.0 未満、または CLI/LSP で `-Zalgebraic-effects` の挙動が不一致になった場合は Phase 2-7 のクリティカルリスクとして即時エスカレーションする。Stage 遷移が遅延する場合、Phase 2-8 の仕様凍結に影響するため優先度を再評価する。

## 参考資料
- [2-4-diagnostics-audit-pipeline.md](2-4-diagnostics-audit-pipeline.md)
- [2-3-to-2-4-handover.md](2-3-to-2-4-handover.md)
- [2-5-spec-drift-remediation.md](2-5-spec-drift-remediation.md)
- [2-6-windows-support.md](2-6-windows-support.md)
- [compiler/ocaml/docs/technical-debt.md](../../../compiler/ocaml/docs/technical-debt.md)
- [reports/diagnostic-format-regression.md](../../../reports/diagnostic-format-regression.md)
- [reports/ffi-bridge-summary.md](../../../reports/ffi-bridge-summary.md)
