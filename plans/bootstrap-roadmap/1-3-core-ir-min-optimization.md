# 1.3 Core IR と最小最適化計画

## 目的
- Phase 1 マイルストーン M3 に向けて、Parser/TypeChecker の出力を Core IR へ正規化し、LLVM 生成に渡す手前の最小最適化を整備する。
- `guides/llvm-integration-notes.md` の Core IR 設計方針を OCaml 実装で具現化し、Phase 2 以降の最適化拡張に備える。

## スコープ
- **含む**: Core IR データ構造の定義、構文糖の剥離、ベーシックブロック構成、定数畳み込み、死コード削除 (DCE)、簡易な代入伝播。
- **含まない**: 高度な最適化（ループ最適化、共通部分式除去、インライン展開）。これらは Phase 2 の検討対象。
- **前提**: TypedAST が安定しており、型付き情報を参照しながら IR を生成できること。

## 作業ブレークダウン
1. **Core IR モデル定義**: `guides/llvm-integration-notes.md` §4 を基に、式・コマンド・制御フローを表現する OCaml 型を設計。
2. **糖衣削除パス**: パターンマッチ、パイプ演算子、`let` 再束縛など高階構文を Core IR のプリミティブへ変換。
3. **ベーシックブロック生成**: 制御フロー分岐をブロック単位に分割し、SSA 変換前提の phi 情報を付与。
4. **最小最適化パス**: 定数畳み込みと DCE をパイプラインとして組み込み、適用順序と停止条件を定義。
5. **IR 検査ツール**: Core IR の pretty printer と `--emit-core` フラグを実装し、差分監視できるようにする。
6. **テストケース**: `samples/language-impl-comparison/` のサンプルを Core IR へ変換し、ゴールデンファイルで比較する仕組みを追加。

## 成果物と検証
- OCaml モジュール `core_ir/` を追加し、`dune runtest core_ir` で各種パスの単体テストを実行。
- Core IR の SSA 検証を簡易チェック（未定義変数の検出等）で実施し、CI に組み込む。
- Core IR ダンプと LLVM IR の比較レポートを自動生成し、`0-3-audit-and-metrics.md` にサマリを記録。

## リスクとフォローアップ
- SSA 生成を Phase 1 で扱わない場合でも、phi ノード設計を前提にしておかないと Phase 2 でリファクタが発生する恐れがあるため、構造だけ先行定義する。
- DCE の適用範囲が広すぎると診断用ノードが除去される可能性があるので、`3-6-core-diagnostics-audit.md` に示されたメタデータを保持するタグを IR 上に確保する。
- Core IR から LLVM IR への写像を検証するため、IR 変換時のタグ情報をログに出力し、`0-3-audit-and-metrics.md` で追跡する。

## 参考資料
- [1-0-phase1-bootstrap.md](1-0-phase1-bootstrap.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- [guides/llvm-integration-notes.md](../../guides/llvm-integration-notes.md)
- [3-6-core-diagnostics-audit.md](../../3-6-core-diagnostics-audit.md)

