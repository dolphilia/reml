# 1.7 x86_64 Linux 検証インフラ計画

## 目的
- Phase 1 の最終マイルストーン M4 までに、x86_64 Linux (System V ABI) を対象とした自動検証環境を GitHub Actions 上に構築する。
- LLVM 15 以上の固定バージョンに基づく CI パイプラインを整備し、Parser/Typer/Core IR/LLVM/ランタイムのスモークテストを一体化する。

## スコープ
- **含む**: GitHub Actions ワークフロー設計、依存キャッシュ、コンパイル・テスト・リンカ実行、成果物の収集、監査ログへの記録。
- **含まない**: Windows/macOS ランナー、長時間ベンチマーク、本番配布。これらは Phase 2 以降で追加。
- **前提**: CLI と各フェーズのテストがコマンドラインから実行可能になっていること。

## 作業ディレクトリ
- `.github/workflows/` : GitHub Actions 定義
- `tooling/ci` : ローカル再現スクリプト、CI 用ユーティリティ
- `compiler/ocaml/` : CI でビルドするソース、テスト資産
- `runtime/native` : ランタイムビルド/リンク検証
- `docs/notes/llvm-spec-status-survey.md`, `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` : CI 結果・指標の記録先

## 作業ブレークダウン

### 1. CI設計とワークフロー定義（9週目）
**担当領域**: GitHub Actions基盤設計

1.1. **ワークフローファイル作成**
- `.github/workflows/bootstrap-linux.yml` の作成
- トリガー設定（push, pull_request, schedule）
- 実行環境指定（`ubuntu-latest`, `runs-on`）

1.2. **ステージ設計**
- Lint: コードフォーマット、静的解析
- Build: OCamlコンパイラ、ランタイムビルド
- Test: 単体テスト、統合テスト、ゴールデンテスト
- Artifact: 成果物の収集と保存

1.3. **依存関係グラフ**
- ステージ間の依存関係定義（`needs`）
- 並行実行可能なジョブの特定
- 失敗時の継続戦略（`continue-on-error`）

**成果物**: `.github/workflows/bootstrap-linux.yml` 初版

### 2. 開発環境セットアップ（9-10週目）
**担当領域**: ビルド依存関係の自動化

2.1. **OCaml環境構築**
- OCaml 4.14以上のインストール
- `opam` のセットアップとキャッシュ
- `dune`, `menhir` 等の依存パッケージインストール

2.2. **LLVM環境構築**
- LLVM 15のインストール（`apt`経由）
- `llvm-config` のパス設定
- `actions/cache` によるLLVMバイナリキャッシュ

2.3. **システム依存関係**
- Cコンパイラ（`gcc` または `clang`）
- `make`, `cmake` ツール
- Valgrind（メモリチェック用）

**成果物**: 依存関係インストールスクリプト

### 3. ビルドジョブ実装（10週目）
**担当領域**: コンパイルステップ

3.1. **OCamlプロジェクトビルド**
- `dune build` の実行
- ビルドログの保存
- ビルド時間の計測

3.2. **ランタイムビルド**
- `make runtime` の実行
- `libreml_runtime.a` の生成確認
- ビルド成果物の検証

3.3. **CLI生成**
- `remlc-ocaml` バイナリの生成
- バージョン情報の埋め込み
- 実行権限の付与

**成果物**: ビルドジョブ設定、ビルド成果物

### 4. テストジョブ実装（10-11週目）
**担当領域**: 自動テスト実行

4.1. **単体テスト実行**
- `dune runtest` の実行
- テスト結果のJUnit XML出力
- カバレッジレポート生成（`bisect_ppx`、オプション）

4.2. **統合テスト実行**
- サンプルコードのコンパイル
- 生成バイナリの実行検証
- 期待出力との比較

4.3. **ゴールデンテスト実行**
- AST, TAST, Core IR, LLVM IRのスナップショット比較
- 差分検出時のエラー報告
- スナップショット更新フロー

**成果物**: テストジョブ設定、テストレポート

### 5. LLVM検証ステップ（11週目）
**担当領域**: LLVM IR品質保証

5.1. **LLVM検証パイプライン**
- `llvm-as` によるアセンブル検証
- `opt -verify` による整合性チェック
- `llc` によるコード生成テスト

5.2. **リンクテスト**
- 生成されたオブジェクトファイルと `libreml_runtime.a` のリンク
- 実行可能バイナリの生成確認
- 実行時クラッシュがないことの確認

5.3. **メモリ検証**
- Valgrind によるメモリリーク検出
- AddressSanitizer（ASan）の実行（オプション）
- 検証結果のレポート生成

**成果物**: LLVM検証ジョブ、検証レポート

### 6. アーティファクト管理（11-12週目）
**担当領域**: 成果物の収集と保存

6.1. **アーティファクト定義**
- コンパイラバイナリ（`remlc-ocaml`）
- ランタイムライブラリ（`libreml_runtime.a`）
- 中間生成物（AST, TAST, Core IR, LLVM IR ダンプ）
- テストレポート、診断ログ

6.2. **アップロード設定**
- `actions/upload-artifact` の使用
- 保持期間の設定（30日）
- アーティファクト名の命名規則

6.3. **ダウンロードとレビュー**
- PR レビュー時のアーティファクト確認手順
- 差分比較のワークフロー
- アーティファクトのクリーンアップ

**成果物**: アーティファクト管理設定

### 7. 監査ログとメトリクス（12週目）
**担当領域**: CI結果の記録と分析

7.1. **実行結果の記録**
- CI実行時のビルド時間、テスト時間
- 成功/失敗の統計
- `0-3-audit-and-metrics.md` への自動追記

7.2. **メトリクス可視化**
- GitHub Actions のステータスバッジ
- 時系列での性能推移グラフ（検討）
- テストカバレッジの追跡

7.3. **失敗時の通知**
- エラーログの抽出と整形
- `0-4-risk-handling.md` への自動issue作成（検討）
- Slack/Email通知の設定（オプション）

**成果物**: 監査ログ自動化、メトリクス記録

### 8. ローカル再現環境（12週目）
**担当領域**: 開発者体験の向上

8.1. **ローカル実行スクリプト**
- `scripts/ci-local.sh` の作成
- CI と同等の手順をローカルで実行
- Dockerコンテナでの実行オプション

8.2. **Dockerイメージ作成**
- `Dockerfile` の作成（OCaml + LLVM環境）
- GitHub Container Registry への発行
- CI でのイメージ利用

8.3. **ドキュメント整備**
- ローカルテスト手順の文書化
- トラブルシューティングガイド
- CI設定の詳細解説

**成果物**: ローカル再現スクリプト、Dockerイメージ、ドキュメント

## 成果物と検証
- GitHub Actions の定期実行（push/pr）で全テストが通過することを確認。
- アーティファクトが 30 日保持され、レビューで差分確認に利用できる。
- ローカル再現スクリプトにより、開発者が CI と同じ手順を実行可能であることを README へ明記。

## リスクとフォローアップ
- LLVM ダウンロードが CI のボトルネックとなる場合、事前ビルド済み Docker イメージを作成し GitHub Container Registry に登録する。
- CI 実行時間が長くなる可能性があるため、Phase 2 でジョブ分割やキャッシュ戦略の再検討を行う。
- バイナリアーティファクトのサイズが増大した場合、`0-3-audit-and-metrics.md` に上限値を記録し整理する。

## 参考資料
- [1-0-phase1-bootstrap.md](1-0-phase1-bootstrap.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- [0-4-risk-handling.md](0-4-risk-handling.md)
- [guides/llvm-integration-notes.md](../../guides/llvm-integration-notes.md)
