# 1.5 ランタイム連携計画

## 目的
- Phase 1 マイルストーン M3/M5 で必要となる最小ランタイム API (`mem_alloc`, `panic`, `inc_ref`, `dec_ref`) を整備し、生成した LLVM IR とリンク可能にする。
- 参照カウント (RC) ベースの所有権モデルを `docs/guides/llvm-integration-notes.md` §5 に沿って実装し、リーク/ダングリング検出テストを提供する。

## スコープ
- **含む**: C/LLVM IR で記述された最小ランタイム、メモリアロケータの抽象化（malloc ベース）、参照カウントヘルパ、エラーハンドラ、テスト用検証フック。
- **含まない**: ガベージコレクタ、Fiber/Async ランタイム、Capability Stage の動的切替。これらは Phase 3 以降。
- **前提**: LLVM IR 生成がランタイム関数を呼び出す設計になっていること。x86_64 Linux のツールチェーンが構築済みであること。

## 作業ディレクトリ
- `runtime/native` : C/LLVM 実装とビルドスクリプト
- `runtime/native/tests`（想定）: RC・メモリアロケータのユニット/統合テスト
- `compiler/ocaml/src/codegen` : ランタイム呼び出し側 (FFI 宣言、リンク設定)
- `compiler/ocaml/tests/codegen` : ランタイム連携を含むエンドツーエンドテスト
- `tooling/ci` : ランタイムをリンクする CI ジョブ、Valgrind/ASan 等の検証スクリプト
- `tooling/ci/docker`（新設）: x86_64 Linux 用 Dockerfile・ビルドスクリプト・ローカル再現ドキュメント

## 作業ブレークダウン

### 1. ランタイムAPI設計（13週目）
**担当領域**: ランタイムインタフェース定義

1.1. **必須API仕様策定**
- 最小ランタイム（`docs/guides/llvm-integration-notes.md` §5.4 / `docs/notes/llvm-spec-status-survey.md` §2.5）と同一の関数セットを採用
  - メモリ管理: `void* mem_alloc(size_t size)`, `void mem_free(void* ptr)`
  - 参照カウント: `void inc_ref(void* ptr)`, `void dec_ref(void* ptr)`
  - エラー処理: `void panic(const char* msg)`
  - 観測用ユーティリティ: `void print_i64(int64_t value)`
- 拡張 API（`runtime_init` 等）は将来の Phase 2 以降で検討し、本フェーズでは設計ノートに TODO として記録

**シグネチャ注意事項**:
- **panic の実装形式**: Phase 1 では LLVM IR 側で `declare void @panic(ptr, i64) noreturn` として FAT ポインタ形式（文字列の `{ptr, len}` 表現）で宣言される。C 実装側は `panic(const char* msg)` として受け取り、NULL 終端文字列として扱う。長さパラメータ（i64）は実装側で無視可能。
- **mem_free および print_i64**: これらはコンパイラから直接呼ばれず、実装内部（dec_ref, デバッグ出力等）で使用される。LLVM IR での明示的な宣言は不要。
- **型付き属性との連携**: `sret`/`byval` 属性は `compiler/ocaml/src/llvm_gen/llvm_attr.ml` + C スタブ経由で生成される。mem_alloc が返すポインタは 8 バイト境界に調整済みであり、ABI 規約に準拠する。

1.2. **データ構造定義**
- ヒープオブジェクトヘッダ: `{ uint32_t refcount; uint32_t type_tag; }`（RC ベース、型タグは `docs/notes/llvm-spec-status-survey.md` の分類に合わせる）
- 型タグの割り当て規則と `panic` 診断コードとの対応表
- アラインメント要件（8バイト境界）

1.3. **ヘッダファイル作成**
- `runtime/reml_runtime.h` の作成
- 関数プロトタイプとドキュメントコメント
- バージョン定義（`REML_RUNTIME_VERSION`）

**成果物**: `runtime/reml_runtime.h`, API仕様書

### 2. メモリアロケータ実装（13-14週目）
**担当領域**: メモリ管理機能

2.1. **基本アロケータ**
- `malloc` ベースの単純実装
- アロケーション失敗時のエラー処理
- ヘッダ領域の初期化（refcount=1, type_tag設定）

2.2. **アラインメント処理**
- 8バイト境界への自動調整
- パディング計算の実装
- 構造体レイアウトの検証

2.3. **デバッグ支援**
- アロケーショントラッキング（DEBUG時）
- 二重解放検出
- メモリリーク検出のフック

**成果物**: `runtime/mem_alloc.c`, メモリ管理実装

### 3. 参照カウント実装（14週目）
**担当領域**: RC所有権モデル

3.1. **RC操作関数**
- `inc_ref`: アトミックなカウンタインクリメント（将来の並行対応）
- `dec_ref`: デクリメント + ゼロ時の解放
- 循環参照検出の基礎（Phase 2で本格化）

3.2. **型別解放処理**
- 型タグに基づくデストラクタディスパッチ
- 再帰的な参照カウント減少（子オブジェクト）
- 文字列・タプル・レコードの解放実装

3.3. **テストケース**
- 単純なオブジェクト生成・解放
- ネストした構造体の正しい解放
- リークゼロの検証

**成果物**: `runtime/refcount.c`, RCテスト

### 4. パニックハンドラ実装（14-15週目）
**担当領域**: エラー処理とクラッシュレポート

4.1. **パニック関数実装**
- エラーメッセージの stderr 出力
- ファイル名・行番号の表示
- スタックトレース取得（libunwind使用、オプション）

4.2. **診断情報収集**
- 実行時情報（PID, 時刻等）の付加
- [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) 形式への整形
- ログファイル出力（設定可能）

4.3. **終了処理**
- `panic` からの異常終了コード（`exit(1)`）
- 追加フックが必要な場合は Phase 2 の TODO として `docs/notes/llvm-spec-status-survey.md` に記録

**成果物**: `runtime/panic.c`, パニックテスト

### 5. ビルドシステム整備（15週目）
**担当領域**: ランタイムのビルド設定

5.1. **ビルドスクリプト作成**
- `Makefile` の作成（`make runtime`）
- オブジェクトファイル生成（`.o`）
- 静的ライブラリ生成（`libreml_runtime.a`）

5.2. **コンパイラフラグ設定**
- 最適化レベル（`-O2` デフォルト）
- 警告の有効化（`-Wall -Wextra`）
- デバッグ情報（`-g` オプション）

5.3. **依存関係管理**
- プラットフォーム検出（Linux/macOS）
- ライブラリ依存（libunwind、pthread等）
- インストールターゲット（`make install`）

**成果物**: `runtime/Makefile`, ビルド設定

### 6. LLVM IR連携（15-16週目）
**担当領域**: コンパイラとランタイムの統合

6.1. **ランタイム関数宣言生成**
- LLVM IRでのランタイムシンボル宣言
- 関数属性の付与（`noreturn` for `panic`等）
- リンケージ設定（external）
- `sret` / `byval` など型付き属性は `compiler/ocaml/src/llvm_gen/llvm_attr.ml` の FFI 経由で生成し、構造体シグネチャに正しい型情報を付与する

6.2. **ランタイム呼び出し挿入**
- メモリ割り当て時の `mem_alloc` 呼び出し
- オブジェクト複製時の `inc_ref` 挿入
- スコープ終了時の `dec_ref` 挿入
- エラー時の `panic` 呼び出し

6.3. **リンク手順統合**
- CLI での `--link-runtime` フラグ実装
- `libreml_runtime.a` の自動リンク
- `panic`/RC 関数のシグネチャ整合チェックを CI に組み込み、追加初期化が必要な場合は TODO を記録

**成果物**: `llvm_gen/runtime_link.ml`, リンク統合

### 7. テストと検証（16週目）
**担当領域**: ランタイム品質保証

7.1. **単体テスト**
- 各API関数の境界値テスト
- エラーケース（NULL、不正型タグ等）
- マルチスレッド安全性（Phase 2準備）

7.2. **統合テスト**
- Remlコードからランタイム呼び出しまでの一貫テスト
- リーク検出（Valgrind、ASan）
- ダングリングポインタ検出（ASan、MSan）

7.3. **性能計測**
- アロケーション性能（malloc比）
- RC操作オーバーヘッド
- `0-3-audit-and-metrics.md` への記録

**成果物**: ランタイムテストスイート、性能レポート

### 8. ドキュメントとCI統合（16週目）
**担当領域**: 文書化とCI設定

8.1. **API仕様書整備**
- `docs/guides/llvm-integration-notes.md` へのランタイムセクション追加
- 各関数の詳細仕様とサンプルコード
- 型タグ一覧表の作成

8.2. **CI設定**
- GitHub Actions でのランタイムビルドジョブ
- テスト実行（Valgrind統合）
- アーティファクト収集（`.a` ファイル）

8.3. **技術文書作成**
- ランタイムアーキテクチャ解説
- RC所有権モデルの説明
- Phase 2への引き継ぎ（GC、非同期等）

**成果物**: 完全なドキュメント、CI統合

### 9. Linux x86_64 Docker 環境整備（15-16週目）
**担当領域**: ローカルテスト・CI 共有基盤

9.1. **ベースイメージ設計**
- `ubuntu:22.04`（または互換性のある LTS リリース）をベースに、System V ABI 準拠の `x86_64-unknown-linux-gnu` ツールチェーンを事前構築
- `opam`, `dune`, `llvm-18`, `clang`, `gcc`, `make`, `valgrind`, `libunwind` をインストールし、Phase 1 のビルド・テストが容器内で完結するようにする
- `tooling/ci/docker/bootstrap-runtime.Dockerfile` を作成し、ベースイメージのタグ・インストール手順・検証コマンドをコメントとして明記
- パッケージバージョンと導入理由を `docs/notes/llvm-spec-status-survey.md` に追記し、LLVM 依存のドリフトを監視する

9.2. **ビルドと配布の自動化**
- `scripts/docker/build-runtime-container.sh` を追加し、`docker buildx` と `podman` の両方でビルドできるようエントリポイントを統一
- GitHub Container Registry (`ghcr.io/reml/bootstrap-runtime:<version>`) へのプッシュ手順を `tooling/ci/README.md` に記載し、CI とローカルで同一イメージを共有
- コンテナビルドログとベースレイヤのハッシュを `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` に記録し、破損時のロールバック手順（最新安定版タグへの切替）を明示
- `1-7-linux-validation-infra.md` のワークフローからコンテナタグを参照し、CI 側でのキャッシュ再利用戦略（`build-push-action`）を同期

9.3. **開発者向け利用ガイド**
- `scripts/docker/run-runtime-tests.sh` を用意し、`dune build`, `dune test`, `scripts/verify_llvm_ir.sh`, `make -C runtime/native runtime` を一括実行できるようにする
- ボリュームマウント（`-v $(pwd):/workspace`）と UID/GID 調整オプションを定義し、macOS/Windows ホストでもパーミッション崩れを防ぐ
- `compiler/ocaml/README.md` のチェックリストへ Docker ワークフローを追加し、開発者が CI と同一環境で検証できることを明示
- Podman/Colima 利用時の注意点（cgroup v2、rootless 実行）を `tooling/ci/docker/README.md` にまとめ、既存の CI ガイドと差分管理する

9.4. **検証とメトリクス**
- コンテナ内で Valgrind/ASan を実行し、ホスト環境のレポートと一致するかを比較
- `tooling/ci/docker/metrics.json` にビルド時間・テスト時間・イメージサイズを記録し、`0-3-audit-and-metrics.md` に集計値を転記
- コンテナ更新後は `scripts/docker/smoke-linux.sh` を追加し、Phase 1 のスモークテストを 5 分以内で完了できるか測定する
- `docs/plans/bootstrap-roadmap/1-7-linux-validation-infra.md` と同期し、CI ジョブの `container` タグ変更時にレビューチェック項目を設ける

## 成果物と検証
- `runtime/` ディレクトリにソースコードとビルド設定が追加され、`make runtime` や `dune build @runtime` が成功。
- RC テストでリークゼロ、ダングリング検出ゼロを確認し、結果を `0-3-audit-and-metrics.md` に記録。
- CLI で `--link-runtime` オプションが利用可能となり、生成バイナリが x86_64 Linux 上で実行できる。
- `tooling/ci/docker/bootstrap-runtime.Dockerfile` に基づくコンテナを `scripts/docker/run-runtime-tests.sh` で起動し、CI と同一手順でのビルド・検証が成功する。

## リスクとフォローアップ
- macOS 等で開発時にクロスビルドが必要になるため、Docker イメージまたは cross toolchain の利用手順を `docs/notes/llvm-spec-status-survey.md` に共有。
- RC のオーバーヘッドが大きい場合に備え、計測値を Phase 3 のメモリ管理戦略検討へフィードバック。
- ランタイム API が今後拡張されることを想定し、ヘッダにバージョンフィールドと互換性ポリシーを記載しておく。
- Docker ベースイメージの脆弱性や LLVM バージョン差異を検知するため、月次で `docker scout cves`（もしくは `trivy`）を実行し、重大度 High 以上は `0-4-risk-handling.md` に登録してホットフィックスイメージを発行する。

## 参考資料
- [1-0-phase1-bootstrap.md](1-0-phase1-bootstrap.md)
- [guides/llvm-integration-notes.md](../../guides/llvm-integration-notes.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- [notes/llvm-spec-status-survey.md](../../notes/llvm-spec-status-survey.md)
- [1-7-linux-validation-infra.md](1-7-linux-validation-infra.md)
