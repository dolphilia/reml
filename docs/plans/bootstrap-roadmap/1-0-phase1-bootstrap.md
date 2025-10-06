# 1.0 Phase 1 — Bootstrap Implementation (OCaml)

Phase 1 は Reml コンパイラの OCaml 実装によって LLVM IR 生成までの最小パイプラインを確立する段階である。`docs/guides/llvm-integration-notes.md` の Bootstrap 設計を実行レベルに具体化し、言語仕様 MVP の検証と計測基盤の整備を同時に進める。

## 1.0.1 目的
- Reml ソースから LLVM IR を生成する OCaml 製コンパイラを構築し、[1-1-syntax.md](../../spec/1-1-syntax.md) で定義された基本構文 (式、関数、`let`, `if`) を網羅的に扱う。
- [2-0-parser-api-overview.md](../../spec/2-0-parser-api-overview.md) と [2-1-parser-type.md](../../spec/2-1-parser-type.md) の API 契約を OCaml 実装に写像し、Core.Parse の仕様妥当性を先行検証する。
- ランタイム連携については `docs/guides/llvm-integration-notes.md` §5 の所有権モデルと最小ランタイム API を準拠実装する。

## 1.0.2 スコープ境界
- **含む**: 字句解析、構文解析、HM 型推論の最小セット（単相 + 基本多相関数）、Core IR 生成、LLVM IR 出力、最小ランタイムリンク。
- **含まない**: 型クラス、代数的効果、DSL プラグイン、Capability Stage の動的切替、JIT 実行。これらは Phase 2 以降で開発する。
- **前提条件**: LLVM 15 以上のツールチェーン、IR 実行器、RC ベースの最小ランタイム（`inc_ref`/`dec_ref`）。

### 1.0.2a 作業ディレクトリと成果物配置

- `compiler/ocaml/src` — パーサー・Typer・Core IR・LLVM 生成コードを配置
- `compiler/ocaml/tests` — Golden AST／型推論スナップショット／IR 検証テストを配置
- `compiler/ocaml/docs` — 実装ノートや設計メモ、検証結果の記録
- `runtime/native` — 最小ランタイム（Phase 1 で使用する C/LLVM 実装）
- `tooling/cli` / `tooling/ci` — CLI 観測機能と CI スクリプト。GitHub Actions は `.github/workflows/` からこれらのスクリプトを参照
- `docs/notes/llvm-spec-status-survey.md` ほか関連ノートへ測定値・リスクを記録

## 1.0.3 成果物とマイルストーン
| マイルストーン | 内容 | 検証方法 | 期限目安 |
|----------------|------|----------|----------|
| M1: Parser MVP | OCaml 実装で `Parser<T>` 相当の API を再現し、[1-1-syntax.md](../../spec/1-1-syntax.md) の式/宣言テストを通過 | Golden AST テスト（`examples/language-impl-comparison/` をベースに追加） | 開始後 4 週 |
| M2: Typer MVP | HM 型推論（単相 + let 多相）を実装し、[1-2-types-Inference.md](../../spec/1-2-types-Inference.md) のサンプルケースを再現 | 型推論スナップショットテスト | 開始後 8 週 |
| M3: CodeGen MVP | Core IR → LLVM IR の降格と最小ランタイム連携を実装 | LLVM `opt`/`llc` による IR 妥当性検証、`print_i64` など基本関数の実行テスト | 開始後 12 週 |
| M4: 診断フレーム | [2-5-error.md](../../spec/2-5-error.md) の基本診断形式（位置情報、期待値提示）を OCaml 実装で出力 | CLI 比較テスト、テキストスナップショット | 開始後 16 週 |

## 1.0.4 実装タスク

> **ターゲット方針**: Phase 1 の配布ターゲットは **x86_64 Linux (System V ABI)** を優先する。開発環境として macOS (ARM64含む) を使用してもよいが、生成される LLVM IR および成果物は x86_64 Linux 向けとする。これは `docs/guides/llvm-integration-notes.md` および `docs/notes/llvm-spec-status-survey.md` で x86_64 が主要ターゲットと定義されているため。ARM64 macOS は Phase 3 以降のクロスコンパイル対応で追加予定。

1. **Parser 実装**
   - Menhir もしくは同等の LL/LR パーサジェネレータで構文規則を定義し、OCaml + Dune 環境でビルド手順を確立する（開発環境は macOS/Linux いずれも可）。
   - Span 情報を AST ノードに保持し、[2-5-error.md](../../spec/2-5-error.md) のエラー範囲モデルに沿う。
   - `precedence` 宣言は Phase 1 では標準演算子のみをサポートし（固定テーブル）、ユーザー定義演算子の動的登録は Phase 2 に延期。
2. **Typer 実装**
   - HM 推論の Unifier を OCaml で構築し、`docs/notes/llvm-spec-status-survey.md` で示された成熟度評価に沿ってテスト項目を抽出。
   - 型注釈の反映、ジェネリクスの一般化/インスタンス化を実装し、64bit 整数幅を前提にした境界ケースを確認。
3. **Core IR と最小最適化**
   - Core IR データ構造を OCaml で定義し、パターンマッチやパイプを糖衣剥がし。
   - 簡易な定数畳み込みと死コード削除を導入し、Phase 2 の最適化へ布石を敷く。
4. **LLVM IR 生成とターゲット設定**
   - **`-target x86_64-unknown-linux-gnu`** を既定にした LLVM IR を生成し、System V ABI の呼出規約に合わせて関数シグネチャを構成する（`docs/guides/llvm-integration-notes.md` §5.0 準拠）。
   - `llc`/`opt` で x86_64 Linux 向けのパイプラインを検証し、Windows/MSVC への対応は Phase 2 のタスクとして整理する。
   - DataLayout 文字列は `e-m:e-p:64:64-f64:64:64-v128:128:128-a:0:64` を使用。
5. **ランタイム連携**
   - `mem_alloc`, `panic`, `inc_ref` など最小ランタイム API を C/LLVM IR で提供し、**x86_64 Linux 向け**にビルドした静的ライブラリとリンク（開発環境でクロスコンパイル可）。
   - 参照カウントの最小検証として、リーク・ダングリング検出テストを準備。
6. **開発者体験**
   - CLI (`remlc-ocaml` 仮称) を整備し、`--emit-ir`・`--emit-ast` など観測用フラグを提供。
   - エラーメッセージは `Result` 形式で整流し、[3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) のフィールド名に合わせる。
7. **x86_64 Linux 検証インフラ**
   - GitHub Actions Linux ランナー (ubuntu-latest) で x86_64 テストジョブを構築し、Parser/CodeGen/ランタイムの最小スモークテストを毎ビルドで実行する。
   - LLVM 15 以上の依存関係バージョンを固定し、手順を `0-3-audit-and-metrics.md` へ記録する。
   - 開発者が macOS 環境を使用する場合は、クロスコンパイルまたは Docker/VM での x86_64 Linux 検証を推奨。

## 1.0.5 測定と検証
- **性能**: 10MB ソースの解析時間を計測し、O(n) 傾向を確認する。`0-3-audit-and-metrics.md` に結果を記録。
- **メモリ**: Peak メモリを入力サイズ 2× 以下に抑えられるかを RSS 計測で検証。
- **診断品質**: 代表ケースを `examples/` に追加し、エラーメッセージの有効性をレビュアがチェック。
- **IR 妥当性**: `llvm-as`/`llvm-dis` ラウンドトリップ、および `opt -verify` を通過させる。
- **ターゲット妥当性**: `llc -mtriple=x86_64-unknown-linux-gnu` で生成した ELF バイナリを x86_64 Linux 環境で実行し、CLI のエンドツーエンドテストを通過させる（Docker/VM/CI ランナー利用可）。

## 1.0.6 リスクとフォローアップ
- **Unifier の複雑度**: Phase 1 で過度な機能を盛り込むとスケジュールが伸びるため、型クラスは Phase 2 に延期し、TODO を `0-4-risk-handling.md` に登録。
- **ランタイム ABI の差異**: x86_64 Linux (System V ABI) を優先検証し、Windows/MSVC の差分調整は Phase 2 のタスクとして記録。開発環境として macOS を使用する場合は、クロスコンパイルによる検証を徹底する。
- **診断整合**: OCaml 実装と将来のセルフホスト実装でフォーマットを揃える必要があるため、出力仕様を `0-3-audit-and-metrics.md` に固定テンプレートとして記載する。
- **MVP 範囲の明確化**: Phase 1 は `docs/guides/llvm-integration-notes.md` で定義された「MVP（最小実装）」範囲に対応し、期間は約 16 週（2-3ヶ月相当）。「本格実装」は Phase 2 の範囲として扱う。

---

Phase 1 の完了によって、Reml仕様の最小機能が実際のコード生成まで確認でき、以降のフェーズで高度な機能（型クラス、Capability Stage、セルフホスト化）を安全に進める土台が整う。
