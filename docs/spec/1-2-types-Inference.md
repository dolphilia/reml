# 1.2 型と推論（Types & Type Inference）— Reml (Readable & Expressive Meta Language) 言語コア仕様

> 目的：**書きやすさ・読みやすさ・高品質エラー**を壊さず、**実用性能**と**静的安全**を両立。
> 方針：**サブタイピングなし**（HM 系の推論をシンプルに保つ）。**ランク1の多相**を基本に、**型クラス風トレイト**で演算子等の静的オーバーロードを提供。

---

## A. 型の群（Type Language）

### A.1 プリミティブ

* 整数：`i8 i16 i32 i64 isize` / `u8 u16 u32 u64 usize`
* 浮動小数：`f32 f64`
* 真偽：`Bool`
* 文字：`Char`（Unicode スカラ値）
* 文字列：`String`（不変・UTF-8）
* 単位：`()`

### A.2 合成

* タプル：`(T1, T2, …, Tn)`（`n≥0`。`()` は単位）
* 配列（固定長）：`[T; N]`
* スライス／動的配列：`[T]`（標準ライブラリ型として提供、実体は `{ptr,len}`）
* レコード：`{ x: T1, y: T2, ... }`（構造的等値）
* 関数：`(A1, A2, …, An) -> R`（右結合、`A -> B -> C` ≡ `A -> (B -> C)`）
* 代数的データ型（ADT）：

  ```reml
  type Option<T> = | Some(T) | None
  type Result<T,E> = | Ok(T) | Err(E)
  ```

  *各コンストラクタは関数型を持つ：`Some : T -> Option<T>`*

### A.3 型変数・スキーム

* 型変数：小文字開始（`a, b, t1 …`）。
* **型スキーム**：`∀a1 … an. τ`（実装上は `Scheme{quantified: [a…], body: τ}`）。
* **多相はランク1**が既定（関数引数にスキームを直接入れない）。高ランクは将来拡張（明示注釈時のみ）。

### A.4 型エイリアス & ニュータイプ

* **エイリアス**（同義）：`type alias Id = i64`
* **ニュータイプ**（零コストの別名型）：`type UserId = new i64`（暗黙変換なし）

### A.5 種（Kind）（必要最小）

* `*`（具体型）／`* -> *`（型コンストラクタ）等。ADT 定義で内的に整合性を検査（ユーザ記述は不要）。

### A.6 数値演算の挙動

* **整数演算（`+ - * / % << >>`）** は **ビルドモードに応じた安全策**を持つ。
  * **デバッグビルド**：各演算前に範囲検査を行い、**オーバーフローや 0 除算を検知した時点で `panic`（トラップ）**する。診断は 2.5 節 D-8 のテンプレートを使用。
  * **リリースビルド**：`+ - *` とビットシフトは **モジュロ演算（2 の冪によるラップ）**で実行し、`/ %` の 0 除算のみデバッグ時と同じく即時 `panic`。
  * 明示的に飽和やラップを選ぶ場合は、標準ライブラリの `Int.{checked_,saturating_,wrapping_}op` 系 API を使用する。
* **浮動小数演算（`f32`/`f64`）** は **IEEE 754:2019 準拠**。
  * 既定の丸めは **“最近接・同距離は偶数”** (`roundTiesToEven`)。
  * `NaN` は静的に伝播し、シグナル/クワイエットの区別は保持しない（すべてクワイエット化）。
  * `+0`/`-0`、`±∞` は保持される。例：`1.0 / +0.0 = +∞`、`1.0 / -0.0 = -∞`。
  * 例外フラグは公開しない（`RunConfig` での切替も現状なし）。
* **`as` キャスト** は下表の規則に従ってランタイム変換を行う。表にない組合せは型検査段階で拒否される。

| ソース | ターゲット | 許可 | 丸め・拡張 | 失敗時動作 |
| --- | --- | --- | --- | --- |
| `iN` / `uN` | 同符号でビット幅拡大 (`N ≤ M`) の整数 | 許可 | 符号/ゼロ拡張で値保持 | 失敗なし |
| `iN` / `uN` | 同符号でビット幅縮小 (`N > M`) の整数 | 許可 | 丸めなし。事前に範囲検査 | 範囲外は `panic(E7101)` |
| `iN` | `uM` | 許可 | 負数禁止。幅縮小時は範囲検査 | 負値または範囲外は `panic(E7101)` |
| `uN` | `iM` | 許可 | 幅縮小時は範囲検査 | 範囲外は `panic(E7101)` |
| `iN` / `uN` | `f32` / `f64` | 許可 | IEEE 754 丸め（最近接・同距離偶数） | 失敗なし（巨大値は `±∞` に飽和） |
| `f32` / `f64` | `f32` / `f64`（狭い側へ） | 許可 | IEEE 754 丸め（最近接・同距離偶数） | `NaN`/`±∞` はそのまま、正規範囲外は `±∞` または `±0` |
| `f32` / `f64` | 整数 (`iN`/`uN`) | 許可（有限値のみ） | 0 方向へ丸め | `NaN`/`±∞`/範囲外は `panic(E7102)` |
| `Bool` | 整数 / 浮動小数 | 許可 | `false→0/0.0`, `true→1/1.0` | 失敗なし |
| 整数 / 浮動小数 | `Bool` | 許可 | `0`/`0.0` は `false`、それ以外は `true` | 失敗なし |
| `Char` | `u32` / `i32` | 許可 | Unicode スカラ値を数値化 | 失敗なし |
| 整数 | `Char` | 許可（Unicode スカラ値範囲内） | 値をコードポイントへ変換 | 範囲外/サロゲートは `panic(E7103)` |

> `panic(E710x)` の診断整形は [2.5 節](2-5-error.md#d-代表エラーの専用処理品質を上げる定形) を参照。`RunConfig.extensions["type"].numeric_defaults` により `i64`/`f64` 以外の既定型へ切り替えると、リテラル解決や診断メッセージの既定表示が変化する。

---

## B. トレイト（型クラス風）と静的オーバーロード

> **実装ステージング**: MVP（最小実装）では基本演算子のトレイトのみ、本格実装でユーザ定義トレイト、完全実装で辞書パッシングによる完全なtypeclass相当機能

### B.1 トレイト宣言（概略）

```reml
trait Add<A, B, R> { fn add(a: A, b: B) -> R }
impl Add<i64, i64, i64> for i64 { fn add(a,b) = a + b }
```

* **目的**：演算子・汎用 API の静的解決（Haskell の typeclass に近い）。
* **演算子**はトレイトに紐づく：`+` は `Add`、`-` は `Sub`、`*` は `Mul`、`/` は `Div`…（Core.Parse.Op に合わせて標準定義）。
* **MVP（最小実装）**: 基本算術・比較演算子の組み込みトレイトのみ（i64, f64, Bool, String対応）
* **本格実装**: ユーザ定義トレイト、where制約、制約解決
* **完全実装**: 辞書パッシング、高階型クラス、特殊化

### B.2 解決と整合性

* **コヒーレンス**：`impl` は **トレイト定義モジュール**か**対象型のモジュール**のどちらかにのみ書ける（孤児規則で衝突防止）。
* **オーバーラップ禁止**（デフォルト）。将来 `where` 制約付きの安全な特殊化を検討。

### B.3 トレイト制約の表記

* 関数型に **制約**を付与：

  ```reml
  fn sum<T>(xs: [T]) -> T where Add<T,T,T>, Zero<T> = ...
  ```

  *推論中は**制約集合**として保持され、呼出側で解決／辞書渡しに具体化。*

---

### B.4 型クラス辞書と Stage 監査連携

- `Iterator` などのイテレーション系トレイトは、辞書生成時に **Stage 要件**と **Capability ID** をメタデータとして保持する。`solve_iterator` は `effect.stage.iterator.required` / `effect.stage.iterator.actual` / `effect.stage.iterator.capability` / `effect.stage.iterator.source` を含む辞書情報を返し、Core IR の `DictMethodCall` に `effect.stage.*` 拡張を付与する。【F:3-1-core-prelude-iteration.md†L160-L200】【F:3-8-core-runtime-capability.md†L210-L260】
- 型推論フェーズは `Diagnostic.extensions.effects.stage` に `required` / `actual` を転記し、監査ログ（`AuditEnvelope.metadata`）へ同一キーで集約する。`tooling/ci/collect-iterator-audit-metrics.py` はこれらキーの有無を検証し、`tooling/ci/sync-iterator-audit.sh` が `verify_llvm_ir.sh` のログと突合することで `0-3-audit-and-metrics.md` へ記録するワークフローを提供する。
- 辞書渡しと PoC モノモルフィゼーションの比較は `compiler/ocaml/scripts/benchmark_typeclass.sh` の `--typeclass-mode={dictionary,monomorph}` を用いる。Phase 2 では `--static-only` を指定して LLVM IR 行数・ビットコードサイズなど静的指標を JSON (`benchmark_results/static_comparison.json`) に出力し、実測ベンチマークは Phase 3 のループ実装後に再開する。

---

## C. 型推論（Inference）

### C.1 基本戦略

* **Hindley–Milner（Algorithm W）** + 制約解決。
* **サブタイピングなし**、**ユニオン/インターセクションなし**（単純化）。
* 変数束縛 `let` で **一般化（generalization）**、使用時に **インスタンス化**。

### C.2 変数の“剛性”

* **柔軟（unification var）**：推論中に他型と単一化される。
* **剛体（rigid/スコープ外）**：注釈や `forall` で導入された量化変数は **occurs check** を厳密化し、誤推論を防ぐ。

### C.3 値制限（Value Restriction）

* **一般化は“確定的な値”のみ**：

  * 右辺が **ラムダ・コンストラクタ・数値/文字列リテラル・純式** → 一般化可。
  * **可変参照・I/O・外部呼び出し**を含む可能性がある右辺は **単相**（将来の効果システムで形式化；1.3 参照）。

### C.4 アノテーション

* **任意**（ローカル）／**推奨**（公開 API）。
* アノテがある場合は **双方向型付け**（bidirectional）で誤差を小さくし、エラー品質を上げる。

### C.5 演算子・リテラルの既定

* **数値リテラル**は `Num<T>` 制約を持つ多相リテラル。曖昧時はデフォルト `i64` / `f64`（小数点の有無で分岐）。
* **演算子**は対応トレイトで解決。`a + b` は `Add<typeof a, typeof b, r>` の `r` を新鮮変数で導入し、単一化。

### C.6 効果行とハンドラの型付け（実験段階）

> `-Zalgebraic-effects` フラグが有効な場合に適用。安定化後に行多相の範囲・ランク制限を再評価する。

* **効果注釈**: 関数型は `A -> B ! {io, panic}` のように効果集合を伴う。省略時は空集合。ハンドラは `handler Console : {Console.log, Console.ask} -> {}` のように捕捉効果と残余効果を宣言する。
* **行多相 (ランク1)**: トップレベル `let` のみ効果行変数 `!ε` を一般化し、スキーム `∀ε. τ ! ε` を生成する。再帰関数は効果集合の収束後に一般化する。
* **制約生成の拡張**:
  ```
  Γ ⊢ e : τ ! Σ
  ------------------------------- (perform)
  Γ ⊢ perform Console.log(x) : Unit ! (Σ ∪ {io})

  Γ ⊢ comp : τ ! Σ_before      Γ ⊢ handler : τ -> σ ! Σ_residual
  Σ_after = (Σ_before - Σ_handler) ∪ Σ_residual
  ------------------------------------------------------------------- (handle)
  Γ ⊢ handle comp with handler : σ ! Σ_after
  ```
  `Σ_handler` は `handler` ブロックまたは `@handles` 属性で宣言された集合。`Σ_residual` はハンドラ本体で発生する効果集合。
* **契約検査**: `@handles` は解析時に `Σ_handler` を確定させ、残余効果 `Σ_after` が `@pure` や `@dsl_export(allows_effects=...)` の条件を満たすか検証する。違反時は `effects.contract.mismatch` または `dsl.export.effect_violation` を報告。
* **Stage と Capability**: `stage = Experimental` の効果を扱う場合、シグネチャに `@requires_capability(stage="experimental")` を含め、Capability Registry が許可した環境でのみビルドできるようにする。

### C.7 失敗時の方針（エラー）

* **期待/実際**・**候補トレイト**・**不足制約**を列挙。
* 量化変数が関係する場合は **“ここで一般化/インスタンス化が必要”** を示す。
* 位置は **式ごとに最狭スパン**で報告（Core.Parse.Err と連携）。

### C.8 `RunConfigTarget` と型検査の整合

```reml
type RunConfigTarget = {
  os: Str,
  family: Str,
  arch: Str,
  abi: Option<Str>,
  vendor: Option<Str>,
  env: Option<Str>,
  profile_id: Option<Str>,
  triple: Option<Str>,
  features: Set<Str>,
  capabilities: Set<Str>,
  stdlib_version: Option<SemVer>,
  runtime_revision: Option<Str>,
  diagnostics: Bool,
  extra: Map<Str, Str>
}
```

* `RunConfigTarget` は CLI が解決した TargetProfile と実行中プラットフォーム (`Core.Env.infer_target_from_env`, `platform_info()`) から統合的に構築され、`RunConfig.extensions["target"]` に注入される。型検査フェーズでは `@cfg` 判定の結果をこの構造体から取得し、条件付き宣言の有効/無効を決定する。正式なフィールド一覧は本節を基準とし、2.6 §B-2 でも同じ定義を参照する。
* `profile_id` が未設定の状態で `@cfg(profile_id = "...")` を評価した場合、型検査は `target.profile.missing` を生成し、宣言を常に無効として扱う。これにより 0-2 指針 1.2 の安全性を満たす（曖昧なターゲットではビルドを進めない）。
* `capabilities` セットは `Core.Runtime` の Capability Registry から初期化される。型検査は Capability 起因で無効化された分岐にアクセスした参照に対し `unresolved.symbol.cfg` を報告し、 Capability を有効化した場合のみ到達可能とみなす。
* `stdlib_version` と `runtime_revision` は ABI 互換性の保証に使用され、宣言が要求するバージョンと一致しない場合は `target.abi.mismatch` を発生させる。診断には `RunConfigTarget` に含まれる `triple` と `extra` の抜粋が添付され、性能 1.1 で求める線形処理を保ったまま原因を特定できる。
* `features` や `extra` を参照する型レベルロジックは単純な等価比較に限定される。複雑な依存を導入する場合は標準ライブラリの設定 API（3-7）で明示的に型を表現する。
* `diagnostics` が `true` の場合、`@cfg` 判定で得た詳細ログを `Diagnostic.extensions["cfg"]` に添付する（2.5 §B-9）。

---

## D. パターンの型付け

### D.1 パターン規則

* `let (x, y) = e`：`e : (a, b)` を要求し、`x:a`, `y:b` を導入。
* レコード：`{x, y: y0}` は `{x: a, y: b}` と単一化、`x:a`, `y0:b`。
* コンストラクタ：`Some(x)` は `e : Option<a>`、`x:a`。
* ガード：`if cond` は `cond : Bool`。

### D.2 網羅性（型付け段階の情報）

* `match` の各分岐で **スクラティニーの型**と**残余集合**（未到達バリアントや値範囲）を追跡し、ガード付きパターンは「一致しても `cond` が偽なら残余へ戻る」ものとして扱う。
* 型推論は `CoverageResidue{missing: Set<VariantLike>, span: Span}`（`VariantLike` は ADT コンストラクタや列挙的リテラル集合を指す）をノードへ付与し、後続のエラー整形（2.5）と効果検査（1.3）に回す。残余が空であれば診断は出ない。
* 残余が非空の場合は **`panic` 効果を伴う式**としてマークされる（実装上は暗黙の `panic_unreachable` が生成されるため）。`@no_panic` や `@pure` を持つ関数／ブロックではこの効果が禁止されるため、**非網羅は即エラー**になる。
* それ以外のケースでは、既定の lint レベル `non_exhaustive_match = Warning` で診断を発行し、プロジェクト設定や CLI（例：`--fail-on-warning` や lint 設定で `error` 指定）で**エラーへ昇格**できる。`@no_panic` が付いていなくても、明示的に `lint.non_exhaustive_match = "error"` を選択した場合は型検査段階で致命扱いとなる。

---

## E. モジュールと汎化境界

* **トップレベル `let`** はモジュール境界で一般化。
* `pub` シンボルは **公開型**で確定（型変数は外向けに量化）。
* `use` により導入されたトレイト/型は **名前解決表**に登録され、推論時に探索対象となる。

---

## G. DSLエクスポートと互換性メタデータ {#dsl-export-typing}

`@dsl_export` 属性（[1.1 §B.1.1](1-1-syntax.md#dsl-entry-declaration)）は型検査段階で **`DslExportSignature`** を生成し、マニフェスト (`reml.toml`) 側の `dsl.<name>` 宣言と突き合わせる。コンパイラは以下の手順でメタデータを構築する。

1. 宣言の最終的な型 `τ` を推論し、次の何れかの形に正規化する。
   - `Parser<T>` もしくは型エイリアスでラップされた `Parser<T>`。
   - `fn(args) -> Parser<T>`（`args` は任意の個数・名前付き引数を含んでもよい）。
   - `ConductorSpec<U>`（`conductor` 宣言から導出されるランタイム表現。詳細は 1.3 §I で扱う）。
2. 正規化結果が上記に一致しない場合は `E1301`（DSL エクスポート型不一致）を報告し、`@dsl_export` を外すか型を修正するよう促す。
3. 属性パラメータを解析し、以下のフィールドを持つ `DslExportSignature` を組み立てる。

```reml
type DslCapabilityRequirement = {
  id: CapabilityId,
  stage: StageRequirement,
  effect_scope: Set<EffectTag>,
}

type DslStageBounds = {
  declared: StageId,
  minimum: StageRequirement,
  maximum: Option<StageId>,
}

type DslExportSignature<T> = {
  name: Str,
  category: DslCategory,
  root_type: TypeRef<T>,
  produces: DslCategory,
  requires: List<DslCategory>,
  capabilities: List<CapabilityId>,
  requires_capabilities: List<DslCapabilityRequirement>,
  allows_effects: Set<EffectTag>,
  stage_bounds: DslStageBounds,
  version: Option<SemVer>,
}
```

- `category` はマニフェストの `dsl.<name>.kind` と同一の文字列で、互換判定ではインターン済みシンボルとして比較する。
- `produces` は省略時に `category` と同値とする。`Parser<T>` の場合は `T` の型情報を持つ DSL 生成物カテゴリを推定し、`T` が `DslOutput<Category>` を実装していればその関連型を採用する。
- `requires` は conductor など複数 DSL を束ねる宣言で使用し、参照する DSL カテゴリが `exports` 内または依存マニフェストに含まれることを検証する。
- `capabilities` は後方互換のために保持している単純な Capability ID 一覧であり、`requires_capabilities` の `id` を投影した派生値として扱う。
- `requires_capabilities` は `@requires_capability` や Capability マニフェストから抽出した Stage 付き要件を格納し、各要素が `effect_scope` で影響範囲を明示する。Stage 判定は 0-1 §1.2 の安全性優先原則に従い、`StageRequirement::AtLeast` の場合でもマニフェスト側の上限を超えないよう検証する。
- `allows_effects` は 1.3 の効果集合に対するサブセットであり、空集合の場合は純粋値として扱う。
- `stage_bounds` は DSL エクスポートそのものの Stage 運用ルールを表し、`declared` に現在の公開ステージ、`minimum` に受け入れ下限、`maximum` に外部ブリッジで許容される上限を記録する。`maximum = None` の場合は `minimum` の判定のみを適用する。

4. `Parser<T>` を返す関数では **引数の型変数を一般化前に固定**し、`DslExportSignature` に引数ごとの型情報（`input_shape`）を添付する。これにより CLI や LSP が利用者へ API ドキュメントを提示できる。

`DslExportSignature` は `Core.Config.Manifest`（3.7）に引き渡され、`dsl.<name>.exports[*]` の `signature` として書き戻される。互換性検査は以下の規則で行う。

- **カテゴリ互換**: 同一カテゴリで major バージョン (`version.major`) が一致しているか、または `reml.toml` で `allow_prerelease=true` を明示している。
- **能力互換**: `requires_capabilities` の各要素について `CapabilityRegistry::verify_capability_stage`（3.8 §1.2）を適用し、Stage 条件と効果境界が満たされているか確認する。未解決の Capability がある場合は `diagnostic("dsl.capability.unsatisfied")` を発行し、Stage 不一致は `diagnostic("dsl.capability.stage_mismatch")` へ昇格する。
- **効果境界**: `allows_effects ⊆ declared_effects(manifest)`。宣言より広い効果集合を持つ場合は型検査エラー `E1302` を報告する。
- **ステージ互換**: `stage_bounds.declared` が `stage_bounds.minimum` を満たし、Capability マニフェストや `reml.toml` 側の Stage 上限が存在する場合は `stage_bounds.maximum` 以下であることを保証する。境界を破った場合は `manifest.dsl.stage_mismatch` を生成し、0-1 §1.2 の安全性レビューに従って適用を拒否する。

互換性の失敗は型付け段階で診断を生成し、`DslExportSignature` の `span` とマニフェスト側の反映先行番号を結び付けた差分が `Core.Diagnostics` へ渡される。

## F. 代表的な型（標準 API・コンビネータ想定）

> パーサーコンビネータ記述が短くなるように、要の関数型は**一読で意図が分かる**シグネチャに。

```reml
// Parser 型（簡略）
type Parser<T> = fn(&mut State) -> Reply<T>  // 実体は 2.1 §A（State/Reply 定義）に従う

// コア・コンビネータ（抜粋）
fn map<A,B>(p: Parser<A>, f: A -> B) -> Parser<B>
fn then<A,B>(p: Parser<A>, q: Parser<B>) -> Parser<(A,B)>
fn or<A>(p: Parser<A>, q: Parser<A>) -> Parser<A>
fn many<A>(p: Parser<A>) -> Parser<[A]>
fn chainl1<A>(term: Parser<A>, op: Parser<(A,A)->A>) -> Parser<A>
fn between<A>(l: Parser<()>, p: Parser<A>, r: Parser<()>) -> Parser<A>

// 典型的な型推論の例
let int  = digit.many1().map(parseInt)            // Parser<i64>
let atom = or(int, between(sym("("), expr, sym(")")))
let expr = chainl1(atom, addOp)                   // Parser<i64>
```

- `Parser<T>` の戻り値型 `Reply<T>` および解析状態 `State` の項目は [2.1 §A](2-1-parser-type.md#a-主要型) にて定義されています。旧来の `Input -> Result<T, ParseError>` 形は `RunConfig.legacy_result` による互換層のみが提供され、型推論仕様の基準からは除外されました。

---

## G. 実装上の規約（コンパイラ側）

1. **単一化（unify τ1 τ2）**：対称・逐次、**occurs check** あり。
2. **一般化**：`let x = e` の型 `τ` から、**外スコープに自由な変数**を除いた集合を量化。
3. **インスタンス化**：使用時に量化変数を新鮮変数へ置換。
4. **制約収集**：トレイト制約は `C = {Add<a,b,r>, …}` の集合として保持。
5. **制約解決**：

   * **第一段**：具体型が決まるたびに `impl` テーブルで一致検索（単一解であること）。
   * **第二段**：残余があれば**呼出側へエスカレーション**（関数型の `where` へ持ち上げ）。
6. **デフォルト**：残余が数値リテラルのみなら `i64`/`f64` を割当（曖昧ならエラー）。

---

## H. 例（推論の挙動）

### H.1 let 一般化

```reml
let id = |x| x               // id : ∀a. a -> a
let n  = id(42)              // inst a := i64 → i64
let s  = id("hi")            // inst a := String → String
```

### H.2 制約の持ち上げ

```reml
fn sum<T>(xs: [T]) -> T where Add<T,T,T>, Zero<T> =
  fold(xs, init=zero(), f=Add::add)
```

呼出側：

```reml
let r1 = sum([1,2,3])        // T := i64, 既存 impl で解決
let r2 = sum(users)          // エラー（Add<User,User,User> が未定義）
```

### H.3 演算子の推論

```reml
let f = |x, y| x + y         // 収集: Add<a,b,r>; 型: a -> b -> r
let g = |n| n + 1            // 収集: Add<a,i64,r>; 不足 → 呼出で決定
```

### H.4 数値リテラルの既定

```reml
let a = 10        // a : i64
let b = 10.0      // b : f64
let c: f32 = 10   // 単一化で c : f32（数値多相の縮退）
```

---

## I. エラーメッセージの形（例）

* **型不一致**

  ```
  type error: expected i64, found String
    --> main.ks:12:17
     12 | let n: i64 = "42"
                     ^^^^^^ expected i64 here
  ```
* **不足トレイト**

  ```
  constraint error: cannot resolve Add<User,User,User>
    --> calc.ks:7:12
     7 | users |> sum
               ^^^ requires Add<User,User,User> and Zero<User>
     help: define `impl Add<User,User,User>` or annotate with a concrete type
  ```
* **汎化の値制限**

  ```
  generalization blocked: expression may be effectful
    --> parse.ks:3:9
     3 | let p = readLine() |> map(...)
             ^ consider adding a type annotation or using a pure binding
  ```

---

## J. ドメイン型拡張

Reml のコア型システムは代数的データ型とトレイト制約を中心に構成され、テンソル計算や設定スキーマ、クラウドリソース識別といった領域特化型は含めません。これらの高度な型は `Core.Data` や `Core.Config` などの標準ライブラリ拡張、もしくはプラグインで opt-in してください。拡張側では本節で示したような証明トレイトや制約生成を再利用できるため、コア仕様を簡潔に保ちつつ領域固有のニーズに対応できます。

---

## K. まとめ（設計の要点）

* **HM + トレイト制約**という最小で強力な骨格。
* **サブタイピングなし**で推論を安定化、**bidirectional + アノテ**でエラー品質を確保。
* **数値多相の既定**と**演算子=トレイト**で、日常コードを短く自然に。
* **一般化の値制限**と**剛体変数**で予期せぬ推論"暴走"を抑止。
* **パターン型付け**・**網羅性**・**制約の持ち上げ**が、Reml→Core→IR の変換を素直に支える。

---

## 関連仕様

* [1.1 構文](1-1-syntax.md) - 言語構文の詳細
* [1.3 効果と安全性](1-3-effects-safety.md) - 効果システムとの連携
* [1.4 文字モデル](1-4-test-unicode-model.md) - Unicode型システム
* [2.5 エラー設計](2-5-error.md) - 型エラーの報告
* [LLVM連携ノート](../guides/llvm-integration-notes.md) - LLVM連携での型情報利用
