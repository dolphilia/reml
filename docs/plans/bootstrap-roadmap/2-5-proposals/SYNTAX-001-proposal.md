# SYNTAX-001 Unicode 識別子制約の暫定整備計画

## 1. 背景と症状
- Chapter 1 では識別子を `XID_Start + XID_Continue*`（Unicode 準拠）と定義しているが（docs/spec/1-1-syntax.md:22-34）、実装の `lexer.mll` は ASCII + `_` の暫定実装に留まっている（compiler/ocaml/src/lexer.mll:46-52）。  
- Unicode 識別子を用いた仕様サンプルが OCaml 実装で拒否され、Chapter 3 の多言語サンプルや DSL 仕様の検証が進まない。  
- 技術的負債リスト ID 22/23（Windows Stage / macOS FFI）の監視項目に Unicode Lexer 拡張が含まれており、Phase 2-7 での対応が前提になっている。

## 2. Before / After
### Before
- 仕様と実装の差分が明示されておらず、Unicode 識別子が利用できるか判断できない。  
- `lexer.mll` では ASCII 制約のコメントのみが残り、Unicode 対応への進捗が共有されていない。

### After
- Chapter 1 に「Phase 2 時点では ASCII + `_` の暫定実装。Unicode XID は Phase 2-7 `lexer-unicode` タスクで導入予定」と脚注を追加し、仕様読者に現状を伝える。  
- `lexer.mll` の Unicode TODO を Phase 2-7 と連携した実装計画に更新し、XID テーブル生成や正規化チェックの導入計画を記録する。  
- `docs/notes/dsl/dsl-plugin-roadmap.md` に Unicode 化のチェック項目を追記し、DSL/プラグイン移行時に同じ制約を共有する。

## 3. 影響範囲と検証
- **テスト**: Unicode 識別子を含むサンプルを `compiler/ocaml/tests/unicode_ident_tests.ml`（新設）に追加し、ASCII 実装ではスキップ、Unicode 実装後に有効化する。  
- **メトリクス**: `0-3-audit-and-metrics.md` に登録済みの `lexer.identifier_profile_unicode` を更新し、Phase 2-5 では 0.0 を記録、Phase 2-7 `lexer-unicode` 完了時に 1.0 を目標とする。  
- **仕様・ガイド**: `docs/spec/1-5-formal-grammar-bnf.md`・`docs/spec/0-2-glossary.md`・`docs/guides/dsl/plugin-authoring.md` 等に ASCII 制限の暫定注意書きと Unicode 対応予定を追記し、索引（`docs/spec/README.md`）のリンク整合を確認する。

## 4. 実施ステップと調査項目

### Phase 2-5（差分可視化と仕様整備）

#### Step 1: Chapter 1/1-5 差分棚卸（週31・Docs/Compiler合同）
- **目的**: `docs/spec/1-1-syntax.md` と `docs/spec/1-5-formal-grammar-bnf.md` における Unicode 定義と実装 (`compiler/ocaml/src/lexer.mll`) の乖離を明示する。  
- **作業**: 既存脚注 `[^lexer-ascii-phase25]` の適用範囲を確認し、`docs/plans/bootstrap-roadmap/2-5-review-log.md` に専用節（SYNTAX-001 Step1）を追加して再現手順と影響箇所を記録する。`compiler/ocaml/tests/test_lexer.ml` へ ASCII 限定挙動の再現テストを追加し、現状の拒否メッセージを保存する。  
- **調査**: `docs/spec/1-4-test-unicode-model.md` の参照条件と `core_parse` 経路の挙動を確認し、Normalization/Bidi 要件が Phase 2-5 で未検証である点を整理する。  
- **成果物**: 差分記録、レビューエントリ、`2-5-spec-drift-remediation.md` の表更新。

##### Step1 実施記録（2025-11-02）
1. 仕様差分の棚卸  
   - Chapter 1 は XID ベースの識別子を前提としながら脚注 `[^lexer-ascii-phase25]` が §A.3 のみで Chapter 1.5 には未適用であることを確認（`docs/spec/1-1-syntax.md:27-43`、`docs/spec/1-5-formal-grammar-bnf.md:279-295`）。Step2 で BNF 側にも脚注を波及させる必要がある。  
2. 実装現状の記録  
   - Lexer が ASCII のみを `xid_start/xid_continue` で受理し、想定外文字を単一バイトとして `Lexer_error` に落とす挙動を整理（`compiler/ocaml/src/lexer.mll:77-84`、`compiler/ocaml/src/lexer.mll:231-234`）。`Core_parse.Lex` は Trivia プロファイル同期のみで Unicode プロファイルを迂回している点も確認（`compiler/ocaml/src/core_parse_lex.ml:1-160`）。  
3. 再現テストとログ整備  
   - `compiler/ocaml/tests/test_lexer.ml:55-229` に Unicode 変数名を含む入力で `Unexpected character: <0xE8>` が発生すること、Span が `4-5` で固定されることをゴールデン化。  
   - `docs/plans/bootstrap-roadmap/2-5-review-log.md` に SYNTAX-001 Step1 エントリを追加し、差分・未検証事項・フォローアップを記録。  
4. フォローアップ  
   - `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` の差分表へ現状を反映し、`0-3-audit-and-metrics.md` の `lexer.identifier_profile_unicode` 指標更新と `0-4-risk-handling.md` への登録を Step4 で実施する。  

#### Step 2: 仕様脚注・索引・用語集の整備（週31-32・Docsチーム）
- **目的**: 仕様読者が ASCII 制限と今後の計画を把握できる状態にする。  
- **作業**: `docs/spec/1-5-formal-grammar-bnf.md` に Phase 2-5 時点の脚注を追加し、`docs/spec/0-2-glossary.md` へ「Unicode identifier profile (暫定)」の定義を追記。`docs/spec/README.md`・`README.md` の索引更新、`docs/plans/repository-restructure-plan.md` に沿ったリンク確認を実施する。  
- **調査**: リンク切れ検証、脚注記号の重複確認、既存の `LEXER-001` 記録との整合性を確認する。  
- **成果物**: 更新済み仕様・索引・用語集、脚注採番表。

##### Step2 実施記録（2026-02-24）
1. 仕様脚注の同期  
   - `docs/spec/1-5-formal-grammar-bnf.md` の `Ident` 産出規則へ `[^lexer-ascii-phase25]` を付与し、Phase 2-5 時点で ASCII (`[A-Za-z0-9_]+`) を暫定プロファイルとする旨を明示した。  
   - 脚注本文を `SYNTAX-001 Step2` として更新し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の `lexer.identifier_profile_unicode` 指標および `docs/plans/repository-restructure-plan.md` のリンク整合ガイドを参照する形で整理した。
2. 用語集と索引の更新  
   - `docs/spec/0-2-glossary.md` に「Unicode 識別子プロファイル（暫定）」エントリを追加し、`LEXER-001` / `SYNTAX-001` 両計画の脚注整備を記録。  
   - `docs/spec/README.md`・ルート `README.md` に Unicode 識別子の暫定運用導線を追記し、仕様索引とトップレベル導線が同じ注意書きを共有する状態を確認した。
3. 整合性チェック  
   - リンク切れ検証を `markdown-link-check` 想定リストに沿って手動確認し、脚注重複が無いことを確認。  
   - `docs/plans/repository-restructure-plan.md` の Phase 2-5 フェーズ条件に照らし、リンクプレフィックスや章構成に変更が生じていないことを確認（追加作業は不要）。

#### Step 3: 検証用サンプルとテストシナリオ整備（週32・Compiler/Testing）
- **目的**: Unicode 実装後に即座に回帰検証できるテスト土台を整える。  
- **作業**: `compiler/ocaml/tests/unicode_ident_tests.ml` を新設し、`REML_ENABLE_UNICODE_TESTS` 環境変数で実行可否を切り替えるスキップ機構を組み込む。`docs/spec/1-1-syntax.md` の例や Chapter 3 多言語サンプルをベースに、文字種別（日本語・ハングル・合成文字）を網羅する入力を準備する。  
- **調査**: `UnicodeData.txt` / `DerivedCoreProperties.txt` を参照し、`XID_Start`/`XID_Continue` の境界ケース（結合文字・サロゲート・ZWNJ/ZWJ）を抽出する。`technical-debt.md`（ID 2b/22/23）の想定ケースと突合して不足がないか確認。  
- **成果物**: スキップ可視化されたテスト群、サンプル入力、レビュー記録の追記。

##### Step3 実施記録（2026-02-24）
1. テストシナリオの整備  
   - `compiler/ocaml/tests/unicode_ident_tests.ml` を作成し、`REML_ENABLE_UNICODE_TESTS` 環境変数が有効な場合のみ Unicode 受理テストを実行するガードを追加。デフォルトでは `[unicode_pending]` メッセージを出力してスキップし、Phase 2-7 `lexer-unicode` 実装後に即座に有効化できるようにした。  
   - 日本語・ギリシャ語・キリル・ハングル・アラビア語（ZWJ）・合成文字（`cafe\u{0301}`）など、docs/spec/1-1-syntax.md §A.3 および docs/spec/1-4-test-unicode-model.md §K の要件をカバーするケースを `acceptance_cases` として列挙。正規化期待値は `normalized` フィールドで NFC 形を指定し、将来の正規化検証に備えた。
2. フィクスチャの準備  
   - `compiler/ocaml/tests/samples/unicode_identifiers.reml` を追加し、サンプルモジュール `module 国際化.識別子` 内で複数スクリプトの識別子を宣言。テスト本体で存在確認を行い、Phase 2-7 以降に parser 連携テストへ転用できるようにした。  
   - `dune` に `unicode_ident_tests` を登録し、既存テスト群と同じライブラリ構成でビルド可能な状態を確認。
3. フォローアップ  
   - Step4 で `0-3-audit-and-metrics.md` の `lexer.identifier_profile_unicode` 指標へ測定値（Phase 2-5 = 0.0）とレビュー日を記録し、CI/Risk 登録 (`0-4-risk-handling.md`) を更新する。  
   - Phase 2-7 では `REML_ENABLE_UNICODE_TESTS=1` を CI で有効化し、lexer 正常動作・正規化挙動・ZWJ/Bidi 防止診断を検証する予定。

#### Step 4: 記録とメトリクス連携（週32末・Docs/QA）
- **目的**: 差分補正の結果をメトリクスとリスク管理に反映し、Phase 2-7 への引き継ぎ資料を整備する。  
- **作業**: `0-3-audit-and-metrics.md` の `lexer.identifier_profile_unicode` に Phase 2-5 時点の測定値（0.0）とレビュー日を記録。`0-4-risk-handling.md` に XID 実装待ちのリスクを登録し、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に SYNTAX-001 セクションを追記する。  
- **調査**: `tooling/ci/collect-iterator-audit-metrics.py` の出力ログを確認し、ASCII プロファイルでの既知制限が監視指標に反映されているか検証する。  
- **成果物**: 更新済みメトリクス、リスク登録、引き継ぎメモ。

##### Step4 実施記録（2026-03-31）
1. メトリクス更新  
   - `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に「Phase 2-5 Step4 記録」を追加し、`lexer.identifier_profile_unicode = 0.0`（Phase 2-5 時点）と週次レビュー日（2026-03-31）を記録。`tooling/ci/collect-iterator-audit-metrics.py --summary` の `parser.runconfig.lex.profile` が全件 `ascii` であることを確認したログを添付根拠とした。  
2. リスク登録  
   - `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` へ「Unicode XID 識別子実装未完了（SYNTAX-001 / LEXER-001）」エントリを追加し、期限を Phase 2-7 完了時点（2026-08-31）に設定。`lexer.identifier_profile_unicode` が 1.0 に到達するまで監視する運用を明記した。  
3. Phase 2-7 連携  
   - `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に「Unicode 識別子プロファイル移行（SYNTAX-001 / LEXER-001）」セクションを追加し、XID テーブル整備・CI ゲート化・脚注撤去を Phase 2-7 の正式タスクとして登録。  
4. フォローアップ  
   - CLI/LSP ゴールデンの ASCII 制限文言撤去、`REML_ENABLE_UNICODE_TESTS=1` 常時実行、`docs/notes/dsl/dsl-plugin-roadmap.md` チェックリスト更新を Phase 2-7 の成果物として紐付けた。

### Phase 2-7（Unicode 実装準備）

#### Step 5: XID テーブル生成とビルドパイプライン設計（着手週 TBD・Compiler/Tooling）
- **目的**: UnicodeData ベースで `XID_Start`/`XID_Continue` テーブルを生成し、ビルドフローに組み込む。  
- **作業**: `scripts/` 配下にテーブル生成スクリプトを追加（`uucp` / `uutf` 等の選定を含む）し、CI でのキャッシュ戦略とライセンス確認を行う。`compiler/ocaml/src/lexer.mll` での参照方法と `Core.Parse.Lex` API への反映方針を LEXER-001 チームと共通化する。  
- **調査**: Unicode 15 以降の更新頻度と差分（`DerivedCoreProperties.txt`）を把握し、テーブル更新手順を `docs/notes/unicode-maintenance.md`（新設予定）に記録する。  
- **成果物**: 生成スクリプト、CI 手順、参照ノート。

##### Step5 実施記録（2026-12-05）
1. テーブル生成  
   - `compiler/ocaml/src/lexer_tables/unicode_xid_tables.ml` を Unicode 15.1.0 の `XID_Start`/`XID_Continue` 範囲（主要スクリプト・結合文字・全角互換ブロックを含む）へ更新し、ASCII フォールバックを無効化。`unicode_xid_manifest.json` に `ascii_fallback=false`・`unicode_version=15.1.0`・範囲数（start=44, continue=63）を記録した。  
2. 既定プロファイル切替  
   - `lexer.mll` の `Identifier_profile` 既定値、および `parser_run_config.ml` の `Lex.default.identifier_profile` を `Unicode` へ変更し、`Core_parse.Lex.Bridge` が CLI/LSP/Streaming へ同一設定を伝播することを確認した（`core_parse_lex.ml` Step5 連携ノート参照）。  
3. 互換モードの保持  
   - ASCII 互換モードは `Lex.Ascii_compat` と `Lexer.Identifier_profile.Ascii_compat` で維持し、`collect-iterator-audit-metrics.py` の `lexer.identifier_profile_unicode` KPI が 1.0 以外に落ちた場合にフォールバック理由をログへ残す運用を固めた。

#### Step 6: CLI/LSP・監査整合の検証（着手週 TBD・Tooling/Docs）
- **目的**: Unicode 識別子導入後の CLI/LSP 表示と監査ログの整合を保証する。  
- **作業**: CLI でのエラー表示改善案を適用し、`Diagnostic.extensions` に識別子正規化の補助情報を追加。LSP/VS Code 拡張の補完・ハイライトに `--lex-profile=unicode` を連動させ、`docs/guides/dsl/plugin-authoring.md`・`docs/notes/dsl/dsl-plugin-roadmap.md` の TODO を消化する。  
- **調査**: `collect-iterator-audit-metrics.py --require-success` の Unicode プロファイル指標、`docs/spec/3-8-core-runtime-capability.md` Stage 表との整合性、`docs/notes/dsl/dsl-plugin-roadmap.md` §7 のチェックリストを追跡。  
- **成果物**: CLI/LSP フィクスチャ更新、監査ログサンプル、レビュー記録。

##### Step6 実施記録（2026-12-05）
1. CLI/LSP メッセージ整備  
   - `lexer.mll` の識別子エラー文言を `識別子の先頭に使用できないコードポイント … (profile=...)` 形式へ統一し、`compiler/ocaml/tests/test_lexer.ml` に Unicode 文字列の受理／ASCII フォールバック検証（`profile=ascii-compat` メタデータ確認を含む）を追加。  
2. ドキュメント更新  
   - `docs/spec/1-1-syntax.md`, `1-5-formal-grammar-bnf.md`, `2-3-lexer.md`, `0-2-glossary.md`, `docs/spec/README.md` から ASCII 暫定脚注を撤去し、`RunConfig.extensions["lex"].identifier_profile` の互換切替手順を追記。  
3. ガイド・ロードマップ同期  
   - `docs/guides/dsl/plugin-authoring.md` と `docs/notes/dsl/dsl-plugin-roadmap.md` を Unicode 既定前提へ改訂し、ASCII フォールバック時の監査要件とハンドオーバー先（ID22/ID23）を明示した。

## 5. フォローアップ
- Phase 2-7 `lexer-unicode` サブタスクで Step5/6 の実装と検証を完了し、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` の Unicode セクションを更新する。  
- Unicode 対応後に Chapter 1 脚注を整理し、`docs/spec/1-5-formal-grammar-bnf.md`・サンプルコード・`docs/spec/README.md` から暫定文言を撤去する。  
- CLI/LSP 側の識別子ハイライトや補完機能が Unicode 対応と整合するかを Step6 の成果物で再確認し、必要な場合は `docs/guides/runtime/runtime-bridges.md` へ脚注を追加する。  
- Phase 3 着手前に `0-3-audit-and-metrics.md` および `0-4-risk-handling.md` の関連項目をレビューし、`lexer.identifier_profile_unicode` が 1.0 を維持していることをセルフホスト移行条件に含める。  
- **タイミング**: Phase 2-5 の間に Step1〜4 を完了し、Unicode 機能の実装自体は Phase 2-7 の着手時に Step5/6 へ移行する。

## 残課題
- XID テーブル生成のビルドフロー（外部ツール利用可否）を Phase 2-7 チームと調整する必要がある。  
- ASCII 制限下での CLI エラー表示（例: Unicode を含む識別子の診断メッセージ）をどこまで改善するか検討したい。
