# 1.1 構文（Syntax）— Reml (Readable & Expressive Meta Language) 言語コア仕様

> 目的：**短く書けて、読みやすく、エラーが説明的**になること。
> 前提：**UTF-8 / Unicode 前提**、式指向、静的型（1.2 で詳細）、パーサーコンビネーターを実装しやすい**素直な構文**。

---

> 参考: 構文全体の形式文法は [1.5 形式文法（BNF）](1-5-formal-grammar-bnf.md) を参照。

## A. 字句（Lexical）

### A.1 文字集合とエンコーディング

* ソースは **UTF-8**。エラー・位置情報はコードポイント／行・列で報告。

### A.2 空白・改行・コメント

* 空白はトークンを分離するために使用。
* 改行は **文末の候補**（B.3 参照）。
* コメント：

  * 行コメント：`// ...`（改行まで）
  * ブロックコメント：`/* ... */`（入れ子可）

### A.3 識別子とキーワード

* 識別子：`XID_Start` + `XID_Continue*`（Unicode 準拠）。
  例）`parse`, `ユーザー`, `_aux1`。
* 予約語（全一覧）：
  - **モジュール/可視性**: `module`, `use`, `as`, `pub`, `self`, `super`
  - **宣言と定義**: `let`, `var`, `fn`, `type`, `alias`, `new`, `trait`, `impl`, `extern`, `effect`, `operation`, `handler`, `conductor`, `channels`, `execution`, `monitoring`
  - **制御構文**: `if`, `then`, `else`, `match`, `with`, `for`, `in`, `while`, `loop`, `return`, `defer`, `unsafe`
  - **効果操作**: `perform`, `do`, `handle`
  - **型制約**: `where`
  - **真偽リテラル**: `true`, `false`
* 将来の拡張に備えて `break`, `continue` を予約語として確保しています。
* 演算子トークン（固定）：`|>`, `~>`, `.` , `,`, `;`, `:`, `=`, `:=`, `->`, `=>`, `(` `)` `[` `]` `{` `}`,
  `+ - * / % ^`, `== != < <= > >=`, `&& ||`, `!`, `?`, `..`.

### A.4 リテラル

* 整数：`42`, `0b1010`, `0o755`, `0xFF`, 下線区切り可（`1_000`）。
* 浮動小数：`3.14`, `1e-9`, `2_048.0`.
* 文字：`'A'`（Unicode スカラ値、1.4 参照）。
* 文字列：

  * 通常：`"hello\n"`（C系エスケープ）
  * 生：`r"^\d+$"`（バックスラッシュ非解釈）
  * 複数行：`"""line1\nline2"""`（内部改行保持）
* ブール：`true`, `false`
* タプル：`(a, b, c)`／**単位**：`()`
* 配列：`[1, 2, 3]`
* レコード：`{ x: 1, y: 2 }`（順序不問）

---

## B. トップレベルと宣言

### B.1 モジュールとインポート

* **ファイル = 1 モジュール**。先頭に `module math.number` を記述すると、このファイルの公開パスを固定できます（未記述時はパッケージ設定とファイルパスから導出）。`module` ヘッダは 1 ファイル 1 回まで。
* モジュールパスは **`.` 区切りの識別子列**で表現し、以下の成分を持ちます。

  | 成分 | 例 | 説明 |
  | --- | --- | --- |
  | ルート指定 | `::Core.Parse` | 先頭の `::` はパッケージ（crate）ルートから解決することを指示。`module` ヘッダで宣言した最上位モジュールもここから辿ります。 |
  | 相対指定 | `self.syntax`, `super.lexer` | `self` は現在のモジュール、`super` は 1 つ上のモジュール。`super.super.io` のように連続利用可能。 |
  | 既定探索 | `Core.Parse.Lex` | ルート指定が無い場合は **(1) 現在のモジュール内の宣言/`use`**, **(2) 親モジュールを遡った宣言**, **(3) ルートモジュール**, **(4) ビルトインプレリュード（`Core` など）** の順に探索します。 |
  | 別名 | `use Core.Parse.Op as Operator` | `as` でローカル名を付与。中括弧の個別項目でも使用可（`{Lex, Op as Operator}`）。 |

* `use` 文で依存を導入します。

  ```reml
  use ::Core.Parse          // ルートから
  use self.checks.Lex       // 現在ファイル配下
  use Core.Parse.{Lex, Op as Operator, Err}
  ```

  中括弧は 1 階層以上のネストに対応し、`use Core.Parse.{Lex, Op.{Infix, Prefix}}` のように部分展開できます。

### B.1.1 DSLエントリーポイント宣言 {#dsl-entry-declaration}

`reml.toml` の `[dsl]` セクションで宣言された `entry` は、1 ファイル 1 モジュールの原則に従い、該当モジュールのトップレベル公開シンボルと一致しなければならない。`exports` 配列の各名前は、以下の要件を満たすトップレベル宣言を指す。

- 宣言は `pub` であり、コンパイラが DSL メタデータを収集できるよう **`@dsl_export` 属性** を付与する。
- 宣言されるシンボルは次のいずれかの形である。
  - `pub let entry_name: Parser<T>` または `pub const entry_name: Parser<T>`（値としてのエクスポート）。
  - `pub fn entry_name(args) -> Parser<T>`（関数としてのエクスポート。`args` の既定値や名前付き引数は通常の関数規則に従う）。
  - `conductor entry_name { ... }`（Conductor パターンをエントリーポイントとして公開する場合）。
- `Parser<T>` を返すエントリは **副作用のない純粋値**が既定であり、`@dsl_export` に `allows_effects=[...]` を明示しない限り `Σ` のいずれの効果も持てない。
- `conductor` を公開する場合は、内部で組み合わされる DSL が `exports` に含まれる他のシンボルと整合するように、同一モジュール内で名前解決できなければならない。
- `@dsl_export` の `category` パラメータは `reml.toml` 側の `dsl.<name>.kind`（後述のマニフェスト仕様）と一致させ、型検査ではこの値を用いて DSL 間の互換性を検証する。

```reml
module sample.config

@dsl_export(category="config", capabilities=["Core.Config.Manifest"], version="0.1")
pub fn config_dsl() -> Parser<AppConfig> =
  root_object(|builder| { ... })

@dsl_export(category="config")
conductor config_orchestrator {
  config_dsl
    |> validate
    |> emit
}
```

`reml.toml` で `entry = "src/sample/config.reml"`、`exports = ["config_dsl", "config_orchestrator"]` と宣言した場合、上記のようにモジュールと公開シンボルが揃っていることを検証する。カテゴリや Capability 情報は Chapter 3 のマニフェスト API と連携して CLI へ引き渡される。

### B.2 可視性と `use` が導入するシンボル

* 既定は **非公開**。宣言に `pub` を付けると、そのモジュールを経由して外部から参照できます。`pub` の可視性境界は **現在のモジュールの親**で、親モジュールからさらに `pub` で公開された場合にパッケージ全体へ伝播します。
* `pub use` は再エクスポートです。`pub use Core.Parse.Lex` は `Lex` を自モジュールの公開 API に含め、呼び出し側からは `current_module.Lex` として参照できます。再エクスポートされた名前は元の宣言と同じ可視性・シンボル種別（型/値/モジュール）を保持します。
* `use` は**最後のセグメント**（または `as` で指定した別名）を現在モジュールに束縛し、モジュール内のトップレベル宣言と同一の名前空間で解決されます。
  * 同一スコープに既存の宣言や他の `use` が同名で存在する場合は **コンパイルエラー**です（一致先が同一シンボルでも明示的に `as` で回避する必要があります）。
  * 束縛は値・型・モジュールを区別せず単一の名前空間で扱うため、衝突を避けるには別名または限定パス（`module.name`）を使用します。
  * `use` で導入したシンボルは読み取り専用のビューであり、再代入や `let` での再束縛はできません。

### B.3 文の終端

* **行末**が文末として解釈される（オフサイドではなく単純な行末）。
* ただし以下では行継続（文末とみなさない）：

  * 行末が **二項演算子／コンマ／ドット／開き括弧/ブラケット** で終わる
  * 次行が **閉じ括弧**で始まる
* `;` は同一行での**明示区切り**として使用可。

### B.4 宣言の種類

* **値束縛と再代入**  \n  `let` は不変束縛、`var` は可変束縛。`var` で導入した変数はブロック内で `:=` による再代入が可能です（C.6 および [効果と安全性](1-3-effects-safety.md) を参照）。

  ```reml
  let answer = 42
  var total = 0
  let (lhs, rhs) = pair
  ```

* **関数宣言**  \n  本体は式かブロックで記述でき、名前付き引数・デフォルト引数・戻り値型をサポートします。`pub` を付けると公開関数になります。

  ```reml
  fn add(a: i64, b: i64) -> i64 = a + b

  pub fn fact(n: i64) -> i64 {
    if n <= 1 then 1 else n * fact(n - 1)
  }
  ```

* **型宣言（ADT・エイリアス・ニュータイプ）**  \n  代数的データ型のほか、`type alias` や `type Name = new T` による零コストラッパを定義できます（詳細は [型と推論](1-2-types-Inference.md)）。

  ```reml
  type Expr =
    | Int(i64)
    | Add(Expr, Expr)
    | Neg(Expr)

  type alias Bytes = [u8]
  type UserId = new i64
  ```

  `type alias`／`type ... = new ...` はどちらも `type` 宣言の派生形であり、型パラメータも他の宣言と同様に付与できます。

* **トレイト定義 (`trait`)**  \n  インターフェースを宣言し、メソッド署名やデフォルト実装を列挙します。型パラメータや `where` 制約を付与できます。

  ```reml
  trait Show<T> {
    fn show(self) -> String
  }
  ```

* **実装 (`impl`)**  \n  トレイト実装 `impl Trait for Type` と、型固有メソッド `impl Type` の両方をサポートします。ブロック内では通常の関数と同様に属性や可視性を付けられます。

  ```reml
  impl Show<i64> for i64 {
    fn show(self) -> String = self.to_string()
  }

  impl Vec<T> {
    pub fn push(mut self, value: T) { ... }
  }
  ```

* **外部宣言 (`extern`)**  \n  FFI で公開された関数を宣言します。呼び出しは `unsafe` 境界内で行います（1.3 節参照）。

  ```reml
  extern "C" fn puts(ptr: Ptr<u8>) -> i32;
  extern "C" {
    fn printf(fmt: Ptr<u8>, ...) -> i32;
  }
  ```


### B.5 効果宣言とハンドラ構文（実験段階）

> `-Zalgebraic-effects` フラグが有効な場合に限り使用可能。安定化後に文言を更新予定。
> ステージ管理と Capability の整合性は [1.3 §I.4](1-3-effects-safety.md#i4-stage-と-capability-の整合) を参照し、仕様上の基準を一元化する。

* **効果宣言**
  `effect` で操作集合を宣言し、既存の効果タグへ紐付ける。

  ```reml
  effect Console : io {
    operation log : Text -> Unit
    operation ask : Text -> Text
  }
  ```

  * `effect <Name> : <tag>` が基本形。タグは 1.3 節の `Σ` から選択し、Capability Registry（3.8 節）で stage を管理する。
  * `operation` には型注釈が必須。戻り値型は `resume` に渡す値と一致させる。

* **効果の実行 (`perform` / `do`)**
  効果操作は式内で呼び出す。`perform Effect.operation(args)` と `do Effect.operation(args)` は同義。

  ```reml
  fn greet() -> Text {
    let name = do Console.ask("name?")
    do Console.log("hi " + name)
    name
  }
  ```

  * 発生した効果は潜在効果集合に追加され、`@pure` や `@dsl_export` の検査対象となる。

* **ハンドラ構文 (`handle ... with`)**
  効果をローカルに捕捉し、任意の挙動を与える。

  ```reml
  handle greet() with
    handler Console {
      operation log(msg, resume) {
        println("LOG: " + msg)
        resume(())
      }
      operation ask(prompt, resume) {
        resume("Reml")
      }
      return value {
        value
      }
    }
  ```

  * `handler <EffectName>` ブロックで対象操作を列挙し、必要に応じて `return` 節を定義する。
  * `resume` はワンショットが既定。複数回呼び出す場合は `@reentrant` 属性と Capability 許可が必要（3.8 節）。

* **属性拡張**
  効果ハンドラ導入に伴い、以下の属性を追加する。詳細な検査規則は 1.3 節および 3.8 節を参照。

  - `@handles(Console, ...)` — 関数やハンドラが捕捉可能な効果を宣言し、残余効果計算と整合させる。
  - `@requires_capability(stage="experimental")` — 実験的 Capability を要求する API に付与し、ランタイム設定で opt-in を強制する。


### B.6 属性（Attributes）

* 宣言やブロックの直前に `@name` 形式で付与し、直後の要素に契約や最適化ヒントを与えます。複数の属性は縦に並べることで併用できます。
* 引数付き属性は `@name(arg1, key=value)` のように括弧で指定します（値は Reml の式）。
* 効果契約（`@pure`, `@no_panic`, `@no_alloc` など）は [効果と安全性](1-3-effects-safety.md) にて意味が定義され、コンパイル時に検査されます。
* 属性は `fn`・`type`・`trait`・`impl`・`extern` の各宣言、およびブロック式 `{ ... }` や `unsafe { ... }` に付与できます。

```reml
@pure
@no_panic
pub fn eval(expr: Expr) -> Result<i64, Error> = expr?

impl Parser<T> {
  @inline
  fn map<U>(self, f: T -> U) -> Parser<U> { ... }
}
```

#### 条件付きコンパイル属性 `@cfg`

* `@cfg` は宣言・`use`・文ブロックに条件付きの有効/無効を与えるコンパイル時属性で、**パース段階**で評価されます。無効化された要素は以降の解析・型検査から完全に除去されます。
* 書式は `@cfg(<predicate>)`。述語は次のキーによる等価比較と論理演算で構成します。

  | キー | 例 | 説明 |
  | --- | --- | --- |
  | `target_os` | `@cfg(target_os = "linux")` | 対象 OS。CLI/ランタイムが提供する既定値と一致する必要がある。 |
  | `target_family` | `@cfg(target_family = "unix")` | OS ファミリ。POSIX 系をまとめるなど広義の分岐に使用。 |
  | `target_arch` | `@cfg(target_arch = "aarch64")` | CPU アーキテクチャ。命令セットの切替に利用。 |
  | `target_abi` | `@cfg(target_abi = "gnu")` | ABI/ツールチェーン識別子。`RunConfigTarget.abi` を参照。 |
  | `target_env` | `@cfg(target_env = "msvc")` | 環境（libc/CRT）。省略時は `None`。 |
  | `target_vendor` | `@cfg(target_vendor = "apple")` | ベンダ識別子（トリプル構成要素）。 |
  | `target_profile` / `profile_id` | `@cfg(profile_id = "desktop-x86_64")` | CLI で選択した TargetProfile の ID。エイリアス `target_profile` も同義。 |
  | `runtime_revision` | `@cfg(runtime_revision = "rc-2024-09")` | 利用中ランタイムの互換リビジョン。 |
  | `stdlib_version` | `@cfg(stdlib_version = "1.0.0")` | バンドルされた標準ライブラリの SemVer。 |
  | `feature` | `@cfg(feature = "gpu_accel")` | ビルド構成フィーチャ（Cargo の feature に相当）。 |
  | `capability` | `@cfg(capability = "unicode.nfc")` | `Core.Runtime` が提供する Capability。`TargetCapability` から初期化。 |
  | `extra.*` | `@cfg(extra.io.blocking = "strict")` | プロジェクト固有キー。`RunConfigTarget.extra` で宣言済みのものに限る。 |
  | `has_target_profile` | `@cfg(has_target_profile = "desktop-x86_64")` | プロファイルの存在確認。CLI で未登録の場合に条件分岐可能。 |

  追加キーは `RunConfig.extensions["target"]` に登録されたもののみ利用可能で、登録されていないキーは `target.config.unknown_key` 診断を生成する。
* 述語構文：
  * `@cfg(key = "value")` … 単純比較
  * `@cfg(any(expr1, expr2, ...))`、`@cfg(all(expr1, expr2, ...))`、`@cfg(not(expr))` を組み合わせて任意のブール式を表現できます。空の `any`/`all` は許可されません。
* 評価順序は **外側の論理演算から短絡評価**します。`RunConfig` から提供されないキー、または値のスペルミスが検出された場合はコンパイラが `target.config.unknown_key` 診断を生成し、ビルドを中断します。`target_profile` / `profile_id` が参照されたのに `RunConfigTarget.profile_id` が空の場合は `target.profile.missing`、`capability` キーで未知の Capability 名が使用された場合は `target.capability.unknown` を報告します。
* `runtime_revision` と `stdlib_version` の不一致（CLI やレジストリが提供した値とコンパイラが生成したメタデータの差異）は `target.abi.mismatch` 診断で扱い、エコシステム仕様に準拠した再ビルドを要求します。
* `capability` キーは `RunConfigTarget.capabilities` セットを参照し、`Core.Runtime` が Capability Registry で公開する識別子と一致している場合のみ有効です。推論段階では Capability の存在が効果契約に反映され、到達不能な宣言は無効化されます。
* 無効化された宣言を参照するコードが存在した場合、その参照は通常の解決フェーズでエラーとなり、`unresolved.symbol.cfg` 診断で報告されます。
* `@cfg` 自体は副作用を持たず、`@cfg` の内側/外側で効果タグの整合性が保たれるように使用する必要があります（詳細は [効果と安全性](1-3-effects-safety.md#cfg-attribute) を参照）。

### B.6 拡張構文 (`schema`)

Reml コア仕様には `schema` キーワードによる設定 DSL は含まれません。設定や構成管理向けの宣言は標準ライブラリ `Core.Config`（[3-7](3-7-core-config-data.md)）のビルダ API とテンプレート機能を利用してください。必要に応じてマクロやプラグインで糖衣構文を導入できますが、これらはコアから独立した拡張と位置付けます。

### B.7 プラグイン関連構文

`package` 宣言や `use plugin` ブロックといったプラグイン配布専用のメタデータ構文は、Reml コアから切り離しました。バージョン管理や Capability 指定は `reml-plugin` CLI と外部マニフェストで扱い、言語仕様としては通常の `use` とモジュールシステムのみを定義します。プラグイン連携の詳細は `guides/DSL-plugin.md` を参照し、必要なプロジェクトで opt-in してください。


---

## B.8 DSL制御ブロック `conductor`

### B.8.1 文法概略

```ebnf
ConductorDecl   ::= "conductor" Ident ConductorBody
ConductorBody   ::= "{" ConductorSection* "}"
ConductorSection::= ConductorDslDef | ConductorChannels | ConductorExecution | ConductorMonitoring
ConductorDslDef ::= Ident ":" Ident ("=" PipelineSpec)? ConductorDslTail*
ConductorDslTail::= "|>" Ident "(" ConductorArgList? ")"
ConductorChannels ::= "channels" ConductorChannelBody
ConductorChannelBody ::= "{" (ChannelRoute NL)* "}"
ChannelRoute    ::= ConductorEndpoint "~>" ConductorEndpoint ":" Type
ConductorExecution ::= "execution" Block
ConductorMonitoring ::= "monitoring" (Ident | ConductorQualifiedName) Block
```

* `conductor` は**トップレベル宣言**として扱い、`fn` や `type` と同等に配置できる。
* `|>` の優先順位は従来通り最下位（D.1 参照）。`~>` は **チャネル定義専用トークン**であり、通常の式パーサには現れない。
* DSL定義内で記述する `depends_on`, `with_capabilities` などのビルダ API はパイプライン合成として解釈され、`Core.Parse`/`Core.Runtime` の API 呼び出しに展開される。

### B.8.2 DSL定義ワークフロー指針

1. **構文設計** — `rule` など Core.Parse コンビネータで DSL のパーサーを定義し、名前付きルールとして登録する。
2. **型モデリング** — DSL が生成する値を ADT/型クラスで表現し、効果タグを明示する。
3. **実行統合** — `conductor` 内で DSL を組み合わせ、依存関係・リソース制限・Capability を宣言する。
4. **観測性整備** — `monitoring` セクションで診断・トレース・メトリクス収集を設定する。

### B.8.3 設計指針（価値と課題の要約）

- **価値**: 型安全な DSL 合成、再利用可能なドメイン抽象、Core.Diagnostics による高い可観測性。
- **主な課題**: 初期構築コストと学習曲線、DSL層とアプリ層のデバッグ複雑性、エコシステム断片化リスク。
- **軽減策**: テンプレート・ジェネレータによる段階的導入、`label`/`recover` など標準エラー機構の活用、Capability/プラグイン標準での横断連携。

## C. 式・項・パターン

### C.1 式は**式指向**（最後の式が値）

* ブロック `{ ... }` の**最後の式**がそのブロックの値。
* `return expr` は関数内のみ（早期脱出）。省略可能（末尾が戻り値）。

### C.2 関数適用・引数

* 関数呼び出し：`f(x, y)`
* **名前付き引数**：`render(src=doc, width=80)`
* **デフォルト引数**（定義側）：`fn render(src: Doc, width: i32 = 80) = ...`
* 可変長（将来）：`fn log(...args: String) = ...`
* **部分適用**（占位）：

  ```reml
  pipe(xs)
    |> map(_ + 1)
  ```

  `_` は左側パイプ値の**代入位置**（D.3 に詳細）。

### C.3 パターン（束縛・`match` で共通）

* 変数：`x`
* ワイルドカード：`_`
* タプル：`(x, y, _)`
* レコード：`{ x, y: y0 }`（`x: x` は `x` に省略可）
* 代数型：`Some(x)`, `Add(Int(a), b)`
* ガード：`p if cond`

### C.4 制御構文

* `if` 式：

  ```reml
  if cond then expr1 else expr2
  ```
* `match` 式（パターンマッチ）：

  ```reml
  match expr with
  | Some(x) -> x
  | None    -> 0
  ```

  * スクラティニー `expr` を**最初に評価**し、その結果を保持したまま各アームを検査する。
  * アームは**上から順に**照合され、先に一致した分岐のみが評価される。
  * ガード `| pat if cond -> ...` の `cond` は、`pat` が一致した後で束縛を共有して評価され、`cond` が偽なら次のアームへ進む（以降のアームでは再評価しない）。
  網羅性は [効果と安全性](1-3-effects-safety.md) および [エラー設計](2-5-error.md) で扱う（警告/エラー方針）。

* ループ：`while`・`for` は式として扱われ、結果は `()`（ユニット）です。`loop` は無条件ループで、`break`/`continue` は今後の拡張に備えて予約されています。

  ```reml
  while cond { work() }

  for item in items {
    total := total + item
  }
  ```

  `for` の左辺にはパターンを置けるため、構造の分解や `Some(x)` などを直接受け取れます。詳細な効果は [1.3 節](1-3-effects-safety.md) を参照してください。

### C.5 無名関数（ラムダ）

* 単行：`|x, y| x + y`
* 型注釈：`|x: i64| -> i64 { x * 2 }`
* ブロック：`|it| { let y = it + 1; y * y }`

### C.6 ブロックと束縛

```reml
{
  let x = 1
  let y = 2
  x + y          // ← ブロックの値
}
```

* 行間区切り、同一行は `;` で区切り可。
* スコープは**静的（レキシカル）**。シャドウイングは許可（ツールで警告可）。
* `var` 束縛は `名前 := 式` で再代入できます。`:=` は式としてユニット `()` を返し、副作用があるため値制限（1.3 節）に従います。
* `defer 式` は現在のブロックを抜ける際に必ず実行される遅延アクションです（リソース解放など）。複数記述すると後入れ先出しで実行されます。

### C.7 `unsafe` ブロック

* `unsafe { exprs }` は未定義動作を引き起こし得る操作（FFI 呼び出し、生ポインタ操作など）を明示的に囲む境界です。内部で発生した `ffi` や `unsafe` 効果はブロック全体に付与されます（[1.3 節](1-3-effects-safety.md)）。
* `unsafe` ブロック自体は式であり、最後の式の値を返します。属性を併用して `@pure` 等を禁止することもできます。

```reml
unsafe {
  let ptr = buf.as_ptr();
  extern_printf(ptr);
}
```

### C.8 伝播演算子 `?`

* `expr?` は `Result<T, E>` や `Option<T>` のような短絡型を対象に、失敗を即座に呼び出し側へ伝播します。成功時は中身の値を返し、失敗時は現在の式全体を早期に終了します。
* 対応する型と変換規則は [効果と安全性](1-3-effects-safety.md) で定義されます。`try` ブロックや `?` を含む関数は暗黙に同じ短絡型を返す必要があります。

```reml
fn read_config(path: String) -> Result<Config, Error> = {
  let text = read_file(path)?;
  parse_config(text)?
}
```


### C.9 評価順序と短絡規則

* **基本方針**：Reml は**正格評価**であり、式は**左から右へ**逐次的に解釈される。
* **関数呼び出し**：呼び出し式では、まず関数オブジェクト（`f` 部分）を評価し、続いて実引数を**記述順に左から右へ**評価する。名前付き引数を含む場合も、リストに書かれた順序で副作用が発生し、すべて完了してから関数本体が実行される。
* **パイプ `|>`**：`lhs |> rhs` は左オペランドを評価し、得られた値を `rhs` の**第1引数（または `_` で示された位置）に挿入した関数呼び出し**として扱う。段が連結されている場合は左から右へ逐次的に評価し、途中でエラーや短絡が起きた時点で後続段は実行されない（デシュガ規則は [3.1 BNF](3-1-bnf.md) の脚注を参照）。
* **論理演算子 `&&` / `||`**：左オペランドを評価し、`&&` では `false`、`||` では `true` の時点で右オペランドの評価を省略（短絡）する。右オペランドが評価されるのは短絡条件に当てはまらない場合のみ。
* **伝播演算子 `?`**：オペランド式を評価し、`Result` なら `Err`、`Option` なら `None` に遭遇した瞬間に現在の関数／ブロックから早期脱出する。成功ケースのときのみ後続の演算が続行される。
* **その他の二項演算子**は、左オペランドの評価が終わってから右オペランドを評価する（`a + b` なら `a` → `b`）。三項条件演算子（導入予定）も同様に条件 → 真分岐 → 偽分岐の順で評価し、不要な分岐は評価しない。

副作用を伴う式の逐次性について：

* `var` 束縛の再代入（`:=`）やミューテーション API は、その右辺を評価してから代入を適用する。
* `defer expr` は宣言された順に**スタックへ積まれ**、現在のブロックを離れる際に**後入れ先出し（LIFO）**で必ず実行される。`return` や `?` による短絡でも同様に発火する。
* `return expr` や `panic`、`?` による早期脱出は、直前までに評価済みの式の副作用がすでに反映された状態でブロックを離れる。脱出時には登録済みの `defer` がすべて走り終わってから外側へ制御が渡る。

> 副作用（`mut`/`io`/`ffi` など）の分類や禁止方法は [1.3 効果と安全性](1-3-effects-safety.md) を参照。評価順序と組み合わせて効果の発生タイミングを設計すること。


---

## D. 演算子と優先順位

### D.1 組み込み演算子の表

（高い → 低い / `assoc` は結合性）

| 優先 | 形式   | 演算子 / 構文                                                 | 結合性 | 例                         |
| ---: | ------ | ------------------------------------------------------------ | :----: | ------------------------- |
| 9 | 後置   | 関数呼び出し `(...)` / 添字 `[...]` / フィールドアクセス `.` / 伝播 `?` |  左  | `f(x)`, `arr[i]`, `value?` |
| 8 | 単項   | `!`（論理否定）, `-`（算術負）                                    |  右  | `-x`, `!ok`               |
| 7 | べき乗 | `^`                                                            |  右  | `a ^ b`                   |
| 6 | 乗除剰 | `*`, `/`, `%`                                                   |  左  | `a * b`, `a / b`          |
| 5 | 加減   | `+`, `-`                                                        |  左  | `a + b`, `a - b`          |
| 4 | 比較   | `<`, `<=`, `>`, `>=`                                            |  非結合 | `a < b`                 |
| 3 | 同値   | `==`, `!=`                                                      |  非結合 | `x == y`               |
| 2 | 論理 AND | `&&`                                                          |  左  | `p && q`                  |
| 1 | 論理 OR  | `||`                                                          |  左  | `p || q`                  |
| 0 | パイプ | `|>`                                                            |  左  | `x |> f |> g(a=1)`        |

* **関数適用（後置）** は最強優先（演算子より強い）。
* `?` は後置演算子として関数適用と同順位で評価され、短絡型の失敗を即座に伝播します（C.8 参照）。
* `^` は右結合（`2 ^ 3 ^ 2 == 2 ^ (3 ^ 2)`)。
* 比較/同値は**非結合**（連鎖不可）：`a < b < c` はエラー。
* **パイプ `|>`** は最弱：左から右へ**データフロー**を明示。

### D.2 パイプの規則

* `x |> f` は `f(x)`。
* `x |> g(a=1)` は `g(x, a=1)`（**左値は第1引数**に入る）。
* **占位 `_`** を使うと位置を指定：
  `x |> fold(init=0, f=(_ + 1))` → `fold(x, init=0, f=...)` / `x |> pow(_, 3)` → `pow(x, 3)`
  `x |> between("(", ")", _)` → 第3引数に挿入。
* **ネスト**は左結合で直列化：`a |> f |> g |> h`。

---

## E. データリテラルとアクセス

### E.1 タプル / レコード / 配列

```reml
let t  = (1, true, "s")
let p  = { x: 10, y: 20 }
let xs = [1, 2, 3]
```

* アクセス：`t.0`, `p.x`, `xs[2]`
* 末尾カンマ許可：`(a, b,)`, `{x:1, y:2,}`

### E.2 代数的データ型（ADT）

```reml
type Option<T> = | Some(T) | None
let v = Some(42)
match v with | Some(n) -> n | None -> 0
```

* コンストラクタ呼び出しは**関数適用と同形**：`Some(x)`。

---

## F. エラーを良くするための構文上の指針

* **ラベル化される構文点**：`match`, `if`, `fn`, `{`/`(`/`[` の開きに対し、パーサが「ここで **何が期待されるか**」を言語側で明確化できるよう、曖昧な省略記法は採用しない。
* **行継続規則**（B.3）により、改行起因の誤解釈を防ぐ。
* **パイプ**と\*\*占位 `_`\*\*はデシュガ可能（2.5 の期待集合にも反映）。

---

## G. 例（仕様の運用感）

```reml
use Core.Parse.{Lex, Op}

// 値と関数
let sep = ", "
fn join3(a: String, b: String, c: String) -> String =
  a + sep + b + sep + c

// ラムダとパイプ
let r = "1 2 3"
  |> split(" ")
  |> map(|s| parse_int(s))
  |> fold(init=0, f=(_ + 1))
  //           ↑ パイプ値の占位

// ADT と match
type Expr = | Int(i64) | Add(Expr, Expr) | Neg(Expr)
fn eval(e: Expr) -> i64 =
  match e with
  | Int(n)     -> n
  | Neg(x)     -> -eval(x)
  | Add(a, b)  -> eval(a) + eval(b)

// ブロックは最後の式が値
fn abs(x: i64) -> i64 {
  if x < 0 then -x else x
}
```

---

## H. 形式的な最小 EBNF（1.1 の範囲）

> 型や意味は 1.2 以降。ここでは**形だけ**。

```
CompilationUnit ::= ModuleHeader? { Attrs? (UseDecl | PubDecl) }+

ModuleHeader   ::= "module" ModulePath NL
ModulePath     ::= Ident { "." Ident }

UseDecl        ::= "use" UseTree NL
UseTree        ::= UsePath ["as" Ident]
                 | UsePath "." UseBrace
UsePath        ::= RootPath
                 | RelativePath
RootPath       ::= "::" ModulePath
RelativePath   ::= RelativeHead { "." Ident }
RelativeHead   ::= "self"
                 | SuperPath
                 | Ident
SuperPath      ::= "super" { "." "super" }
UseBrace       ::= "{" UseItem { "," UseItem } [","] "}"
UseItem        ::= Ident ["as" Ident] [ "." UseBrace ]

PubDecl        ::= ["pub"] Decl NL*
Decl           ::= ValDecl
                 | FnDecl
                 | TypeDecl
                 | TraitDecl
                 | ImplDecl
                 | ExternDecl
                 | EffectDecl
                 | HandlerDecl
                 | ConductorDecl

Attrs          ::= Attribute+
Attribute      ::= "@" Ident [AttrArgs]
AttrArgs       ::= "(" AttrArg { "," AttrArg } [","] ")"
AttrArg        ::= Expr

GenericParams  ::= "<" Ident { "," Ident } ">"
GenericArgs    ::= "<" Type { "," Type } ">"
WhereClause    ::= "where" Constraint { "," Constraint }
Constraint     ::= Ident "<" Type { "," Type } ">"

ValDecl        ::= ("let" | "var") Pattern [":" Type] "=" Expr NL
AssignStmt     ::= LValue ":=" Expr NL
DeferStmt      ::= "defer" Expr NL

FnDecl         ::= FnSignature ("=" Expr | Block)
FnSignature    ::= "fn" Ident [GenericParams] "(" Params? ")" [RetType] [WhereClause] [EffectAnnot]
Params         ::= Param { "," Param }
Param          ::= Pattern [":" Type] ["=" Expr]
RetType        ::= "->" Type
EffectAnnot    ::= "!" "{" EffectTags? "}"
EffectTags     ::= Ident { "," Ident }

TypeDecl       ::= "type" TypeDeclBody NL
TypeDeclBody   ::= "alias" Ident [GenericParams] "=" Type
                 | Ident [GenericParams] "=" SumType
                 | Ident [GenericParams] "=" "new" Type
SumType        ::= Variant { "|" Variant }
Variant        ::= Ident "(" Types? ")"
Types          ::= Type { "," Type }

TraitDecl      ::= "trait" Ident [GenericParams] [WhereClause] TraitBody
TraitBody      ::= "{" TraitItem* "}"
TraitItem      ::= Attrs? FnSignature (";" | Block)

ImplDecl       ::= "impl" [GenericParams] ImplHead [WhereClause] ImplBody
ImplHead       ::= TraitRef "for" Type | Type
TraitRef       ::= Ident [GenericArgs]
ImplBody       ::= "{" ImplItem* "}"
ImplItem       ::= Attrs? (FnDecl | ValDecl)

ExternDecl     ::= "extern" StringLiteral ExternBody
ExternBody     ::= FnSignature ";" | "{" ExternItem* "}"
ExternItem     ::= Attrs? FnSignature ";"

EffectDecl     ::= "effect" Ident ":" Ident EffectBody NL
EffectBody     ::= "{" OperationDecl+ "}"
OperationDecl  ::= Attrs? "operation" Ident ":" Type NL

HandlerDecl    ::= "handler" Ident HandlerBody NL
HandlerBody    ::= "{" HandlerEntry+ "}"
HandlerEntry   ::= "operation" Ident "(" HandlerParams? ")" HandlerBlock
                 | "return" Ident HandlerBlock
HandlerParams  ::= Param { "," Param }
HandlerBlock   ::= Block

ConductorDecl  ::= "conductor" Ident ConductorBody NL*
ConductorBody  ::= "{" NL* ConductorSection* "}"
ConductorSection ::= ConductorDslDef
                   | ConductorChannels
                   | ConductorExecution
                   | ConductorMonitoring
ConductorDslDef ::= Ident ":" Ident ["=" PipelineSpec] ConductorDslTail* NL*
ConductorDslTail ::= NL* "|>" Ident "(" ConductorArgs? ")"
ConductorArgs  ::= ConductorArg { "," ConductorArg } [","]
ConductorArg   ::= [Ident ":"] Expr
PipelineSpec   ::= Expr
ConductorChannels ::= "channels" ConductorChannelBody NL*
ConductorChannelBody ::= "{" (ChannelRoute NL)* "}"
ChannelRoute   ::= ConductorEndpoint "~>" ConductorEndpoint ":" Type
ConductorEndpoint ::= Ident { "." Ident }
ConductorExecution ::= "execution" Block NL*
ConductorMonitoring ::= "monitoring" ConductorMonitoringSpec? Block NL*
ConductorMonitoringSpec ::= "with" ModulePath
                          | ConductorEndpoint

Block          ::= Attrs? "{" BlockElems? "}"
BlockElems     ::= { Stmt StmtSep }* [Expr]
StmtSep        ::= NL | ";"

Stmt           ::= ValDecl
                 | AssignStmt
                 | DeferStmt
                 | ReturnStmt
                 | Expr

ReturnStmt     ::= "return" Expr NL
LValue         ::= PostfixExpr

Expr           ::= PipeExpr
PipeExpr       ::= OrExpr { "|>" CallExpr }
CallExpr       ::= PostfixExpr [ "(" Args? ")" ]
Args           ::= Arg { "," Arg }
Arg            ::= [Ident ":"] Expr

OrExpr         ::= AndExpr { "||" AndExpr }
AndExpr        ::= EqExpr { "&&" EqExpr }
EqExpr         ::= RelExpr { ("==" | "!=") RelExpr }
RelExpr        ::= AddExpr { ("<" | "<=" | ">" | ">=") AddExpr }
AddExpr        ::= MulExpr { ("+" | "-") MulExpr }
MulExpr        ::= PowExpr { ("*" | "/" | "%") PowExpr }
PowExpr        ::= UnaryExpr { "^" UnaryExpr }
UnaryExpr      ::= PostfixExpr
                 | ("-" | "!") UnaryExpr

PostfixExpr    ::= Primary { PostfixOp }
PostfixOp      ::= "." Ident
                 | "[" Expr "]"
                 | "(" Args? ")"
                 | "?"

Primary        ::= Literal
                 | Ident
                 | "(" Expr ")"
                 | TupleLiteral
                 | RecordLiteral
                 | ArrayLiteral
                 | Lambda
                 | IfExpr
                 | MatchExpr
                 | WhileExpr
                 | ForExpr
                 | LoopExpr
                 | UnsafeBlock
                 | Block
                 | PerformExpr
                 | DoExpr
                 | HandleExpr

TupleLiteral   ::= "(" Expr "," Expr { "," Expr } [","] ")"
RecordLiteral  ::= "{" FieldInit { "," FieldInit } [","] "}"
FieldInit      ::= Ident ":" Expr
ArrayLiteral   ::= "[" Expr { "," Expr } [","] "]"

IfExpr         ::= "if" Expr "then" Expr ["else" Expr]
MatchExpr      ::= "match" Expr "with" MatchArm { MatchArm }
MatchArm       ::= "|" Pattern "->" Expr
WhileExpr      ::= "while" Expr Block
ForExpr        ::= "for" Pattern "in" Expr Block
LoopExpr       ::= "loop" Block
UnsafeBlock    ::= Attrs? "unsafe" Block

Lambda         ::= "|" ParamList? "|" ["->" Type] LambdaBody
ParamList      ::= Param { "," Param }
LambdaBody     ::= Expr | Block

EffectPath     ::= Ident { "." Ident }
PerformExpr    ::= "perform" EffectPath "(" Args? ")"
DoExpr         ::= "do" EffectPath "(" Args? ")"
HandleExpr     ::= "handle" Expr "with" HandlerLiteral
HandlerLiteral ::= "handler" Ident HandlerBody

Pattern        ::= "_"
                 | Ident
                 | TuplePattern
                 | RecordPattern
                 | ConstructorPattern

TuplePattern   ::= "(" Pattern { "," Pattern } [","] ")"
RecordPattern  ::= "{" FieldPattern { "," FieldPattern } [","] "}"
FieldPattern   ::= Ident [":" Pattern]
ConstructorPattern ::= Ident "(" Pattern { "," Pattern } [","] ")"

Type           ::= SimpleType
                 | FnType
                 | TupleType
                 | RecordType

SimpleType     ::= Ident [GenericArgs]
FnType         ::= "(" Type { "," Type } ")" "->" Type
TupleType      ::= "(" Type { "," Type } [","] ")"
RecordType     ::= "{" FieldType { "," FieldType } [","] "}"
FieldType      ::= Ident ":" Type

Literal        ::= IntLiteral
                 | FloatLiteral
                 | StringLiteral
                 | CharLiteral
                 | "true"
                 | "false"

Ident          ::= *Unicode XID スタート + 続行を満たす識別子*
StringLiteral  ::= *UTF-8 文字列 (通常/生/複数行)*
IntLiteral     ::= *基数 / 桁区切り付き整数*
FloatLiteral   ::= *指数/小数表記を含む浮動小数*
CharLiteral    ::= *Unicode スカラ値 1 文字*
NL             ::= 行末（B.3 の規則に従う）
```

`conductor` ブロックの運用指針は本節 B.8 と [guides/conductor-pattern.md](guides/conductor-pattern.md) を参照。ブロックや `unsafe` に付与する属性の評価は B.6 に記載した規則に従う。


---

### まとめ

* **行末ベースの簡潔な文法**＋**式指向**＋**強い後置（適用/アクセス）**で、DSL/コンビネータ記述が短く素直に書けます。
* **パイプ `|>` と占位 `_`**がデシュガ可能な**一貫ルール**で、読みやすいデータフローを保証。
* **パターン・ADT・ブロック終端式**で、構文も AST も"自然に"Reml→Core→IR へ落ちます。

---

## 関連仕様

* [1.2 型と推論](1-2-types-Inference.md) - 型システムと推論規則
* [1.3 効果と安全性](1-3-effects-safety.md) - 効果システムと安全性保証
* [1.4 文字モデル](1-4-test-unicode-model.md) - Unicode処理の詳細
* [3.1 BNF文法仕様](3-1-bnf.md) - 形式的文法定義
* [2.3 字句レイヤ](2-3-lexer.md) - 字句解析の実装詳細
