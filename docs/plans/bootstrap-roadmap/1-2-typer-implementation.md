# 1.2 Typer 実装詳細計画

## 目的
- Hindley–Milner 基盤の型推論を OCaml で実装し、Phase 1 マイルストーン M2 (`Typer MVP`) の達成を保証する。
- [1-2-types-Inference.md](../../spec/1-2-types-Inference.md) に記載された単相/let 多相のケースを再現し、辞書渡し・効果タグなしの単純化モデルで安定動作させる。
- 解析結果を Core IR へ橋渡しできる `TypedAST` を作り、後続の最適化と LLVM 生成に供給する。

## スコープ
- **含む**: 型推論エンジン、型注釈の取り込み、一般化/インスタンス化、型別名・レコード・列挙の最小サポート、エラー生成。
- **含まない**: 型クラス、効果タグ、サブタイピング、所有権解析。これらは Phase 2 以降の課題とする。
- **前提**: Parser 実装が TypedAST 用の構造を提供していること、`docs/notes/llvm-spec-status-survey.md` の M1 計測対象が把握されていること。

## 進捗状況（2025-10-06 時点）
- ✅ Week 1: `types.ml`・`type_env.ml`・`constraint.ml` を整備し、型表現・型環境・制約生成/単一化の最小セットを実装済み。
- ✅ Week 2-3: `typed_ast.ml`・`type_inference.ml`・`type_error.ml` を追加し、リテラル/変数/関数適用/ラムダ/if/let を対象とした推論経路と一般化/インスタンス化を実装済み。
- 🚧 Week 4-6: パターンマッチ推論、診断整備、スナップショットテスト、`--emit-tast` CLI オプションの追加を着手予定。
- ⚠️ 既知の制約: `dune test` は `tests/test_parser.exe` の TODO ケース（handler ブロック）が失敗するため、個別テスト実行時は既知不具合として扱うこと（詳細は `compiler/ocaml/docs/technical-debt.md` 参照）。
- 📌 進捗ログ: 最新のタスク状況と実行コマンドは `compiler/ocaml/README.md` に日次で追記する。

## 作業ディレクトリ
- `compiler/ocaml/src/typer`（想定）: 型表現・制約生成・Unifier 実装
- `compiler/ocaml/tests/typer` : 型推論スナップショットと回帰テスト
- `compiler/ocaml/docs` : 型推論設計メモや計測結果の記録
- `docs/notes/core-library-outline.md`, `docs/notes/llvm-spec-status-survey.md` : 仕様差分・性能指標のログ

## 作業ブレークダウン

### 1. 型システム基盤設計（5週目）
**担当領域**: 型表現とデータ構造

1.1. **型表現の定義**
- `Type` バリアント: `TVar | TCon | TApp | TArrow | TTuple | TRecord | TArray | TSlice | TUnit | TNever`（`TNever` は `Core.Prelude` が提供する空型 `Never` の表現）
- `TypeScheme`: `∀α₁...αₙ. τ` の表現（量化変数リスト + 型）
- 型変数ID生成器の実装（単調増加、スレッドセーフ不要）
- 組み込み型の定義: 1-2 §A.1/A.2 に列挙された整数族（`i8`〜`i64`,`u8`〜`u64`,`isize`,`usize`）、浮動小数（`f32`,`f64`）、`Bool`、`Char`、`String`、`()` を基準とし、`Never` は [3-1-core-prelude-iteration.md](../../spec/3-1-core-prelude-iteration.md) の定義に従って `TNever` に写像

1.2. **型環境（Type Environment）設計**
- `Env`: 識別子 → TypeScheme のマップ
- スコープネストの表現（親環境への参照）
- 初期環境: `Core.Prelude`/`Core.Iter` の基本関数、`Option`/`Result` コンストラクタ、演算子トレイト別の既定辞書（Phase 2 との接続）
- `RunConfig` の既定数値/浮動設定との整合（1-2 §C.5）

1.3. **型制約システム**
- `Constraint`: `Unify(τ₁, τ₂)` の表現
- 制約収集と解決の分離設計
- [1-2-types-Inference.md](../../spec/1-2-types-Inference.md) との整合確認

**成果物**: `typer/types.ml`, 型環境モジュール

### 2. 型推論エンジン実装（5-6週目）
**担当領域**: Hindley-Milner 推論

2.1. **制約ベース推論**
- AST走査による制約収集
- 式ごとの型変数割り当て
- 関数適用での引数・返り値制約生成

2.2. **Unification アルゴリズム**
- `unify(subst, τ₁, τ₂) -> Result<Subst, TypeError>`
- Occurs-check の実装（無限型の検出）
- 代入合成 `compose(s₁, s₂)`

2.3. **型代入の適用**
- `apply(subst, τ) -> τ`（型への代入適用）
- `apply_env(subst, env) -> env`（環境への適用）
- 正規化処理（型変数のリネーム）

**成果物**: `typer/unify.ml`, 単体テスト

### 3. Let多相の実装（6週目）
**担当領域**: 一般化とインスタンス化

3.1. **型スキームの一般化**
- `generalize(env, τ) -> TypeScheme`
- 自由型変数の計算 `ftv(τ)`, `ftv(env)`
- let束縛時の量化変数決定
- 値制限（1-2 §C.3）に基づき、副作用や可変参照を含む式は単相に制限

3.2. **型スキームのインスタンス化**
- `instantiate(scheme) -> τ`
- 新鮮な型変数の生成
- 量化変数の置換

3.3. **多相再帰の処理**
- 再帰関数の型推論戦略
- 暫定型の仮定と制約解決
- [1-2-types-Inference.md](../../spec/1-2-types-Inference.md) §3.2 の再帰パターン検証

**成果物**: 一般化/インスタンス化モジュール、再帰テスト

### 4. 型注釈の統合（6-7週目）
**担当領域**: 明示的型情報の処理

4.1. **型注釈のパース結果統合**
- Parser出力の型注釈をType表現へ変換
- ユーザー指定型と推論型の照合
- 型エイリアスの展開

4.2. **型検査の強化**
- 注釈付き関数の型チェック
- パターンマッチでの型refinement
- レコード型のフィールド検査

4.3. **複合型のサポート**
- タプル型の推論と検査
- レコード型（構造的部分型なし）
- 代数的データ型の基本サポート（Phase 2で本格化）

**成果物**: 型注釈統合版Typer、複合型テスト

### 5. TypedAST 生成（7週目）
**担当領域**: 型付きASTの構築

5.1. **TypedAST 構造定義**
- `TypedExpr`: AST + 型情報 + Span
- `TypedDecl`: 宣言 + 型スキーム
- `TypedPattern`: パターン + 束縛変数の型

5.2. **AST → TypedAST 変換**
- 推論結果の型をASTノードに付与
- 暗黙の型変換・強制の明示化
- Core IR生成への境界整理

5.3. **型情報の保存**
- TypedASTのシリアライズ（デバッグ用）
- `--emit-tast` CLI オプション実装
- 型情報の可視化（pretty printer）

**成果物**: `typer/typed_ast.ml`, TypedAST生成パス

### 6. エラー診断の実装（7-8週目）
**担当領域**: 型エラーメッセージ

6.1. **型エラー表現**
- `TypeError` バリアント: `Mismatch | OccursCheck | Unbound | ...`
- Span情報の保持
- エラーコンテキスト（期待型、実際の型）

6.2. **診断メッセージ生成**
- [2-5-error.md](../../spec/2-5-error.md) フォーマットへの準拠
- 型の人間可読表示（型変数の名前付け）
- 複数エラーの収集と報告

6.3. **診断品質の向上**
- 型不一致時の詳細説明
- 修正提案の生成（可能な場合）
- エラー位置の正確な特定

**成果物**: エラー診断システム、診断テスト

### 7. 性能最適化と計測（8週目）
**担当領域**: 性能改善

7.1. **計測フック実装**
- 推論ステップ数カウンタ
- Unify呼び出し回数の追跡
- メモリ使用量プロファイリング

7.2. **最適化実装**
- 型変数の共有（メモリ削減）
- 代入合成の遅延評価
- Occurs-check の高速化（パスコンプレッション）

7.3. **性能測定**
- `examples/language-impl-comparison/` でのベンチマーク
- 10MB相当のコードでの推論時間測定
- `0-3-audit-and-metrics.md` への記録

**成果物**: 最適化版Typer、性能レポート

### 8. テストとドキュメント（8週目）
**担当領域**: 品質保証と文書化

8.1. **包括的テストスイート**
- 単相型推論テスト
- Let多相テスト（ネスト、再帰含む）
- エラーケーステスト（全TypeErrorバリアント）
- 性能回帰テスト

8.2. **ゴールデンテスト**
- 型推論結果のスナップショット
- 診断メッセージのスナップショット
- CI での自動検証

8.3. **ドキュメント整備**
- 型システム実装の詳細説明
- アルゴリズム選択の根拠
- M2マイルストーン達成報告
- Phase 2への引き継ぎ事項（型クラス準備）

**成果物**: 完全なテストスイート、技術文書

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
- [1-2-types-Inference.md](../../spec/1-2-types-Inference.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- [notes/llvm-spec-status-survey.md](../../notes/llvm-spec-status-survey.md)
