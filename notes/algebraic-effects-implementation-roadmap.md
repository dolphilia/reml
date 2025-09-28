# Reml言語への代数的効果ハンドラ導入計画書

> 作成日: 2025年9月28日
> 対象バージョン: Reml v2.0以降
> 優先度: 中長期（MVP後の主要機能拡張）
> 根拠: 2024-2025年代数的効果実装状況調査結果

## 概要

本計画書は、Reml (Readable & Expressive Meta Language) への代数的効果ハンドラ（Algebraic Effects and Handlers）の段階的導入について策定する。既存の軽量効果システム（1.3節）を基盤として、型安全性と実用性能を両立しながら、モジュラーな制御フロー抽象化機能を提供することを目的とする。

## 背景と動機

### 現状分析

**Remlの既存効果システム（1.3節）**
- 軽量な効果タグ集合 `Σ = Σ_core ∪ Σ_system`
- 属性ベースの効果契約（`@pure`, `@no_panic`, `@no_alloc`等）
- 純粋関数をデフォルト、副作用は明示
- HM型推論 + 値制限による安全性保証

**制約事項**
- 制御フローの抽象化が限定的
- 例外、非同期、ジェネレータが個別実装
- パーサーコンビネーター以外の高レベル抽象化に課題

### 代数的効果の利点

1. **統一的抽象化**: 例外、状態、非同期、ジェネレータを単一の仕組みで表現
2. **関数の色問題解決**: 効果多相により同期/非同期コードの統合
3. **モジュラー設計**: ハンドラによる制御フローの構造化

### 他言語の実装状況（2024-2025年調査結果）

**Koka v3.2**
- エビデンス渡しによる効率的コンパイル
- 型指向の選択的CPS変換
- 研究言語としての位置付け、プロダクション利用には課題

**OCaml 5.3**
- ネイティブサポート、1%未満のオーバーヘッド
- 型安全性は保証せず（実行時例外）
- 実用性重視、同期プログラミングに特化

**最新研究動向**
- 型安全コード生成技術の進歩（GPCE 2024）
- パフォーマンス最適化手法の確立（ICFP 2024）
- 効果システム抽象化の理論的発展

## 提案する設計方針

### 基本原則

1. **段階的導入**: 既存コードの互換性を保持
2. **型安全性**: Remlの静的保証を強化
3. **実用性能**: パーサーコンビネーター性能を維持
4. **可読性**: 直感的な構文と明確なエラーメッセージ

### アーキテクチャ概要

```
既存効果システム → 基礎代数的効果 → 効果多相 → 高度最適化
    (1.3節)         (フェーズ1)     (フェーズ2)  (フェーズ3)
     ↓               ↓              ↓           ↓
  効果タグ        → 効果宣言    → 効果変数   → エビデンス渡し
  属性契約        → ハンドラ    → 行多相     → 継続最適化
  純粋性保証      → 型安全性    → 制約解決   → 実用性能
```

## 実装フェーズ

### フェーズ1: 基礎代数的効果 (v2.0-v2.2)

**目標**: シンプルな効果宣言とハンドラの導入

#### 新機能

**1. 効果宣言構文**
```reml
effect State<S> {
  get(): S,
  put(s: S): ()
}

effect Fail {
  fail(msg: String): Never
}

effect IO {
  read_file(path: String): Result<String, IOError>,
  write_file(path: String, content: String): Result<(), IOError>
}
```

**2. ハンドラ構文**
```reml
fn withState<S,A>(init: S, comp: () -> A) -> (S, A)
  handles State<S> = {
    get() -> resume(state),
    put(s) -> resume((), s)
  } in comp()

// 使用例
let (finalState, result) = withState(0, || {
  let n = do State.get()
  do State.put(n + 1)
  n * 2
})
```

**3. 効果操作の呼び出し**
```reml
fn increment() -> () = {
  let n = do State.get()
  do State.put(n + 1)
}

fn safeDiv(a: i64, b: i64) -> i64 = {
  if b == 0 {
    do Fail.fail("division by zero")
  } else {
    a / b
  }
}
```

#### 型システム拡張

**基本的な効果注釈**
```reml
fn foo() -> A ! State<i64>, Fail
fn bar() -> B ! {} // 純粋関数
```

**ハンドラ型**
```reml
type Handler<E, A, B> = (A ! E) -> B
```

**既存属性との統合**
```reml
@pure // 任意の代数的効果を禁止
@no_panic // Fail効果を禁止
@handles(State<i64>) // State<i64>をハンドル可能
```

#### 実装要件

- 単純な継続実装（one-shot limitation）
- 既存効果タグとの共存
- エラーメッセージの拡張
- 段階的型検査の強化

### フェーズ2: 効果多相と型安全性 (v2.3-v2.5)

**目標**: 型レベルでの効果追跡と多相性

#### 新機能

**1. 効果多相**
```reml
fn map<A,B,E>(f: A -> B ! E, xs: [A]) -> [B] ! E =
  xs |> List.map(f)

fn traverse<A,B,E>(f: A -> B ! E, xs: [A]) -> [B] ! E =
  match xs {
    [] -> [],
    [x, ...rest] -> {
      let y = f(x)
      let ys = traverse(f, rest)
      [y, ...ys]
    }
  }
```

**2. 効果制約**
```reml
fn pureMath<E>(x: f64) -> f64 ! E
  where E excludes IO, State =
  Math.sin(x) * Math.cos(x)

fn computation<E>(data: [i64]) -> i64 ! E
  where E includes State<i64> =
  data |> map(|x| { do State.put(x); x * 2 }) |> sum
```

**3. 効果エイリアス**
```reml
type alias ParseEffects = {Parse.Error, Parse.Backtrack}
type alias IOEffects = {IO, Fail}

fn parseAndSave<E>(input: String) -> () ! E ∪ IOEffects
  where E ⊇ ParseEffects =
  let ast = parseModule(input) // ! ParseEffects
  saveToFile(ast) // ! IOEffects
```

#### 型システム強化

**効果変数と制約解決**
```reml
// 効果変数の導入
effect E1, E2, E3

// 効果制約の表現
constraint E1 ⊆ {State<i64>, IO}
constraint E2 ∩ {Fail} = ∅
constraint E3 = E1 ∪ E2
```

### 3.6 LSP/IDE 可視化要件（追加）

### 3.7 LSP API ドラフト（effectsTree）

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "textDocument/effectsTree",
  "params": {
    "textDocument": { "uri": "file:///src/example.reml" },
    "position": { "line": 42, "character": 5 }
  }
}
```

```json
// Response
{
  "jsonrpc": "2.0",
  "result": {
    "stage": "experimental",
    "tree": {
      "function": "collect_logs",
      "effects": {
        "before": ["io"],
        "handled": ["io"],
        "residual": []
      },
      "handlers": [
        {
          "name": "Console",
          "operations": ["log", "ask"],
          "location": { "uri": "file:///src/example.reml", "range": [[10,2],[24,3]] }
        }
      ]
    },
    "diagnostics": [
      {
        "code": "effects.contract.mismatch",
        "stage": "experimental",
        "residual": ["io"],
        "range": [[12,4],[18,5]]
      }
    ]
  }
}
```

- リクエストはカーソル位置を受け取り、その関数に紐づく効果ツリー (`effects.before/handled/residual`) とハンドラ一覧を返す。
- レスポンスの `stage` は `Diagnostic.extensions["effects"].stage` と同じ列挙を利用し、IDE は Stage 表示に使用する。
- `diagnostics` は該当位置の効果関連診断（残余効果、未捕捉操作など）を含み、IDE のサイドバーで表示する。
- `handlers[].operations` は未実装操作を確認するための一覧で、IDE が警告アイコンを付与できる。

### 3.8 診断拡張 API

```json
// Example diagnostic payload
{
  "code": "effects.handler.unhandled_operation",
  "message": "operation ask is not handled",
  "data": {
    "effects": {
      "before": ["io"],
      "handled": ["io.Console.log"],
      "residual": ["io.Console.ask"],
      "stage": "beta",
      "handler": "Console"
    }
  }
}
```

- `data.effects` に `before`/`handled`/`residual`/`stage`/`handler` を含め、IDE がツリービューで未処理効果を表示できるようにする。
- Stage が `Experimental` の場合は警告色で表示し、昇格後は色が変わる。
- CLI は `--effects-debug` を指定した場合に同じペイロードを JSON で出力する。

- 効果ハンドラ導入後、LSP で `effectsTree`（仮称）エンドポイントを提供し、各関数の潜在効果・ハンドラ捕捉状況・残余効果をツリー表示できるようにする。
- `Diagnostic.extensions["effects"]` の `before`/`handled`/`residual` 情報を IDE でサイドバー表示し、未捕捉効果を強調する。
- Stage (`Experimental | Beta | Stable`) を hover / code lens で表示し、Experimental の場合は警告を付与。昇格後は `stage` 表示が更新されることを確認する。
- 未実装 operation に対して `effects.handler.unhandled_operation` をコードアクションで提示し、自動補完テンプレートを提供する。
- CLI は Stage 昇格コマンド後に `effects.stage.promote_without_checks` を確認し、IDE へ通知する。

**効果行多相（Row Polymorphism for Effects）**
```reml
type EffectRow<tail> = {State<i64>, IO | tail}

fn withExtra<A, E>(comp: A ! EffectRow<E>) -> A ! E =
  withState(0, || withIO(comp))
```

**より精密な効果推論**
- HM推論の効果変数への拡張
- 制約生成と解決アルゴリズム
- 効果の最小主型（principal types for effects）

#### パフォーマンス初期最適化

**tail-resumptive効果の最適化**
```reml
// 末尾再開は継続なしで最適化
fn tailCall() -> i64 ! State<i64> = {
  do State.put(42)  // 末尾位置 -> 継続不要
}
```

**単純なエビデンス渡し**
- ハンドラを辞書として実行時に渡す
- 単純な場合の静的解決

### フェーズ3: 高度な最適化と実用化 (v2.6+)

**目標**: プロダクション品質の性能と機能

#### 最適化技術

**1. 一般化エビデンス渡し**
```reml
// コンパイル時にハンドラを解決
effect State<S> compiled evidence {
  get(): S         -> getStateImpl(evidence),
  put(s: S): ()    -> putStateImpl(evidence, s)
}
```

**2. 継続最適化**
- Zero-cost抽象化の実現
- インライン化とスペシャライゼーション
- 軽量スレッド実装

**3. 効果融合**
```reml
// 複数効果の自動合成
handle comp() with
  State<i64>, IO, Fail -> OptimizedCompositeHandler
```

#### 高度な機能

**高階効果（Higher-order Effects）**
```reml
effect Control<A> {
  call_cc<B>(f: (A -> Never) -> B): B
}
```

**効果のスコープ制御**
```reml
scoped effect Session {
  begin(): SessionId,
  end(id: SessionId): ()
}
```

**並行効果とスケジューラ統合**
```reml
effect Async {
  spawn<A>(comp: () -> A): Future<A>,
  await<A>(fut: Future<A>): A
}
```

## 技術仕様詳細

### 構文設計

#### 効果宣言の完全構文
```reml
effect EffectName<T1, T2, ...>
  extends ParentEffect
  capabilities [cap1, cap2]
  realm "effect.domain"
{
  operation1(arg1: Type1, arg2: Type2) -> ReturnType,
  operation2(args...) -> ReturnType where Constraint,
}
```

#### ハンドラ宣言の完全構文
```reml
handle expr with {
  EffectName.operation1(args) -> {
    // ハンドラ本体
    resume(value)
  },
  EffectName.operation2(args) -> {
    // 非再開ケース
    defaultValue
  },
  return(x) -> {
    // 正常終了ケース
    finalProcess(x)
  },
  finally(result) -> {
    // クリーンアップ
    cleanup()
    result
  }
}
```

#### 効果注釈の完全構文
```reml
// 基本形
fn name(args...) -> ReturnType ! EffectSet

// 効果変数付き
fn name<E>(args...) -> ReturnType ! E where EffectConstraint

// 複合形
fn name(args...) -> ReturnType ! {Effect1, Effect2, ...E}

// ハンドラ型
fn handler(comp: A ! E1) -> B ! E2 handles E1
```

### 型システム統合

#### 効果型の内部表現
```reml
type Effect = {
  name: EffectName,
  params: [Type],
  operations: [Operation],
  capabilities: [CapabilityId],
  realm: EffectRealm
}

type EffectSet = Set<Effect>
type EffectVar = String
type EffectType =
  | Concrete(EffectSet)
  | Variable(EffectVar)
  | Union(EffectType, EffectType)
  | Intersection(EffectType, EffectType)
```

#### 型推論への統合

**制約生成規則**
```
Γ ⊢ e : τ ! ε
─────────────────────
Γ ⊢ do Effect.op(e) : σ ! ε ∪ {Effect}

Γ ⊢ comp : τ ! ε₁    Γ ⊢ handler : Handler<E, τ, σ> ! ε₂
───────────────────────────────────────────────────────
Γ ⊢ handle comp with handler : σ ! (ε₁ \ E) ∪ ε₂
```

**制約解決アルゴリズム**
1. 効果変数の単一化
2. 包含制約の解決
3. 最小効果セットの計算

### 既存システムとの統合

#### 1.3節効果システムとの関係

**既存効果タグの代数的効果での表現**
```reml
// 1.3節のmut効果
effect Mutation<T> : mut {
  read_ref(ref: &mut T) -> T,
  write_ref(ref: &mut T, value: T) -> ()
}

// 1.3節のio効果
effect IO : io {
  read_file(path: String) -> Result<String, IOError>,
  write_file(path: String, content: String) -> Result<(), IOError>,
  current_time() -> Time
}

// 1.3節のpanic効果
effect Panic : panic {
  panic(message: String) -> Never,
  assert_fail(condition: String) -> Never
}
```

**属性システムとの統合**
```reml
// @pureは全効果を禁止
@pure
fn computation(x: i64) -> i64 = x * 2  // OK

@pure
fn badComputation(x: i64) -> i64 = {
  do IO.read_file("config")  // ERROR: @pure関数でio効果
  x * 2
}

// 新しい属性
@handles(State<i64>, IO)
fn statefulIO<A>(comp: A ! {State<i64>, IO}) -> A = {
  handle comp with StateHandler, IOHandler
}
```

#### パーサーコンビネーターとの統合

**パーサー効果の定義**
```reml
effect Parser<I> : parser {
  consume() -> Option<I>,
  backtrack() -> Never,
  fail(error: ParseError) -> Never,
  cut() -> (),
  trace(message: String) -> ()
}

// 効果を使ったパーサー実装
fn many<A,I>(p: Parser<A> ! {Parser<I>}) -> Parser<[A]> ! {Parser<I>} = {
  handle {
    let first = p()
    let rest = many(p)
    [first, ...rest]
  } with {
    Parser.fail(_) -> return([]),
    return(result) -> result
  }
}

// 従来のParser<T>との互換性
type Parser<T> = () -> T ! {Parser<Input>}
```

## 実装計画

### マイルストーン詳細

#### M1: 基礎実装 (6ヶ月)

**月次計画**
- 月1-2: 効果宣言のパースと基本AST
- 月3-4: 基本的なハンドラ実装とランタイム
- 月5: 単純な継続サポート
- 月6: 型検査の基礎拡張とテスト

**成果物**
- [ ] 効果宣言のパース
- [ ] 基本的なハンドラ実装
- [ ] 単純な継続サポート
- [ ] 型検査の基礎拡張
- [ ] 基本的なエラーメッセージ
- [ ] 実験的機能フラグ

#### M2: 型システム統合 (4ヶ月)

**月次計画**
- 月1-2: 効果多相の実装
- 月3: 制約解決アルゴリズム
- 月4: 統合テストとバグ修正

**成果物**
- [ ] 効果多相の実装
- [ ] 制約解決アルゴリズム
- [ ] エラーメッセージの改善
- [ ] 既存効果システムとの統合
- [ ] 性能ベンチマーク基盤

#### M3: 最適化 (8ヶ月)

**月次計画**
- 月1-3: エビデンス渡しの実装
- 月4-6: 継続最適化
- 月7: ベンチマークとプロファイリング
- 月8: ドキュメント整備と安定化

**成果物**
- [ ] エビデンス渡しの実装
- [ ] 継続最適化
- [ ] ベンチマークとプロファイリング
- [ ] ドキュメント整備
- [ ] LSP統合
- [ ] 実用アプリケーションでの検証

### リソース要件

#### 開発体制
- **コア開発者**: 2-3名（言語実装、ランタイム）
- **型システム専門家**: 1名（効果推論、制約解決）
- **パフォーマンス最適化**: 1名（継続最適化、エビデンス渡し）
- **ドキュメント・QA**: 1名（仕様書、テスト、ユーザビリティ）

#### 開発環境
- 継続的インテグレーション環境
- パフォーマンステスト環境
- 大規模サンプルプロジェクト

#### 検証環境
- 既存パーサーコンビネーターテストスイート
- マイクロベンチマーク
- 実アプリケーションでの性能測定
- メモリ使用量プロファイリング

## リスク分析と対策

### 技術的リスク

#### 1. パフォーマンス劣化
- **リスク**: 継続オーバーヘッドによる性能低下
- **影響度**: 高（パーサーコンビネーターの性能が重要）
- **対策**:
  - 段階的最適化の実装
  - ベンチマーク継続監視
  - tail-resumptive最適化
- **軽減策**: opt-out可能な設計、従来APIの並行提供

#### 2. 型推論の複雑化
- **リスク**: エラーメッセージの可読性低下
- **影響度**: 中（ユーザビリティに影響）
- **対策**:
  - 双方向型付けの強化
  - 専用診断メッセージ
  - 段階的エラー表示
- **軽減策**: 型注釈の推奨、IDE支援強化

#### 3. 実装複雑性
- **リスク**: コンパイラの複雑化とバグ増加
- **影響度**: 高（品質とメンテナンス性）
- **対策**:
  - 十分なテストカバレッジ
  - 形式検証の部分適用
  - コードレビュー強化
- **軽減策**: 機能のオプション化、段階的導入

#### 4. メモリ使用量増加
- **リスク**: 継続オブジェクトによるメモリ消費
- **影響度**: 中（大規模処理時）
- **対策**:
  - 軽量継続実装
  - ガベージコレクション最適化
  - メモリプール利用
- **軽減策**: メモリ制限設定、プロファイリングツール

### プロジェクト運営リスク

#### 1. 学習コスト
- **リスク**: 開発者の習得負担
- **影響度**: 中（採用率に影響）
- **対策**:
  - 充実したドキュメント
  - 段階的導入ガイド
  - 実用例の提供
- **軽減策**: 既存パターンとの対応表、トレーニング資料

#### 2. エコシステム分断
- **リスク**: 新旧APIの混在
- **影響度**: 中（互換性問題）
- **対策**:
  - 長期間の互換性保証
  - 移行ツール提供
  - 段階的非推奨化
- **軽減策**: 自動変換サポート、明確な移行パス

#### 3. 仕様変更リスク
- **リスク**: 実装中の仕様変更
- **影響度**: 高（開発コスト増加）
- **対策**:
  - 早期プロトタイプ検証
  - ユーザーフィードバック収集
  - 仕様凍結期間設定
- **軽減策**: 実験フラグでの段階公開

## 成功指標

### 技術指標

#### 1. 性能目標
- **パーサーコンビネーター**: 既存性能の95%以上維持
- **代数的効果使用時**: OCaml5同等（5%以内のオーバーヘッド）
- **メモリ使用量**: 現在の110%以内
- **コンパイル時間**: 現在の120%以内

#### 2. 品質目標
- **型エラー解決時間**: 既存の120%以内
- **学習効率**: ドキュメント完走率80%以上
- **バグ密度**: 1000行あたり0.5個以下

#### 3. 機能目標
- **抽象化カバレッジ**: 90%以上の制御フロー抽象化を代数的効果で表現可能
- **互換性**: 既存効果システムとの完全互換性
- **拡張性**: 新しい効果の定義が容易

### ユーザー体験指標

#### 1. 採用率
- 新機能リリース後6ヶ月で20%のプロジェクトが使用
- 12ヶ月で50%のプロジェクトが部分採用

#### 2. 満足度
- ユーザー調査で平均4.0/5.0以上
- GitHub issue解決率90%以上
- コミュニティ活動度維持

#### 3. 教育効果
- チュートリアル完了率80%以上
- エラーメッセージ理解度85%以上

## 長期ビジョン

### 技術的発展

#### Phase 4: 高度な並行性 (v3.0+)
- 軽量スレッドとスケジューラ統合
- アクターモデルとの統合
- 分散計算サポート

#### Phase 5: 形式検証統合 (v3.5+)
- 効果の形式仕様
- 自動テスト生成
- 契約プログラミング統合

### エコシステム拡張

#### 1. DSL統合
- ドメイン特化効果の標準化
- DSLツールチェーンとの統合
- 効果ベースのDSL設計パターン

#### 2. ツール拡張
- 視覚的効果デバッガー
- 効果フローアナライザー
- パフォーマンスプロファイラー

#### 3. ライブラリエコシステム
- 標準効果ライブラリ
- サードパーティ効果パッケージ
- 効果パターンカタログ

## 結論

代数的効果ハンドラのReml言語への導入は技術的に実現可能であり、段階的なアプローチにより実用的な実装が期待できる。既存の効果システム（1.3節）とHM型推論（1.2節）を基盤として、型安全性と性能を両立した設計が可能である。

特に以下の点でRemlに適している：

1. **既存基盤の活用**: 軽量効果システムの自然な拡張
2. **パーサーコンビネーター親和性**: 制御フロー抽象化の統一
3. **段階的導入**: 学習コストと互換性の両立
4. **実用性重視**: 研究的機能の実用化

この計画に従って実装を進めることで、Remlはパーサーコンビネーター特化言語から、より汎用的で表現力豊かな関数型言語へと発展できると考えられる。代数的効果により、例外処理、状態管理、非同期処理、ジェネレータを統一的に扱えるようになり、DSLの表現力と保守性が大幅に向上することが期待される。

---

## 関連資料

- [1.2 型と推論](../1-2-types-Inference.md) - HM型推論システム
- [1.3 効果と安全性](../1-3-effects-safety.md) - 既存効果システム
- [2.1 パーサ型](../2-1-parser-type.md) - パーサーコンビネーター型定義
- [2.2 コアコンビネーター](../2-2-core-combinator.md) - パーサー抽象化
- [2.5 エラー設計](../2-5-error.md) - エラーハンドリング
- [代数的効果実装状況調査](algebraic-effects-handlers-assessment.md) - 調査結果詳細