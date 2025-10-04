# 1.0 Phase 1 — Bootstrap Implementation (OCaml)

Phase 1 は Reml コンパイラの OCaml 実装によって LLVM IR 生成までの最小パイプラインを確立する段階である。`guides/llvm-integration-notes.md` の Bootstrap 設計を実行レベルに具体化し、言語仕様 MVP の検証と計測基盤の整備を同時に進める。

## 1.0.1 目的
- Reml ソースから LLVM IR を生成する OCaml 製コンパイラを構築し、`1-1-syntax.md` で定義された基本構文 (式、関数、`let`, `if`) を網羅的に扱う。
- `2-0-parser-api-overview.md` と `2-1-parser-type.md` の API 契約を OCaml 実装に写像し、Core.Parse の仕様妥当性を先行検証する。
- ランタイム連携については `guides/llvm-integration-notes.md` §5 の所有権モデルと最小ランタイム API を準拠実装する。

## 1.0.2 スコープ境界
- **含む**: 字句解析、構文解析、HM 型推論の最小セット（単相 + 基本多相関数）、Core IR 生成、LLVM IR 出力、最小ランタイムリンク。
- **含まない**: 型クラス、代数的効果、DSL プラグイン、Capability Stage の動的切替、JIT 実行。これらは Phase 2 以降で開発する。
- **前提条件**: LLVM 15 以上のツールチェーン、IR 実行器、RC ベースの最小ランタイム（`inc_ref`/`dec_ref`）。

## 1.0.3 成果物とマイルストーン
| マイルストーン | 内容 | 検証方法 | 期限目安 |
|----------------|------|----------|----------|
| M1: Parser MVP | OCaml 実装で `Parser<T>` 相当の API を再現し、`1-1-syntax.md` の式/宣言テストを通過 | Golden AST テスト（`samples/language-impl-comparison/` をベースに追加） | 開始後 4 週 |
| M2: Typer MVP | HM 型推論（単相 + let 多相）を実装し、`1-2-types-Inference.md` のサンプルケースを再現 | 型推論スナップショットテスト | 開始後 8 週 |
| M3: CodeGen MVP | Core IR → LLVM IR の降格と最小ランタイム連携を実装 | LLVM `opt`/`llc` による IR 妥当性検証、`print_i64` など基本関数の実行テスト | 開始後 12 週 |
| M4: 診断フレーム | `2-5-error.md` の基本診断形式（位置情報、期待値提示）を OCaml 実装で出力 | CLI 比較テスト、テキストスナップショット | 開始後 16 週 |

## 1.0.4 実装タスク
1. **Parser 実装**
   - Menhir もしくは同等の LL/LR パーサジェネレータで構文規則を定義し、ARM64 macOS (Apple Clang + Dune) 環境でビルド手順を確立する。
   - Span 情報を AST ノードに保持し、`2-5-error.md` のエラー範囲モデルに沿う。
   - `precedence` 宣言は Phase 1 では固定テーブルで代替し、Phase 2 で動的拡張へ移行。
2. **Typer 実装**
   - HM 推論の Unifier を OCaml で構築し、`notes/llvm-spec-status-survey.md` で示された成熟度評価に沿ってテスト項目を抽出。
   - 型注釈の反映、ジェネリクスの一般化/インスタンス化を実装し、ARM64 固有の整数幅 (64bit) を前提にした境界ケースを確認。
3. **Core IR と最小最適化**
   - Core IR データ構造を OCaml で定義し、パターンマッチやパイプを糖衣剥がし。
   - 簡易な定数畳み込みと死コード削除を導入し、Phase 2 の最適化へ布石を敷く。
4. **LLVM IR 生成とターゲット設定**
   - `-target arm64-apple-macos12.0` を既定にした LLVM IR を生成し、AAPCS64 + Mach-O の呼出規約に合わせて関数シグネチャを構成する。
   - `llc`/`opt` で Apple Silicon 向けのパイプラインを検証し、Linux/SysV・Windows への展開は Phase 2 以降の TODO として整理する。
5. **ランタイム連携**
   - `mem_alloc`, `panic`, `inc_ref` など最小ランタイム API を C/LLVM IR で提供し、`clang -target arm64-apple-macos12.0` でビルドした静的ライブラリまたは dylib とリンク。
   - 参照カウントの最小検証として、リーク・ダングリング検出テストを準備。
6. **開発者体験**
   - CLI (`remlc-ocaml` 仮称) を整備し、`--emit-ir`・`--emit-ast` など観測用フラグを提供。
   - エラーメッセージは `Result` 形式で整流し、`3-6-core-diagnostics-audit.md` のフィールド名に合わせる。
7. **ARM64 macOS 検証インフラ**
   - GitHub Actions macOS ランナーまたは社内 CI で Apple Silicon テストジョブを構築し、Parser/CodeGen/ランタイムの最小スモークテストを毎ビルドで実行する。
   - Homebrew/LLVM 15 の依存関係バージョンを固定し、手順を `0-3-audit-and-metrics.md` へ記録する。

## 1.0.5 測定と検証
- **性能**: 10MB ソースの解析時間を計測し、O(n) 傾向を確認する。`0-3-audit-and-metrics.md` に結果を記録。
- **メモリ**: Peak メモリを入力サイズ 2× 以下に抑えられるかを RSS 計測で検証。
- **診断品質**: 代表ケースを `samples/` に追加し、エラーメッセージの有効性をレビュアがチェック。
- **IR 妥当性**: `llvm-as`/`llvm-dis` ラウンドトリップ、および `opt -verify` を通過させる。
- **ターゲット妥当性**: `llc -mtriple=arm64-apple-macos12.0` で生成した Mach-O を Apple Silicon 上で実行し、CLI のエンドツーエンドテストを通過させる。

## 1.0.6 リスクとフォローアップ
- **Unifier の複雑度**: Phase 1 で過度な機能を盛り込むとスケジュールが伸びるため、型クラスは Phase 2 に延期し、TODO を `0-4-risk-handling.md` に登録。
- **ランタイム ABI の差異**: ARM64 macOS で優先的に検証するため、Linux/SysV と Windows/MSVC の差分調整は Phase 2 のタスクとして記録。
- **診断整合**: OCaml 実装と将来のセルフホスト実装でフォーマットを揃える必要があるため、出力仕様を `0-3-audit-and-metrics.md` に固定テンプレートとして記載する。

---

Phase 1 の完了によって、Reml仕様の最小機能が実際のコード生成まで確認でき、以降のフェーズで高度な機能（型クラス、Capability Stage、セルフホスト化）を安全に進める土台が整う。
