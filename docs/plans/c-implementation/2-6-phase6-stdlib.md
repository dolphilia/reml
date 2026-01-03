# フェーズ 6: 標準ライブラリとセルフホスティング

このフェーズでは、`Core` ライブラリを充実させ、実世界のコードでコンパイラの正当性を検証することに焦点を当てます。

## 6.0 目的と前提
- **目的**: `examples/spec_core` と `examples/practical` を安定して実行できる標準ライブラリとランタイムを整備する。
- **前提**: フェーズ 5 までに BigInt/Unicode/ADT/参照型/効果行/効果ハンドラの最低限が実装済みであること。
- **非スコープ**: パフォーマンス最適化（JIT/高度な最適化）や完全な標準ライブラリ網羅は次フェーズ以降で扱う。

## 6.1 ランタイムライブラリ戦略
- **場所**: `compiler/c/lib/runtime/` (例: `runtime.c`, `runtime.h`)。
- **設計指針**:
  - API は C 側の ABI を安定化し、Reml 側は薄いラッパーに留める。
  - エラーは `Result` 相当の構造体と診断 ID を返し、未捕捉例外を避ける。
  - 文字列は UTF-8 を前提とし、Unicode 正規化は `Core` 側で統一する。
- **内容**:
  - メモリ割り当てラッパー（アリーナ/GC フック、OOM 戦略）。
  - IO プリミティブ（`print`, `read_file`, `write_file`, `stderr`）。
  - システムプリミティブ（`time`, `env`, `args`, `cwd`）。
  - 例外/パニック相当（`panic` のメッセージ整形と終了コード）。
- **バインディング**: C 関数を Reml の `@intrinsic` で公開し、ABI 一覧を `docs/` にまとめる。

### 6.1.1 作業ステップ（詳細）
- [x] `compiler/c/lib/runtime/` を作成し、`runtime.h` / `runtime.c` の最低限スケルトンを配置する。
- [x] 失敗時の共通 ABI として `reml_result`（`ok`/`err`、診断 ID、メッセージ）を定義し、`reml_panic` の終了コード規約を決める。
- [x] 文字列/バイト表現（`reml_string`/`reml_bytes` など）を `ptr + len` で定義し、UTF-8 検証の責務を明確化する。
- [x] メモリ割り当てラッパー（`reml_alloc`/`reml_free`/`reml_arena_*`）と OOM ハンドリング方針を追加する。
- [x] IO プリミティブ（`reml_print`/`reml_eprint`/`reml_read_file`/`reml_write_file`）を追加し、`Result` でエラーを返す。
- [x] システムプリミティブ（`reml_time_now`/`reml_env_get`/`reml_args`/`reml_cwd`）を追加する。
- [x] `compiler/c/lib/runtime/CMakeLists.txt` を追加し、`reml_runtime` ターゲットを `reml_core` からリンク可能にする。
- [x] `@intrinsic` へ公開する ABI 一覧を `docs/plans/c-implementation/runtime-abi.md` に整理し、関数名・引数・戻り値・エラーコードを記述する。

## 6.2 Core ライブラリの移植
- **ソース**: `compiler/c/lib/core/` (Reml ファイル)。
- **対象モジュール（最小セット）**:
  - `Core.Prelude`, `Core.Result`, `Core.Option`, `Core.String`, `Core.Bytes`
  - `Core.Collections`（`List`, `Map`, `Set` は最小 API のみ）
  - `Core.Math`, `Core.Int`, `Core.Float`, `Core.BigInt`
  - `Core.IO`, `Core.Env`, `Core.Time`
- **タスク**:
  - `compiler/rust` から API と仕様を抽出し、互換インタフェースを定義する。
  - `@intrinsic` 対象を一覧化し、C 実装に移譲する範囲を明確化する。
  - Unicode 関連 API（分割・正規化・幅計算）を `utf8proc` / `libgrapheme` に接続する。
  - エラーメッセージと診断 ID を `docs/spec/3-6` 系に合わせる。

### 6.2.1 進捗メモ
- `compiler/c/lib/core/` を作成し、`Core.*` の最小構成ファイルを配置した。
- `compiler/c/lib/core/README.md` に `@intrinsic` 対象と C 実装の対応表、Unicode 連携方針、診断 ID の暫定コードを整理した。

## 6.3 セルフホスティングテスト (Spec Core)
- **目標**: `examples/spec_core/` のテストスイートをパスする。
- **ハーネス**:
  - `spec_core` 内の各 `.reml` ファイルをコンパイルして実行するテストランナーを作成。
  - `Assertion Failed` または成功終了を検証する。
  - 失敗時は診断 JSON を保存し、再現性のある出力を残す。
- **基準**:
  - 期待値との比較（標準出力/終了コード）。
  - 同一入力で実行結果が安定すること（非決定要素は固定）。

## 6.4 実用的な例
- **目標**: `examples/practical/http_server.reml` (または類似) を実行する。
- **タスク**:
  - 必要な `Core.Net` プリミティブ (socket, bind, listen) を C で実装。
  - 非同期 IO の挙動を検証（最初は同期ラッパーでも可、後で差し替え可能にする）。
  - macOS/Linux/Windows の差分は `platform/` 層で吸収する。

## 6.5 ビルド統合と配布形態
- **ビルド**:
  - `runtime` を静的/動的の両方で生成可能にし、`compiler/c` のターゲットとリンク。
  - `Core` の Reml ソースをビルド時に同梱し、バージョンを固定する。
- **配布**:
  - `CPack` 設定（.deb/.msi/.dmg）。
  - ランタイム・標準ライブラリ・ライセンスの同梱方針を明文化。
- **UX**:
  - `reml help` と診断メッセージの最終調整（エラー位置/期待値/提案）。

## チェックリスト
- [ ] ランタイム ABI と `@intrinsic` 一覧が確定している。
- [ ] `Core` ライブラリの最小セットが C で実装された。
- [ ] `stdlib` がコンパイルされリンクされる。
- [ ] `examples/spec_core` のパス率が > 90% になる。
- [ ] `examples/practical` の代表例が動作する。
- [ ] `reml help` と診断 JSON 出力が安定している。
