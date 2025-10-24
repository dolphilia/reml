# SYNTAX-001 Unicode 識別子制約の暫定整備提案

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
- **メトリクス**: `0-3-audit-and-metrics.md` に `lexer.unicode_support` の項目を追加し、Phase 2-7 完了時に PASS へ更新。  
- **ガイド**: `docs/guides/plugin-authoring.md` 等に ASCII 制限の暫定注意書きと、Unicode 対応予定を追記。

## 4. フォローアップ
- Phase 2-7 `lexer-unicode` サブタスクで XID テーブル生成（UnicodeData ベース）および Bidi/Normalization 検査の導入を確約する。  
- Unicode 対応後、仕様脚注を削除し `docs/spec/1-5-formal-grammar-bnf.md` / サンプルコードを更新。  
- CLI/LSP 側の識別子ハイライトや補完機能が Unicode 対応と整合するか確認する。
- `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に Unicode 対応の残課題（CI 支援ツール、互換モード切替）を記録しておき、Phase 3 移行時のチェックリストに含める。

## 確認事項
- XID テーブル生成のビルドフロー（外部ツール利用可否）を Phase 2-7 チームと調整する必要がある。  
- ASCII 制限下での CLI エラー表示（例: Unicode を含む識別子の診断メッセージ）をどこまで改善するか検討したい。
