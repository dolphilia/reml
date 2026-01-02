# Linux CI ローカル再現セッション計画（Ubuntu 24.04 LTS）

## 目的
- `docs/plans/bootstrap-roadmap/2-4-diagnostics-audit-pipeline.md` の Phase 2 M3 作業の一環として、Ubuntu 24.04 LTS 上で Linux CI 失敗（LLVM リンクエラー）を再現・解消する。
- GitHub Actions `bootstrap-linux.yml` の `Build` ジョブで観測された `LLVMConstStringInContext2` 欠落と `terminfo` 系シンボル未解決の要因をローカルで分解し、解決策と CI フィードバックループを構築する。
- 実施ログと問題解析の記録を残し、再発時の復元手順およびワークフロー修正提案に活用する。

## 前提
- ホストは Ubuntu 24.04 LTS クリーンインストール直後で、導入済みのアプリケーションは Chrome / Visual Studio Code / Git のみ。
- ネットワークアクセスは常時可能と想定し、必要なパッケージのインストールを許可済み。
- `docs/notes/backend/linux-ci-llvm-link-error-report.md` の調査内容をベースに手順を補う。

## 1. 初期環境確認
1. `neofetch` または `hostnamectl` で OS バージョンとカーネルを記録。
2. `sudo apt update && sudo apt upgrade` を実行し、適用結果をログとして保存（`logs/2025-ubuntu2404-apt-upgrade.log` を想定）。
3. `dpkg -l | rg llvm`・`dpkg -l | rg clang` を実行し、プリインストール済み LLVM / Clang バージョンを把握。

## 2. 必要パッケージの導入
以下コマンドを順に実行し、各ステップで `script` コマンドによるログを取得する。

```bash
sudo apt install -y build-essential clang-18 lld-18 cmake ninja-build pkg-config libncurses-dev libtinfo-dev libffi-dev libgmp-dev libssl-dev zlib1g-dev python3-full python3-venv python3-pip git curl rsync
sudo update-alternatives --install /usr/bin/clang clang /usr/bin/clang-18 50
sudo update-alternatives --install /usr/bin/clang++ clang++ /usr/bin/clang++-18 50
```

- `docs/notes/backend/linux-ci-llvm-link-error-report.md` で指摘された `terminfo` 依存解決のため `libncurses-dev` / `libtinfo-dev` を必須とする。
- `clang-18` は opam スイッチ内 LLVM と整合させるために導入（`llvm-config` のバージョン差異調査に使用）。
- 実行ログ: `logs/2025-ubuntu2404-apt-install.log`（対象パッケージはいずれも最新で追加インストールなし。自動導入済みの `libllvm19` について `sudo apt autoremove` 提案あり）。
- `update-alternatives` 実行ログ: `logs/2025-ubuntu2404-update-alternatives.log`, `logs/2025-ubuntu2404-update-alternatives-2.log`（両コマンドとも正常終了）。
- `update-alternatives` のリンク状態確認: `logs/2025-ubuntu2404-toolchain-status.md`（`clang`/`clang++` が 18 系を指していることを記録）。
- 不要ライブラリ削除: 同ログに `sudo apt autoremove` の結果を保存し、`libllvm19` のアンインストールと 129 MB 解放を反映。

## 3. opam スイッチ構築
1. `sudo apt install -y opam`（未導入の場合）。
2. `opam init --bare --disable-sandboxing`（CI 再現に焦点を当て、sandbox の影響を回避）。
3. `opam switch create reml-ci 5.2.1` を実行。
4. `eval "$(opam env)"` を `~/.bashrc` と `~/.profile` に追記し、再ログイン後も反映されることを確認。
5. `opam repo add default https://opam.ocaml.org`（必要に応じて）。

- 実行ログ: `logs/2025-ubuntu2404-opam-setup.log`（`opam` パッケージ群のインストール、`opam init`、`opam switch create`、`opam repo add` を連続記録。`opam` が最新ではない旨の警告あり）。
- 環境適用コマンドは `logs/2025-ubuntu2404-opam-env.sh` に保存し、`eval "$(cat ...)"` で即時反映。
- `~/.bashrc` / `~/.profile` に `eval "$(opam env --switch reml-ci --set-switch)"` を追記済み（重複チェック済み）。
### 3.1 依存パッケージ導入
- コマンドは `compiler/ocaml` ディレクトリ内で実行し、`reml_ocaml.opam` を参照できるようにする（ログ取得コマンドでは明示的に `cd` する）。
```bash
opam install . --deps-only --with-test --yes
opam install conf-llvm-18 llvm.18.1.8 conf-pkg-config conf-zlib conf-libffi --yes
```
- `opam install . --deps-only --with-test --yes` は `compiler/ocaml` で実行し、`logs/2025-ubuntu2404-opam-deps.log` に記録。Ubuntu 24.04 + opam default repo では `conf-llvm-static.19` と `llvm.19-static` が導入され、`libzstd-dev` / `llvm-19-dev` などの system パッケージが追加で必要になった。
- `conf-llvm-18` / `llvm.18.1.8` は同日時点のリポジトリに存在せず、同ログにエラー（パッケージ未検出）を記録。代替案の検討が必要。
- `which llvm-config` と `opam var prefix` の確認は `logs/2025-ubuntu2404-opam-deps.log` に記録。プレーンな `llvm-config` は PATH 上に存在しない一方で、`/usr/bin/llvm-config-18` と `/usr/bin/llvm-config-19` が確認できた（`logs/2025-ubuntu2404-toolchain-status.md` 参照）。バージョン付きバイナリを利用する運用へ切り替える検討が必要。
- `llvm-config-18 --version` は 18.1.3、`llvm-config-19 --version` は 19.1.1（`logs/2025-ubuntu2404-toolchain-status.md`）。`LLVM_CONFIG=/usr/bin/llvm-config-19` のように環境変数を指定するか、`/usr/local/bin/llvm-config` へシンボリックリンク（例: `sudo ln -s /usr/bin/llvm-config-19 /usr/local/bin/llvm-config`）を張る案を要検討。
- `python3 compiler/ocaml/scripts/gen_llvm_link_flags.py` の実行結果を `logs/2025-ubuntu2404-llvm-config19.log`（`-lLLVM-19`）と `logs/2025-ubuntu2404-llvm-config18.log`（`-lLLVM-18`）に記録。両者とも opam ライブラリ → system ライブラリの順で `-L` が並び、`-Wl,-rpath` が付与されていることを確認。
- `ldd compiler/ocaml/_build/default/src/main.exe` の結果を `logs/2025-ubuntu2404-ldd-main.log` に保存。現状は標準 `glibc` / `libz` / `libzstd` / `libstdc++` を参照しており、`rpath` 経由での LLVM 参照は動的リンク時に解決される。
- `compiler/ocaml/_build/default/src/llvm_gen/llvm-link-flags.sexp` を `logs/2025-ubuntu2404-llvm-link-flags.sexp.log` に保存（生成物は `_build/default/...` 配下に出力される点に注意）。
- 現状は `llvm-config-19` を優先採用し、失敗時に `llvm-config-18` へ切り替える方針とする。環境変数での明示指定とログ取得コマンドは以下を利用。  
  - 19 を試す場合  
    ```bash
    script -q -c 'bash -lc "set -euo pipefail; cd /home/dolphilia/github/reml; eval \"$(cat logs/2025-ubuntu2404-opam-env.sh)\"; LLVM_CONFIG=/usr/bin/llvm-config-19 python3 compiler/ocaml/scripts/gen_llvm_link_flags.py"' logs/2025-ubuntu2404-llvm-config19.log
    script -q -c 'bash -lc "set -euo pipefail; cd /home/dolphilia/github/reml; eval \"$(cat logs/2025-ubuntu2404-opam-env.sh)\"; opam exec -- env LLVM_CONFIG=/usr/bin/llvm-config-19 dune build -j1 --verbose"' logs/2025-ubuntu2404-dune-build-llvm19.log
    ```
  - 18 を試す場合  
    ```bash
    script -q -c 'bash -lc "set -euo pipefail; cd /home/dolphilia/github/reml; eval \"$(cat logs/2025-ubuntu2404-opam-env.sh)\"; LLVM_CONFIG=/usr/bin/llvm-config-18 python3 compiler/ocaml/scripts/gen_llvm_link_flags.py"' logs/2025-ubuntu2404-llvm-config18.log
    script -q -c 'bash -lc "set -euo pipefail; cd /home/dolphilia/github/reml; eval \"$(cat logs/2025-ubuntu2404-opam-env.sh)\"; opam exec -- env LLVM_CONFIG=/usr/bin/llvm-config-18 dune build -j1 --verbose"' logs/2025-ubuntu2404-dune-build-llvm18.log
    ```
- `dune build` の成功/失敗と出力差分をログ化し、リンクエラー再現の有無を記録する。
- `LLVM_CONFIG=/usr/bin/llvm-config-19` で `dune build -j1 --verbose` を実行し、`logs/2025-ubuntu2404-dune-build-llvm19.log` に成功ログを保存（`script` が PTY 要求で失敗したため標準出力リダイレクトで取得）。リンクエラーは再現せず。
- `LLVM_CONFIG=/usr/bin/llvm-config-18` でも同コマンドを実行し、`logs/2025-ubuntu2404-dune-build-llvm18.log` に成功ログを保存（キャッシュヒットのため出力は最小）。
- シェル起動時に同設定を適用する場合は `~/.bashrc` 等へ `export LLVM_CONFIG=/usr/bin/llvm-config-19` を追記（権限不足のため未適用、手動対応が必要）。
- `opam switch list` と `opam list --installed | rg llvm` を実行し、`reml-ci` スイッチと `conf-llvm-static.19` / `llvm.19-static` の導入状況をログ化。

## 4. レポジトリ取得とクリーンビルド準備
1. `git clone git@github.com:<org>/reml.git` または HTTPS で取得。
2. `cd reml` 後、`git submodule update --init --recursive`（必要なら）。
3. `opam exec -- dune clean` と `rm -rf _build tmp` を実行してキャッシュを除去。

## 5. リンクフラグ生成と確認手順
1. `python3 compiler/ocaml/scripts/gen_llvm_link_flags.py` を実行。
2. `cat compiler/ocaml/src/llvm_gen/llvm-link-flags.sexp` をログに保存し、`-Wl,-rpath` と `-lncursesw` が含まれるかチェック。
3. `llvm-config --version --libdir --system-libs --components` を opam / system 両方で取得し、差異を比較。

## 6. ビルド実行と診断
1. `opam exec -- dune build -j1 --verbose 2>&1 | tee logs/2025-dune-build.log` を実行。
2. 失敗した場合はログから `LLVMConstStringInContext2` や `set_curterm` 等のエラー有無を抽出して Markdown に追記。
3. 成功時は `ldd _build/default/src/main.exe | tee logs/2025-ldd-main.log` でリンク先を検証。

## 7. 調査ログ整理テンプレート
`docs/notes/backend/linux-ci-local-setup-2025.md` と同階層に `logs/` ディレクトリを作成し、以下テンプレートで Markdown を記録する。

```
## YYYY-MM-DD セッション
- **ホスト情報**: `uname -a` / `lsb_release -a`
- **実行ステップ**:
  1. コマンド + 目的
  2. 実行ログファイルパス
- **結果概要**: 成功/失敗、主要エラーメッセージ
- **仮説**: 原因と次の検証案
- **CI 連携メモ**: GitHub Actions へのフィードバック項目、必要なワークフロー修正
```

## 8. GitHub Actions へのフィードバック準備
- ローカルで得た `llvm-link-flags.sexp` と `ldd` の差分を `docs/notes/backend/linux-ci-llvm-link-error-report.md` へ追記する案を整理。
- `bootstrap-linux.yml` の `Build` ジョブで追跡すべき追加ログ（`llvm-config --version`, `cat llvm-link-flags.sexp`）を列挙し、Pull Request のチェックリストとして提案する。
- Linux ランナーでは `export LLVM_CONFIG=/usr/bin/llvm-config-19` をワークフロー環境に設定し、失敗時のみ 18 へフォールバックする手順を検討（手元検証ログ: `logs/2025-ubuntu2404-dune-build-llvm19.log`, `logs/2025-ubuntu2404-dune-build-llvm18.log`）。
- ローカルの `ldd` 結果（`logs/2025-ubuntu2404-ldd-main.log`）は glibc 系ライブラリのみで、LLVM は静的リンク済み。GitHub Actions 側でも同様の `ldd` 出力を取得し、差分が出るか確認する。
- ワークフローへの追記案（`bootstrap-linux.yml` の該当ジョブ内）:
  ```yaml
        - name: Dump llvm linker flags
          run: cat compiler/ocaml/_build/default/src/llvm_gen/llvm-link-flags.sexp

        - name: Inspect main binary deps
          run: ldd compiler/ocaml/_build/default/src/main.exe
  ```

## 9. 次のステップ
1. 上記手順を順番に実行し、ログファイルと Markdown 記録を作成。
2. 取得ログを分析し、`llvm-config` の指すライブラリ順序と `terminfo` 依存の解決状況を整理。
3. 解決策が明確になった段階で、`tooling/ci` スクリプトや CI ワークフローの修正案を起草し、レビューに備える。

## 10. ログ記録進捗（2025-10-23 更新）
- 初期環境確認（`uname -a`, `lsb_release -a`, `hostnamectl`, `dpkg -l | rg llvm`, `dpkg -l | rg clang`）の結果を `logs/2025-ubuntu2404-initial-baseline.md` に記録。
- `sudo apt update && sudo apt upgrade` の実行ログを `logs/2025-ubuntu2404-apt-upgrade.log` に保存済み。
- 必要パッケージインストールと `update-alternatives` 設定のログを `logs/2025-ubuntu2404-apt-install.log`, `logs/2025-ubuntu2404-update-alternatives.log`, `logs/2025-ubuntu2404-update-alternatives-2.log` に保存済み。
- `update-alternatives` の確認結果と `sudo apt autoremove` による `libllvm19` 削除ログを `logs/2025-ubuntu2404-toolchain-status.md` に追記済み。
- `opam` セットアップ（`opam init` / `switch create` / `repo add`）を `logs/2025-ubuntu2404-opam-setup.log` に記録し、環境適用スクリプトを `logs/2025-ubuntu2404-opam-env.sh` に保存済み。
- 依存パッケージ導入（`opam install . --deps-only --with-test --yes`）、`conf-llvm-18` 未検出エラー、および `which llvm-config` / `opam switch list` 等の確認ログを `logs/2025-ubuntu2404-opam-deps.log` に追記済み。
- `llvm-config` 切り替えの出力（`logs/2025-ubuntu2404-llvm-config19.log`, `logs/2025-ubuntu2404-llvm-config18.log`）を取得済み。
- `LLVM_CONFIG=/usr/bin/llvm-config-19` / `llvm-config-18` を指定した `dune build -j1 --verbose` の成功ログを `logs/2025-ubuntu2404-dune-build-llvm19.log`, `logs/2025-ubuntu2404-dune-build-llvm18.log` に保存。
- 動的リンク検証 (`logs/2025-ubuntu2404-ldd-main.log`) と `llvm-link-flags.sexp` 内容 (`logs/2025-ubuntu2404-llvm-link-flags.sexp.log`) を保存。

## 4. 2025-10-24 調査メモ（CI/CD 統合準備）

### 4.1 調査サマリー
- `docs/plans/bootstrap-roadmap/2-4-diagnostics-audit-pipeline.md` の CI/CD 統合項目を踏まえ、Linux CI とローカル再現フローの差分を精査した。
- `bootstrap-linux.yml` 現行定義と `scripts/ci-local.sh` を比較し、`--emit-audit`/`--audit-store` の統一入口が未導入である点を確認した。
- `logs/2025-ubuntu2404-llvm-link-flags.sexp.log` から、`llvm-config` の優先順が混在しており、opam 由来の LLVM ライブラリを必ずしも先に解決できていない状況を把握した。

### 4.2 判明した課題と仮説
- ローカル/CI 双方で `LLVM_CONFIG` が system インストール（/usr/bin/llvm-config-19 等）へフォールバックするケースがあり、`libLLVM` のバージョンがずれると `LLVMConstStringInContext2` 未解決が再発し得る。
- `gen_llvm_link_flags.py` は opam ディレクトリを優先追加しているが、`dune clean` を挟まない場合や `LLVM_CONFIG` を外部で上書きした場合に旧フラグが残存する恐れがある。
- CI 側 `audit-matrix` ジョブがアーティファクト収集・メトリクス計測を自動化しているのに対し、ローカルフローでは `collect-iterator-audit-metrics.py` を明示的に実行しないと pass_rate ゲートを再現できない。

### 4.3 直近アクションプラン
- `scripts/ci-local.sh` に `--emit-audit` と `--audit-store` 引数を追加し、`REMLC` 実行後に `scripts/ci-validate-audit.sh` と `tooling/ci/collect-iterator-audit-metrics.py` をチェーンさせる。成果物は CI と同じく `reports/audit/<platform>/<run_id>/` へ配置する。
- ローカル再現時は `LLVM_CONFIG="$(opam var prefix)/bin/llvm-config"` を明示し、`dune clean` → `python3 compiler/ocaml/scripts/gen_llvm_link_flags.py` → `cat compiler/ocaml/src/llvm_gen/llvm-link-flags.sexp` → `ldd _build/default/src/main.exe` をワンセットで実行して差分を記録する。
- テストフェーズを `dune runtest` / `make runtime` / Valgrind / ASan に段階分割し、失敗ログを `logs/2025-ubuntu2404-test-*.log` として収集。特に Valgrind で `libasan` を利用するステップは `clang-18`/`lld-18` インストール有無を再確認する。

### 4.4 参照ファイルとログ
- 仕様・計画: `docs/plans/bootstrap-roadmap/2-4-diagnostics-audit-pipeline.md`, `docs/plans/bootstrap-roadmap/2-4-status.md`
- CI 定義: `.github/workflows/bootstrap-linux.yml`, `scripts/ci-local.sh`
- ログ: `logs/2025-ubuntu2404-llvm-link-flags.sexp.log`, `logs/2025-ubuntu2404-dune-build-llvm19.log`, `logs/2025-ubuntu2404-toolchain-status.md`

### 4.5 テスト実行結果（2025-10-24 夕方）
- `compiler/ocaml` で `dune runtest` を実行したところ、`llc` と `llvm-as` が PATH 上に存在せず LLVM パイプライン系テストが失敗。
  - `tests/test_user_impl_execution.ml` 内 `test_llvm_ir_validation` / `test_ir_to_object` が `llc` 未検出 (`exit code 127`) により失敗。
  - `tests/test_typeclass_execution.ml` でも同様に `llc` 未検出で `test_ir_to_object` が失敗。
  - `tests/test_llvm_verify.ml` は `llvm-as` を呼び出せず、メッセージとして「LLVM 15+ を想定」と出力。
- 原因: `sudo apt install llvm-18 llvm-18-tools` はインストール済みだが、`/usr/bin/llc` などのエイリアスを作成する `sudo ln -sf /usr/bin/llc-19 /usr/bin/llc`（または `-18`）等の手順を未実施。
- 対応予定アクション:
  1. `.github/workflows/bootstrap-linux.yml` と同じく `llvm-as`, `opt`, `llc` へのシンボリックリンクを作成し、`llc --version` 等で確認。
  2. 再度 `dune runtest` を実行し、LLVM 関連テストの通過を確認。成功ログは `logs/2025-ubuntu2404-dune-runtest-after-llvm-tools.log` に保存する。
  3. テスト成功後に Valgrind / ASan ステップも順次検証し、必要な依存パッケージが揃っているかを確認する。
- 対策: `compiler/ocaml/tests/support/llvm_toolchain_helpers.ml` を追加し、`llc`/`llvm-as`/`opt` を `llc-19`（なければ `-18`）などのバージョン付きバイナリから自動検出するよう更新した。`verify_llvm_ir.sh` も同様にフォールバック探索を実装し、ローカル環境でバージョン番号付きコマンドしか存在しない場合でもテスト実行が継続できる。
