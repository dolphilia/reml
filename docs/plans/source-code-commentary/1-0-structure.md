# Remlソースコード完全解説: 構成案

このドキュメントは、解説書の目次案の概要を示します。

## 第1部: アーキテクチャ概要

**目的**: 道を歩む前に、読者に領域の地図を与えること。

- **第1章: Remlコンパイルパイプライン**
  - ソースコードからバイナリまで：ハイレベルなデータフロー図。
  - フロントエンド → バックエンド → ランタイムの責務分離。
- **第2章: リポジトリ構造**
  - `compiler/frontend`: 言語の頭脳。
  - `compiler/backend`: 機械との対話。
  - `compiler/runtime`: 実行環境と標準ライブラリ。
  - `compiler/adapter`: プラットフォーム差異吸収。
  - `compiler/ffi_bindgen`: FFI バインディング生成。
  - `compiler/xtask`: 開発支援タスク。
- **第3章: 実行の入口**
  - `reml_frontend` と `remlc` の役割。
  - 設定・診断・出力の流れ（JSON/CLI）。

## 第2部: フロントエンド（解析）

**目的**: テキストがどのように意味になるかを説明すること。

- **第4章: 字句解析 (Lexical Analysis)**
  - `frontend/src/lexer` の紹介。
  - `Token` 構造体 (`frontend/src/token.rs`)。
  - Unicodeとソーススパン（`span.rs`, `unicode.rs`）の処理。
- **第5章: 構文解析 (Parsing)**
  - パーサ構成（`frontend/src/parser` の解説）。
  - エラー回復戦略。
  - 具象構文木（CST）と抽象構文木（AST）。
- **第6章: 診断と出力**
  - `frontend/src/diagnostic` と `frontend/src/output`。
  - 診断モデルと出力フォーマットの関係。
- **第7章: 型チェックと型推論**
  - `frontend/src/typeck`: 安全性の中心。
  - 型の表現。
  - 単一化（Unification）と推論アルゴリズム。
  - テレメトリ/デバッグ出力の読み方。

## 第3部: 意味論と中間表現

**目的**: 検証と変換について説明すること。

- **第8章: 意味解析 (Semantic Analysis)**
  - `frontend/src/semantics`。
  - 名前解決とスコープ。
  - 型付きASTの整合性。
- **第9章: 実行パイプライン**
  - `frontend/src/pipeline` と `frontend/src/streaming`。
  - ストリーミング実行の責務と制約。
- **第10章: エフェクトとFFI実行**
  - `frontend/src/effects` と `ffi_executor.rs`。
  - 外部呼び出しの境界と安全性。

## 第4部: バックエンド（合成）

**目的**: 実行可能コードを生成する方法を説明すること。

- **第11章: LLVMへのラウアリング**
  - `backend/llvm`: LLVM IRへの架け橋。
  - `type_mapping.rs` / `codegen.rs` / `ffi_lowering.rs` の責務。
  - Reml型のメモリレイアウトとABI。
- **第12章: ランタイム連携と検証**
  - `runtime_link.rs` と `verify.rs`。
  - `target_machine.rs` / `target_diagnostics.rs` の役割。

## 第5部: ランタイムと標準ライブラリ

**目的**: 実行環境について説明すること。

- **第13章: ランタイムの全体像**
  - `runtime/src/runtime` と `embedding.rs` の役割。
  - 設定・ステージ制御（`run_config.rs`, `stage.rs`）。
  - プラグイン管理（`runtime/src/runtime/plugin*.rs`）。
- **第14章: Capability と監査**
  - `runtime/src/capability` と `runtime/src/audit`。
  - 実行制約とログの設計。
- **第15章: 標準ライブラリのプリミティブ**
  - `runtime/src/collections`, `io`, `text`, `numeric`, `time`, `path`。
  - 典型的なデータ構造と API 群。
- **第16章: 解析・DSL・診断**
  - `runtime/src/parse`, `runtime/src/dsl`, `runtime/src/diagnostics`。
  - ドメイン固有ロジックの整理方法。
- **第17章: FFI とネイティブ連携**
  - `runtime/ffi` と `runtime/native` の境界。
  - ABI とメモリの扱い。
- **第18章: LSP/システム補助**
  - `runtime/src/lsp`, `runtime/src/system`。
  - 開発体験を支える補助モジュール。

## 第6部: ツールと周辺領域

**目的**: 周辺ツールと補助的なコードを整理すること。

- **第19章: Adapter レイヤ**
  - `compiler/adapter` 全体の責務。
  - Env/FS/Network/Time/Random/Process/Target の横断設計。
- **第20章: FFI Bindgen**
  - `compiler/ffi_bindgen` の構成と生成フロー。
- **第21章: 開発支援ツール**
  - `compiler/xtask` と監査ワークフロー。

## 第7部: テストと運用

**目的**: 品質保証と運用手順を整理すること。

- **第22章: テスト戦略**
  - ユニットテスト vs 統合テスト (`tests/`, `examples/`)。
  - 主要テストの読み方と追加指針。
- **第23章: 仕様との同期**
  - `docs/spec` の対応関係と更新手順。

## 付録

- **A: 用語集と索引**
  - モジュール名と機能の対応表。
- **B: エンドツーエンド実行トレース**
  - `reml_frontend` → `backend` → `runtime` の最小経路。
