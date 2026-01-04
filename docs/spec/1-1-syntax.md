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

* 識別子：`XID_Start` + (`XID_Continue` / `U+200D` / `U+FE0F` / `Extended_Pictographic` / `Emoji_Component`)*（Unicode 準拠）。
  例）`parse`, `ユーザー`, `_aux1`, `foo👨‍👩‍👧‍👦bar`。
  - **拒否規則**: bidi 制御文字（`U+200E..U+200F`, `U+202A..U+202E`, `U+2066..U+2069`）が混入した識別子は診断で拒否する。
  - **トークン分類**: Lexer は先頭文字が大文字の識別子を `UPPER_IDENT`、それ以外を `IDENT` として返す。両者とも同じ Unicode 制約を共有し、構文解析では `ident` 非終端を通じて共通的に扱う。
  - **目的**: `UPPER_IDENT` を導入することで、パターン文脈で列挙子（`Option.None` など）と変数の曖昧さを解消し、Menhir 上での縮約衝突を防ぐ。
  - **互換プロファイル**: `RunConfig.extensions["lex"].identifier_profile` で `ascii-compat` を指定すると、Phase 1 系ツールとの互換用に ASCII 限定モードへ切り替えられる。CLI／LSP／Streaming いずれのランナーでも同じ設定キーを共有し、監査ログでは `unicode.identifier_profile` で実際に使用されたプロファイルを記録する。
  - **バックエンド内部名**: LLVM IR などの内部表現では非 ASCII の識別子を `_uXXXX`（必要に応じて `_uXXXXXX`）形式へ正規化し、先頭が数字になる場合は `_` を付与する。これは内部名の変換であり、言語仕様上の識別子範囲を狭めるものではない。
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
* レコード：`{ x: 1, y: 2 }`（構文上は順序不問、レイアウトは E.1.2 に従う）

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
  なお **ルートモジュール（`module` ヘッダが `Spec.Core.*` などパッケージ直下を指す場合）では `super` を利用できません**。`super` は親モジュールを辿る相対参照であり、最上位には親が存在しないため、`use super.Core.Prelude` のような記法は `language.use.invalid_super` 診断として拒否されます。ルートからの参照が必要な場合は `use ::Core.Prelude` など明示的なルート指定を使用してください。

> 監査ノート: `examples/docs-examples/spec/1-1-syntax/use_nested.reml` が本節の正準サンプルです。Rust Frontend (2025-11-21 Streaming ランナー) は `module`/`use` の受理に加えて `fn ... { ... }` ブロックと `match` 構文も解析できるようになり、`TraceEvent::{ModuleHeaderAccepted,UseDeclAccepted}` を `reports/spec-audit/ch1/use_nested-YYYYMMDD-trace.md` へ記録します。Streaming 実行と CLI 実行の双方で診断 0 件が揃ったため、フォールバックの `use_nested_rustcap.reml` は参照用途のみとし、監査ベースラインは `reports/spec-audit/ch1/streaming_use_nested-YYYYMMDD-diagnostics.json`（`CI_RUN_ID=rust-frontend-streaming-20251121.1` など）を含む正準サンプルで固定します。

### B.1.1 DSLエントリーポイント宣言 {#dsl-entry-declaration}

`reml.toml` の `[dsl]` セクションで宣言された `entry` は、1 ファイル 1 モジュールの原則に従い、該当モジュールのトップレベル公開シンボルと一致しなければならない。`exports` 配列の各名前は、以下の要件を満たすトップレベル宣言を指す。

- 宣言は `pub` であり、コンパイラが DSL メタデータを収集できるよう **`@dsl_export` 属性** を付与する（`conductor` は `exports` 指定で公開されるため `@dsl_export` は省略可）。
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
  root_object(|builder| builder)

conductor config_orchestrator {
  config: config_dsl
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

* **トップレベル式の扱い**: 既定ではトップレベル式を許可しません。仕様サンプル検証や移行用途で必要な場合は `RunConfig.allow_top_level_expr = true` を明示し、CLI では `--allow-top-level-expr` で有効化します。

* **値束縛と再代入**  \n  `let` は不変束縛、`var` は可変束縛。`var` で導入した変数はブロック内で `:=` による再代入が可能です（C.6 および [効果と安全性](1-3-effects-safety.md) を参照）。

  ```reml
  let answer = 42
  var total = 0
  let (lhs, rhs) = pair
  let { name, version, .. } = manifest
  let Cons(head, tail) = list
  ```

  束縛パターンは `match` と同じ構文を受け付け、タプル・レコード・列挙・リストの分解や `..` による残余束縛、`_` による無視束縛を利用できる。網羅性が満たされない場合はコンパイル時に拒否されるため、初期化コードで `panic` を誘発する心配がない。【F:../examples/language-impl-samples/reml/pl0_combinator.reml†L150-L173】

* **関数宣言**  \n  本体は式かブロックで記述でき、名前付き引数・デフォルト引数・戻り値型をサポートします。`pub` を付けると公開関数になります。宣言名は `QualifiedName` を受理し、`Core.Dsl.Object.call` / `Core::Dsl::Object::call` のいずれの区切りも許容します。構文解析後は `::` 区切りに正規化して保持します。

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

  type alias Bytes = Vec<u8>
  type UserId = new { value: i64 }
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
    pub fn push(self, value: T) {
      let _ = value
    }
  }
  ```

* **外部宣言 (`extern`)**  \n  FFI で公開された関数を宣言します。呼び出しは `unsafe` 境界内で行います（1.3 節参照）。

  ```reml
  extern "C" fn puts(ptr: Ptr<u8>) -> i32;
  extern "C" {
    fn printf(fmt: Ptr<u8>, ...) -> i32;
  }
  ```

* **アクティブパターン宣言 (`pattern`)**  \n  入力を部分的・完全に分解するロジックをパターンとして公開します。

  ```reml
  pattern (|IntLit|_|)(src: String) = parse_int(src) // Option<T> を返す部分パターン
  pattern (|Normalize|)(x) = normalize(x)             // 常に成功する完全パターン
  ```

  * `(|Name|_|)` 形式は **部分パターン**。`Option<T>` を返し、`Some` でマッチ成功、`None` で次のアームへ進む。
  * `(|Name|)` 形式は **完全パターン**。`T` を返し常に成功するため、網羅性検査では「到達済み」として扱われる。
  * `Result` を返す実装は原則非推奨で、`Option` への変換を求める。副作用を伴う場合は `@pure` との整合を 1.3 節の効果規約で確認する。


### B.5 効果宣言とハンドラ構文（実験段階）

> `-Zalgebraic-effects` フラグが有効な場合に限り使用可能。安定化後に文言を更新予定。[^effects-syntax-poc-phase25]
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
  * 残余効果集合 `Σ_before` / `Σ_after` の記録と PoC 指標 ( `syntax.effect_construct_acceptance`, `effects.syntax_poison_rate`) は Phase 2-5 `EFFECT-002` の Step4 仕様に従い、詳細は [1-3 §I.2](1-3-effects-safety.md#i2-効果発生と潜在効果集合) と `docs/notes/effects/effect-system-tracking.md` を参照。[^effects-sigma-poc-phase25]

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

> 監査ノート: 効果構文の正準サンプルは `examples/docs-examples/spec/1-1-syntax/effect_handler.reml` です。Rust Frontend (2025-11-21 Streaming ランナー) は `ExprParser` と effect handler 実装に加えて `StreamFlowState` の橋渡しを検証し、`reports/spec-audit/ch1/effect_handler-YYYYMMDD-diagnostics.json` や `streaming_effect_handler-YYYYMMDD-diagnostics.json`（例: `...-20251121-diagnostics.json`）に診断 0 件のログを保存しています。`effect_handler_rustcap.reml` 等のフォールバックは完全に撤廃済みであり、監査では `effect_handler-YYYYMMDD-trace.md`/`effect_handler-YYYYMMDD-dualwrite.md` と Streaming ログを組み合わせて参照してください。PoC を再現する際は `cargo test --manifest-path compiler/frontend/Cargo.toml streaming_metrics -- --nocapture` による `bridge_signal_roundtrip` テストと `scripts/poc_dualwrite_compare.sh effect_handler` を併用します。

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
  fn map<U>(self, f: T -> U) -> Parser<U> {
    let _ = f
    self
  }
}
```

#### ネイティブ intrinsic 属性 `@intrinsic`

* `@intrinsic` は **関数宣言にのみ**付与できる属性で、LLVM intrinsic などのネイティブ実装へ直接マッピングすることを示す。
* 書式は `@intrinsic("llvm.sqrt.f64")` の **文字列リテラル 1 つ**に限定する。識別子式や補間文字列、名前付き引数（`name=...`）は許可しない。
* `extern` 宣言や `trait` / `impl` 内のメソッド宣言には付与できない。対象は `fn` 宣言に限る。
* `@cfg` との併用は可能で、並び順は問わない。`@cfg` により無効化された宣言は `@intrinsic` の検証対象外となる。
* 構文違反（引数が文字列リテラルでない、引数が複数ある、関数宣言以外に付与など）は `native.intrinsic.invalid_syntax` として報告する。

```reml
@intrinsic("llvm.sqrt.f64")
fn sqrt_f64(x: f64) -> f64 = x

@cfg(target_arch = "aarch64")
@intrinsic("llvm.ctpop.i64")
fn popcount(x: i64) -> i64 = x
```

#### 不安定機能属性 `@unstable`

* `@unstable` は **関数宣言または `unsafe` ブロック** に付与できる属性で、Inline ASM / LLVM IR 直書きの実験機能を明示します。
* 書式は `@unstable("inline_asm")` または `@unstable("llvm_ir")` の **文字列リテラル 1 つ**に限定する。識別子式や補間文字列、名前付き引数（`kind=...`）は許可しない。
* Inline ASM / LLVM IR を含むブロックや関数では `@cfg(target_...)` を併用し、対象ターゲットを明示する。無効化された要素は `@unstable` の検証対象外とする。
* フロントエンドは `@unstable("inline_asm")` を `unstable:inline_asm`、`@unstable("llvm_ir")` を `unstable:llvm_ir` の内部属性に変換し、後続フェーズへ渡す。
* `feature = "native-unstable"` が無効な場合、バックエンドは `native.unstable.disabled` を報告する。

```reml
@cfg(target_arch = "x86_64")
@unstable("inline_asm")
fn read_cycle_counter() -> i64 {
  unsafe {
    inline_asm("rdtsc")
  }
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

`package` 宣言や `use plugin` ブロックといったプラグイン配布専用のメタデータ構文は、Reml コアから切り離しました。バージョン管理や Capability 指定は `reml-plugin` CLI と外部マニフェストで扱い、言語仕様としては通常の `use` とモジュールシステムのみを定義します。API 契約は [5-7-core-parse-plugin.md](5-7-core-parse-plugin.md) を参照し、運用面のベストプラクティスは `../guides/dsl/DSL-plugin.md` を参照してください。


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
- **埋め込み DSL 契約**: `conductor` に登録する埋め込み DSL は `dsl_id` を持ち、Capability/効果/スコープ引き継ぎの契約を明示する。親子 DSL の境界は診断情報に記録し、`Diagnostic.source_dsl` で発生源を追跡できるようにする。
- **並列安全性フラグ**: 埋め込み DSL が独立区間で並列解析可能かを `EmbeddedMode::ParallelSafe` のようなフラグで宣言し、`execution` の並列戦略と整合させる。並列不可の DSL は順序保証を優先する。
- **関連ノート**: 埋め込み DSL の標準化方針は [dsl-enhancement-proposal.md](../notes/dsl/dsl-enhancement-proposal.md) の 3.6 を参照。

#### B.8.3.1 埋め込み DSL の最小契約（草案）

```reml
let embedded = embedded_dsl(
  dsl_id = "reml",
  start = "```reml",
  end = "```",
  parser = Reml.Parser.main,
  lsp = Reml.Lsp.server,
  mode = EmbeddedMode::ParallelSafe,
  context = ContextBridge::inherit(["scope", "type_env"])
)
```

- `dsl_id` は必須であり、`Diagnostic.source_dsl` と `AuditEnvelope.metadata["dsl.id"]` に同一の値を記録する。
- `start`/`end` は境界トークンとして扱い、境界内で発生した診断は親 DSL の診断と混在しないよう `source_dsl` で分離する。
- `EmbeddedMode::ParallelSafe` が指定された場合、`execution` の並列戦略に反映され、監査ログに `dsl.embedding.mode` を記録する。
- `ContextBridge::inherit` は `scope`/`type_env`/`config` のうち指定された要素のみを継承する。未指定の値は親 DSL へ逆流しない。
- `dsl.embedding.span` は `Span` の JSON 形式 `{ "start": Int, "end": Int }` で監査ログへ出力する。

埋め込み DSL の実行契約と診断キーは [2-2 Core Combinator](2-2-core-combinator.md) と [3-6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md) の草案節に合わせて更新する。

#### B.8.3.2 `with_embedded` の合成契約（草案）

```reml
conductor docs_pipeline {
  markdown: markdown_parser
    |> with_embedded([embedded])
    |> with_capabilities(["core.parse", "core.lsp"])
}
```

- `with_embedded` は `embedded_dsl` の配列を受け取り、親 DSL のパース/実行コンテキストに境界情報を登録する。
- `with_embedded` 経由で登録された埋め込み DSL は `dsl_id` を必須とし、境界内診断に `source_dsl` を付与する。
- 親 DSL の診断は `source_dsl = None` を維持し、親/子 DSL の診断が混在しないことを保証する。
- `dsl_id` は `conductor` 内で一意でなければならず、重複した場合は `conductor.dsl_id.duplicate` 診断を発行する。

### B.8.4 テンプレート DSL 安全設計指針

- **Core.Text.Template の活用**: テンプレート構文を DSL として公開する際は、`Core.Text.Template` の `TemplateSegment`/`TemplateFilter` を利用し、レンダリング面の機能を標準APIに委譲する。これにより Unicode 幅計算・正規化・ストリーム処理の最適化が既定で有効になり、0-1章で定義した性能基準（10MB 線形処理など）を満たす経路を保持できる。[^purpose-perf]
- **安全なフィルター登録**: HTML エスケープやシリアライザなど副作用を伴うフィルターは `TemplateFilterRegistry.register_secure` を通じて登録し、Capability Registry (`Core.Runtime.Capability`) と診断モジュール (`Core.Diagnostics`) を連携させる。`Result` で失敗を返し、未登録フィルターや署名検証失敗時は `Diagnostic` を即時生成する。[^purpose-safe]
- **効果と Capability の両立**: テンプレート実行で `effect {io, runtime, security}` を要求する場合は `@dsl_export` の残余効果と照合し、`conductor` で `with_capabilities(TemplateCapability::RenderHtml)` のように明示する。Capability の欠落はステージング時に `template.capability.missing` 診断を発生させ、監査ログへ転送する。
- **テストと監査**: テンプレート DSL を導入したプロジェクトでは、`Core.Diagnostics.Audit` の `record_dsl_failure` を利用してレンダリング失敗・エスケープ逸脱・フィルター例外を監査ストアに記録し、CI や LSP での可観測性を確保する。`../guides/plugin-authoring.md` のテンプレート拡張例と合わせ、エラー再現手順とロールバック方針を共有する。

[^purpose-perf]: [0-1-project-purpose.md](0-1-project-purpose.md) §1.1「実用に耐える性能」を参照。テンプレートレンダリングでも線形時間処理とメモリ制約を満たすことを目標とする。
[^purpose-safe]: [0-1-project-purpose.md](0-1-project-purpose.md) §1.2「安全性の確保」を参照。`Result` ベースのエラー処理と Capability 検証により、未ハンドル例外や権限逸脱を防止する。

[^effects-syntax-poc-phase25]:
    Phase 2-5 `SYNTAX-003 S0` の整理として、効果構文は `-Zalgebraic-effects` フラグを有効化した PoC 提供に限定される。正式実装は Phase 2-7 以降で `parser.mly`・型推論・効果解析を統合予定。Stage 契約やロードマップの詳細は `docs/plans/bootstrap-roadmap/2-5-proposals/SYNTAX-003-proposal.md` と `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` の `SYNTAX-003` 項を参照。

[^effects-sigma-poc-phase25]:
    Phase 2-5 `EFFECT-002 Step4`（2026-04-18 完了）で効果構文 PoC の `Σ` 記録フォーマットと CI 指標 (`syntax.effect_construct_acceptance`, `effects.syntax_poison_rate`) を定義し、`docs/notes/effects/effect-system-tracking.md` および `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` にハンドオーバー条件を整理した。正式実装は Phase 2-7 で Stage 昇格後に反映する。

### B.8.5 Capability 検証契約と `with_capabilities`

`with_capabilities` はコンパイル時に **Capability 契約** を生成し、`CapabilityRegistry::verify_capability_stage`（3-8 §1.2）へ渡すメタデータを蓄積する。契約は次の項目で構成される。

```reml
type ConductorCapabilityRequirement = {
  id: CapabilityId,
  stage: StageRequirement,        // 既定は StageRequirement::AtLeast(StageId::Stable)
  declared_effects: Set<EffectTag>,
  source_span: SourceSpan,
}

type StageRequirement =
  | Exact(StageId)
  | AtLeast(StageId)
```

`StageId` は 3-8 §1.2 で定義された効果ステージ列挙を参照する。

契約生成と検証は以下の順序で行われる。

1. DSL パーサは `with_capabilities(["io.async", ...])` の呼び出しを検知し、指定文字列を `CapabilityId` として登録する。`StageRequirement` を明示しない場合は `AtLeast(StageId::Stable)` を採用し、`@requires_capability(stage="experimental")` などの属性が同一チャネル内に存在する場合は `AtLeast(StageId::Experimental)` に引き上げる。
2. `@cfg(capability = "...")` が付与されたブロックやチャネルは、解析段階で `CapabilityId` を抽出し、`with_capabilities` に未登録であれば構文エラーとする。これにより 0-1-project-purpose.md §1.2 の安全性指針に従い、実行環境差による挙動分岐を未然に防ぐ。
3. `conductor` 宣言の型検査フェーズでは、全チャネルの残余効果集合と `declared_effects` を比較する。差分が存在する場合は `effects.contract.mismatch` 標準診断を発行し、契約が満たされるまでビルドを中断する。
4. 上記で確定した `ConductorCapabilityRequirement` の集合を `CapabilityRegistry::verify_conductor_contract`（3-8 §1.2）へ連携する。CLI (`reml lint`, `reml build`) と IDE/LSP は同一契約を利用し、Capability ステージが `StageRequirement` を満たさない場合はビルドエラーとして報告する。

`with_capabilities` を複数回呼び出した場合、要求集合は和集合で解釈される。同一 `CapabilityId` に異なる Stage 要件が提示された場合は最も厳しい（高い）要件を採用し、重複エントリは自動的に正規化される。これらの挙動は `ExecutionPlan` の静的検証（3-9 §1.4.3）と共有され、CLI の `--deny-capability` オプションや CI ポリシーと一致させる。


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
fn increment_all(xs) {
  pipe(xs)
    |> map(|value| value + 1)
}
  ```

  `_` は左側パイプ値の**代入位置**（D.3 に詳細）。

### C.3 パターン（束縛・`match` で共通）

* 変数：`x`
* ワイルドカード：`_`
* タプル：`(x, y, _)`
* レコード：`{ x, y: y0 }`（`x: x` は `x` に省略可）
* 代数型：`Some(x)`, `Add(Int(a), b)`
  - **モジュール修飾列挙子**: コンストラクタは `Option.None` や `DSL.Node(tag)` のように `.` 区切りで修飾できる。`Option.None` の末尾 `None`（先頭大文字）が列挙子とみなされ、前置の `Option` はモジュール／型名として扱われる。
* Or パターン：`Some(A | B)`（左結合。網羅性診断は Or 全体で判定し、到達不能は `pattern.unreachable_arm` を使用）
* スライスパターン：`[head, ..tail]`, `[first, .., last]`（カンマ区切りで複数要素を記述し、`..` は 1 回のみ。対象型がコレクションでない場合は `pattern.slice.type_mismatch`、`..` 重複時は `pattern.slice.multiple_rest`）
* 範囲パターン：`1..=10`, `'a'..'z'`（下限・上限はいずれも省略可で、`..` 単体はワイルドカード扱い。型が比較不可能な場合は `pattern.range.type_mismatch`、数値リテラルで逆転している場合は `pattern.range.bound_inverted`）
  - `..=` は字句上 `..` と `=` に分割して解釈する（`..=` を単一トークンとしては扱わない）。
* バインディング：`pat as name`（推奨）／`name @ pat`（エイリアス糖衣）。`when` ガードと併用可。同じ識別子を重複束縛した場合は `pattern.binding.duplicate_name` を報告する（例: `examples/spec_core/chapter1/match_expr/bnf-match-binding-duplicate.reml`）。
* 正規表現パターン：`r"^\\d+$" as digits`（文字列/バイト列対象。全体一致に限定。対象が文字列系でない場合は `pattern.regex.unsupported_target`、リテラル構文が無効な場合は `pattern.regex.invalid_syntax`。サンプルは `examples/spec_core/chapter1/match_expr/bnf-match-regex-ok.reml` / `bnf-match-regex-unsupported-target.reml` を参照）
* ガード：`p when cond`（`if` は互換用に受理するが警告対象）
* アクティブパターン：`(|Name|_|)` / `(|Name|)` で定義した分解ロジックをパターンとして使用  
  - **定義**: `pattern (|Name|_|)(args) = expr` は部分パターンとして `Option<T>` を返し、`Some` ならマッチ成功、`None` なら次のアームへフォールスルー。`pattern (|Name|)(args) = expr` は常に成功する完全パターンで、戻り値 `T` を束縛する。  
  - **使用**: `match input with | (|Hex|_|) n -> ... | (|Total|) v -> ...` のように他のパターンと同列で使用でき、`when` ガードや `as`/`@` と併用可。  
  - **副作用/診断**: `@pure` 文脈で副作用を持つ場合は `pattern.active.effect_violation`、戻り値が契約外（`Option` 以外の部分パターンや `Result`）の場合は `pattern.active.return_contract_invalid` を報告する（[2-5 エラー設計](2-5-error.md)）。
  - **診断**: パターン内で同じ識別子を `as`/`@` で重複束縛した場合は `pattern.binding.duplicate_name` を発行する。正規表現パターンを文字列/バイト列以外へ適用した場合は `pattern.regex.unsupported_target` を報告する。

### C.4 制御構文

* `if` 式：

  ```reml
if cond then expr1 else expr2
  ```
* `match` 式（パターンマッチ）：

  ```reml
let value =
  match opt with
  | Some(x) -> x
  | None    -> 0

// リテラルパターンの使用例
let message =
  match status_code with
  | 0   -> "success"
  | 404 -> "not found"
  | 500 -> "server error"
  | _   -> "unknown"

message
  ```

  * スクラティニー `expr` を**最初に評価**し、その結果を保持したまま各アームを検査する。
  * アームは**上から順に**照合され、先に一致した分岐のみが評価される。
  * パターンには、ワイルドカード `_`、リテラル（整数、文字列、真偽値など）、変数、タプル、レコード、コンストラクタに加え、Or/スライス/範囲/バインディング/正規表現/アクティブパターンを使用できる。
  * ガード `| pat when cond -> ...` の `cond` は、`pat` が一致した後で束縛を共有して評価され、`cond` が偽なら次のアームへ進む（以降のアームでは再評価しない）。`if` ガードは互換目的で受理するが、正規形は `when`。ガードと `as`/`@` エイリアスの記述順は順不同で受理し、AST では `guard -> alias` の順に正規化する。
  * アクティブパターン `(|Name|_|) value` は `Option` を返す関数を介して分解し、`Some` のときに成功する。`(|Name|) value` は常に成功する完全パターンとして扱われる。
  網羅性は [効果と安全性](1-3-effects-safety.md) および [エラー設計](2-5-error.md) で扱う（警告/エラー方針）。

  ```reml
// 複雑なパターンは段階的に分割して扱う
match input with
| Some(value) -> handle_ab(value)
| None -> "other"
  ```

  重複束縛を避けたバインディング例：

  ```reml
match value with
| x @ Some(inner) -> inner
  ```

* ループ：`while`・`for` は式として扱われ、結果は `()`（ユニット）です。`loop` は無条件ループで、`break`/`continue` は今後の拡張に備えて予約されています。

  ```reml
while cond { work() }

var total = 0
for item in items {
  total := total + item
}

total
  ```

  `for` の左辺にはパターンを置けるため、構造の分解や `Some(x)` などを直接受け取れます。詳細な効果は [1.3 節](1-3-effects-safety.md) を参照してください。

### C.5 無名関数（ラムダ）

* 単行：`|x, y| x + y`
* 型注釈：`|x: i64| -> i64 { x * 2 }`
* ブロック：`|it| { let y = it + 1; y * y }`

### C.6 ブロックと束縛

```reml
let total = {
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
fn write_buf(buf) -> () {
  unsafe {
    let _ = buf
  }
}
```

### C.7.1 ネイティブエスケープハッチ（Inline ASM / LLVM IR）

`inline_asm` と `llvm_ir!` はネイティブエスケープハッチとして **`unsafe` ブロック内**でのみ使用でき、`@unstable("inline_asm")` / `@unstable("llvm_ir")` を伴う必要がある。加えて `@cfg(target_...)` によるターゲット限定が必須となる（効果契約は [1.3 §F.1](1-3-effects-safety.md#f1-effect-native-の意味と境界) を参照）。

**Inline ASM**

```reml
unsafe {
  inline_asm(
    "rdtsc",
    outputs("=a": lo, "=d": hi),
    clobbers("rcx", "r11"),
    options("volatile")
  )
}
```

* 先頭引数は **テンプレート文字列**で、後続に `outputs` / `inputs` / `clobbers` / `options` を任意順で列挙する。
* `outputs("<constraint>": <lvalue>)` は出力先を表し、`inputs("<constraint>": <expr>)` は入力値を表す。
* `clobbers` / `options` は LLVM の規約に準じる文字列リストで、詳細は `docs/guides/compiler/llvm-integration-notes.md` を参照する。

**LLVM IR 直書き**

```reml
unsafe {
  let sum = llvm_ir!(i32) {
    "%0 = add nsw i32 $0, $1",
    inputs(a, b)
  }
  sum
}
```

* `llvm_ir!(Type) { "template", inputs(...) }` の形式を取り、`$0` / `$1` は `inputs` の順序に対応する。
* テンプレートは 1 つの文字列リテラルで表し、`inputs` は式の並びで指定する。

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
* **パイプ `|>`**：`lhs |> rhs` は左オペランドを評価し、得られた値を `rhs` の**第1引数（または `_` で示された位置）に挿入した関数呼び出し**として扱う。段が連結されている場合は左から右へ逐次的に評価し、途中でエラーや短絡が起きた時点で後続段は実行されない（デシュガ規則は [1.5 形式文法](1-5-formal-grammar-bnf.md) の脚注を参照）。
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
  `x |> fold(0, |acc| acc + 1)` → `fold(x, 0, |acc| acc + 1)` / `x |> pow(_, 3)` → `pow(x, 3)`
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
* レコードフィールドは `key: expr` と `key = expr` を等価に扱い、`{ x, y = rhs }` のように値を省略した場合は `x: x` へデシュガされる（punning）。評価順はソース順、格納順は E.1.2 の正規化順に従う。

#### E.1.1 MIR/JSON のリテラル表現（Frontend 出力）

フロントエンドが出力する MIR/JSON では、`MirExpr.kind = "literal"` の直下に `Literal` が入り、`Literal` は `value` フィールドで `LiteralKind` を包む。`LiteralKind` は `kind` を持つ内部タグ形式で直列化する。

```json
{
  "kind": "literal",
  "value": {
    "value": {
      "kind": "int",
      "value": 1,
      "raw": "1",
      "base": "base10"
    }
  }
}
```

`LiteralKind` の JSON 形状（主要リテラル）は以下の通り。

- Int: `{ "kind": "int", "value": i64, "raw": "1_000", "base": "base10|base2|base8|base16" }`
- Float: `{ "kind": "float", "raw": "3.14" }`
- Char: `{ "kind": "char", "value": "A" }`（1 文字の `String`）
- String: `{ "kind": "string", "value": "...", "string_kind": "normal|raw|multiline" }`
- Bool: `{ "kind": "bool", "value": true }`
- Unit: `{ "kind": "unit" }`
- Tuple: `{ "kind": "tuple", "elements": [Expr, ...] }`
- Array: `{ "kind": "array", "elements": [Expr, ...] }`
- Record: `{ "kind": "record", "type_name": Ident?, "fields": [ { "key": Ident, "value": Expr }, ... ] }`
  - `Ident` は `{ "name": String, "span": Span }` の形で直列化される。

#### E.1.2 リテラル実行時 ABI（Backend/Runtime）

Backend はリテラル値を Runtime API 経由でヒープオブジェクト化し、`REML_TAG_*` によって型識別を行う。タグ値と最小 ABI は `compiler/runtime/native/include/reml_runtime.h` に準拠する。

| 型 | タグ | 備考 |
| --- | --- | --- |
| Int | `REML_TAG_INT` | 即値はボックス化して扱う |
| Float | `REML_TAG_FLOAT` | `reml_box_float` / `reml_unbox_float` |
| Bool | `REML_TAG_BOOL` | 即値はボックス化して扱う |
| String | `REML_TAG_STRING` | `reml_string_t` を使用 |
| Tuple | `REML_TAG_TUPLE` | `reml_tuple_t` |
| Record | `REML_TAG_RECORD` | `reml_record_t` |
| Char | `REML_TAG_CHAR` | `reml_char_t`（Unicode scalar） |
| Array | `REML_TAG_ARRAY` | `reml_array_t` |

```c
typedef uint32_t reml_char_t; // Unicode scalar value (U+0000..U+10FFFF)

typedef struct {
    int64_t len;
    void** items;
} reml_tuple_t;

typedef struct {
    int64_t field_count;
    void** values;
} reml_record_t;

typedef struct {
    int64_t len;
    void** items;
} reml_array_t;
```

- `items` / `values` は `void*` 配列へのポインタで、各スロットは RC 対象のヒープポインタを保持する。非ポインタ値はボックス化して格納する。
- 配列バッファは `malloc/calloc` 相当で確保し、`reml_destroy_tuple` / `reml_destroy_record` / `reml_destroy_array` が `dec_ref` と合わせて解放する。
- Char は Unicode scalar value を `reml_char_t` で表現する（UTF-8 文字列ではない）。
- Record の `values` 配列順序は **フィールド名の正規化順（Unicode スカラ値の昇順）**で固定する。ロケールやケース折り畳みは行わず、識別子の表記そのものを比較する。
- レコードリテラルのフィールド式は **ソース順で評価**し、**格納順序のみ**正規化順へ整列する。
- フィールド名は Runtime に保持せず、`record.x` の解決はコンパイル時に正規化順インデックスへ変換する。
- 同名フィールドの重複は許可しない。

**例（ソース順とレイアウト順）**

```reml
let r = { b: 1, a: 2 }
```

`values` は `{ a, b }` の順序（`a` → `b`）で格納される。

**診断**

- `type.record.literal.duplicate_field`: 同名フィールド重複。
- `type.record.literal.missing_field`: 型注釈に存在するがリテラルに欠けるフィールド。
- `type.record.literal.unknown_field`: リテラルに存在するが注釈型に存在しないフィールド。
- `type.record.access.unknown_field`: 存在しないフィールドへのアクセス。

#### E.1.3 Array リテラルの意味論（型付けの概要）

Array リテラルは **既定で `[T]`（動的配列）** として型付けされる。**期待型**または**明示注釈**が `[T; N]` の場合のみ固定長配列として型付けし、`[T; N]` と `[T]` の **暗黙変換は行わない**。詳細な推論ルールと診断は [1.2 型と推論](1-2-types-Inference.md) を参照。

```reml
let xs = [1, 2, 3]           // 既定は [i64]
let ys: [i64; 3] = [1, 2, 3]  // 注釈がある場合は固定長
let zs: [String] = []        // 空配列は注釈または期待型が必須
```

### E.2 代数的データ型（ADT）

```reml
type Option<T> = | Some(T) | None
let v = Some(42)
match v with
| Some(n) -> n
| None -> 0
```

* コンストラクタ呼び出しは**関数適用と同形**：`Some(x)`。
* レコード型ペイロードは **レコードリテラルを引数に渡す** 形で構築する：
  `Named({ name = "Ada", age = 36 })`。`match` パターンも同様に
  `Named({ name, age })` のように記述する。

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
  |> fold(0, |acc| acc + 1)
  //           ↑ パイプ値の占位

// ADT と match
type Expr = | Int(i64) | Add(Expr, Expr) | Neg(Expr)
fn negate(x: i64) -> i64 = x
fn eval(e: Expr) -> i64 =
  match e with
  | Int(n)     -> n
  | Neg(x)     -> negate(eval(x))
  | Add(a, b)  -> eval(a) + eval(b)

// ブロックは最後の式が値
fn abs(x: i64) -> i64 {
  if x < 0 then negate(x) else x
}
```

---

## H. 形式的な最小 EBNF（1.1 の範囲）

> 型や意味は 1.2 以降。ここでは**形だけ**。

```
CompilationUnit ::= ModuleHeader? { Attrs? (UseDecl | PubDecl) }+

ModuleHeader   ::= "module" ModulePath NL
ModulePath     ::= Ident { "." Ident }
QualifiedName  ::= Ident { ("." | "::") Ident }

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
FnSignature    ::= "fn" QualifiedName [GenericParams] "(" Params? ")" [RetType] [WhereClause] [EffectAnnot]
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
                 | InlineAsmExpr
                 | LlvmIrExpr
                 | Block
                 | PerformExpr
                 | DoExpr
                 | HandleExpr

TupleLiteral   ::= "(" Expr "," Expr { "," Expr } [","] ")"
RecordLiteral  ::= "{" FieldInit { "," FieldInit } [","] "}"
FieldInit      ::= Ident [ (":" | "=") Expr ]
                // `key = expr` は `key: expr` と同義、`key` 単独は `key: key` にデシュガ
ArrayLiteral   ::= "[" Expr { "," Expr } [","] "]"

IfExpr         ::= "if" Expr "then" Expr ["else" Expr]
MatchExpr      ::= "match" Expr "with" MatchArm { MatchArm }
MatchArm       ::= "|" Pattern MatchGuard? MatchAlias? "->" Expr
MatchGuard     ::= "when" Expr
MatchAlias     ::= "as" Ident
WhileExpr      ::= "while" Expr Block
ForExpr        ::= "for" Pattern "in" Expr Block
LoopExpr       ::= "loop" Block
UnsafeBlock    ::= Attrs? "unsafe" Block
InlineAsmExpr  ::= "inline_asm" "(" StringLiteral InlineAsmTail? ")"
InlineAsmTail  ::= "," InlineAsmArg { "," InlineAsmArg } [","]
InlineAsmArg   ::= InlineAsmOutputs
                 | InlineAsmInputs
                 | InlineAsmClobbers
                 | InlineAsmOptions
InlineAsmOutputs ::= "outputs" "(" InlineAsmOutputList? ")"
InlineAsmInputs  ::= "inputs" "(" InlineAsmInputList? ")"
InlineAsmOutputList ::= InlineAsmOutput { "," InlineAsmOutput } [","]
InlineAsmInputList  ::= InlineAsmInput { "," InlineAsmInput } [","]
InlineAsmOutput ::= StringLiteral ":" LValue
InlineAsmInput  ::= StringLiteral ":" Expr
InlineAsmClobbers ::= "clobbers" "(" StringLiteral { "," StringLiteral } [","] ")"
InlineAsmOptions  ::= "options" "(" StringLiteral { "," StringLiteral } [","] ")"

LlvmIrExpr     ::= "llvm_ir!" "(" Type ")" LlvmIrBlock
LlvmIrBlock    ::= "{" StringLiteral LlvmIrTail? "}"
LlvmIrTail     ::= "," LlvmIrInputs
LlvmIrInputs   ::= "inputs" "(" LlvmIrInputList? ")"
LlvmIrInputList ::= Expr { "," Expr } [","]

Lambda         ::= "|" ParamList? "|" ["->" Type] LambdaBody
ParamList      ::= Param { "," Param }
LambdaBody     ::= Expr | Block

EffectPath     ::= Ident { "." Ident }
PerformExpr    ::= "perform" EffectPath "(" Args? ")"
DoExpr         ::= "do" EffectPath "(" Args? ")"
HandleExpr     ::= "handle" Expr "with" HandlerLiteral
HandlerLiteral ::= "handler" Ident HandlerBody

Pattern        ::= "_"
                 | Literal
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

`conductor` ブロックの運用指針は本節 B.8 と [guides/conductor-pattern.md](../guides/dsl/conductor-pattern.md) を参照。ブロックや `unsafe` に付与する属性の評価は B.6 に記載した規則に従う。


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
* [1.5 形式文法](1-5-formal-grammar-bnf.md) - 形式的文法定義
* [2.3 字句レイヤ](2-3-lexer.md) - 字句解析の実装詳細
