# LEXER-001 Unicode プロファイル調整計画

## 1. 背景と症状
- Chapter 2 の字句仕様では `identifier(profile=DefaultId)` を含む Unicode プロファイルを前提としているが（docs/spec/2-3-lexer.md:92-170）、実装は ASCII ベースの識別子のみをサポートし、`identifier(profile=...)` が利用できない（compiler/ocaml/src/lexer.mll:46-52）。  
- DSL プラグインや Capability 名で Unicode 別名を使用するケースが仕様上許容されているが、OCaml 実装では拒否されるため差分レビューが困難。  
- `docs/notes/dsl-plugin-roadmap.md` では Unicode プロファイルを導入する計画があるため、差分補正フェーズで現状の制約とロードマップを明記する必要がある。

## 2. Before / After
### Before
- 字句プロファイル API が未提供のため、仕様記載の `ConfigTriviaProfile` や `identifier(profile=...)` を利用できない。  
- Unicode 対応の進捗が明文化されておらず、各チームが別々に制約を把握する状態。

### After
- Chapter 2 に ASCII 限定であることを脚注で明示し、Phase 2-7 `lexer-unicode` タスクで Unicode プロファイルを導入する計画を整理。  
- `docs/notes/dsl-plugin-roadmap.md` に `identifier` プロファイル差分と対応予定を追加し、DSL/プラグインチームと共有。  
- 実装側は `Core.Parse.Lex` 抽出（LEXER-002）と連携して、Unicode 対応時の API 表面を定義する。

## 3. 影響範囲と検証
- **テスト**: Unicode 識別子の字句解析テストを追加し、Phase 2-7 完了時に有効化。ASCII 限定の間はスキップして回帰指標を維持する。  
- **メトリクス**: `0-3-audit-and-metrics.md` に `lexer.identifier_profile_unicode` を追加し、導入時に PASS へ更新。  
- **ドキュメント**: `docs/notes/dsl-plugin-roadmap.md` に「Unicode プロファイル導入準備」セクションと依存タスクを追記。
- **用語集**: `docs/spec/0-2-glossary.md` に `Unicode identifier profile` の暫定定義を追加し、仕様読者が ASCII 制限期間中の扱いを把握できるようにする。

## 4. 実装ステップと調査項目

### Phase 2-5（差分可視化と準備）

#### Step 1: ASCII 実装棚卸（週31, Lexer チーム）
- **目的**: 現行の ASCII 限定挙動と仕様の乖離を定量化し、補正対象を確定する。  
- **作業**: `compiler/ocaml/src/lexer.mll` と `Core.Parse.Lex.Bridge` の識別子読み取り経路を洗い出し、ASCII 以外の入力で発生する拒否理由を `docs/plans/bootstrap-roadmap/2-5-review-log.md` に記録する。最小再現となる `.reml` スニペットを既存の字句テストに追加し、現在の失敗メッセージを保存する。  
- **調査**: `docs/spec/2-3-lexer.md` D-1/D-2 と `docs/spec/1-1-syntax.md` の識別子定義を照合し、差分がどこで発生しているかを特定する。
- **成果物**: レビュー記録の更新、`2-5-spec-drift-remediation.md` への差分リンク、`Phase 2-5` 差分リストでの `LEXER-001` エントリ確定。

##### Step1 実施記録（2025-12-12）
- `compiler/ocaml/src/lexer.mll:41-78` の ASCII 固定パターンと `Core.Parse.Lex.Bridge` の Trivia 同期のみで識別子プロファイルを扱っていない点を棚卸し、レビュー記録へ反映。  
- フィクスチャ `compiler/ocaml/tests/samples/lexer_unicode_identifier.reml` とユニットテスト `compiler/ocaml/tests/test_lexer.ml:88` を追加し、`Unexpected character: �`（`span 4-5`）という現状の失敗メッセージを保存。  
- `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` の `LEXER-001` 差分リストへ進捗脚注を追記し、Phase 2-7 へのフォローアップ（脚注整備・ロードマップ更新）を継続する。

#### Step 2: 仕様脚注と索引の整備（週31-32, Docs チーム）
- **目的**: 仕様読者へ ASCII 制限を通知し、差分補正の判断材料を共有する。  
- **作業**: `docs/spec/2-3-lexer.md` D-1 に ASCII 制約脚注を追加し、`docs/spec/1-1-syntax.md` の Identifier セクションと `README.md`（索引用）を更新する。同時に `docs/spec/0-2-glossary.md` へ暫定用語定義を追加し、`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` の脚注に参照を追記する。  
- **調査**: `docs/spec/0-3-code-style-guide.md` の命名規則、`docs/plans/repository-restructure-plan.md` のリンクポリシーを確認し、移行中のパス変更へ備える。
- **成果物**: 脚注反映済みの仕様書、索引更新、`0-3-audit-and-metrics.md` へ記録したレビュー結果。

#### Step 3: DSL/プラグインチーム連携（週32, DSL チーム）
- **目的**: DSL プラグインの別名ポリシーに Unicode プロファイル差分を反映し、Phase 2-7 までの運用ルールを確立する。  
- **作業**: `docs/notes/dsl-plugin-roadmap.md` に「Unicode プロファイル導入準備」節を追加し、Capability 名の別名運用と互換モード利用時の制約を整理する。`docs/guides/plugin-authoring.md` へ脚注リンクを付与し、プラグイン作者への周知経路を確保する。  
- **調査**: `docs/spec/3-8-core-runtime-capability.md` の Stage 契約と `docs/guides/runtime-bridges.md` のブリッジ要件を参照し、Capability ID が ASCII に固定されている箇所を洗い出す。
- **成果物**: DSL ノートの更新、Stage/Capability 関連 TODO の登録、`compiler/ocaml/docs/technical-debt.md`（ID22/23）との整合メモ。

#### Step 4: 測定指標と CI スイッチの定義（週32, Metrics チーム）
- **目的**: Phase 2-7 で Unicode プロファイルを導入した際に CI で差分を検知できる状態を作る。  
- **作業**: `0-3-audit-and-metrics.md` に `lexer.identifier_profile_unicode` 指標の計測項目と PASS 判定条件を記述し、`tooling/ci/collect-iterator-audit-metrics.py` にプレースホルダ集計を実装する。Unicode テストをスキップする CI タグを CLI オプション（`--lex-profile=ascii|unicode`）として設計し、Phase 2-7 で切り替え可能にする。  
- **調査**: `scripts/validate-diagnostic-json.sh` の既存スキップ機構、`compiler/ocaml/tests/test_cli_diagnostics.ml` の CLI 実行方法を確認し、Unicode 導入後の差分検知に使えるか評価する。
- **成果物**: メトリクス項目の追加、CI プレースホルダ実装、差分検知向けテストスキップ戦略の文書化。

### Phase 2-7（Unicode 実装と移行）

#### Step 5: Lex API 設計の固定化（Phase 2-7 開始週, Lexer/Parser チーム）
- **目的**: `Core.Parse.Lex` と `ConfigTriviaProfile` の API を Unicode 対応後も破綻させない形に固定する。  
- **作業**: `LEXER-002` と連携し、`identifier(profile=...)` が参照する `IdentifierProfile` 構造体と `lex.profile` 設定フックを設計する。`compiler/ocaml/docs/parser_design.md` に API 更新案を記載し、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` へ承認待ち項目として登録する。  
- **調査**: `docs/spec/2-0-parser-api-overview.md` と `docs/spec/2-1-parser-type.md` の `Parser<T>` インタフェース、`docs/spec/2-6-execution-strategy.md` の実行戦略を照合し、Unicode プロファイル導入で追加の同期が必要か評価する。
- **成果物**: API 設計メモ、`Core.Parse.Lex` の公開型定義案、`Phase 2-7` 着手条件のチェックリスト。

#### Step 6: Unicode データ生成パイプラインの確立（Phase 2-7 中盤, Infra チーム）
- **目的**: UAX #31/#39 で要求される `XID_Start`/`XID_Continue`、正規化、紛らわし検査を自動生成し、ビルド時に再現可能な形で管理する。  
- **作業**: `UnicodeData.txt` と `DerivedCoreProperties.txt` を取得するスクリプト（`tooling/unicode/generate-identifier-profile.ml` など）を設計し、生成物を `compiler/ocaml/generated/unicode/` 配下へ出力する。NFC チェックには `uutf`/`uucp` を利用し、ライセンスと更新頻度を `docs/notes/unicode-data-update-policy.md`（新規）にまとめる。  
- **調査**: `docs/spec/3-3-core-text-unicode.md` の既存要件、`docs/spec/1-5-formal-grammar-bnf.md` の字句規則、`docs/spec/0-1-project-purpose.md` の性能制約を踏まえ、テーブルサイズと初期化コストを見積もる。
- **成果物**: データ生成スクリプト、生成物のバージョン固定方法、CI での検証タスク（`tooling/ci/collect-iterator-audit-metrics.py --check-unicode`）の追加。

#### Step 7: 互換モードと検証フローの構築（Phase 2-7 終盤, CLI/LSP チーム）
- **目的**: ASCII プロファイルを継続利用するユーザーへの互換性を確保しつつ、Unicode 化後の回帰を防ぐ。  
- **作業**: `RunConfig`（CLI/LSP 共通）に `lex.identifier_profile` 切替フラグを追加し、ASCII/Unicode で診断メッセージが変わる箇所を `test_cli_diagnostics.ml`・`streaming_runner_tests.ml` 等で比較できるようテストを整理する。`docs/guides/ai-integration.md` にフラグ利用例を記載し、AI ツールが誤った Suggestion を出さないよう指針を追加する。  
- **調査**: `compiler/ocaml/src/diagnostic.ml` のハイライト生成、`docs/guides/core-parse-streaming.md` のストリーム実行要件を参照し、Unicode 文字を含むポジション計算の妥当性を確認する。
- **成果物**: 実装済みの互換フラグ、ASCII/Unicode 双方で通過するゴールデンテスト、CI メトリクスの PASS 判定。

## 5. フォローアップ
- Phase 2-7 で XID テーブル生成や正規化検査を導入する際、`ConfigTriviaProfile` と `identifier` の API を統合して公開する。  
- Unicode 対応後に脚注を削除し、`docs/spec/1-1-syntax.md` / `docs/spec/2-3-lexer.md` のサンプルを再検証する。  
- CLI/LSP のハイライト・補完機能が Unicode プロファイルと整合するようチーム間で調整する。
- **タイミング**: Phase 2-5 では早期に脚注とロードマップを整備し、Unicode 対応そのものは Phase 2-7 の `lexer-unicode` タスク開始時に実装を進める。

## 6. 残課題
- 字句プロファイルを Phase 2-7 でどの順序で実装するか（`identifier` → `lexeme` → `ConfigTriviaProfile`）を Parser/Lexer チームで合意する必要がある。  
- Unicode 対応へ切り替える際の互換モード（ASCII 限定の維持フラグ）を提供するか検討したい。
