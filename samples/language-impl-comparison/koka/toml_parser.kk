// TOML風設定ファイルパーサー：Kokaの効果システムを活用した実装。
//
// 対応する構文（TOML v1.0.0準拠の簡易版）：
// - キーバリューペア: `key = "value"`
// - テーブル: `[section]`
// - 配列テーブル: `[[array_section]]`
// - データ型: 文字列、整数、浮動小数点、真偽値、配列、インラインテーブル
// - コメント: `# comment`
//
// Kokaの特徴を活かした実装：
// - 効果システムによる型安全なパーサー実装
// - 代数的効果を活用したエラーハンドリング
// - 型推論による簡潔な記述
// - ネストした構造を自然に表現できる代数的データ型
//
// Reml実装との比較ポイント：
// - Kokaは効果システムが明示的（parse効果）
// - エラー回復はchooseによるバックトラックで実現
// - コレクション操作はstd/coreのリスト関数を使用

import std/core
import std/core/types
import std/text/parse

// TOML値の表現。
type toml-value
  TString(value: string)
  TInteger(value: int)
  TFloat(value: float64)
  TBool(value: bool)
  TArray(items: list<toml-value>)
  TInlineTable(entries: list<(string, toml-value)>)

// TOMLテーブルの表現。
alias toml-table = list<(string, toml-value)>

// TOMLドキュメントの表現。
struct toml-document
  root: toml-table
  tables: list<(list<string>, toml-table)>  // (セクション名パス, テーブル)

// パーサー型（簡易実装）。
alias parser<a> = parse<a>

// 水平空白のスキップ。
fun hspace(): <parse> ()
  whitespace0()

// コメントのスキップ（`#` から行末まで）。
fun comment(): <parse> ()
  char('#')
  many-until(fn() satisfy(fn(c) c != '\n'), fn() peek(fn() char('\n')))
  ()

// 空白・コメントのスキップ。
fun sc(): <parse> ()
  many(fn()
    choose([
      fn() { whitespace1(); () },
      fn() comment()
    ])
  )
  ()

// 字句解析：トークン後の空白をスキップ。
fun lexeme(p: () -> <parse> a): <parse> a
  val result = p()
  sc()
  result

// シンボルのパース。
fun symbol(text: string): <parse> ()
  pstring(text)
  sc()
  ()

// キー名のパース（識別子または引用符付き文字列）。
fun key(): <parse> string
  choose([
    // 引用符付きキー
    fn() {
      char('"')
      val chars = many-until(fn() satisfy(fn(c) c != '"'), fn() peek(fn() char('"')))
      char('"')
      sc()
      chars.string
    },
    // ベアキー（英数字・`-`・`_`のみ）
    fn() {
      val first = satisfy(fn(c) c.is-alpha || c.is-digit || c == '-' || c == '_')
      val rest = many(fn() satisfy(fn(c) c.is-alpha || c.is-digit || c == '-' || c == '_'))
      sc()
      (Cons(first, rest)).string
    }
  ])

// ドットで区切られたキーパス（例：`section.subsection.key`）。
fun key-path(): <parse> list<string>
  sep-by1(key, fn() symbol("."))

// 文字列値のパース（基本文字列のみの簡易実装）。
fun string-value(): <parse> toml-value
  char('"')
  val chars = many(fn()
    choose([
      // エスケープシーケンス
      fn() { char('\\'); char('"'); '"' },
      fn() { char('\\'); char('\\'); '\\' },
      fn() { char('\\'); char('n'); '\n' },
      fn() { char('\\'); char('t'); '\t' },
      // 通常の文字
      fn() satisfy(fn(c) c != '"' && c != '\\')
    ])
  )
  char('"')
  sc()
  TString(chars.string)

// 整数値のパース。
fun integer-value(): <parse> toml-value
  val sign-opt = optional(char('-'))
  val n = pint()
  sc()
  match sign-opt
    Nothing -> TInteger(n)
    Just(_) -> TInteger(0 - n)

// 浮動小数点値のパース（簡易実装）。
fun float-value(): <parse> toml-value
  val sign-opt = optional(char('-'))
  val integer-part = many1(fn() digit())
  char('.')
  val decimal-part = many1(fn() digit())
  sc()

  val int-str = integer-part.string
  val dec-str = decimal-part.string
  val float-str = int-str ++ "." ++ dec-str
  val f = parse-float64(float-str).default(0.0)

  match sign-opt
    Nothing -> TFloat(f)
    Just(_) -> TFloat(0.0 - f)

// 真偽値のパース。
fun boolean-value(): <parse> toml-value
  choose([
    fn() { symbol("true"); TBool(True) },
    fn() { symbol("false"); TBool(False) }
  ])

// 配列のパース。
fun array-value(): <parse> toml-value
  symbol("[")
  val items = sep-by(toml-value, fn() symbol(","))
  optional(fn() symbol(","))  // トレーリングカンマ許可
  symbol("]")
  TArray(items)

// インラインテーブルのパース（`{ key = value, ... }`）。
fun inline-table(): <parse> toml-value
  symbol("{")
  val entries = sep-by(
    fn()
      val k = key()
      symbol("=")
      val v = toml-value()
      (k, v)
    ,
    fn() symbol(",")
  )
  optional(fn() symbol(","))  // トレーリングカンマ許可
  symbol("}")
  TInlineTable(entries)

// TOML値のパース（再帰的）。
fun toml-value(): <parse> toml-value
  choose([
    // 文字列
    fn() string-value(),
    // 真偽値（整数より前にチェック）
    fn() boolean-value(),
    // 浮動小数点（整数より前にチェック）
    fn() optional(fn() {
      val pos = current-pos()
      val result = float-value()
      result
    }).default(throw("not a float")),
    // 整数
    fn() integer-value(),
    // 配列
    fn() array-value(),
    // インラインテーブル
    fn() inline-table()
  ])

// キーバリューペアのパース（`key = value`）。
fun key-value-pair(): <parse> (list<string>, toml-value)
  val path = key-path()
  symbol("=")
  val value = toml-value()
  (path, value)

// テーブルヘッダーのパース（`[section.subsection]`）。
fun table-header(): <parse> list<string>
  symbol("[")
  val path = key-path()
  symbol("]")
  path

// 配列テーブルヘッダーのパース（`[[array_section]]`）。
fun array-table-header(): <parse> list<string>
  symbol("[[")
  val path = key-path()
  symbol("]]")
  path

// ドキュメント要素（キーバリューペアまたはテーブル定義）。
type document-element
  KeyValue(path: list<string>, value: toml-value)
  Table(path: list<string>)
  ArrayTable(path: list<string>)

// ドキュメント要素のパース。
fun document-element(): <parse> document-element
  choose([
    fn() { val path = array-table-header(); ArrayTable(path) },
    fn() { val path = table-header(); Table(path) },
    fn() { val (path, value) = key-value-pair(); KeyValue(path, value) }
  ])

// ネストしたキーパスに値を挿入する補助関数。
fun insert-nested(table: toml-table, path: list<string>, value: toml-value): toml-table
  match path
    Nil -> table
    Cons(key, Nil) ->
      Cons((key, value), table)
    Cons(key, rest) ->
      // 既存のネストしたテーブルを取得または作成
      val nested = table.lookup(fn((k, _)) k == key)
        .flatmap(fn((_, v))
          match v
            TInlineTable(entries) -> Just(entries)
            _ -> Nothing
        ).default([])
      val updated-nested = insert-nested(nested, rest, value)
      Cons((key, TInlineTable(updated-nested)), table.filter(fn((k, _)) k != key))

// ドキュメント全体のパース。
fun document(): <parse> toml-document
  sc()
  val elements = many(fn()
    val elem = document-element()
    sc()
    elem
  )
  eof()

  // 要素をグループ化してドキュメント構造を構築
  var current-table := []
  var root := []
  var tables := []

  elements.foreach fn(elem)
    match elem
      Table(path) ->
        current-table := path
        // テーブルが存在しない場合は追加
        if !tables.any(fn((p, _)) p == path) then
          tables := Cons((path, []), tables)

      ArrayTable(path) ->
        current-table := path
        // 配列テーブルは簡易実装では通常テーブルと同じ扱い
        if !tables.any(fn((p, _)) p == path) then
          tables := Cons((path, []), tables)

      KeyValue(path, value) ->
        if current-table.is-nil then
          // ルートテーブルに追加
          root := insert-nested(root, path, value)
        else
          // 現在のテーブルに追加
          val existing-table = tables.lookup(fn((p, _)) p == current-table)
            .map(fn((_, t)) t).default([])
          val updated-table = insert-nested(existing-table, path, value)
          tables := Cons(
            (current-table, updated-table),
            tables.filter(fn((p, _)) p != current-table)
          )

  Toml-document(root, tables)

// パブリックAPI：TOML文字列をパース。
pub fun parse-toml(input: string): parse toml-document
  parse(input.slice, document)

// 簡易的なレンダリング（検証用）。
pub fun render-to-string(doc: toml-document): string
  val root-str = render-table(doc.root, [])

  val tables-str = doc.tables.map(fn((path, table))
    val header = "\n[" ++ path.join(".") ++ "]\n"
    val content = render-table(table, [])
    header ++ content
  ).join("")

  root-str ++ tables-str

fun render-table(table: toml-table, prefix: list<string>): string
  table.map(fn((key, value))
    val full-key = if prefix.is-nil then key else (prefix ++ [key]).join(".")
    match value
      TInlineTable(nested) ->
        render-table(nested, prefix ++ [key])
      _ ->
        full-key ++ " = " ++ render-value(value) ++ "\n"
  ).join("")

fun render-value(value: toml-value): string
  match value
    TString(s) -> "\"" ++ s ++ "\""
    TInteger(n) -> n.show
    TFloat(f) -> f.show
    TBool(True) -> "true"
    TBool(False) -> "false"
    TArray(items) ->
      "[" ++ items.map(render-value).join(", ") ++ "]"
    TInlineTable(entries) ->
      "{ " ++ entries.map(fn((k, v))
        k ++ " = " ++ render-value(v)
      ).join(", ") ++ " }"

// テスト例：`reml.toml` 風の設定。
pub fun test-reml-toml(): <console> ()
  val example-toml = @"
# Reml パッケージ設定

[package]
name = "my_project"
version = "0.1.0"
authors = ["Author Name"]

[dependencies]
core = "1.0"

[dev-dependencies]
test_framework = "0.5"

[[plugins]]
name = "system"
version = "1.0"

[[plugins]]
name = "memory"
version = "1.0"
"

  println("--- reml.toml 風設定のパース ---")
  match try(fn() parse-toml(example-toml))
    Ok(doc) ->
      println("パース成功:")
      println(render-to-string(doc))
    Error(err) ->
      println("パースエラー: " ++ err.show)

// 複数のテスト例を実行。
pub fun test-examples(): <console> ()
  val examples = [
    ("simple_key_value", @"key = ""value"""),
    ("integer", @"answer = 42"),
    ("float", @"pi = 3.14"),
    ("boolean", @"debug = true"),
    ("array", @"colors = [""red"", ""green"", ""blue""]"),
    ("inline_table", @"server = { host = ""localhost"", port = 8080 }"),
    ("table", @"[database]\nhost = ""localhost""\nport = 5432"),
    ("nested_keys", @"server.host = ""localhost""\nserver.port = 8080")
  ]

  examples.foreach fn((name, toml-str))
    println("--- " ++ name ++ " ---")
    match try(fn() parse-toml(toml-str))
      Ok(doc) ->
        println("パース成功:")
        println(render-to-string(doc))
      Error(err) ->
        println("パースエラー: " ++ err.show)
    println("")

// Kokaの実装における特徴と課題：
//
// 1. **効果システムの活用**
//    - parse効果により、パーサー操作が型レベルで明示的
//    - エラーハンドリングがexn効果で型安全に実装される
//
// 2. **代数的データ型**
//    - ネストした構造を自然に表現できる
//    - パターンマッチによる安全な分岐処理
//
// 3. **型推論**
//    - 多くの型注釈を省略できる
//    - 関数型プログラミングのイディオムが簡潔に書ける
//
// 4. **課題**
//    - パーサーコンビネーターライブラリがRemlほど充実していない
//    - エラーメッセージのカスタマイズが限定的
//    - 可変状態（var）の使用が必要な場面がある
//
// Remlとの比較：
//
// - **Remlの利点**:
//   - パーサーコンビネーター専用のAPIが豊富
//   - cut/commitによる高品質なエラー報告
//   - recoverによる部分的なパース継続
//   - 字句レイヤの柔軟性が高い
//
// - **Kokaの利点**:
//   - 効果システムによる型安全性
//   - 代数的効果の表現力
//   - 型推論による簡潔さ
//
// - **共通点**:
//   - 関数型プログラミングのアプローチ
//   - 代数的データ型の活用
//   - 高階関数によるコンビネーター構築
//
// 実装の改善点（将来的な拡張）：
//
// 1. **エラー品質の向上**
//    - カスタムエラー型の定義
//    - エラー位置情報の詳細化
//    - 期待集合（expected set）の実装
//
// 2. **完全なTOML v1.0.0対応**
//    - 複数行文字列（"""..."""、'''...'''）
//    - リテラル文字列（'...'）
//    - 日時型（RFC 3339形式）
//    - 16進数・8進数・2進数整数
//    - 特殊浮動小数点値（inf、nan）
//
// 3. **パフォーマンス最適化**
//    - メモ化（Packrat）の導入
//    - ストリーミングパース
//    - 遅延評価の活用
//
// 4. **実用的な機能**
//    - スキーマ検証
//    - デシリアライゼーション（型付きデータ構造への変換）
//    - インクリメンタルパース