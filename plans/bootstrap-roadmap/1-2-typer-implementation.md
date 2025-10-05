# 1.2 Typer 実装詳細計画

## 目的
- Hindley–Milner 基盤の型推論を OCaml で実装し、Phase 1 マイルストーン M2 (`Typer MVP`) の達成を保証する。
- `1-2-types-Inference.md` に記載された単相/let 多相のケースを再現し、辞書渡し・効果タグなしの単純化モデルで安定動作させる。
- 解析結果を Core IR へ橋渡しできる `TypedAST` を作り、後続の最適化と LLVM 生成に供給する。

## スコープ
- **含む**: 型推論エンジン、型注釈の取り込み、一般化/インスタンス化、型別名・レコード・列挙の最小サポート、エラー生成。
- **含まない**: 型クラス、効果タグ、サブタイピング、所有権解析。これらは Phase 2 以降の課題とする。
- **前提**: Parser 実装が TypedAST 用の構造を提供していること、`notes/llvm-spec-status-survey.md` の M1 計測対象が把握されていること。

## 作業ブレークダウン
1. **型表現の定義**: `1-2-types-Inference.md` の型記法を OCaml の変数/関数/レコード型へ写像し、`TypeScheme` と `Type` のバリアントを設計。
2. **Unifier 実装**: 合一アルゴリズム（発見的 occurs-check を含む）を作成し、単体テストで `samples/language-impl-comparison/` の型ケースを検証。
3. **一般化処理**: let 多相の一般化/インスタンス化を別モジュール化し、型推論本体から副作用を切り出す。
4. **診断フォーマット**: `2-5-error.md` に従い、型エラー時の期待/検出型・位置情報を同梱した構造体を定義。
5. **TypedAST 生成**: Parser 出力に型情報・型注釈を付与した `TypedExpr` を構築し、Core IR 生成用の境界を明確にする。
6. **性能計測フック**: 推論ステップ数・ユニファイ回数を記録するトレーサを埋め込み、`0-3-audit-and-metrics.md` で測定する指標を定義。

## 成果物と検証
- OCaml モジュール `typer/` 以下でユニットテスト (`dune runtest typer`) を実行できるようにする。
- 型推論結果を JSON でスナップショット保存し、回帰テストを GitHub Actions 上で自動化。
- CLI の `--emit-tast` オプションで TypedAST を確認できるようにする。

## リスクとフォローアップ
- Occurs-check の計算量が入力によっては高くなるため、`0-3-audit-and-metrics.md` にテストケース別のステップ数を記録し、Phase 2 でヒューリスティック最適化の検討を行う。
- Parser の構文拡張が Phase 2 で追加される場合に備え、TypedAST の定義をモジュール化し差分追加しやすい構造体にしておく。
- 将来の型クラス導入を見据え、型変数 ID 生成子を抽象化しておき、辞書生成ステップと競合しないようにする。

## 参考資料
- [1-0-phase1-bootstrap.md](1-0-phase1-bootstrap.md)
- [1-2-types-Inference.md](../../1-2-types-Inference.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- [notes/llvm-spec-status-survey.md](../../notes/llvm-spec-status-survey.md)

