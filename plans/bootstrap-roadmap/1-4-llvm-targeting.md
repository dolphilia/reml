# 1.4 LLVM IR 生成とターゲット設定計画

## 目的
- Phase 1 マイルストーン M3 において LLVM IR を確実に生成し、x86_64 Linux (System V ABI) を既定ターゲットとしてコンパイルできるようにする。
- `guides/llvm-integration-notes.md` §5 の ABI 要件と `notes/llvm-spec-status-survey.md` のギャップを解消し、後続フェーズでのマルチターゲット化に備える。

## スコープ
- **含む**: LLVM IR ビルダーの実装、データレイアウト (`e-m:e-p:64:64-f64:64:64-v128:128:128-a:0:64`) 設定、関数シグネチャ/ABI マッピング、x86_64 Linux 用ランタイムシンボルとのリンケージ。
- **含まない**: Windows/MSVC や ARM64 向けのターゲットコード生成、JIT 実行、デバッグ情報の高度な最適化。
- **前提**: Core IR が安定しており、型情報が LLVM 型へマッピングできる状態であること。

## 作業ブレークダウン
1. **LLVM ビルダー基盤**: OCaml から LLVM C API を利用するバインディング設定を整備し、`dune` で依存解決をスクリプト化。
2. **型マッピング表**: Primitive 型と複合型のマッピング表を `guides/llvm-integration-notes.md` に基づいて作成し、単体テストを用意。
3. **関数シグネチャ生成**: 呼出規約 (System V) に従って引数配置/戻り値ハンドリングを実装し、`inc_ref` 等ランタイム関数の宣言を組み込む。
4. **DataLayout/TargetMachine 設定**: デフォルトターゲットを `x86_64-unknown-linux-gnu` に固定し、CLI フラグで変更できる余地を残す。
5. **IR 検証**: 生成 IR を `llvm-as` → `opt -verify` → `llc` のパイプラインで検証するスクリプトを CI に追加。
6. **成果物出力**: `--emit-ir` オプションを CLI に実装し、IR ファイルと人間可読ダンプの両方を生成。

## 成果物と検証
- LLVM IR のゴールデンテストを `tests/llvm-ir/` に追加し、代表例（関数呼び出し、条件分岐、ループ）で期待する出力を固定。
- CI で x86_64 Linux 用のクロスコンパイルを実行し、`llc` と `clang` を通して実行可能バイナリを生成。
- DataLayout とターゲットトリプルが `0-3-audit-and-metrics.md` に登録され、逸脱が報告されるようにする。

## リスクとフォローアップ
- LLVM バージョン差異により API が変化する可能性があるため、`0-3-audit-and-metrics.md` にバージョン固定と検証手順を追記。
- クロスコンパイルを macOS 上で実行する場合の依存解決が複雑なため、Docker/Podman のベースイメージ構成を `notes/llvm-spec-status-survey.md` に共有。
- 生成 IR とランタイム ABI の不整合が検出された場合は、`0-4-risk-handling.md` に即時登録し、Phase 2 の Windows 対応タスクで流用できるようにする。

## 参考資料
- [1-0-phase1-bootstrap.md](1-0-phase1-bootstrap.md)
- [guides/llvm-integration-notes.md](../../guides/llvm-integration-notes.md)
- [notes/llvm-spec-status-survey.md](../../notes/llvm-spec-status-survey.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)

