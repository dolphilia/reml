# 1.5 ランタイム連携計画

## 目的
- Phase 1 マイルストーン M3/M5 で必要となる最小ランタイム API (`mem_alloc`, `panic`, `inc_ref`, `dec_ref`) を整備し、生成した LLVM IR とリンク可能にする。
- 参照カウント (RC) ベースの所有権モデルを `guides/llvm-integration-notes.md` §5 に沿って実装し、リーク/ダングリング検出テストを提供する。

## スコープ
- **含む**: C/LLVM IR で記述された最小ランタイム、メモリアロケータの抽象化（malloc ベース）、参照カウントヘルパ、エラーハンドラ、テスト用検証フック。
- **含まない**: ガベージコレクタ、Fiber/Async ランタイム、Capability Stage の動的切替。これらは Phase 3 以降。
- **前提**: LLVM IR 生成がランタイム関数を呼び出す設計になっていること。x86_64 Linux のツールチェーンが構築済みであること。

## 作業ブレークダウン
1. **ランタイム API 設計**: 必須シンボルと関数シグネチャを決定し、ヘッダ (`runtime/reml_runtime.h` 仮) を用意。
2. **C 実装/ビルド**: 参照カウント付き構造体、panic ハンドラ、メモリアロケータラッパを C で実装し、`cmake` または `make` を用いたビルドスクリプトを準備。
3. **OCaml 連携**: LLVM IR 生成時にランタイムシンボルを宣言し、リンク手順を CLI (`remlc-ocaml`) に組み込む。
4. **検証テスト**: RC の増減を検査する単体テストを追加し、`LD_PRELOAD` を用いたメモリリーク検出（Valgrind 等）のガイドを記載。
5. **CI 統合**: GitHub Actions x86_64 Linux ジョブでランタイムのビルドとリンクテストを実行し、成果物をアーティファクトとして収集。
6. **ドキュメント整備**: ランタイム API 仕様とテスト手順を `guides/llvm-integration-notes.md` に追記し、`0-3-audit-and-metrics.md` へ記録。

## 成果物と検証
- `runtime/` ディレクトリにソースコードとビルド設定が追加され、`make runtime` や `dune build @runtime` が成功。
- RC テストでリークゼロ、ダングリング検出ゼロを確認し、結果を `0-3-audit-and-metrics.md` に記録。
- CLI で `--link-runtime` オプションが利用可能となり、生成バイナリが x86_64 Linux 上で実行できる。

## リスクとフォローアップ
- macOS 等で開発時にクロスビルドが必要になるため、Docker イメージまたは cross toolchain の利用手順を `notes/llvm-spec-status-survey.md` に共有。
- RC のオーバーヘッドが大きい場合に備え、計測値を Phase 3 のメモリ管理戦略検討へフィードバック。
- ランタイム API が今後拡張されることを想定し、ヘッダにバージョンフィールドと互換性ポリシーを記載しておく。

## 参考資料
- [1-0-phase1-bootstrap.md](1-0-phase1-bootstrap.md)
- [guides/llvm-integration-notes.md](../../guides/llvm-integration-notes.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- [notes/llvm-spec-status-survey.md](../../notes/llvm-spec-status-survey.md)

