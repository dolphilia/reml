# 1.5 ランタイム連携計画

## 目的
- Phase 1 マイルストーン M3/M5 で必要となる最小ランタイム API (`mem_alloc`, `panic`, `inc_ref`, `dec_ref`) を整備し、生成した LLVM IR とリンク可能にする。
- 参照カウント (RC) ベースの所有権モデルを `docs/guides/compiler/llvm-integration-notes.md` §5 に沿って実装し、リーク/ダングリング検出テストを提供する。

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
- 最小ランタイム（`docs/guides/compiler/llvm-integration-notes.md` §5.4 / `docs/notes/backend/llvm-spec-status-survey.md` §2.5）と同一の関数セットを採用
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
- ヒープオブジェクトヘッダ: `{ uint32_t refcount; uint32_t type_tag; }`（RC ベース、型タグは `docs/notes/backend/llvm-spec-status-survey.md` の分類に合わせる）
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
- 追加フックが必要な場合は Phase 2 の TODO として `docs/notes/backend/llvm-spec-status-survey.md` に記録

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
- `docs/guides/compiler/llvm-integration-notes.md` へのランタイムセクション追加
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
- パッケージバージョンと導入理由を `docs/notes/backend/llvm-spec-status-survey.md` に追記し、LLVM 依存のドリフトを監視する

9.2. **ビルドと配布の自動化**
- `scripts/docker/build-runtime-container.sh` を追加し、`docker buildx` と `podman` の両方でビルドできるようエントリポイントを統一
- GitHub Container Registry (`ghcr.io/reml/bootstrap-runtime:<version>`) へのプッシュ手順を `tooling/ci/README.md` に記載し、CI とローカルで同一イメージを共有
- コンテナビルドログとベースレイヤのハッシュを `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` に記録し、破損時のロールバック手順（最新安定版タグへの切替）を明示
- `1-7-linux-validation-infra.md` のワークフローからコンテナタグを参照し、CI 側でのキャッシュ再利用戦略（`build-push-action`）を同期

9.3. **開発者向け利用ガイド**
- `scripts/docker/run-runtime-tests.sh` を用意し、`dune build`, `dune test`, `scripts/verify_llvm_ir.sh`, `make -C runtime/native runtime` を一括実行できるようにする
- クロスコンパイル成果物をコンテナ内で検証するためのラッパー `scripts/docker/run-cross-binary.sh` を追加し、`artifacts/cross/` 配下の ELF を `run-runtime-tests.sh -- "<cmd>"` で安全に起動できるようにする（exit code と stdout/stderr をホストに反映）
- ボリュームマウント（`-v $(pwd):/workspace`）と UID/GID 調整オプションを定義し、macOS/Windows ホストでもパーミッション崩れを防ぐ
- `compiler/ocaml/README.md` のチェックリストへ Docker ワークフローを追加し、開発者が CI と同一環境で検証できることを明示
- Podman/Colima 利用時の注意点（cgroup v2、rootless 実行）を `tooling/ci/docker/README.md` にまとめ、既存の CI ガイドと差分管理する

9.4. **検証とメトリクス**
- コンテナ内で Valgrind/ASan を実行し、ホスト環境のレポートと一致するかを比較
- `tooling/ci/docker/metrics.json` にビルド時間・テスト時間・イメージサイズを記録し、`0-3-audit-and-metrics.md` に集計値を転記
- コンテナ更新後は `scripts/docker/smoke-linux.sh` を追加し、Phase 1 のスモークテストを 5 分以内で完了できるか測定する
- `docs/plans/bootstrap-roadmap/1-7-linux-validation-infra.md` と同期し、CI ジョブの `container` タグ変更時にレビューチェック項目を設ける

### 10. macOS → Linux x86_64 クロスコンパイル環境整備（15-16週目）
**担当領域**: ネイティブ macOS からのクロスビルドとエミュレーションテスト

**依存関係と準備**
- `docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md` で定義したターゲット設定と DataLayout を前提とし、`compiler/ocaml/src/llvm_gen/target_config.ml` の出力と整合させる。
- `docs/notes/backend/cross-compilation-spec-update-plan.md` の Phase A/B に沿って `RunConfigTarget`・`TargetCapability` を維持し、ツールチェーン側のメタデータと突き合わせる。
- `compiler/ocaml/docs/technical-debt.md` の High 優先度項目（型マッピング TODO、CFG 線形化）を並行解消し、クロスビルド検証での偽陽性を避ける。

**完了基準**
- macOS (ARM64/Intel) で `scripts/toolchain/prepare-linux-x86_64.sh` → `scripts/cross/run-linux-remote.sh` の順に実行し、SSH 経由で接続した Linux x86_64 検証ノード上で `examples/hello.reml` 相当の出力が一致する。ローカルでエミュレータを利用できる場合は `scripts/cross/run-linux-qemu.sh` をオプションとして提供し、選択した経路を `tooling/toolchains/metrics.json` に記録する。
- クロスコンパイル成果物のメタデータ (`RunArtifactMetadata`) が `x86_64-unknown-linux-gnu` / `e-m:e-p:64:64-f64:64:64-v128:128:128-a:0:64` と一致し、`0-3-audit-and-metrics.md` に記録される。
- `.github/workflows/bootstrap-linux.yml` の macOS ジョブでクロスビルド → `run-linux-remote.sh --ci` を用いたリモート実行を 10 分以内に完走し、検証ログと計測値が `tooling/toolchains/metrics.json` に反映される。

10.1. **クロスツールチェーン選定と取得**
- LLVM/Clang 既存パイプラインを活用し、`clang --target=x86_64-unknown-linux-gnu` と `ld.lld` を既定とする。Homebrew から `llvm@18`, `lld`, `gnu-tar`, `coreutils`, `pkg-config` を取得し、バージョンを `tooling/toolchains/versions.toml`（新設）で固定する。
- Debian 12 (bookworm) の `sysroot` アーカイブを `tooling/toolchains/cache/debian-bookworm-x86_64.tar.zst` として保存し、解凍後に `usr/lib/x86_64-linux-gnu` などを `sysroot/` 以下に再配置する。glibc バージョンは `2.37` を想定し、更新時は `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` に差分を登録する。
- ツール配置先: `tooling/toolchains/x86_64-unknown-linux-gnu/{bin,lib,sysroot,share}` を階層化し、`bin/x86_64-unknown-linux-gnu-*` 形式でシンボリックリンクを張る。PATH 汚染を避けるため `env.sh` に `export REML_TOOLCHAIN_HOME=$(pwd)/tooling/toolchains/x86_64-unknown-linux-gnu` を記録する。

| アプローチ | 主な構成 | 長所 | 短所 | 運用コスト評価 |
| --- | --- | --- | --- | --- |
| **LLVM/Clang + Debian sysroot**（採用） | Homebrew `llvm` + `lld`, Debian 12 glibc/sysroot tarball, gnu binutils ラッパ | 既存 CodeGen/verify フローと整合、Clang ドライバの挙動をそのまま使える、GLIBC 更新手順が確立済み | sysroot の定期更新が必要、glibc バージョン差異を追跡する負荷 | 🟢 低〜中（四半期更新で管理可能） |
| Zig cc バンドル | `zig` 提供のクロスコンパイル機能（libc, ld 一体） | 単一バイナリで依存が少ない、sysroot 準備不要 | Clang front-end と警告挙動が異なる、Zig リリースサイクルに依存 | 🟠 中（リリース差分検証が必要） |
| crosstool-ng ビルド | 独自にビルドした GCC/glibc toolchain | フルコントロール、glibc/ld のバリエーションを細かく調整可能 | 構築時間・学習コストが高い、LLVM 主体のフローと二重化 | 🔴 高（メンテナンス負荷が大きい） |

> **採用方針**: Phase 1 の LLVM ベースパイプラインを最優先するため、`clang --target=x86_64-unknown-linux-gnu` + Debian sysroot 構成を標準ツールチェーンとして採用する。Zig cc・crosstool-ng はバックアップ案として保持し、問題発生時に検討する。選定理由と比較結果は `docs/notes/backend/llvm-spec-status-survey.md` に追記する。

10.2. **ツールチェーン構築スクリプト**
- `scripts/toolchain/prepare-linux-x86_64.sh` を作成し、Homebrew 経由（オンライン）、アーカイブ展開（オフライン）、事前キャッシュ利用の 3 モードに対応させる。引数で `--brew`, `--archive`, `--cache` を切り替え、再実行時は idempotent になるよう `STAMP` ファイルを配置する。
- ダウンロード済みアーカイブのハッシュを `tooling/toolchains/checksums.txt` にまとめ、`shasum -a 256 --check` を CI で毎回検証する。整合性エラーは `exit 1` し、`docs/plans/bootstrap-roadmap/0-4-risk-handling.md` にイベントを記録する。
- `binutils` 相当（`x86_64-linux-gnu-ar`, `ld`, `ranlib`, `strip`, `objcopy`, `objdump`, `readelf`）を macOS 用に同梱し、`PATH`／`LLVM_CONFIG` の衝突を避けるエイリアスを `tooling/toolchains/env.sh` に定義する。既存の `llvm-config` とは `REML_LLVM_HOME` 変数で切り替える。
- 構築後に `tooling/toolchains/README.md`（新設）へ利用手順・依存パッケージ・確認コマンド（`clang --target=... -v`、`ld.lld --version`、`readelf --help`）を記載し、サポート対象 macOS バージョン（11 以降）と Rosetta 2 の扱いを明記する。

10.3. **クロスリンク & ランタイムビルド検証**
- `make CROSS=1` で `llc` 生成物から ELF バイナリを生成できるよう `runtime/Makefile` にオプションを追加し、`AR`, `LD`, `RANLIB`, `STRIP`, `OBJCOPY` を `$(REML_TOOLCHAIN_HOME)/bin` 側に切り替える。`make CROSS=1 check` でランタイム単体テストが sysroot 上の glibc とリンクできることを確認する。
- `compiler/ocaml/scripts/verify_llvm_ir.sh` に `--cross` フラグを追加し、`llc -mtriple=x86_64-unknown-linux-gnu` → `ld.lld` → `objcopy --set-section-flags` のフローを実行する。実行ログは `artifacts/cross/verify.log` として保存し、CI ではアーティファクト化する。
- サンプルプログラムをクロスビルドして `tooling/toolchains/examples/hello-linux` として保存し、手順書から参照する。成功時の `readelf -h` 出力を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に貼り付け、エントリポイント・ABI を固定する。
- クロス生成した ELF は `scripts/docker/run-cross-binary.sh -- artifacts/cross/hello-linux` などで Docker コンテナ内からスモークテストを実施し、stdout/stderr と exit code を `tooling/toolchains/metrics.json`・`artifacts/cross/remote-run.log` と整合させる。結果はリモート実行結果と並列で比較し、差異があれば `0-4-risk-handling.md` に記録する。
- 生成物の依存ライブラリを `readelf -d`/`ldd`（リモート検証ノード上または sysroot chroot 内）で確認し、必要に応じて `sysroot/lib` へシンボリックリンクを追加する。glibc 以外の依存が発生した場合は `tooling/toolchains/patches/` に差分を記録する。

10.4. **リモート Linux 実行パイプライン**
- Linux x86_64 実機または仮想マシンを検証ノードとして用意し、ホスト名・ユーザー・検証用ディレクトリを `tooling/toolchains/remote-hosts.example.yaml`（新設）に記録する。SSH 鍵は `~/.ssh/reml-linux` を既定とし、CI では GitHub Secrets に格納したキーを参照する。
- `scripts/cross/run-linux-remote.sh` を追加し、`dune build` → クロスリンク → 成果物のパッケージ化（`artifacts/cross/<timestamp>.tar.zst`）→ `scp` で検証ノードへ転送 → `ssh` で実行 → 期待出力の照合 → ログの取得までを自動化する。ログは `artifacts/cross/remote-run.log` に保存し、失敗時はリモート側の stderr/stdout をダンプする。
- 環境差異に備えて `LD_LIBRARY_PATH`, `REML_SYSROOT`, `PATH` を `run-linux-remote.sh` で明示的にエクスポートし、使用した sysroot のハッシュと Linux カーネルバージョンを `tooling/toolchains/metrics.json` に追記する。`--dump-env` オプションで診断情報を収集し、`tooling/ci/README.md` にトラブルシューティングを追加する。
- ローカル即時検証が必要な場合は `scripts/cross/run-linux-vm.sh`（任意、UTM/Parallels 等の VM ラッパー）または従来の `run-linux-qemu.sh` を補助スクリプトとして提供し、どの手段を採用したかを `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` へ記録する。

10.5. **CI・ドキュメント連携**
- `.github/workflows/bootstrap-linux.yml` に `macos-latest` クロスビルドジョブを追加し、`toolchains` キャッシュ（`actions/cache`）と `run-linux-remote.sh --ci` を組み合わせた smoke テストを行う。CI 落下時は `cache: restore-keys` を利用して差分診断を高速化する。
- `compiler/ocaml/README.md` の「直近の準備チェックリスト」にクロスツールチェーン導入手順とリモート実行フロー（および代替エミュレーション手段）を追記し、フェーズ進捗レポートには `Runtime Cross Build Status: GREEN/AMBER/RED` を追加する。
- クロスビルド手順を `docs/guides/compiler/llvm-integration-notes.md` §6 と `docs/notes/backend/cross-compilation-spec-update-plan.md` に反映し、Docker ベースの運用（§9）との役割分担を明文化する。ローカル実行と CI 実行の差異は表形式で整理する。
- 失敗時のトリアージフロー（ツールチェーン破損、sysroot ドリフト、リモートホスト障害／代替エミュレータ不整合）を `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` に登録し、`Severity` と `Mitigation` を記入する。トリアージ担当は Phase 1 チーム（macOS）と Phase 1-7 Linux 検証チームで二重化する。

10.6. **クロス環境監査と継続的メンテナンス**
- 月次で `scripts/toolchain/audit-linux-x86_64.sh` を実行し、`clang --version`・`ld.lld --version`・`readelf -V`・glibc バージョンを収集して `tooling/toolchains/audit.log` に追記する。結果は `0-3-audit-and-metrics.md` にサマリを記録する。
- ツールチェーン更新時は `docs/notes/backend/llvm-spec-status-survey.md` のクロスコンパイルセクションに差分を追記し、`docs/notes/backend/cross-compilation-spec-intro.md` にリンクする。互換性が崩れた場合は `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` で `Severity: High` として管理する。
- `tooling/toolchains/versions.toml` を基準に Renovate/Dependabot で月次チェックを行い、更新候補が出た場合は `docs/plans/bootstrap-roadmap/IMPLEMENTATION-GUIDE.md` の週次レビュー項目に追加する。
- クロスビルド計測値（ビルド時間、ELF サイズ、リモート実行時間／往復遅延）は Phase 3 のセルフホスト移行指標として `docs/plans/bootstrap-roadmap/1-7-linux-validation-infra.md` に転記し、教訓を次フェーズへフィードバックする。

## 成果物と検証
- `runtime/` ディレクトリにソースコードとビルド設定が追加され、`make runtime` や `dune build @runtime` が成功。
- RC テストでリークゼロ、ダングリング検出ゼロを確認し、結果を `0-3-audit-and-metrics.md` に記録。
- CLI で `--link-runtime` オプションが利用可能となり、生成バイナリが x86_64 Linux 上で実行できる。
- `tooling/ci/docker/bootstrap-runtime.Dockerfile` に基づくコンテナを `scripts/docker/run-runtime-tests.sh` で起動し、CI と同一手順でのビルド・検証が成功する。
- macOS 環境で `scripts/toolchain/prepare-linux-x86_64.sh` → `scripts/cross/run-linux-remote.sh` を実行し、クロス生成バイナリが Linux x86_64 検証ノード上で期待通り動作する（必要に応じて `run-linux-qemu.sh` などのローカル代替手段も使用可）。

## リスクとフォローアップ
- macOS 等で開発時にクロスビルドが必要になるため、Docker イメージまたは cross toolchain の利用手順を `docs/notes/backend/llvm-spec-status-survey.md` に共有。
- RC のオーバーヘッドが大きい場合に備え、計測値を Phase 3 のメモリ管理戦略検討へフィードバック。
- ランタイム API が今後拡張されることを想定し、ヘッダにバージョンフィールドと互換性ポリシーを記載しておく。
- Docker ベースイメージの脆弱性や LLVM バージョン差異を検知するため、月次で `docker scout cves`（もしくは `trivy`）を実行し、重大度 High 以上は `0-4-risk-handling.md` に登録してホットフィックスイメージを発行する。
- クロスツールチェーンの sysroot 更新やリモート検証ノード／補助エミュレータ（QEMU 等）のドリフトでビルドが不安定化する可能性があるため、`tooling/toolchains/metrics.json` の更新トリガとロールバック手順を `0-4-risk-handling.md` に明記してメンテナンス負荷を可視化する。

## 参考資料
- [1-0-phase1-bootstrap.md](1-0-phase1-bootstrap.md)
- [guides/llvm-integration-notes.md](../../guides/llvm-integration-notes.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- [notes/llvm-spec-status-survey.md](../../notes/llvm-spec-status-survey.md)
- [1-7-linux-validation-infra.md](1-7-linux-validation-infra.md)
