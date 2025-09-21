# Reml オブジェクト指向支援 API 調査メモ

## 1. 調査背景と目的
- Reml はパーサーコンビネーターに最適化された言語であり、DSL やコンパイラを迅速に構築することを主眼としている。
- 標準 API に「オブジェクト指向」機能を組み込むことで、利用者が Reml 上で OOP 的特徴を持つ言語を短期間で設計できるかを評価する。
- 本調査では Reml の設計哲学・型システム・既存シナリオを確認し、OOP 支援 API の妥当性を検討するための観点を整理する。

## 2. Reml 設計の前提
- **実用性最優先**: 実運用レベルの性能・FFI・LLVM/JIT 連携を確保することが最上位ゴール（`0-1-overview.md`, `0-2-project-purpose.md`）。
- **小さく強いコア**: 言語機能も Core.Parse も 12〜15 の本質的プリミティブへ還元し、合成で拡張する思想を徹底。
- **宣言で終わらせる操作性**: 演算子優先度・空白・字句規則などを宣言 DSL として提供し、利用者が意図を短く表現できるようにする。
- **高品質な診断・ツール連携**: SpanTrace、期待集合、RunConfig などの診断情報を保持し IDE/LSP 連携を重視する。
- **Unicode ファースト**: `byte/char/grapheme` の 3 層モデルを前提にした入力処理が標準で求められる。

## 3. 型システムと標準 API の制約
- **Hindley–Milner 型推論**を採用し、トレイト（型クラス風）で静的オーバーロードを扱う。サブタイピングやクラス継承は標準では想定していない。
- Core.Parse は `consumed`/`committed` ビット、Packrat メモ化、SpanTrace などによって高品質なエラーと性能を両立しており、追加 API も同水準の診断を提供する必要がある。
- Core.Config や Core.Data など既存モジュールは宣言的 DSL と型安全性を両立しており、新たな OOP 支援もこの一貫性を維持することが求められる。

## 4. オブジェクト指向支援 API 検討観点
1. **コア哲学との整合性**
   - 小さく強いプリミティブを崩さず、合成可能な最小 API 群として設計できるか。
2. **宣言的表現の確保**
   - クラス/メソッドに相当する構造を宣言 DSL やビルダーで定義でき、利用者が意図を簡潔に書けること。
3. **型システムとの適合**
   - サブタイピングに頼らない多相オブジェクト、型クラス的ディスパッチ、ADT を活用した合成など HM と矛盾しないモデルが必要。
4. **診断品質・ツール連携**
   - SpanTrace や期待集合と連携したエラーメッセージ、IDE 補完・リファクタリング支援用メタデータを提供可能か。
5. **性能と実運用性**
   - Packrat/左再帰制御、JIT/FFI との橋渡しを阻害しない抽象化レイヤであること。
6. **シナリオ適合性**
   - ゲームエンジン向け DSL やエンタープライズツールなど、実シナリオで OOP 支援が効果的かどうかを評価する指標を持つ。

## 5. 想定シナリオとの接点
- **ゲームエンジン向けスクリプト**: 役割やコンポーネント単位の API を OOP スタイルで宣言できれば、エンジン統合やホットリロード運用に有利。
- **エンタープライズ DSL / Web フレームワーク**: ドメインモデルの階層構造や責務分離を表現する際に、OOP 的抽象化が開発体験を向上させる可能性がある。
- **IDE/ツール統合**: オブジェクト構造情報を LSP へ公開できれば、補完やナビゲーション品質の向上が期待できる。

## 6. 今後の調査計画
1. オブジェクト指向パラダイム（継承ベース、プロトタイプ、トレイト/ミックスイン、データ指向 OOP など）の文献を収集し、Reml の要件との適合性を一次評価する。
2. HM 型推論と相性が良い抽象化（型クラス、辞書渡し、ADT 組合せなど）を整理し、必要な API インターフェイスを草案化する。
3. Core.Parse / Core.Data / ParserPlugin との連携要件を列挙し、診断メタデータや宣言 DSL の拡張方針を検討する。
4. 想定シナリオごとに OOP 支援がもたらす効果と代替手段を比較し、導入判断の評価指標を策定する。

---

本メモは OOP 支援 API の妥当性調査に向けた出発点であり、次段階では各パラダイムの研究・実装事例を収集して詳細な比較検討を行う。

## 7. オブジェクト指向パラダイムの文献調査と一次評価

| パラダイム | 代表的文献・事例 | Reml 要件との適合性メモ |
| --- | --- | --- |
| 継承ベース OOP | G. Booch *Object-Oriented Analysis and Design* / A. Snyder "Encapsulation and Inheritance" / Java 言語仕様 | 階層型の型システムを前提とするケースが多く HM と衝突しやすい。クラス定義 DSL を提供する場合でも `extends` 連鎖は制限的に扱う必要がある。静的ディスパッチと VTable 生成を字句/宣言 DSL に落とし込む設計検討が必要。 |
| プロトタイプベース | D. Ungar *Self: The Power of Simplicity* / ECMAScript Standard / R. Culpepper "Object-Oriented Scheme" | 実行時に辞書ベースの更新を行うモデルは Reml の静的解析志向と相性が悪い。宣言 DSL を通して固定化したプロトタイプを生成し、Core.Data のレコード更新を流用するなら適用可能。型推論ではレコード多相を活用する案が有望。 |
| トレイト/ミックスイン | S. Ducasse *Traits: A Mechanism for Fine-Grained Reuse* / Scala トレイト / Rust トレイト | サブタイピングなしで振る舞い合成ができ、型クラスとも接続しやすい。Reml の「小さく強いプリミティブ」を維持しつつ宣言 DSL で `trait { ... }` を組み立てる方針と一致。DI 設計にも流用可能で有望。 |
| データ指向 OOP / ECS | M. Acton "Data-Oriented Design" / Unity DOTS / Flecs ECS | コンストラクタの代わりにデータテーブルとシステム関数を分離するモデル。Reml のレコード/列指向データと親和性が高く、宣言 DSL でシステムパイプラインを定義できる。診断メタデータもコンポーネント境界で収集しやすい。 |
| マルチメソッド / CLOS 風 | G. Kiczales *The Art of the Metaobject Protocol* / Dylan 言語仕様 | 汎用関数によりディスパッチを宣言するアプローチは HM と直接両立しにくいが、`where` 制約付きの型クラスと組合せれば限定的に導入可能。Core.Parse への影響は小さいが実行時性能に注意。 |
| インターフェイス + デフォルトメソッド | Java 8 インターフェイス / Haskell typeclass default | トレイト系とほぼ同じ位置付けで、メソッド既定値を DSL で宣言し、型クラス辞書を裏で生成する構成が可能。Reml の API としてはトレイトと統合して扱う。 |

- 共通課題: いずれのパラダイムも Reml の HM 型推論と診断品質を損なわないよう、静的に展開可能な宣言 DSL として設計する必要がある。
- 推奨方向: トレイト/ミックスイン + データ指向 OOP をコアに据え、継承ベースとプロトタイプはラッパー的な DSL とするハイブリッド案が現実的。

## 8. HM 型推論と親和性の高い抽象化と API 草案

### 8.1 採用候補の抽象化
- **型クラス／トレイト**: `trait` 宣言によってインターフェイスを定義し、`impl` ブロックで型ごとの実装を与える。HM の辞書渡し変換に自然に落とし込める。
- **辞書渡し (`Dictionary Passing`)**: コンパイル時にトレイト実装を辞書へ変換し、呼び出し箇所へ暗黙引数として供給。部分適用や高階関数と相性が良い。
- **ADT 組合せ**: 代数的データ型の積・和を用いてオブジェクト状態と振る舞いを分離。バリアントによる動的ディスパッチの代替として利用。
- **レコード多相 (`Row Polymorphism`)**: 必須フィールド+拡張フィールドを許す設計で、プロトタイプ的拡張やデフォルトフィールドを安全に表現可能。
- **エフェクト注釈との結合**: `effect` 付きのメソッド宣言で、副作用トラッキングと OOP 風の API 呼び出しを整合させる。

### 8.2 API 草案（抜粋）

```reml
trait TraitName<T> {
  effect io;
  fn method(self: T, ctx: Context) -> Result<Output>;
  default fn fallback(self: T) -> Output { ... }
}

impl TraitName<MyStruct> {
  fn method(self, ctx) -> Result<Output> { ... }
}

fn with_trait<T>(value: T) where TraitName<T> {
  let dict = TraitName::dictionary<T>();
  dict.method(value, Context::current())
}
```

- `trait` 宣言は Core.Parse のビルダー DSL を拡張して `trait` / `impl` ブロックを導入する案。
- `default fn` により HM 上の既定実装を辞書へ埋め込み、実体型が提供しない場合のフォールバックを生成。
- `dictionary()` のような暗黙辞書取得関数をコード生成段階で挿入し、利用者が明示的に扱う必要を最小化。

### 8.3 コンパイル戦略
- **辞書生成**: トレイトごとにレコード型 `{ method: fn, ... }` を生成。ADT によるケース分岐が必要な場合はタグ付きユニオンを組み合わせる。
- **インスタンス解決**: Core.Data の `Resolver` 仕組みを流用し、`where` 節からインスタンス探索を行う。重複検出や曖昧性診断は既存の型クラス診断を再利用。
- **エフェクト統合**: メソッド署名に付与された effect アノテーションを結合し、呼び出し元のエフェクトセットへ伝播する。既存の `effect` 推論と整合。
- **IDE 支援**: トレイト辞書にメタデータ (`SpanTrace`, `DocString`) を含め、LSP でメソッド補完やジャンプを提供。

## 9. Core.* モジュールとの連携要件と DSL 拡張方針

### 9.1 Core.Parse
- **構文拡張ポイント**: `trait`, `impl`, `object` キーワード追加。演算子宣言 DSL (`2-4-op-builder.md`) に新しい優先度レイヤを追加し、`trait` 定義内のメソッド宣言を式と区別。
- **エラーレポート**: `expect` 集合へ「型クラスインスタンス」「トレイトメソッド」のタグを追加。`SpanTrace` に trait/impl 生成元を記録し循環参照時の診断を強化。
- **ParserPlugin 連携**: Trait 宣言を検出するプラグインに AST ノードを渡し、辞書生成・名前解決をコンパイル前処理で実施。プラグイン API に `register_trait(ast, ctx)` などのフックを追加。

### 9.2 Core.Data
- **辞書表現**: `Dict<Trait>` 型を Core.Data に追加し、`lookup`, `merge`, `override` 操作を提供。診断用に `origin: SpanTrace` を常に保持。
- **レコード統合**: レコード多相を実現するため `Row<T>` メタ型を導入し、プロトタイプ的拡張を静的検証。`Row.extend` API によってフィールド追加時の型整合性を確認。
- **メタデータ管理**: トレイトやオブジェクト宣言に `Doc`, `Attribute`, `Visibility` 情報を付与し、Core.Data 側でシリアライズ。LSP へ `SymbolKind::Trait` を新設。

### 9.3 ParserPlugin
- **ビルダーフェーズ**: パーサープラグインに「トレイトスロット」概念を追加し、複数ファイルに跨る宣言を集約。インターフェイス衝突時に差分報告を生成。
- **診断メタデータ**: プラグイン API の戻り値に `DiagnosticPayload` を追加し、IDE へトレイト実装の欠落や未解決メソッドを通知。
- **宣言 DSL の拡張手順**: DSL での `trait` ブロック生成をプラグインが補助し、`coreDsl.trait("Parser") { method(...) }` のような宣言的 API を提供。既存の `operator`, `token` と同系統のビルダー構文を踏襲。

## 10. シナリオ別効果評価と導入判断指標

### 10.1 シナリオ比較

| シナリオ | OOP 支援導入の効果 | 代替手段 | 留意事項 |
| --- | --- | --- | --- |
| ゲームエンジン DSL | シーン/エンティティをトレイト合成で宣言し、ホットリロード対象を明示化。LSP でコンポーネント API の補完が向上。 | ECS ベースでシステム関数を関数合成のみで表現。 | ランタイムにおける辞書引き解決コスト。`effect` を伴うメソッドの非同期化戦略。 |
| エンタープライズ Web/業務 DSL | ドメインモデルを trait + ADT で分割し、サービス境界ごとに契約を明示。 | モジュール/名前空間 + 関数型 API による層分割。 | トレイト実装数が多くなるため自動生成とインスタンス管理が鍵。メタデータのガバナンスが必要。 |
| IDE/言語ツール連携 | トレイト・メソッド情報をシンボルテーブル化することで、ジャンプやコードアクションの精度が向上。 | 既存の AST 解析のみで補完。 | ParserPlugin でのメタデータ抽出コスト。IDE 側とのプロトコル拡張。 |
| DSL 拡張フレームワーク (ParserPlugin) | trait 経由でプラグイン API の契約を提示し、第三者が安全に拡張可能。 | 生の関数セットやレコードで契約定義。 | バージョン管理と互換性確保のため trait の進化戦略 (default メソッド) が必須。 |
| データ分析/ETL DSL | データソースを trait 化し、ストリーミング/バッチなど複数実装を差し替え。 | ファンクター/モナド変換でパイプラインを表現。 | 辞書渡しがホットパスになるためキャッシュ戦略が必要。 |

### 10.2 導入判断の評価指標
- **型整合性インパクト**: HM 推論に追加制約を導入する箇所の数と複雑度。型エラーの説明可能性を `SpanTrace` で検証。
- **宣言 DSL の簡潔性**: 既存 DSL と比較して宣言ステップ数がどれだけ増減するか。典型 API で 10 行以内に収まるかを目標とする。
- **ランタイムコスト**: 辞書解決・継承シミュレーションによるオーバーヘッドを `O(呼び出し回数)` で分析。Packrat メモ化との相互作用を測るベンチを用意。
- **診断充実度**: IDE/LSP で提供できる補完・リファクタリング支援メニューの数。エラー分類の明確さ (未実装・重複・曖昧インスタンス等)。
- **拡張容易性**: ParserPlugin 開発者が独自 trait を追加・配布するまでの手順数。Core.Data のメタデータ拡張 API が十分かを確認。
- **移行性・互換性**: 既存 DSL との相互運用や段階導入が可能か。trait 導入前後で同じバイトコード/IR を生成できるかの比較。

## 11. 次のステップ
- 各パラダイムで挙げた文献から具体的な構文・実装例を抽出し、Reml 向けサンプル DSL を作成。
- 型クラス/辞書生成の試作コードを ParserPlugin で仮実装し、辞書サイズ・解決コストを測定。
- Core.Data の `Dict<Trait>`・`Row` 機能の詳細設計を別ドキュメントに切り出し、API シグネチャを確定。
- 評価指標に基づく PoC 計画 (ゲーム DSL / Web DSL / ツール連携) をそれぞれ 1 スプリント単位で策定。

## 12. Reml 向けサンプル DSL ドラフト

### 12.1 コンポーネント指向ゲーム DSL (Traits × ECS)
- **文献出典**: Trait 合成 (Rust) + Data-Oriented Design。
- **DSL 目的**: シーン内のエンティティをコンポーネント単位で宣言し、トレイトによる振る舞い差し替えを可能にする。

```reml
trait Tick<T> {
  effect frame;
  fn update(self: T, dt: Float, world: WorldState) -> WorldState;
}

trait Render<T> {
  effect gpu;
  fn draw(self: T, frame: FrameCtx);
}

object Player {
  component Transform { position: Vec3, velocity: Vec3 }
  component Health { hp: Int, max: Int }

  impl Tick<Player> {
    fn update(self, dt, world) -> WorldState {
      world.apply(self.Transform.velocity * dt)
    }
  }

  impl Render<Player> {
    fn draw(self, frame) {
      frame.sprite("player", self.Transform.position)
    }
  }
}

system Physics where Tick<T>, Row<T, Transform> {
  fn run(entity, dt, world) = Tick::dictionary<T>().update(entity, dt, world)
}

scene Demo {
  spawn Player { Transform { position: vec3(0,0,0), velocity: vec3(1,0,0) } }
  pipeline [ Physics, RenderLoop ]
}
```

- **特徴**: `object` ブロックでコンポーネントとトレイト実装を束ねる。`Row` 制約が Transform を保持するエンティティだけを抽出。
- **PoC 実装方針**: ParserPlugin で `object` 宣言を解析し、Core.Data へ `Dict<Tick>` 辞書を生成。Physics システムでは辞書キャッシュとエフェクト伝播を検証する。

### 12.2 サービス契約 DSL (Trait + ADT)
- **文献出典**: Booch のオブジェクトモデリング + Scala/Akka サービスパターン。
- **DSL 目的**: 業務 DSL においてサービスのインターフェイスと実装差し替えを宣言的に管理し、テスト用モックを同一 DSL で記述。

```reml
trait CustomerRepo<R> {
  effect io;
  fn fetch(self: R, id: CustomerId) -> Result<Option<Customer>>;
  fn save(self: R, model: Customer) -> Result<()>;
}

type RepoBackend =
  | Postgres { pool: PgPool }
  | InMemory { data: Map<CustomerId, Customer> }
  | RemoteApi { endpoint: Uri }

impl CustomerRepo<Postgres> { ... }
impl CustomerRepo<InMemory> { ... }

service CustomerService {
  provide repo: RepoBackend with CustomerRepo<RepoBackend>;

  fn findById(self, id) where CustomerRepo<repo> {
    match CustomerRepo::dictionary<repo>().fetch(self.repo, id) {
      Some(c) => Ok(c),
      None => Err(NotFound(id))
    }
  }
}

test CustomerServiceTests {
  using service CustomerService { repo = InMemory {} }
  assert findById(CustomerId::new("42")) == Err(NotFound("42"))
}
```

- **特徴**: `service` 宣言で依存するトレイト辞書を束縛し、`where CustomerRepo<repo>` により HM 制約を明確化。ADT により実装の切替を静的に記述。
- **PoC 実装方針**: `service` ブロックを ParserPlugin で AST 化し、依存辞書の自動注入コードを生成。テスト DSL との統合と効果追跡を評価する。

### 12.3 ParserPlugin 拡張 DSL (Trait as Contract)
- **文献出典**: MOP/CLOS の汎用関数契約 + Core.Parse 既存 DSL。
- **DSL 目的**: サードパーティが ParserPlugin 経由で拡張 API を定義・配布する際の契約 DSL を提供し、IDE 診断をフック。

```reml
trait ParserExtension<P> {
  fn register(self: P, builder: ParserBuilder) -> ParserBuilder;
  fn diagnostics(self: P) -> List<DiagnosticPayload> default [];
}

plugin JsonSupport {
  export fn provide() -> impl ParserExtension<JsonConfig> {
    builder => builder
      .token("LBrace", "{")
      .token("RBrace", "}")
      .rule("jsonValue", parseValue)
  }
}

host CoreParseHost {
  use plugin JsonSupport;
  register ParserExtension::dictionary<JsonConfig>();
}

diagnostic pass ParserExtension<JsonConfig> as dt {
  for issue in dt.diagnostics() { emit(issue) }
}
```

- **特徴**: `plugin` 宣言で trait 実装をエクスポートし、ホスト側が辞書を一括登録。`diagnostic pass` により IDE/LSP へのメタデータ連携が明示化。
- **PoC 実装方針**: ParserPlugin ランタイムに `ParserExtension` trait の辞書登録処理を追加し、診断パス API を実装。Core.Parse と IDE プロトコルの結線を検証する。
