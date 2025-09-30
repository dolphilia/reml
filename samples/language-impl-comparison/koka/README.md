# Koka 実装サンプル

このディレクトリには、Koka を使用した Reml 比較用の小規模言語実装が含まれています。

## Koka の特徴

- **効果型推論のリファレンス実装**: Microsoft Research による研究言語
- **代数的効果とハンドラー**: Reml と同様の効果システムを持つ
- **高性能**: 参照カウントベースのメモリ管理で、GC不要
- **型レベル効果追跡**: 関数の副作用が型シグネチャに現れる

## Reml との比較ポイント

### 1. **効果システムの設計**

**Koka の効果定義:**
```koka
effect state<s>
  fun get() : s
  fun put(x : s) : ()

effect exn
  fun raise(msg : string) : a
```

**Reml の効果定義:**
```reml
effect State<S> {
  operation get() -> S
  operation put(s: S) -> ()
}

effect Except<E> {
  operation raise(err: E) -> Never
}
```

- 構文は非常に類似
- Koka は `ctl` (control handler) と `fun` (final handler) を区別
- Reml は統一された `handler` 構文

### 2. **効果ハンドラー**

**Koka:**
```koka
fun state-handler(init : s, action : () -> <state<s>|e> a) : e a
  var st := init
  handle(action)
    return x -> x
    fun get() -> resume(st)
    fun put(x) -> { st := x; resume(()) }
```

**Reml:**
```reml
handler state_handler<A>(init_state: S) -> (A, S)
  for State<S>
{
  operation get() resume ->
    let (result, final_state) = resume(init_state)
    (result, final_state)

  operation put(new_state: S) resume ->
    let (result, _) = resume(())
    (result, new_state)

  return value ->
    (value, init_state)
}
```

- Koka は変数 (`var`) による状態管理が自然
- Reml は関数型スタイルで状態を明示的に渡す
- どちらも resumption（継続）の扱いは類似

### 3. **効果型推論**

**Koka:**
```koka
fun eval(expr : expr) : <state<int>, exn, ndet> int
  // 効果が自動的に推論されるが、型シグネチャに明示できる
```

**Reml:**
```reml
fn eval(expr: Expr) -> Int with State<Int>, Except<String>, Choose
  // 効果が自動推論され、型シグネチャは任意
```

- どちらも効果の自動推論をサポート
- Koka は型シグネチャでの明示がより一般的
- Reml は `with` 節でより読みやすく表現

### 4. **パフォーマンス**

**Koka:**
- 参照カウントベースで、予測可能な性能
- ガーベジコレクション不要
- C バックエンドにより高速

**Reml:**
- 実装方式は仕様では規定しない（処理系依存）
- ARC（Automatic Reference Counting）を想定
- 効果の最適化が進めば Koka と同等の性能が期待できる

### 5. **Unicode 処理**

**Koka:**
- 標準ライブラリは UTF-8 対応
- Grapheme 処理は限定的（外部ライブラリが必要）

**Reml:**
- 3層モデル（Byte/Char/Grapheme）が組み込み
- より明示的で安全な Unicode 処理

## 実装予定のサンプル

このディレクトリには以下のサンプルを追加予定：

1. **代数的効果ミニ言語** (`algebraic_effects.kk`)
   - State + Except + Ndet の組み合わせ
   - Reml との効果ハンドラー記法の比較

2. **JSON パーサー** (`json_parser.kk`)
   - 効果を持たないパーサーの例
   - 型推論と性能の比較

3. **簡易評価器** (`evaluator.kk`)
   - 複数の効果を組み合わせた実用例

## 参考資料

- [Koka 公式サイト](https://koka-lang.github.io/)
- [Koka 言語リファレンス](https://koka-lang.github.io/koka/doc/book.html)
- [効果システムの論文](https://www.microsoft.com/en-us/research/project/koka/)
- [Koka GitHub リポジトリ](https://github.com/koka-lang/koka)

## ビルド方法

```bash
# Koka コンパイラをインストール後
koka -c algebraic_effects.kk
koka algebraic_effects.kk
```

> **注記**: Koka は研究言語であり、Reml の効果システム設計の重要な参考実装です。このディレクトリの実装は、両言語の効果システムの違いと類似点を明確化することを目的としています。