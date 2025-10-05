# 1.1 Parser 実装詳細計画

## 目的
- Phase 1 のマイルストーン M1 を達成できるよう、`1-1-syntax.md` の式・宣言構文を全て OCaml 製パーサで扱える状態にする。
- `2-0-parser-api-overview.md` が定義する `Parser<T>` 契約を OCaml の抽象データ型で写像し、後続フェーズで Reml 実装へ移植しやすい構造を確立する。
- エラーレポート用の Span 情報を AST に格納し、`2-5-error.md` の診断モデルと整合を取る。

## スコープ
- **含む**: 字句解析・構文解析の OCaml 実装、Menhir 等のパーサジェネレータ設定、AST 生成、Span 付与、演算子優先順位テーブル（固定版）。
- **含まない**: DSL 用構文拡張、ユーザー定義演算子テーブル、パーサと連動する型解決。これらは Phase 2 以降で実装する。
- **前提**: LLVM toolchain セットアップ、Menhir または ocamlyacc のビルド環境、既存サンプル（`samples/language-impl-comparison/`）。

## 作業ブレークダウン
1. **文法資産の抽出**: `1-1-syntax.md` から AST ノード一覧・構文図を抽出し、Menhir 用 `.mly`、lexer 用 `.mll` のドラフトを作成。
2. **AST と Span 設計**: `guides/llvm-integration-notes.md` §3 の IR フローを参照しつつ OCaml レコードで AST を定義。Span はバイトオフセット範囲で保持し、診断で利用するフィールドを決める。
3. **パーサジェネレータ統合**: Dune プロジェクトに Menhir を組み込み、差分ビルド可能なルールと CI 用ビルド手順をまとめる。
4. **エラー回復戦略**: `2-5-error.md` の期待値提示に合わせ、よくある破損パターン（`;` 抜け、括弧閉じ忘れ等）に対するエラー回復を定義。
5. **テスト整備**: `samples/language-impl-comparison/` を基に AST ゴールデンテストを追加し、スナップショットの差分確認を自動化。
6. **ドキュメント更新**: 作成した構文表・優先順位を `1-0-phase1-bootstrap.md` から参照できるよう脚注と TODO を記録し、メトリクスは `0-3-audit-and-metrics.md` に追記。

## 成果物と検証
- `parser/` ディレクトリ（仮）に OCaml 実装を配置し、CI で `dune build parser` が通ること。
- AST ゴールデンテストと診断サンプルテストを GitHub Actions x86_64 Linux ランナーで実行し、失敗時に差分を表示する仕組みを導入。
- Span 情報を含む AST ダンプ (`--emit-ast`) を CLI で出力できるようにする。

## リスクとフォローアップ
- Menhir の依存が開発者環境ごとにばらつくリスクがあるため、`0-3-audit-and-metrics.md` に必要なバージョン固定情報を記録。
- 演算子優先順位の固定テーブルが Phase 2 で拡張される見込みなので、テーブル定義を外部 JSON/再読込可能な形式に切り出し、後続フェーズで差し替えやすくしておく。
- ストリーミングパーサ API (`2-7-core-parse-streaming.md`) との整合は Phase 3 で必須となるため、現段階で AST 生成を純粋関数化し副作用を最小限にする。

## 参考資料
- [1-0-phase1-bootstrap.md](1-0-phase1-bootstrap.md)
- [1-1-syntax.md](../../1-1-syntax.md)
- [2-0-parser-api-overview.md](../../2-0-parser-api-overview.md)
- [2-5-error.md](../../2-5-error.md)
- [guides/llvm-integration-notes.md](../../guides/llvm-integration-notes.md)

