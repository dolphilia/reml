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
- `docs/notes/dsl-plugin-roadmap.md` に Unicode 化のチェック項目を追記し、DSL/プラグイン移行時に同じ制約を共有する。

## 3. 影響範囲と検証
- **テスト**: Unicode 識別子を含むサンプルを `compiler/ocaml/tests/unicode_ident_tests.ml`（新設）に追加し、ASCII 実装ではスキップ、Unicode 実装後に有効化する。  
- **メトリクス**: `0-3-audit-and-metrics.md` に登録済みの `lexer.identifier_profile_unicode` を更新し、Phase 2-5 では 0.0 を記録、Phase 2-7 `lexer-unicode` 完了時に 1.0 を目標とする。  
- **仕様・ガイド**: `docs/spec/1-5-formal-grammar-bnf.md`・`docs/spec/0-2-glossary.md`・`docs/guides/plugin-authoring.md` 等に ASCII 制限の暫定注意書きと Unicode 対応予定を追記し、索引（`docs/spec/README.md`）のリンク整合を確認する。

## 4. 実施ステップと調査項目

### Phase 2-5（差分可視化と仕様整備）

#### Step 1: Chapter 1/1-5 差分棚卸（週31・Docs/Compiler合同）
- **目的**: `docs/spec/1-1-syntax.md` と `docs/spec/1-5-formal-grammar-bnf.md` における Unicode 定義と実装 (`compiler/ocaml/src/lexer.mll`) の乖離を明示する。  
- **作業**: 既存脚注 `[^lexer-ascii-phase25]` の適用範囲を確認し、`docs/plans/bootstrap-roadmap/2-5-review-log.md` に専用節（SYNTAX-001 Step1）を追加して再現手順と影響箇所を記録する。`compiler/ocaml/tests/test_lexer.ml` へ ASCII 限定挙動の再現テストを追加し、現状の拒否メッセージを保存する。  
- **調査**: `docs/spec/1-4-test-unicode-model.md` の参照条件と `core_parse` 経路の挙動を確認し、Normalization/Bidi 要件が Phase 2-5 で未検証である点を整理する。  
- **成果物**: 差分記録、レビューエントリ、`2-5-spec-drift-remediation.md` の表更新。

#### Step 2: 仕様脚注・索引・用語集の整備（週31-32・Docsチーム）
- **目的**: 仕様読者が ASCII 制限と今後の計画を把握できる状態にする。  
- **作業**: `docs/spec/1-5-formal-grammar-bnf.md` に Phase 2-5 時点の脚注を追加し、`docs/spec/0-2-glossary.md` へ「Unicode identifier profile (暫定)」の定義を追記。`docs/spec/README.md`・`README.md` の索引更新、`docs/plans/repository-restructure-plan.md` に沿ったリンク確認を実施する。  
- **調査**: リンク切れ検証、脚注記号の重複確認、既存の `LEXER-001` 記録との整合性を確認する。  
- **成果物**: 更新済み仕様・索引・用語集、脚注採番表。

#### Step 3: 検証用サンプルとテストシナリオ整備（週32・Compiler/Testing）
- **目的**: Unicode 実装後に即座に回帰検証できるテスト土台を整える。  
- **作業**: `compiler/ocaml/tests/unicode_ident_tests.ml` を新設し、`dune` テスト設定に `[@tags unicode_pending]`（仮）を付与して Phase 2-5 ではスキップする。`docs/spec/1-1-syntax.md` の例や Chapter 3 多言語サンプルをベースに、文字種別（日本語・ハングル・合成文字）を網羅する入力を準備する。  
- **調査**: `UnicodeData.txt` / `DerivedCoreProperties.txt` を参照し、`XID_Start`/`XID_Continue` の境界ケース（結合文字・サロゲート・ZWNJ/ZWJ）を抽出する。`technical-debt.md`（ID 2b/22/23）の想定ケースと突合して不足がないか確認。  
- **成果物**: スキップ可視化されたテスト群、サンプル入力、レビュー記録の追記。

#### Step 4: 記録とメトリクス連携（週32末・Docs/QA）
- **目的**: 差分補正の結果をメトリクスとリスク管理に反映し、Phase 2-7 への引き継ぎ資料を整備する。  
- **作業**: `0-3-audit-and-metrics.md` の `lexer.identifier_profile_unicode` に Phase 2-5 時点の測定値（0.0）とレビュー日を記録。`0-4-risk-handling.md` に XID 実装待ちのリスクを登録し、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に SYNTAX-001 セクションを追記する。  
- **調査**: `tooling/ci/collect-iterator-audit-metrics.py` の出力ログを確認し、ASCII プロファイルでの既知制限が監視指標に反映されているか検証する。  
- **成果物**: 更新済みメトリクス、リスク登録、引き継ぎメモ。

### Phase 2-7（Unicode 実装準備）

#### Step 5: XID テーブル生成とビルドパイプライン設計（着手週 TBD・Compiler/Tooling）
- **目的**: UnicodeData ベースで `XID_Start`/`XID_Continue` テーブルを生成し、ビルドフローに組み込む。  
- **作業**: `scripts/` 配下にテーブル生成スクリプトを追加（`uucp` / `uutf` 等の選定を含む）し、CI でのキャッシュ戦略とライセンス確認を行う。`compiler/ocaml/src/lexer.mll` での参照方法と `Core.Parse.Lex` API への反映方針を LEXER-001 チームと共通化する。  
- **調査**: Unicode 15 以降の更新頻度と差分（`DerivedCoreProperties.txt`）を把握し、テーブル更新手順を `docs/notes/unicode-maintenance.md`（新設予定）に記録する。  
- **成果物**: 生成スクリプト、CI 手順、参照ノート。

#### Step 6: CLI/LSP・監査整合の検証（着手週 TBD・Tooling/Docs）
- **目的**: Unicode 識別子導入後の CLI/LSP 表示と監査ログの整合を保証する。  
- **作業**: CLI でのエラー表示改善案を適用し、`Diagnostic.extensions` に識別子正規化の補助情報を追加。LSP/VS Code 拡張の補完・ハイライトに `--lex-profile=unicode` を連動させ、`docs/guides/plugin-authoring.md`・`docs/notes/dsl-plugin-roadmap.md` の TODO を消化する。  
- **調査**: `collect-iterator-audit-metrics.py --require-success` の Unicode プロファイル指標、`docs/spec/3-8-core-runtime-capability.md` Stage 表との整合性、`docs/notes/dsl-plugin-roadmap.md` §7 のチェックリストを追跡。  
- **成果物**: CLI/LSP フィクスチャ更新、監査ログサンプル、レビュー記録。

## 5. フォローアップ
- Phase 2-7 `lexer-unicode` サブタスクで Step5/6 の実装と検証を完了し、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` の Unicode セクションを更新する。  
- Unicode 対応後に Chapter 1 脚注を整理し、`docs/spec/1-5-formal-grammar-bnf.md`・サンプルコード・`docs/spec/README.md` から暫定文言を撤去する。  
- CLI/LSP 側の識別子ハイライトや補完機能が Unicode 対応と整合するかを Step6 の成果物で再確認し、必要な場合は `docs/guides/runtime-bridges.md` へ脚注を追加する。  
- Phase 3 着手前に `0-3-audit-and-metrics.md` および `0-4-risk-handling.md` の関連項目をレビューし、`lexer.identifier_profile_unicode` が 1.0 を維持していることをセルフホスト移行条件に含める。  
- **タイミング**: Phase 2-5 の間に Step1〜4 を完了し、Unicode 機能の実装自体は Phase 2-7 の着手時に Step5/6 へ移行する。

## 残課題
- XID テーブル生成のビルドフロー（外部ツール利用可否）を Phase 2-7 チームと調整する必要がある。  
- ASCII 制限下での CLI エラー表示（例: Unicode を含む識別子の診断メッセージ）をどこまで改善するか検討したい。
