# compiler/ocaml ワークスペース

**現在のフェーズ**: Phase 3 進行中（Core IR & LLVM 生成 - Week 9/16）

Phase 1 ブートストラップ計画に基づき、OCaml 製 Reml コンパイラを構築するための作業領域です。対応するタスクは主に [`docs/plans/bootstrap-roadmap/1-x`](../../docs/plans/bootstrap-roadmap/) に定義されています。

## 📊 進捗状況

### ✅ Phase 1 完了（2025-10-06）
- **M1: Parser MVP** - 完全実装
- **パターンマッチ検証** - 35+ テストケース全て成功
- **テストインフラ** - 165+ テストケース

**詳細**: [Phase 1 完了報告書](docs/phase1-completion-report.md)

### ✅ Phase 2 完了（2025-10-07）
- **M2: Typer MVP** - 完全実装
- **型推論エンジン** - Hindley-Milner 型システム（単相・let多相）
- **テストインフラ** - 103+ テストケース（全成功）
- **診断システム** - 15種類の型エラー（E7001-E7015）

**詳細**: [Phase 2 完了報告書](docs/phase2-completion-report.md)

### 🚀 Phase 3 進行中（2025-10-07 開始）
- **M3: CodeGen MVP** - Core IR → LLVM IR 生成（Week 9-12）
- **進捗**: Week 10 完了 - 糖衣削除パス拡張実装 ✅
  - Core IR データ構造設計完了（Week 9）
  - 糖衣削除パス完全実装完了（Week 9-10）
    - リテラル・変数・関数適用の変換実装完了
    - パイプ演算子展開実装完了
    - if式・ブロック式変換実装完了
    - **タプルパターン変換実装完了** ← NEW
    - **レコードパターン変換実装完了** ← NEW
    - **コンストラクタパターン変換実装完了（ADT表現）** ← NEW
    - **ネストパターン完全展開実装完了** ← NEW
    - **ガード条件処理実装完了** ← NEW
    - **let再束縛正規化完了** ← NEW
  - パターンマッチ決定木の完全実装完了
  - 17件のテストケース全成功（Week 9: 8件 → Week 10: 17件）
- **次**: Week 10-11 - ベーシックブロック生成とCFG構築
- **引き継ぎ**: [Phase 3 ハンドオーバー](docs/phase3-handover.md)
- **計画書**: [1-3-core-ir-min-optimization.md](../../docs/plans/bootstrap-roadmap/1-3-core-ir-min-optimization.md)

## ディレクトリ
- `src/`: コンパイラ本体（パーサー、型推論、Core IR、LLVM 出力など）
- `tests/`: ゴールデン AST・型推論スナップショット・IR 検証などのテストコード
- `docs/`: 実装メモ、設計ノート、Phase 移行ドキュメント

## セットアップ

### 前提条件
- OCaml >= 4.14 (推奨: 5.2.1)
- Dune >= 3.0
- Menhir >= 20201216

### 詳細なセットアップ手順

**📖 [環境セットアップガイド](docs/environment-setup.md)** を参照してください。

macOS、Linux、Windows (WSL) での詳細な手順を提供しています。

### クイックスタート（macOS）

```bash
# opamのインストール
brew install opam
opam init --auto-setup --yes
eval $(opam env)

# OCaml 5.2.1のインストール
opam switch create 5.2.1
eval $(opam env --switch=5.2.1)

# 必要なパッケージをインストール
opam install dune menhir --yes
```

## ビルド方法

```bash
# プロジェクトルート (compiler/ocaml) で実行
dune build

# opam のスイッチを明示したい場合
opam exec -- dune build
```

- Dune が生成する実行バイナリのファイル名は `remlc.exe` ではなく `main.exe` になる（`src/dune` で `name main` を指定しているため）。
- 直接実行する場合は `./_build/default/src/main.exe` を利用する。
- 最も確実なのは `dune exec -- remlc …` を使う方法で、クロスプラットフォームに同じ引数で動作する。

## 使用方法

```bash
# AST を出力
dune exec -- remlc --emit-ast <input.reml>

# Typed AST を出力（型推論結果）
dune exec -- remlc --emit-tast <input.reml>

# ビルド成果物を直接呼び出したい場合（macOS）
./_build/default/src/main.exe --emit-ast <input.reml>
./_build/default/src/main.exe --emit-tast <input.reml>

# 例
dune exec -- remlc --emit-ast ../../examples/language-impl-comparison/reml/pl0_combinator.reml
dune exec -- remlc --emit-tast tests/simple.reml
```

## テスト実行

### すべてのテストを実行

```bash
dune test
```

### 個別のテストを実行

```bash
# Lexer ユニットテスト
dune exec -- ./tests/test_lexer.exe

# Parser ユニットテスト
dune exec -- ./tests/test_parser.exe

# Golden テスト
dune exec -- ./tests/test_golden.exe

# パターンマッチ専用テスト
dune exec -- ./tests/test_pattern_matching.exe

# 型システムユニットテスト
dune exec -- ./tests/test_types.exe
```

### テストの説明

- **test_lexer**: 字句解析の境界ケースと基本機能を検証
  - キーワード、識別子、リテラル（整数、浮動小数、文字、文字列）
  - 演算子、コメント（行コメント、入れ子ブロックコメント）
  - 複合トークン列

- **test_parser**: 構文解析の成功ケースを検証
  - モジュールヘッダ、use宣言
  - let/var/fn/type/extern/handler 宣言（trait/impl は TODO として失敗を期待）
  - 式（リテラル、二項演算、パイプ、関数呼び出し、if/match/while/for、unsafe など）
  - 未実装の構文（フィールドアクセス、`loop` など）は `todo` テストで明示
  - パターンマッチ、属性、基本的な効果宣言
  - エラーケース（構文エラーの検出）

- **test_golden**: サンプルファイルのAST出力をスナップショットと比較
  - `tests/simple.reml`: 基本的な宣言と式のゴールデンテスト
  - `tests/qualified_patterns.reml`: モジュール修飾列挙子・レコードパターンを含む検証
  - ゴールデンファイル (`tests/golden/*.golden`) が存在しない場合は失敗し、`tests/golden/_actual/` に最新出力を保存
  - 差分が出た場合も `_actual` ディレクトリへ出力するので、意図した変更ならゴールデンを更新する

### テスト対象ファイル

- `tests/simple.reml`: Phase 1 の基本機能テスト用サンプル

## 現在の実装状況 (M1 マイルストーン)

### ✅ 完了
- [x] AST 定義 (`src/ast.ml`)
- [x] トークン定義 (`src/token.ml`)
- [x] Lexer 実装 (`src/lexer.mll`)
  - Unicode XID 準拠識別子 (Phase 1: ASCII のみ)
  - 整数・浮動小数・文字・文字列リテラル
  - コメント処理 (行コメント、入れ子ブロックコメント)
  - エスケープシーケンス
- [x] Parser 実装 (`src/parser.mly`)
  - 基本的な式・宣言の構文解析
  - 演算子優先順位 (Menhir %left/%right)
  - Span 情報付与
- [x] Dune ビルドシステム
- [x] CLI エントリポイント (`src/main.ml`)
- [x] テストインフラ整備
  - Lexer ユニットテスト (`tests/test_lexer.ml`)
  - Parser ユニットテスト (`tests/test_parser.ml`)
  - ゴールデンテスト (`tests/test_golden.ml`)
  - Dune テストルール (`tests/dune`)
- [x] `Parser_driver` による Result ベースの診断出力と CLI 連携

### ✅ 完了（2025-10-06 更新）
- [x] **後置演算子の実装**
  - フィールドアクセス (`expr.field`)
  - タプルアクセス (`expr.0`)
  - インデックスアクセス (`expr[i]`)
  - 伝播演算子 (`expr?`)
- [x] **制御フロー構文の拡張**
  - `match` 式（複数アーム、ネストパターン、ガード条件対応）
  - `while` 式（基本ケース）
  - `for` 式（パターン分解対応）
  - `loop` 式
  - ブロック式 `{ ... }` の関数本体対応
- [x] **複雑ケーステスト追加**
  - ネストしたループ構文
  - パターン分解を伴う `for` ループ
  - 制御フロー専用テストセクション追加
- [x] **リテラルパターンの実装**
  - 整数、浮動小数、文字列、文字、真偽値のパターンマッチ対応
- [x] **match 式の複数アーム対応**
  - 複数アームの正しいパース
  - ネストした match 式
  - ガード条件 (`if`) 付きパターン

### ✅ 完了（2025-10-06 更新 - 代入文対応）
- [x] **代入文の左辺値拡張**
  - `LValue := Expr` の `LValue` を `ident` から `postfix_expr` に拡張
  - フィールドアクセス (`obj.field := value`)、インデックスアクセス (`arr[i] := value`)、タプルアクセス (`tuple.0 := value`) に対応
  - AST定義、パーサルール、AST Printerを更新し、仕様書 §D.2 `AssignStmt ::= LValue ":=" Expr` に準拠

### ✅ 完了（2025-10-06 更新 - パターンマッチの完全検証）
- [x] **パターンマッチの網羅的テスト実装**
  - ネストパターン（2層・3層）の完全検証: `Some(Some(x))`, `Ok(Some(value))`, `((a, b), (c, d))`
  - ガード条件の複雑ケース: 複数変数参照、ネストパターン+ガード
  - リテラルパターン（整数・文字列・文字・真偽値）の網羅的テスト
  - レコードパターン+コンストラクタ+rest の組み合わせ
  - 専用テストスイート `tests/test_pattern_matching.ml` (35+ テストケース) を追加
  - 実用例を含むサンプルファイル `tests/pattern_examples.reml` を追加
  - **Phase 1 で要求される全パターンマッチ機能の動作を確認済み**

### ✅ ガード付きレコードパターン（2025-10-06 更新）
- Lexer を `IDENT` / `UPPER_IDENT` に分割し、モジュール修飾付き列挙子（例: `Option.None`）をゼロ/多引数コンストラクタとして扱えるよう更新済み。
- `tests/qualified_patterns.reml` / `qualified_patterns.golden` により、`{ status: Option.None, .. }` を含むレコードパターンがパースできることを継続的に検証。
- 詳細は [技術的負債メモ](compiler/ocaml/docs/technical-debt.md) の「レコードパターンの複数アーム制限」節を参照。

### 📝 Phase 3 への移行

**Phase 2 は完了しました。Phase 3（Core IR & LLVM 生成）の準備が整いました。**

#### Phase 3 開始前に確認すべきドキュメント

1. **[Phase 2 完了報告書](docs/phase2-completion-report.md)**
   - Phase 2 の成果物と統計情報
   - 型推論エンジンの実装詳細
   - Phase 3 への引き継ぎ事項

2. **[Phase 3 ハンドオーバー](docs/phase3-handover.md)**
   - Phase 3 の目標とタスク（M3: CodeGen MVP）
   - 既存コードベースの構造
   - 実装する主要コンポーネント

3. **[技術的負債リスト](docs/technical-debt.md)**
   - Phase 2 で解消された問題
   - Phase 3 に持ち越す項目

#### Phase 2 実装完了サマリー（2025-10-07）

**✅ 全タスク完了 (Week 1-11)**

Phase 2 の全実装が完了しました。詳細は [Phase 2 完了報告書](docs/phase2-completion-report.md) を参照してください。

以下は主要な実装項目の概要です：

**✅ Week 1: 型システム基盤**

- ✅ **型システム基盤** (`src/types.ml`, `src/type_env.ml`, `src/constraint.ml`)
  - 型表現とスキームの定義
  - 型環境とスコープ管理
  - 型制約システムと単一化アルゴリズム
  - 165+ ユニットテスト全て成功

- ✅ **Typed AST 定義** (`src/typed_ast.ml`)
  - 型付き式ノード (`typed_expr`): 推論された型情報を保持
  - 型付き宣言ノード (`typed_decl`): 型スキームを含む
  - 型付きパターンノード (`typed_pattern`): 束縛変数と型のマッピング
  - デバッグ用の文字列表現関数
  - ビルド成功、警告ゼロ

- ✅ **型推論エンジン基礎** (`src/type_inference.ml`)
  - 型注釈の変換 (AST型注釈 → Types.ty)
  - 一般化 (`generalize`): let束縛で自由型変数を量化
  - インスタンス化 (`instantiate`): 型スキームを具体化
  - リテラルの型推論 (i64, f64, Bool, Char, String)
  - 変数参照の型推論 (型環境から検索してインスタンス化)

**✅ 完了 (Week 2-3: 2025-10-06)**

- ✅ **型エラーの定義** (`src/type_error.ml`)
  - 型不一致、無限型検出、未定義変数など
  - 人間可読なエラーメッセージ生成

- ✅ **関数適用の型推論**
  - 制約収集と単一化
  - 引数と返り値の型チェック
  - 位置引数・名前付き引数のサポート

- ✅ **ラムダ式の型推論**
  - パラメータの型推論（型注釈あり/なし）
  - 関数型の構築
  - 返り値型注釈の処理

- ✅ **if式の型推論**
  - 条件式のBool型チェック
  - then/else分岐の型統一
  - else無しifのUnit型チェック

- ✅ **let束縛の型推論**
  - 式の推論と一般化
  - 型環境への追加
  - 型注釈のサポート

**✅ 完了 (Week 4: 2025-10-06)**

- ✅ **パターンマッチの型推論**
  - 全パターン種別の実装完了（PatVar, PatWildcard, PatLiteral, PatTuple, PatConstructor, PatRecord, PatGuard）
  - ネストパターンのサポート（2層、3層以上）
  - ガード条件の型推論（Bool型）
  - match式の型推論（複数アーム、型統一）
  - コンストラクタ型環境の整備（Option<T>, Result<T,E>）

- ✅ **型推論テストスイート** (`tests/test_type_inference.ml`)
  - 基本パターン: 3テストケース
  - タプルパターン: 1テストケース
  - コンストラクタパターン: 2テストケース
  - ネストパターン: 1テストケース
  - match式: 2テストケース
  - **全テスト成功（9/9）**

- ✅ **パターンマッチ固有のエラーメッセージ**
  - ConstructorArityMismatch（コンストラクタ引数数不一致）
  - TupleArityMismatch（タプル要素数不一致）
  - RecordFieldMissing（レコードフィールド不足）
  - RecordFieldUnknown（レコードフィールド不明）
  - NotARecord, NotATuple, EmptyMatch

**✅ 完了 (Week 5: 2025-10-06)**

- ✅ **ブロック式の型推論**
  - 空のブロック: `{} : ()`
  - 式のみのブロック: `{ 42 } : i64`
  - let束縛を含むブロック: `{ let x = 1; x } : i64`
  - 複数の文を含むブロック
  - ブロックの最後が宣言文 → Unit型
  - ネストしたブロック
  - 代入文 (`:=`) のサポート
  - defer文のサポート
  - 6個の新規テストケース追加（全成功: 15/15）

**✅ 完了 (Week 5: 2025-10-06 更新)**

- ✅ **関数宣言の型推論**
  - パラメータの型推論（型注釈あり/なし）
  - 関数本体の型推論（式/ブロック）
  - 返り値型の検証
  - 再帰関数のサポート（暫定型による自己参照）
  - ジェネリック型パラメータの基本対応
  - 関数型の一般化（let多相）
  - 5個の新規テストケース追加（全成功: 20/20）

**✅ 完了 (Week 5: 2025-10-06 更新)**

- ✅ **二項演算の型推論**
  - 算術演算子（+, -, *, /, %, ^）のサポート
  - 比較演算子（==, !=, <, <=, >, >=）のサポート
  - 論理演算子（&&, ||）のサポート
  - パイプ演算子（|>）のサポート
  - 6個の新規テストケース追加（全成功: 26/26）

**✅ 完了 (Week 6: 2025-10-07)**

- ✅ **CLI統合**: `--emit-tast` オプション
  - `infer_compilation_unit` 関数の実装
  - `string_of_typed_compilation_unit` の追加
  - `main.ml` への `--emit-tast` フラグ追加
  - 型エラーメッセージの統合
  - 動作確認完了（`tests/simple.reml`, `tests/test_tast.reml`）

**✅ 完了 (Week 6: 2025-10-07)**

- ✅ **エラー診断品質の向上**
  - 仕様書 2-5 準拠の Diagnostic モジュール拡張
  - 全15種類の型エラーに対する診断変換（E7001-E7015）
  - 日本語エラーメッセージの実装
  - FixIt（修正提案）の自動生成
    - 型不一致時の型注釈追加提案
    - レコードフィールド不足時の補完提案
  - 類似変数名の提案（Levenshtein距離ベース）
  - 型差分の構造的な説明（タプル・関数型）

**✅ 完了 (Week 7: 2025-10-07)**

- ✅ **エラーケーステストスイートの作成**
  - 全15種類の型エラー（E7001-E7015）に対するテストを実装
  - 30個のテストケース（すべて成功）
  - 診断メッセージの品質検証機能を追加
  - 未実装機能：タプルリテラル推論、一部のパターンマッチエラー

**✅ 完了 (Week 8: 2025-10-07)**

- ✅ **複合リテラル（Tuple/Record）の型推論実装**
  - タプルリテラル `(1, "hello", true)` の型推論
  - レコードリテラル `{ x: 42, y: "test" }` の型推論
  - ネストしたタプル・レコードのサポート
  - 空タプル `()` の Unit 型推論
  - 7個の新規テストケース追加（全成功: 33/33）
  - 配列リテラルは Phase 2 後半に延期（エラーケーステスト追加済み）

**✅ 完了 (Week 9: 2025-10-07)**

- ✅ **パターンマッチエラーの改善**
  - 専用エラー型の完全実装（TupleArityMismatch, ConstructorArityMismatch, RecordFieldMissing, RecordFieldUnknown, NotARecord, EmptyMatch）
  - `type_error_with_message` の汎用エラーを専用型に置き換え（6箇所）
  - 診断メッセージの品質向上
    - TupleArityMismatch: ワイルドカード補完の FixIt 追加
    - ConstructorArityMismatch: 正しい使用例を Notes に追加
  - パターンマッチエラーテスト6件追加（全成功: 39/39）
    - E7009: ConstructorArityMismatch（Some(), None(x)）
    - E7010: TupleArityMismatch
    - E7013: NotARecord
    - E7015: EmptyMatch
    - ネストパターンエラー: Some(None(x))
  - **成果補足** (Week 10: 2025-10-07): 文脈依存ヘルパー導入と追加サンプルにより
    `test_type_errors` 30件がすべて成功。詳細は
    [technical-debt.md §7](docs/technical-debt.md#7-型エラー生成順序の問題) を参照。
    - 追加カバレッジ: `match` アーム型不一致、論理演算子のオペランド、
      `match` ガード、パターンガード、`|>` の適用先

**✅ 完了 (Week 9-10: 2025-10-07)**

- ✅ **CLI統合と診断出力の改善**
  - 型エラー → 診断変換関数の実装（`Type_error.to_diagnostic`）
  - 日本語エラーメッセージの実装（全15種類のエラーコード: E7001-E7015）
  - FixIt（修正提案）の自動生成（型注釈追加、フィールド補完、タプルワイルドカード）
  - Span変換ヘルパーの改善（バイトオフセット → 行列番号計算）
  - `main.ml` の診断統合（`to_diagnostic_with_source` の使用）
  - **実装内容**:
    - `Type_error.compute_line_column`: ソース文字列から行列番号を計算
    - `Type_error.span_to_diagnostic_span_with_source`: 正確な位置情報を含む診断Spanを生成
    - `Type_error.to_diagnostic_with_source`: ソース情報を使った診断生成（全15種類対応）
    - 型差分の構造的説明（`explain_type_mismatch`）
    - 類似変数名の提案（Levenshtein距離ベース）

**✅ 完了 (Week 10-11: 2025-10-07)**

- ✅ **let多相の網羅的テスト**
  - カテゴリA: 基本的なlet多相（7件成功、1件スキップ）
  - カテゴリB: 再帰関数の多相（2件成功）
  - カテゴリC: 値制限（3件成功）
  - カテゴリD: 演算子と制約（3件成功）
  - カテゴリE: 高階関数（2件成功）
  - カテゴリF: エラーケース（実装詳細によりスキップ）
  - 専用テストスイート `tests/test_let_polymorphism.ml` を追加（17件成功）
  - 仕様書 1-2 §H の推論例を網羅的に検証
  - identity関数の一般化、複数型でのインスタンス化、ネストlet束縛を確認
  - 高階関数（apply, compose）の多相型推論を検証
  - 数値リテラルのデフォルト型決定を確認

**📋 後続タスク (Week 11-12)**

- 🔜 **Phase 2 完了報告書の作成**

### 🚀 Phase 3 開始（2025-10-07）

**Phase 3 では Core IR & LLVM 生成を実装します。**

計画書: [1-3-core-ir-min-optimization.md](../../docs/plans/bootstrap-roadmap/1-3-core-ir-min-optimization.md)

**✅ 完了 (Week 9: 2025-10-07)**

- ✅ **Core IR データ構造設計** (`src/core_ir/ir.ml` - 383行)
  - 変数ID生成器（SSA形式準備）: 一意IDによる変数識別
  - ラベル生成器: 基本ブロックラベルの自動生成
  - プリミティブ演算: 20種類の演算子（算術、比較、論理、ビット演算）
  - 効果とCapabilityメタデータ: 診断・監査用の効果集合、Capability要件
  - クロージャ情報: 環境キャプチャと関数ポインタ
  - 辞書参照: 型クラス/トレイトの辞書パッシング準備
  - Core IR式: 14種類の式コンストラクタ（Literal, Var, App, Let, If, Match, Primitive, Closure, DictLookup, CapabilityCheck, TupleAccess, RecordAccess, ArrayAccess, ADTConstruct/Project）
  - Core IR文: 6種類（Assign, Return, Jump, Branch, Phi, EffectMarker, ExprStmt）
  - 終端命令: 5種類（TermReturn, TermJump, TermBranch, TermSwitch, TermUnreachable）
  - 基本ブロック: ラベル、パラメータ、命令列、終端命令を含む構造
  - 関数定義: パラメータ、返り値型、ブロックリスト、メタデータ
  - モジュール定義: 型定義、グローバル変数、関数定義

- ✅ **IR Pretty Printer** (`src/core_ir/ir_printer.ml` - 312行)
  - 人間可読な階層表示（インデント付き）
  - 型情報の明示: すべての式に型注釈を表示
  - すべてのIR構造に対応: 式、文、ブロック、関数、モジュール

- ✅ **Core IR テストスイート** (`tests/test_core_ir.ml` - 143行)
  - 変数ID生成テスト: 一意性とリセット機能の検証
  - ラベル生成テスト: 自動インクリメントの確認
  - 基本式構築テスト: リテラル、変数、プリミティブ演算
  - Let式・If式テスト: 複合式の構築と型検証
  - メタデータテスト: 最適化フラグと効果集合のデフォルト値
  - Pretty Printerテスト: 演算子とパターンの文字列化
  - **全9テスト成功** ✅

**📊 Week 9 統計**
- 総実装行数: 695行（ir.ml 383行 + ir_printer.ml 312行）
- テスト行数: 143行
- テスト成功率: 100% (9/9)
- ビルド警告: 0件
- 全体テストスイート: 112テスト全成功（Phase 1-3 統合）

**🔑 重要な設計判断**
- **SSA形式への準備**: Phiノード、基本ブロックパラメータを先行定義。変数IDに一意識別子を付与し、将来のSSA変換を容易に。
- **メタデータの完全保持**: Span情報（診断用）、効果集合（`effect {diagnostic}` 準拠）、Capability情報（Phase 2後半で拡張予定）、最適化フラグ（DCE除外マーカー含む）を全て保持。
- **型情報の完全マッピング**: Typed ASTの型（`Types.ty`）をそのまま引き継ぐ。プリミティブ型、複合型（タプル、レコード、ADT）を網羅し、LLVM IR生成時の型レイアウト決定を容易に。

**🎯 次のステップ (Week 9-10)**
- 糖衣削除（Desugaring）パス: パターンマッチ、パイプ演算子、let再束縛の正規化
- Typed AST → Core IR 変換: `ir_builder.ml`の実装
- CFG構築: 制御フローグラフの生成

**📝 技術的メモ**

- **名前衝突の解決**: `Typed_ast.TVar` (型付き式の変数参照) と `Types.TVar` (型の型変数) の衝突を `Types.TVar` と明示的に修飾して解決
- **ビルドシステム**: Duneに `typed_ast`, `type_inference`, `test_type_inference` を追加済み
- **型等価性判定**: `Types.type_equal` を追加し、構造的等価性をサポート
- **コンストラクタ型環境**: `initial_env` で Option/Result の型変数を正しく量化
- **複合リテラルの型推論**: `infer_literal` のシグネチャを `env -> literal -> span -> (ty * literal * substitution, type_error) result` に変更し、タプル・レコードリテラルをサポート。`infer_tuple_elements` と `infer_record_fields` の補助関数を追加

## 技術詳細

設計ドキュメント: [docs/parser_design.md](docs/parser_design.md)

### AST 設計
- すべてのノードに `span: { start: int; end_: int }` を付与
- バイトオフセットで位置を記録 (行・列番号は診断時に計算)
- 仕様書 [1-1-syntax.md](../../docs/spec/1-1-syntax.md) に準拠

### 演算子優先順位
仕様書 §D.1 の固定優先順位表をMenhirの %left/%right で実装:
- 最高優先: 後置演算子 (関数呼び出し、フィールドアクセス、`?`)
- 最低優先: パイプ `|>` (左結合)

### Unicode 対応
- Phase 1: ASCII 識別子のみサポート (`[a-zA-Z_][a-zA-Z0-9_]*`)
- Phase 2 以降: Unicode XID 完全対応予定

### AST ダンプ
- `src/ast_printer.ml` で CLI とテスト向けの共通 AST 文字列表現を提供
- ゴールデンテストと `--emit-ast` の出力はこのプリンタを利用
