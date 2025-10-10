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

## ステータス要約（2025-10-10 時点）
- ✅ GitHub Actions の Linux ワークフロー（`.github/workflows/bootstrap-linux.yml`）を Lint/Build/Test/Artifact の 4 ジョブ構成で運用し、`dune runtest` とランタイム検証（Valgrind/ASan）を自動化。最終ジョブで CI バンドル（`linux-ci-bundle.tar.gz`）を生成。
- ✅ Docker ベースの検証環境（`tooling/ci/docker/bootstrap-runtime.Dockerfile`）と補助スクリプト（`scripts/docker/build-runtime-container.sh`, `scripts/docker/run-runtime-tests.sh`）を整備し、`tooling/ci/docker/metrics.json` に初期計測値を記録済み。
- ✅ LLVM バージョンは OCaml 実装で実績のある **LLVM 18 系** を正式採用。Phase 1 では LLVM 15 へのダウングレードを想定せず、将来のセルフホスト移行時に必要であれば更なる新バージョンへの追従を検討する。
- ⏳ LLVM IR 検証フローは `dune runtest`（`test_llvm_verify.ml` 経由）で実行されるが、CI ワークフロー内に明示的な `llvm-as` → `opt -verify` → `llc` ステップやアーティファクト収集がまだ組み込まれていない。
- ⏳ 監査ログの自動反映と `scripts/ci-local.sh`（ローカル追試スクリプト）は未実装。手動更新の `tooling/ci/README.md` では TODO として管理中。

## 作業ブレークダウン

### 1. CI設計とワークフロー定義（9週目）
**担当領域**: GitHub Actions基盤設計

**ステータス（2025-10-10）**
- ✅ `.github/workflows/bootstrap-linux.yml` で push / pull_request / workflow_dispatch トリガーと `ubuntu-latest` 実行環境を設定済み。
- ✅ Lint → Build → Test → Artifact のジョブ分割と `needs` 依存（`lint → build → test → artifact`）を整備し、失敗時も最終ジョブでアーティファクトを取得できるよう `if: always()` を設定。
- ✅ ワークフロー名とファイル名を `bootstrap-linux.yml` に統一。

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

**ステータス（2025-10-10）**
- ✅ OCaml 5.2.1 と `opam install . --deps-only --with-test` の自動化を CI で実施。
- ✅ LLVM 18 系を `apt` から導入し、`llvm-as` / `opt` / `llc` のシンボリックリンクを作成（`bootstrap-linux` ワークフロー）。
- ⏳ 計画で要求していた LLVM 15 系へのバージョン整合、`actions/cache` を用いた LLVM / opam キャッシュ最適化は未実装。
- ✅ Valgrind の自動インストールを行っているが、`cmake` などの追加ツール配布状況は今後の拡張で確認が必要。

2.1. **OCaml環境構築**
- OCaml 4.14以上のインストール
- `opam` のセットアップとキャッシュ
- `dune`, `menhir` 等の依存パッケージインストール

2.2. **LLVM環境構築**
- LLVM 18 のインストール（`apt`経由）
- `llvm-config` のパス設定
- `actions/cache` によるLLVMバイナリキャッシュ

> **決定（2025-10-10）**: OCaml 実装では LLVM 18 が最も安定しており、Phase 1 では LLVM 15 へのダウングレードを行わない。セルフホスト版で新しい LLVM を採用する場合は `bootstrap-linux.yml` のバージョンを引き上げる。逆方向（古いバージョン）へのロールバックは想定外。

2.3. **システム依存関係**
- Cコンパイラ（`gcc` または `clang`）
- `make`, `cmake` ツール
- Valgrind（メモリチェック用）

**成果物**: 依存関係インストールスクリプト

### 3. ビルドジョブ実装（10週目）
**担当領域**: コンパイルステップ

**ステータス（2025-10-10）**
- ✅ `opam exec -- dune build` を CI で実行し、ビルドログが GitHub Actions 上で取得可能。
- ✅ `make runtime`（`runtime/native`）をワークフロー内で実行、`libreml_runtime.a` を生成。
- ⏳ CLI バイナリの命名整理（`remlc-ocaml`）、バージョン埋め込みと恒久的なアーティファクト保存は未設定。

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

**ステータス（2025-10-10）**
- ✅ `opam exec -- dune runtest` を CI に統合し、単体テスト・統合テスト・ゴールデンテスト（`test_llvm_golden.ml` など）を実行中。
- ✅ `runtime/native` の `make test` による C ランタイム単体テスト・Valgrind チェックを自動化。
- ⏳ テスト結果の JUnit 形式出力、カバレッジ計測、失敗時アーティファクト収集の網羅は未対応。

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

**ステータス（2025-10-10）**
- ✅ `compiler/ocaml/scripts/verify_llvm_ir.sh` と `test_llvm_verify.ml` を利用した `llvm-as` → `opt -verify` → `llc` チェックをテストスイートに統合済み。
- ⏳ CI ワークフロー内に専用ステップとして検証ログを保存する処理、クロスリンク実行（`--cross`）、結果アーティファクト化が未構築。
- ⏳ Valgrind / ASan 以外の LLVM パス（`opt -passes='default<O2>'` 等）やリンクテストは今後の拡張が必要。

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

**ステータス（2025-10-10）**
- ✅ ランタイム成果物（`libreml_runtime.a`・`.o`）を `actions/upload-artifact@v4` で保持（成功時 30 日）。
- ⏳ コンパイラバイナリ、LLVM IR・Bitcodeなど中間生成物の収集、成功時の統合レポート整理は未実装。
- ⏳ 定期的なアーティファクト削除・命名規則レビューが未着手。

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

**ステータス（2025-10-10）**
- ✅ `tooling/ci/docker/metrics.json` にコンテナ計測値（ビルド時間・スモークテスト時間）を記録し始めている。
- ⏳ CI 実行結果から `0-3-audit-and-metrics.md` への自動追記、GitHub Actions バッジ整備、失敗時の `0-4-risk-handling.md` 連携は未実装。
- ⏳ メトリクス可視化（グラフ化）や Slack/メール通知は検討段階。

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

**ステータス（2025-10-10）**
- ✅ Docker イメージと実行スクリプト（`scripts/docker/build-runtime-container.sh`, `scripts/docker/run-runtime-tests.sh`, `scripts/docker/smoke-linux.sh`）を整備し、Linux 環境の追試を再現可能。
- ⏳ `scripts/ci-local.sh` など CI 手順を一括再現するスクリプトは未作成。`tooling/ci/README.md` に TODO として記載。
- ⏳ Docker イメージの公開タグ管理、ドキュメントの常時更新（Troubleshooting など）は継続タスク。

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

## macOS 向けビルド計画の再検討

### 現状認識
- 開発者の主環境は macOS だが、正式な macOS ビルドパイプラインは Phase 3 以降に後ろ倒しされている。
- Phase 2 では Windows 対応（[2-6-windows-support.md](2-6-windows-support.md)）がクリティカルパスになるため、macOS を長期間放置すると検証遅延が顕著になる。
- 既存の Linux CI 設計（本ドキュメント）を流用できるが、ランタイム ABI（Mach-O）、署名/Notarization、ARM64 対応など macOS 固有要素の調整が必要。

### 着手タイミング候補の比較

| タイミング | 長所 | 課題・前提 | 推奨アクション |
|------------|------|------------|-----------------|
| **1.8（Phase 1 内新設）** | Linux CI 設計をほぼそのまま転用可能。開発者環境との不整合を早期解消。Phase 2 で Windows と並行する負荷を軽減。 | GitHub Actions macOS ランナーの安定運用、LLVM 18（もしくは同世代）の Homebrew 提供状況と Xcode Command Line Tools の整備、Mach-O ランタイム差分の暫定整理。 | 1-8 計画書を新設し、クロスビルド + 最小テストの確立をゴールに設定。`0-3-audit-and-metrics.md` に macOS 測定項目を追加。 |
| **2.7（Phase 2 内新設）** | Windows 対応と同じフェーズでマルチターゲット化を図れる。型クラス・診断強化との整合を同時検証。 | Phase 2 の負荷増大（Windows と macOS の二正面作戦）。開発者の手元検証は引き続き Linux クロスビルド依存。 | 2-7 計画書を合わせて起案し、Windows 2-6 の成果物（クロスツールチェーン）を流用。macOS ARM64 のテスト観測点を Windows 計画と共通化。 |
| **Phase 3 以降** | Self-Host 準備と同時に ARM64 macOS を含む本格対応ができる。 | macOS 開発者が Phase 2 まで常にクロスビルドを強いられる。セルフホスト移行と衝突すると検証リソースが逼迫。 | Phase 3 の 3-3 クロスコンパイル計画に吸収。Docker/仮想化に頼る前提で進行。 |

現状の開発リソースと macOS 利用率を踏まえ、**1.8 での早期着手**を推奨する。Phase 1 のコンテキストに留めることで、Linux CI と構成要素を共有しつつ、Phase 2 の Windows 対応と分離してリスクを平準化できる。

### 推奨シナリオ（1.8 macOS プレビルド対応）
1. **1-8 計画書の雛形作成**  
   - スコープ: GitHub Actions macOS ランナーでの `dune build` / `dune test`、Mach-O ターゲットの LLVM 検証（`-target x86_64-apple-darwin`）。  
   - 成果物: `bootstrap-macos.yml`（ワークフロー）、`tooling/ci/macos/`（セットアップスクリプト）。
2. **ランタイム互換性評価**  
   - `runtime/native` のビルド手順を `clang` + `libtool` ベースで再検証し、Mach-O 向けビルドルールを追加。  
   - Phase 1-5 ランタイム連携計画と矛盾しないよう `docs/plans/bootstrap-roadmap/1-5-runtime-integration.md` に必要な TODO を明示。
3. **ツールチェーン差分の記録**  
   - `docs/notes/llvm-spec-status-survey.md` に macOS 向けの LLVM 入手手順、`codesign` テストの観測点を追記。  
   - `0-3-audit-and-metrics.md` へ macOS 計測行項目（ビルド時間、IR 検証結果、ランタイムサイズ）を追加。
4. **移行準備リストの整備**  
   - `compiler/ocaml/README.md` に macOS 手元検証手順（Homebrew での依存導入、`opam switch create 4.14.2` 等）を記載。  
   - `docs/plans/bootstrap-roadmap/SUMMARY.md` に 1-8 の新タスクを登録し、Phase 2 以降での依存関係を更新。

### Phase 1-7 と並行して進める前倒し準備
- **CI 設計の抽象化**: Linux ワークフローのジョブ定義を `tooling/ci/templates/` に切り出し、macOS ジョブとの共通化を見据える。
- **キャッシュ/依存パラメータの整理**: `actions/cache` のキーをターゲット（`linux`, `macos`）別に分離できるよう、1-7 の段階で戦略を文書化。
- **クロスビルドチェックポイント**: 1-7 で生成するアーティファクトに `llvm-ir/macos` プレースホルダを用意し、後続フェーズで macOS 産出物を追加する導線を確保。
- **リスク登録**: macOS 固有問題（Xcode CLT の更新ズレ、GitHub Actions コスト）を `0-4-risk-handling.md` に記録し、1.8 計画の開始条件として管理。

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
