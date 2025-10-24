# LEXER-001 Unicode プロファイル調整提案

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

## 4. フォローアップ
- Phase 2-7 で XID テーブル生成や正規化検査を導入する際、`ConfigTriviaProfile` と `identifier` の API を統合して公開する。  
- Unicode 対応後に脚注を削除し、`docs/spec/1-1-syntax.md` / `docs/spec/2-3-lexer.md` のサンプルを再検証する。  
- CLI/LSP のハイライト・補完機能が Unicode プロファイルと整合するようチーム間で調整する。

## 確認事項
- 字句プロファイルを Phase 2-7 でどの順序で実装するか（`identifier` → `lexeme` → `ConfigTriviaProfile`）を Parser/Lexer チームで合意する必要がある。  
- Unicode 対応へ切り替える際の互換モード（ASCII 限定の維持フラグ）を提供するか検討したい。
