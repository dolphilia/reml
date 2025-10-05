// 代数的効果を使うミニ言語 - Swift 版
// Reml との比較: Result 型とクロージャによる効果のエミュレーション

import Foundation

// ミニ言語の式定義
enum Expr {
    case lit(Int)
    case `var`(String)
    case add(Expr, Expr)
    case mul(Expr, Expr)
    case div(Expr, Expr)
    case get
    case put(Expr)
    case fail(String)
    case choose(Expr, Expr)
}

typealias Env = [String: Int]

// 効果の結果型
// State<Int> × Except<String> × Choose を配列で表現
typealias EffectResult = (Int) -> Result<[(Int, Int)], String>

// 効果モナドの操作
enum Effect {
    // return（純粋な値）
    static func pure<A>(_ x: A) -> (Int) -> Result<[(A, Int)], String> {
        { state in .success([(x, state)]) }
    }

    // bind（モナド合成）
    static func flatMap<A, B>(
        _ m: @escaping (Int) -> Result<[(A, Int)], String>,
        _ f: @escaping (A) -> (Int) -> (Int) -> Result<[(B, Int)], String>
    ) -> (Int) -> Result<[(B, Int)], String> {
        { state in
            switch m(state) {
            case .failure(let err):
                return .failure(err)
            case .success(let results):
                return results.reduce(.success([])) { acc, item in
                    let (value, st) = item
                    switch acc {
                    case .failure:
                        return acc
                    case .success(let accList):
                        switch f(value)(st)(st) {
                        case .failure(let err):
                            return .failure(err)
                        case .success(let newResults):
                            return .success(accList + newResults)
                        }
                    }
                }
            }
        }
    }

    // map（関数適用）
    static func map<A, B>(
        _ m: @escaping (Int) -> Result<[(A, Int)], String>,
        _ f: @escaping (A) -> B
    ) -> (Int) -> Result<[(B, Int)], String> {
        { state in
            switch m(state) {
            case .success(let results):
                return .success(results.map { (f($0.0), $0.1) })
            case .failure(let err):
                return .failure(err)
            }
        }
    }

    // State.get
    static var get: EffectResult {
        { state in .success([(state, state)]) }
    }

    // State.put
    static func put(_ newState: Int) -> (Int) -> Result<[((), Int)], String> {
        { _ in .success([((), newState)]) }
    }

    // Except.raise
    static func raise<A>(_ msg: String) -> (Int) -> Result<[(A, Int)], String> {
        { _ in .failure(msg) }
    }

    // Choose（非決定的選択）
    static func choose<A>(
        _ left: @escaping (Int) -> Result<[(A, Int)], String>,
        _ right: @escaping (Int) -> Result<[(A, Int)], String>
    ) -> (Int) -> Result<[(A, Int)], String> {
        { state in
            switch (left(state), right(state)) {
            case (.success(let l), .success(let r)):
                return .success(l + r)
            case (.failure(let err), _):
                return .failure(err)
            case (_, .failure(let err)):
                return .failure(err)
            }
        }
    }
}

// 環境から変数を検索
func lookupEnv(_ name: String, _ env: Env) -> Int? {
    env[name]
}

// 式の評価関数（効果を持つ）
//
// Reml の perform に相当する操作を Effect の関数で記述：
// - Effect.flatMap で連鎖
// - Effect.get, Effect.put, Effect.raise で効果を発行
func eval(_ expr: Expr, _ env: Env) -> EffectResult {
    switch expr {
    case .lit(let n):
        return Effect.pure(n)

    case .var(let name):
        if let value = lookupEnv(name, env) {
            return Effect.pure(value)
        } else {
            return Effect.raise("未定義変数: \(name)")
        }

    case .add(let left, let right):
        return Effect.flatMap(eval(left, env)) { l in { _ in
            Effect.flatMap(eval(right, env)) { r in { _ in
                Effect.pure(l + r)
            }}
        }}

    case .mul(let left, let right):
        return Effect.flatMap(eval(left, env)) { l in { _ in
            Effect.flatMap(eval(right, env)) { r in { _ in
                Effect.pure(l * r)
            }}
        }}

    case .div(let left, let right):
        return Effect.flatMap(eval(left, env)) { l in { _ in
            Effect.flatMap(eval(right, env)) { r in { _ in
                if r == 0 {
                    return Effect.raise("ゼロ除算")
                } else {
                    return Effect.pure(l / r)
                }
            }}
        }}

    case .get:
        return Effect.get

    case .put(let e):
        return Effect.flatMap(eval(e, env)) { v in { _ in
            Effect.flatMap(Effect.put(v)) { _ in { _ in
                Effect.pure(v)
            }}
        }}

    case .fail(let msg):
        return Effect.raise(msg)

    case .choose(let left, let right):
        return Effect.choose(eval(left, env), eval(right, env))
    }
}

// すべての効果を処理して結果を返す
//
// Reml の handle ... do ... do ... に相当するが、
// Swift では関数呼び出しで State × Except × Choose を管理。
func runWithAllEffects(_ expr: Expr, _ env: Env, _ initState: Int) -> Result<[(Int, Int)], String> {
    eval(expr, env)(initState)
}

// テストケース
func exampleExpressions() -> [(String, Expr)] {
    [
        ("単純な加算", .add(.lit(10), .lit(20))),
        ("乗算と除算", .div(.mul(.lit(6), .lit(7)), .lit(2))),
        ("状態の取得", .add(.get, .lit(5))),
        ("状態の更新", .put(.add(.get, .lit(1)))),
        ("ゼロ除算エラー", .div(.lit(10), .lit(0))),
        ("非決定的選択", .choose(.lit(1), .lit(2))),
        ("複雑な例", .add(
            .choose(.lit(10), .lit(20)),
            .put(.add(.get, .lit(1)))
        ))
    ]
}

// テスト実行関数
func runExamples() {
    let examples = exampleExpressions()
    let env: Env = [:]
    let initState = 0

    for (name, expr) in examples {
        print("--- \(name) ---")
        switch runWithAllEffects(expr, env, initState) {
        case .success(let results):
            for (value, state) in results {
                print("  結果: \(value), 状態: \(state)")
            }
        case .failure(let err):
            print("  エラー: \(err)")
        }
    }
}

// Reml との比較メモ:
//
// 1. **効果の表現**
//    Reml: effect State<S> { operation get() -> S; operation put(s: S) -> () }
//    Swift: typealias EffectResult = (Int) -> Result<[(Int, Int)], String>
//    - Reml は言語レベルで効果を定義
//    - Swift は関数型（State モナド風）でエンコード
//
// 2. **ハンドラーの実装**
//    Reml: handler state_handler<A>(init) for State<S> { ... }
//    Swift: Effect enum で flatMap/pure を実装
//    - Reml はハンドラーが明示的で再利用可能
//    - Swift はクロージャのネストが深い
//
// 3. **非決定性の扱い**
//    Reml: choose_handler で分岐を自動収集
//    Swift: Effect.choose で手動でリストを結合
//    - どちらもリストを使うが、Reml の方が宣言的
//
// 4. **型推論**
//    Reml: 効果が型レベルで推論される
//    Swift: 型注釈が必要な場合が多い（特に @escaping クロージャ）
//    - Reml の方が型注釈を省略しやすい
//
// 5. **可読性**
//    Reml: with State<Int>, Except<String>, Choose で効果が明確
//    Swift: クロージャのネストが読みづらい
//    - Reml の方が効果の意図が分かりやすい
//
// 6. **メモリ管理**
//    Reml: 効果システムが自動管理
//    Swift: @escaping クロージャによるキャプチャに注意が必要
//    - Reml の方がメモリ管理が不要
//
// **結論**:
// Swift の Result 型とクロージャは強力だが、代数的効果の表現には向いていない。
// Reml の effect/handler 構文はより直感的で、効果の合成が容易。
// 特にクロージャのネストが深くなる場合、Reml の方が記述性に優れる。

// テスト実行例
// runExamples()