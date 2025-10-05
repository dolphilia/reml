# 1.6 開発者体験整備計画

## 目的
- `remlc-ocaml` CLI を Phase 1 で整備し、開発者が解析結果・IR・診断を観測できる開発体験を提供する。
- `3-6-core-diagnostics-audit.md` のフィールド定義に従い、出力フォーマットを将来のセルフホスト版と一致させる。

## スコープ
- **含む**: CLI インタフェース設計、サブコマンド実装、出力フォーマット整備、診断メッセージの国際化方針（日本語 + 原語併記）、ドキュメント整備。
- **含まない**: GUI、LSP サーバ、IDE プラグイン。これらは Phase 2 以降に計画。
- **前提**: Parser/TypeChecker/Core IR/LLVM が CLI から呼び出せる状態であること。

## 作業ブレークダウン
1. **CLI 設計**: `remlc-ocaml` のエントリポイントとオプションセット（`--emit-ast`, `--emit-tast`, `--emit-core`, `--emit-ir`, `--link-runtime`）を設計。
2. **診断出力統合**: `Diagnostic` 構造体を JSON (機械判読) とテキスト (人間向け) で出力し、`3-6-core-diagnostics-audit.md` に沿ったキー名称を実装。
3. **サマリ統計**: コンパイル時間・メモリ使用量（実測値）の表示を追加し、`0-3-audit-and-metrics.md` と同期。
4. **ログ/トレース整備**: `--trace` フラグで各フェーズの内部ステップ（parser/time, typer/time 等）をログ出力し、デバッグしやすくする。
5. **ユーザーガイド草案**: CLI 使い方を `guides/llvm-integration-notes.md` 補遺または `guides/` 新規文書で案内。
6. **サンプルプロジェクト**: `samples/language-impl-comparison/` に CLI 用ワークフローを追加し、README を更新。

## 成果物と検証
- CLI コマンドが `dune exec remlc-ocaml -- --help` で利用可能になり、各オプションが CI のスモークテストで網羅される。
- `Diagnostic` JSON のスキーマを `jsonschema` 形式で管理し、CI で検証。
- ドキュメント (`guides/llvm-integration-notes.md` または新規ガイド) に CLI 利用手順が掲載される。

## リスクとフォローアップ
- 出力フォーマットの変更がフェーズ間で発生しやすいため、バージョンタグを付与し後方互換性を `0-4-risk-handling.md` で管理。
- CLI のオプションが増えすぎると UX が低下するため、Phase 2 で LSP 計画へ引き継ぎ、現段階では観測用途に絞る。
- 多言語対応は Phase 4 のエコシステム移行で本格化するため、日本語テキストに英語キーワードを括弧書きで併記する程度に留める。

## 参考資料
- [1-0-phase1-bootstrap.md](1-0-phase1-bootstrap.md)
- [3-6-core-diagnostics-audit.md](../../3-6-core-diagnostics-audit.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- [guides/llvm-integration-notes.md](../../guides/llvm-integration-notes.md)

