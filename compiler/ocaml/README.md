# compiler/ocaml ワークスペース（Phase 1）

Phase 1 ブートストラップ計画に基づき、OCaml 製 Reml コンパイラを構築するための作業領域です。対応するタスクは主に [`docs/plans/bootstrap-roadmap/1-x`](../../docs/plans/bootstrap-roadmap/) に定義されています。

## ディレクトリ
- `src/`: コンパイラ本体（パーサー、型推論、Core IR、LLVM 出力など）
- `tests/`: ゴールデン AST・型推論スナップショット・IR 検証などのテストコード
- `docs/`: 実装メモ、設計ノート、調査結果

各ディレクトリには暫定的な `.gitkeep` のみ配置しています。実作業を開始する際に適宜削除してください。

## TODO
- [ ] `1-1-parser-implementation.md` に沿った Parser 実装スケルトンの配置
- [ ] `1-2-typer-implementation.md` で求められる Typed AST/型推論テストのひな型作成
- [ ] `1-3`〜`1-5` のタスクに合わせた Core IR/LLVM/ランタイム連携の stub 追加
- [ ] CLI エントリポイントと計測フック（`1-6`, `1-7`）の連携手順を記録
