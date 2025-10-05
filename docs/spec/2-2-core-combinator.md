# 2.2 コア・コンビネータ

> 目的：**小さく強い核**で、書きやすさ・読みやすさ・高品質エラー・実用性能（ゼロコピー／Packrat／左再帰）を同時に満たす。
> 前提：2.1 の型と実行意味（`Reply{consumed, committed}`）に準拠。**Unicode 前提**。
> 方針：\*\*最小公理系（12-15個）**を厳選し、残りは**派生（derived）\*\*として提供。

---

## A. コア（最小公理系）

> これだけで通常のパーサは書ける。各シグネチャは Reml 風擬似記法。

### A-1. 基本

```reml
fn ok<T>(v: T) -> Parser<T>                    // 成功・非消費
fn fail(msg: String = "") -> Parser<Never>     // 失敗・非消費（期待集合は空）
fn eof() -> Parser<()>                         // 入力末尾のみ成功（非消費）
fn rule<T>(name: String, p: Parser<T>) -> Parser<T> // 名前/ID 付与（Packrat/診断）
fn label<T>(name: String, p: Parser<T>) -> Parser<T> // 失敗時の期待名を差し替え
```

* `eof` は `RunConfig.require_eof` と相補。
* `rule` は **ParserId** を固定化し、メモキーとトレースに使う。

### A-2. 直列・選択

```reml
fn then<A,B>(p: Parser<A>, q: Parser<B>) -> Parser<(A,B)>     // 直列
fn andThen<A,B>(p: Parser<A>, f: A -> Parser<B>) -> Parser<B> // = flatMap
fn skipL<A,B>(p: Parser<A>, q: Parser<B>) -> Parser<B>        // 左を捨てる
fn skipR<A,B>(p: Parser<A>, q: Parser<B>) -> Parser<A>        // 右を捨てる
fn or<A>(p: Parser<A>, q: Parser<A>) -> Parser<A>             // 左優先の選択
fn choice<A>(xs: [Parser<A>]) -> Parser<A>                    // 左から順に or
```

* **失敗統合規則**（2.1 準拠）

  * `or` は：`p` が `Err(consumed=true or committed=true)` なら **`q` を試さない**。
  * `p` が **空失敗**（`consumed=false, committed=false`）なら `q` を試す。

### A-3. 変換・コミット・回復

```reml
fn map<A,B>(p: Parser<A>, f: A -> B) -> Parser<B>
fn cut<T>(p: Parser<T>) -> Parser<T>                 // p 内の失敗を committed=true に
fn cut_here() -> Parser<()>                           // ゼロ幅コミット
fn attempt<T>(p: Parser<T>) -> Parser<T>              // 失敗時に消費を巻き戻す（空失敗化）
fn recover<T>(p: Parser<T>, until: Parser<()>, with: T) -> Parser<T>
// p 失敗時、入力を until まで読み捨て with で継続（診断を残す）
fn trace<T>(p: Parser<T>) -> Parser<T>                // 追跡ON時のみスパンを収集
```

* **使用指針**

  * 迷ったら **`attempt` を選択分岐の直前**に置く（`try` 相当）。
  * \*\*「ここからはこの構文で確定」\*\*という位置に **`cut_here()`**。
  * エラーから**同期**して処理を続けたい時は **`recover`**。

### A-4. 繰り返し・任意

```reml
fn opt<A>(p: Parser<A>) -> Parser<Option<A>>              // 空成功可（非消費）
fn many<A>(p: Parser<A>) -> Parser<[A]>                   // 0回以上
fn many1<A>(p: Parser<A>) -> Parser<[A]>                  // 1回以上
fn repeat<A>(p: Parser<A>, min: usize, max: Option<usize>) -> Parser<[A]>
fn sepBy<A,S>(p: Parser<A>, sep: Parser<S>) -> Parser<[A]>
fn sepBy1<A,S>(p: Parser<A>, sep: Parser<S>) -> Parser<[A]>
fn manyTill<A,End>(p: Parser<A>, end: Parser<End>) -> Parser<[A]>
```

* **無限ループ安全**：`many` 系は **空成功パーサ**を検出したらエラーにする（メッセージ：「繰り返し本体が空成功」）。

### A-5. 括り・前後関係

```reml
fn between<A>(open: Parser<()>, p: Parser<A>, close: Parser<()>) -> Parser<A>
fn preceded<A,B>(pre: Parser<A>, p: Parser<B>) -> Parser<B>
fn terminated<A,B>(p: Parser<A>, post: Parser<B>) -> Parser<A>
fn delimited<A,B,C>(a: Parser<A>, b: Parser<B>, c: Parser<C>) -> Parser<B>
```

### A-6. 先読み・否定

```reml
fn lookahead<A>(p: Parser<A>) -> Parser<A>          // 成功しても非消費
fn notFollowedBy<A>(p: Parser<A>) -> Parser<()>     // p が失敗すれば成功（非消費）
```

* `lookahead` は**成功しても消費しない**ため、分岐予告や曖昧性解消に有効。
* `notFollowedBy` はキーワード衝突（`ident` だが直後が英数字ならNG 等）に便利。

### A-7. チェーン（演算子の左/右結合）

```reml
fn chainl1<A>(term: Parser<A>, op: Parser<(A, A) -> A>) -> Parser<A>
fn chainr1<A>(term: Parser<A>, op: Parser<(A, A) -> A>) -> Parser<A>
```

* **実装規約**：内部で `attempt` を適切に使い、`term op term op ...` の途中失敗が**手前の選択**へ波及しないようにする。
* べき乗など右結合は `chainr1`。

### A-8. スパン・位置

```reml
fn spanned<A>(p: Parser<A>) -> Parser<(A, Span)>      // 値とスパン
fn position() -> Parser<Span>                         // ゼロ幅で現在位置
```

* AST 構築で**位置情報**を付与するための基本ユーティリティ。

---

## B. 前後空白（字句インターフェイス）

> 文字モデル/Unicode の扱いは 1.4、Lex は 2.3 で詳細化。

```reml
fn padded<A>(p: Parser<A>, space: Parser<()>) -> Parser<A>  // 前後に space を食う
fn lexeme<A>(space: Parser<()>, p: Parser<A>) -> Parser<A>  // 後ろのみ space
fn symbol(space: Parser<()>, s: Str) -> Parser<()>          // 文字列シンボル＋lexeme
```

* **推奨**：`let sc = Lex.spaceOrTabsOrNewlines | Lex.comment... |> Lex.skipMany` を `space` に。
* `symbol(sc, "(")` → `(` を読んで後続の空白/コメントを食う。

### B-1. 空白プロファイルの共有 {#parser-with-space}

```reml
impl<A> Parser<A> {
  fn with_space(self, space: Parser<()>) -> Parser<A>
}

impl Parser<()> {
  fn space_id(&self) -> ParserId
}
```

* `with_space` は `Parser` 全体に既定の空白パーサを紐付ける。内部で生成される `lexeme` / `symbol` / `keyword` 等の字句ユーティリティは、この設定を検出すると `space` を既定値として利用する。複数回呼び出した場合は**最後に適用した空白**で上書きされる。
* `space` は通常 `config_trivia(profile)` など `Parser<()>` を使い、字句レイヤ（2.3）で定義したコメント・ホワイトスペース処理を再利用する。`RunConfig.extensions["lex"].profile` を共有すると IDE/CLI/テストが同じスキップ戦略を採用できる。`with_space` はパーサの意味論を変えず、空白処理が省略された箇所（例：`symbol("if")`）へ自動注入する糖衣である。
* `space_id` は空白パーサに安定した `ParserId` を割り当てる。`rule` で既に ID が確保されている場合はその値を返し、未登録の場合は内部で匿名の `rule("space")` を差し込んで ID を生成する。`RunConfig.extensions["lex"]` 等に格納して IDE/CLI と共有する用途を想定している。【参照: 2-3-lexer.md §L-4】
* `space_id` が返す ID は Packrat メモ化と同じ仕組みを利用する。したがって `Parser<()>` をコピーしても ID は保持され、0-1 §1.1 の性能要件（共有メモ化）を満たす。

---

## C. 便利だが派生（derived）に落とすもの

> コアを太らせないため、以下は **コアの合成**で提供（実装は標準ライブラリ側）。

```reml
fn separatedPair<A,B,S>(a: Parser<A>, sep: Parser<S>, b: Parser<B>) -> Parser<(A,B)>
fn tuple2<A,B>(a: Parser<A>, b: Parser<B>) -> Parser<(A,B)>        // ~ then/map
fn list1<A,S>(elem: Parser<A>, sep: Parser<S>) -> Parser<[A]>      // ~ sepBy1
fn atomic<T>(p: Parser<T>) -> Parser<T>                             // = label+cut の糖衣
fn expect<T>(name: String, p: Parser<T>) -> Parser<T>               // = label(name, cut(p))
fn separatedListTrailing<A,S>(elem: Parser<A>, sep: Parser<S>) -> Parser<[A]> // 末尾区切り許容
fn expect_keyword(space: Parser<()>, kw: Str) -> Parser<()>        // = expect(kw, keyword(space, kw))
fn expect_symbol(space: Parser<()>, text: Str) -> Parser<()>        // = expect(text, symbol(space, text))
```

`expect_keyword`/`expect_symbol` は `Core.Parse.Lex` のトークン API と `expect` を合成した糖衣で、キーワードや記号の欠落時に「`then` を期待しました」のようなメッセージを自動生成する。PL/0 サンプルで多用される `skipL(sc, kw("while"))`／`label+cut` の記述を 1 行にまとめ、DSL の差分実装時に診断の一貫性を確保できる。【F:../examples/language-impl-comparison/reml/pl0_combinator.reml†L103-L111】

---

## D. 消費／コミットの要点（実務上の指針）

* **分岐の手前に `attempt`**：

  ```reml
  attempt(sym("if").then(expr).then(block))
    .or(attempt(sym("while").then(expr).then(block)))
    .or(stmtSimple)
  ```

  → 先頭のキーワード以降で失敗しても、**空失敗**として次の分岐へ進める。
* **「ここからはこの形」→ `cut_here()`**：

  ```reml
  sym("let").then(ident).then(cut_here()).then(sym("=").then(expr))
  ```

  → `let x` まで来たら **`=` が絶対必要**。以降の失敗は**コミット済み**として報告。
* **繰り返しの本体は空成功禁止**：`many(p)` の `p` が空成功だと**停止しない**。ライブラリが検出してエラーに。
* **`lookahead` は非消費**：曖昧性の解消・キーワードの後判定に。

---

## E. 例：四則演算（べき乗右結合、カッコ、単項 -）

```reml
use Core.Parse
use Core.Parse.Lex

let sc     = Lex.spaceOrTabsOrNewlines |> Lex.skipMany
let sym(s) = symbol(sc, s)
let int    = lexeme(sc, Lex.int(10))

let expr: Parser<i64> = rule("expr", chainl1(term, addOp))
let term: Parser<i64> = rule("term", chainl1(factor, mulOp))

let addOp: Parser<(i64,i64)->i64> =
  (sym("+").map(|_| (|a,b| a+b)))
  .or(sym("-").map(|_| (|a,b| a-b)))

let mulOp: Parser<(i64,i64)->i64> =
  (sym("*").map(|_| (|a,b| a*b)))
  .or(sym("/").map(|_| (|a,b| a/b)))

let factor: Parser<i64> = rule("factor",
  (sym("(").then(cut(expr)).then(sym(")")).map(|(_,v,_)| v))   // 括弧に cut
    .or(sym("-").then(factor).map(|(_,x)| -x))                  // 単項 -
    .or(int)
)
```

* `cut(expr)` により、開き括弧の後は**閉じ括弧が必須**。
* べき乗を足すなら `chainr1(base, powOp)` を `term` より上に挿入。

---

## F. エラー品質のための慣習

* **`rule` と `label` を積極的に**：失敗メッセージに人間語が出る。
* **`expect(name, p)`（= `label+cut`）** で「ここは絶対に *name*」を宣言。
* **`recover`** は**同期トークン**を必ず明示（例：`;` や行末）し、診断に「ここまでスキップ」を記録。

---

## G. 仕様チェックリスト

* [ ] コア 8 群（基本／直列選択／変換・コミット／繰返し／括り／先読み／チェーン／スパン・位置）。
* [ ] `attempt` を含め、**消費／コミットの 2bit**と整合する選択規則。
* [ ] `many` 系の**空成功検出**。
* [ ] `eof`・`position`・`spanned` のゼロ幅挙動。
* [ ] `rule` による **ParserId** 固定と診断（Packrat/左再帰に必須）。
* [ ] 前後空白 3 兄弟（`padded/lexeme/symbol`）は Unicode の `Lex` と噛み合う。
* [ ] すべて**純関数**（2.1／1.3 の効果方針）。

---

### まとめ

* **12-15 個の最小コア**で、実務に必要な記法（分岐、繰返し、先読み、コミット、位置）が網羅され、
* `attempt / cut / label / recover` の**四点セット**で **高品質エラー**と**制御可能なバックトラック**を両立。
* 追加の便利関数は **派生**として提供し、コアを痩せたまま保つ。

---

## H. Core.Regex 連携ガイド

> 目的：正規表現エンジン（`Core.Regex`）を `Parser` 上で安全かつ高速に利用できるよう、責務境界と既定ポリシーを明確化する。

### H-1. 派生コンビネータ（`Core.Parse.Regex` 名前空間）

| API | 説明 | 効果 |
| --- | --- | --- |
| `regex(handle: RegexHandle) -> Parser<Str>` | 入力の先頭から `handle` にマッチした範囲を返す。失敗時は空失敗扱い。 | `effect {regex}` |
| `regex_capture(handle: RegexHandle) -> Parser<List<Str>>` | キャプチャ群を `List<Str>` で返す。ゼロ幅先読み（lookaround）は `Str::empty` を返す。 | `effect {regex}` |
| `regex_token(handle: RegexHandle, to_token: Str -> Token) -> Parser<Token>` | マッチ結果をトークン化し、`Core.Parse.Lex` のトークン列へ組み込む。 | `effect {regex}` |

* これらは **派生コンビネータ**であり、実装は `regex(handle)` → `position()` → `Core.Regex.run` の合成で表現される。`Core.Parse` の最小公理系には追加されない。
* `RegexHandle` の取得は [3.3 §9 Regex エンジン](3-3-core-text-unicode.md#regex-engine) の `Core.Regex.compile` を通じて行う。
* `regex_token` は [2.3 字句レイヤ](2-3-lexer.md) の `Lex.token` と組み合わせ、字句層での効率的なパターン処理を提供する。

### H-2. 責務境界

1. **`Core.Regex`**（[3.3 §9](3-3-core-text-unicode.md#regex-engine)）はパターン解析・オートマトン構築・Unicode クラス互換性を担当する。
2. **`Core.Parse`** は `RegexHandle` を受け取り、入力スライスと `Span` を管理する。`Parser` の `Reply` 契約（2.1）を尊重し、消費位置とコミットビットを正しく更新する。
3. **`RunConfig`**（[2.6 §F](2-6-execution-strategy.md#regex-run-policy)）は Packrat/メモ化ポリシーとエンジン選択を制御し、既定値として `memo = Auto` を採用する。`Auto` は `regex_capture` が 3 段以上ネストしたケースでのみ Packrat を要求し、通常の字句認識ではキャッシュを使わない。
4. JIT ベースのエンジンは [3.8 §1.4](3-8-core-runtime-capability.md#regex-capability) の Capability を満たすプラットフォームでのみ有効化される。Capability が無い場合は安全な NFA 実装にフォールバックする。

### H-3. Unicode クラスの互換保証

* `RegexHandle` は常に `UnicodeClassProfile`（3.3 §9-2）を保持し、`Core.Parse` は入力側の `UnicodeVersion` と一致するか検証する。差異がある場合は `DiagnosticDomain::Regex` の `regex.unicode.mismatch` を即時報告し、解析を中断する。
* `RunConfig.extensions["regex"].unicode_profile` を指定すると、`Core.Regex.compile` が互換性チェックを行う。未指定時は `Unicode 15.0` を既定値とし、将来の更新は `feature {unicode-next}` フラグ経由で試験導入する。
* Grapheme 単位での照合は `regex_capture` 後に `Core.Text.grapheme_seq` を利用し、`display_width` 計算（3.3 §5.1）と整合する。`regex(handle)` はバイト境界を返すが、`regex_capture` に `@g` フラグを付与すると書記素境界での切り出しを強制する。

> **0-1 指針との適合**：`Auto` メモ化と Capability 連携により、性能原則（1.1）と安全性原則（1.2）を損なわずに正規表現 DSL を段階的に導入できる。

---

## I. Capability 要求パターン

Reml のパーサープラグインは、登録時に `Core.Parse.Plugin` 拡張が提供する `register_capability` API を介して機能を公開し、利用側は `with_capabilities` で必要機能を要求する。以下は **拡張モジュールが定義する Capability** と対応するコンビネータである（純粋なコアのみを使用する場合は読み飛ばしてよい）。

| Capability | 対応コンビネータ/機能 | 要約 |
| --- | --- | --- |
| `parser.atomic` | `atomic(p)` | 分岐打ち切り (`label+cut`) を伴う原子的シーケンス。`Parser` は `Core.Parse.Plugin.Recoverable` トレイトを実装している必要がある。 |
| `parser.recover` | `recover(p, with=...)` | 回復処理・診断集約を提供。同期トークンと監査ログ（`audit`）を要求。 |
| `parser.trace` | `trace(p, tag)` | トレースイベントを生成し、`RunConfig.extensions["lsp"].syntaxHighlight` と整合する JSON メトリクスを出力。 |
| `parser.chain` | `chainl1` / `chainr1` / `chain` | 演算子テーブル構築で左/右結合チェインを提供。 |
| `parser.syntax.highlight` | `syntax.highlight(p)` | Semantic tokens を生成し、IDE へトークンストリームを供給。 |
| `parser.capability.packrat` | `packrat(p)` | Packrat キャッシュを内部で保持。メモリ上限を `RunConfig` で指定する。 |

**利用規約**

1. プラグインは `register_capability({"parser.atomic", ...})` を呼び出し、提供可能な capability の集合を登録する。登録されていない capability を `with_capabilities` で要求した場合は `PluginError::MissingCapability` を返す。
2. `with_capabilities(required, parser)` は `required ⊆ provided` であることを検査し、失敗時はコンパイル時に警告 (`W4201`)・実行時にエラーを生成する。
3. `parser.recover` を利用するプラグインは、`2-5-error.md` の `Diagnostic` 拡張（`domain`, `audit_id`, `change_set`）との整合を保証すること。`recover` で復旧させた場合でも監査ログに `recovery` イベントを残す。
4. `parser.syntax.highlight` と `parser.trace` は `RunConfig.extensions["lsp"].syntaxHighlight=true`（LSP 拡張）を有効にしたときのみ効果を発揮し、通常モードではゼロコストになるよう実装する。

**サンプル**

```reml
let render =
  htmlTemplate
    |> with_capabilities({"parser.atomic", "parser.trace"})
    |> trace("templating.render")

let metadata = PluginMetadata {
  id = "Reml.Web.Templating",
  version = SemVer(1,4,0),
  checksum = None,
  description = Some("HTML テンプレート DSL"),
  homepage = Some(Url::parse("https://example.com")),
  license = Some("Apache-2.0"),
  required_core = SemVerRange::from("^1.4"),
  required_cli = Some(SemVerRange::from("^1.3")),
}

let cap_atomic = ParserPluginCapability {
  name = "parser.atomic",
  version = SemVer(1,4,0),
  stage = StageRequirement::AtLeast(Stable),
  effect_scope = Set::from(["parser", "audit"]),
  traits = Set::from(["cut"]),
  since = Some(SemVer(1,4,0)),
  deprecated = None,
}

let cap_trace = ParserPluginCapability {
  name = "parser.trace",
  version = SemVer(1,4,0),
  stage = StageRequirement::AtLeast(Beta),
  effect_scope = Set::from(["parser", "telemetry"]),
  traits = Set::from(["semantic-tokens"]),
  since = Some(SemVer(1,3,0)),
  deprecated = None,
}

register_plugin(ParserPlugin {
  metadata = metadata,
  capabilities = [cap_atomic.clone(), cap_trace.clone()],
  dependencies = [],
  register = |reg| {
    reg.register_capability(cap_atomic.clone())?;
    reg.register_capability(cap_trace.clone())?;
    reg.register_parser("render", || render)?;
  }
})?
```

`Core.Parse.Plugin` 拡張は上記 capability をすべて実装しており、コアのみを読み込んだ場合は `with_capabilities` を呼んでも効果がない（no-op）。プラグインは必要な最小 capability のみ要求し、過剰な要求を避けることが推奨される。

> `Core.Parse.Plugin.Recoverable` トレイトは、回復可能なパーサが `recover` や `atomic` を使用する際の補助契約を提供する。コア API だけを利用する場合は意識する必要はない。
