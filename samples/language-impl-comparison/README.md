# 言語実装比較サンプル

このディレクトリは Reml の記述性や読みやすさを他言語と比較するための試行的な小規模言語実装をまとめています。対象は以下の通りです。

## Remlと比較対象となる言語・機能

- Reml（擬似コード）: `reml/`
- Rust: `rust/`
- OCaml: `ocaml/`
- OCaml 5: `ocaml5/`
- Haskell: `haskell/`
- Elixir: `elixir/`
- F#: `fsharp/`
- Koka: `koka/`
- Raku: `raku/`
- Scala 3: `scala3/`
- Elm: `elm/`
- Swift: `swift/`
- Nim: `nim/`
- Go: `go/`
- PureScript: `purescript/`
- パーサーコンビネーター利用例: `parser-combinator/`
- パーサージェネレーター利用例: `parser-generator/`

## 実装する小規模言語

1. **ミニ Lisp 評価機** (`mini_lisp.*`)
2. **JSON パーサー** (`json_parser.*`)
3. **PL/0 風トイ言語コンパイラ断片** (`pl0.*`)
4. **Markdown風軽量マークアップパーサー** (`markdown_parser.*`)
5. **SQL風クエリ言語パーサー** (`sql_parser.*`)
6. **代数的効果を使うミニ言語** (`algebraic_effects.*`)
7. **正規表現エンジン** (`regex_engine.*`)
8. **YAML風パーサー** (`yaml_parser.*`)
9. **TOML風設定ファイルパーサー** (`toml_parser.*`)
10. **テンプレート言語** (`template_engine.*`)
11. **JSON拡張版** (`json_extended.*`)
12. **Basic言語インタープリタ** (`basic_interpreter.*`)
13. **Pratt Parser実装** (`pratt_parser.*`)
14. **Hindley-Milner型推論器** (`hindley_milner.*`)

Reml 版は仕様記述に合わせて `reml` 言語タグのコードブロックを用いています。他言語のサンプルはコンパイル可能な構造を意識しつつも、比較のため読みやすさを優先した最小実装に留めています。関数型言語（Haskell/OCaml）では純粋性と型推論の違い、Elixir では BEAM 上のパターンマッチとプロセス指向という観点で Reml と対比できるよう構成しています。

Reml ディレクトリには手続き的な解析器に加え、`Core.Parse` コンビネーターを用いた JSON / Lisp / PL/0 の実装（`json_parser_combinator.reml`、`mini_lisp_combinator.reml`、`pl0_combinator.reml`）も収録しており、Chapter 2 の標準 API を利用した記述スタイルを確認できます。

### Basic言語インタープリタについて

`basic_interpreter.*` は、LET, PRINT, IF, FOR, WHILE, GOTO, GOSUB, RETURN, DIM, END などの基本命令を持つBasic言語のインタープリタ実装です。各言語で次の特徴を比較できます：

- **Swift**: Result型とenum-based ADT、パターンマッチを活用した関数型スタイル
- **Scala 3**: 新しいenum構文、Either型を用いた関数合成、不変データ構造
- **Nim**: object variants、Result型、手続き型と関数型のハイブリッドスタイル
- **Koka**: 効果システムを活用したエラーハンドリング（runtime効果）、代数的データ型
- **Go**: インターフェースベースの型表現、エラーハンドリング、再帰的実行
- **Rust**: 代数的データ型（ADT）、Result型、所有権システム、パターンマッチング

各実装は言語の慣用的な書き方を重視しており、エラーハンドリングの違い（例外 vs Result型 vs 効果システム）やデータ構造の表現方法（tagged union, enum, interface, object variants）を比較する良い教材となります。

### Pratt Parser実装について

`pratt_parser.*` は、演算子優先度解析の古典的手法であるPratt Parsingアルゴリズムの実装です。四則演算、累乗、単項演算子を持つ数式パーサーを通じて、以下の特徴を比較できます：

- **PureScript**: Either型モナド、Effect型システム、Row Polymorphism、関数合成
- **Reml**: Result型、`2-4-op-builder.md` のコンビネーターアプローチとの対比

Pratt Parserは `binding_power`（束縛力）と `nud`/`led`（前置/中置解析）の概念に基づいており、多くの産業用パーサー（V8, Rust-Analyzer, TypeScript等）が採用しています。一方、Remlの `OpBuilder` コンビネーターは宣言的な演算子定義を提供します：

**Pratt Parser方式**（手続き的）:

```reml
// 束縛力で優先度と結合性を数値的に制御
Plus -> { left: 20, right: 21 }   // 左結合
Power -> { left: 40, right: 39 }  // 右結合（right < left）
```

**OpBuilder方式**（宣言的）:

```reml
// infix_left/infix_right で明示的に宣言
OpBuilder.new()
  .infix_left(20, [("+", Add), ("-", Sub)])
  .infix_right(40, [("^", Pow)])
```

Pratt Parserは非常に高速で動的な演算子追加が容易ですが、OpBuilderは型安全性が高く、他のコンビネーターと自然に統合できます。この実装を通じて、両アプローチの設計トレードオフを理解できます。

### Hindley-Milner型推論器について

`hindley_milner.*` は、関数型言語の型推論の基礎となるHindley-Milner型システムの実装です。Algorithm Wによる単一化ベース型推論を通じて、以下の特徴を比較できます：

- **OCaml**: ref による破壊的単一化、Map モジュール、例外ベースエラー処理
- **Reml**: Ref型、Result型、`1-2-types-inference.md` の型システム仕様との対応

主要な概念:

1. **Algorithm W**: 最も一般的な型を自動推論（Damas-Milner）
2. **単一化（Unification）**: 型変数の制約解決と無限型の防止（occurs check）
3. **let多相（Let-polymorphism）**: `generalize`/`instantiate` による型の量化
4. **レベルベース一般化**: ネストレベルで多相化の範囲を制御

実装の比較ポイント:

**OCaml**:

```ocaml
(* ref による暗黙の破壊的更新 *)
type tyvar = Unbound of int * int | Link of ty
let tvr = ref (Unbound (0, 0))
tvr := Link ty
```

**Reml**:

```reml
// Ref型で明示的に管理
type TyVar = Unbound(id: Int, level: Int) | Link(ty: Ty)
let tvr = Ref.new(Unbound(0, 0))
Ref.set(tvr, Link(ty))
```

この実装は `1-2-types-inference.md` で定義されたRemlの型システムの基礎であり、効果システム（`1-3-effects-safety.md`）やCapabilityシステム（`3-8-core-runtime-capability.md`）と統合することで、安全性と表現力を両立させています。

> **注記**: これらの実装はドキュメント用途のため、依存管理やビルドスクリプトは付属しません。必要に応じて利用者側で補完してください。

## 関連ガイド

- `yaml_parser.reml` や `template_engine.reml` のコレクション操作を改善する際は、[`コレクション収集戦略ガイド`](../../guides/collection-pipeline-guide.md) を参照してください。`Iter`/`Collector` と `List`/`Map`/`Table` の連携パターンや、`CollectError` を `Diagnostic` へ橋渡しする手順をまとめています。
