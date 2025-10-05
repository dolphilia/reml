// 代数的効果を使うミニ言語 - Rust 版
// Reml との比較: Result 型と手動状態管理による効果のエミュレーション

use std::collections::HashMap;

// ミニ言語の式定義
#[derive(Debug, Clone)]
enum Expr {
    Lit(i32),
    Var(String),
    Add(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    Get,
    Put(Box<Expr>),
    Fail(String),
    Choose(Box<Expr>, Box<Expr>),
}

type Env = HashMap<String, i32>;

// 効果の結果型
// State<Int> × Except<String> × Choose をベクタで表現
type EffectResult = Result<Vec<(i32, i32)>, String>;

// 式の評価関数（効果を持つ）
//
// Reml の perform に相当する操作を手動で記述：
// - State: state を引数で渡して結果と共に返す
// - Except: Result<T, String> で表現
// - Choose: Vec<(value, state)> で複数の結果を収集
fn eval(expr: &Expr, env: &Env, state: i32) -> EffectResult {
    match expr {
        Expr::Lit(n) => Ok(vec![(*n, state)]),

        Expr::Var(name) => match env.get(name) {
            Some(&value) => Ok(vec![(value, state)]),
            None => Err(format!("未定義変数: {}", name)),
        },

        Expr::Add(left, right) => {
            let left_results = eval(left, env, state)?;
            let mut all_results = Vec::new();
            for (l_value, l_state) in left_results {
                let right_results = eval(right, env, l_state)?;
                for (r_value, r_state) in right_results {
                    all_results.push((l_value + r_value, r_state));
                }
            }
            Ok(all_results)
        }

        Expr::Mul(left, right) => {
            let left_results = eval(left, env, state)?;
            let mut all_results = Vec::new();
            for (l_value, l_state) in left_results {
                let right_results = eval(right, env, l_state)?;
                for (r_value, r_state) in right_results {
                    all_results.push((l_value * r_value, r_state));
                }
            }
            Ok(all_results)
        }

        Expr::Div(left, right) => {
            let left_results = eval(left, env, state)?;
            let mut all_results = Vec::new();
            for (l_value, l_state) in left_results {
                let right_results = eval(right, env, l_state)?;
                for (r_value, r_state) in right_results {
                    if r_value == 0 {
                        return Err("ゼロ除算".to_string());
                    }
                    all_results.push((l_value / r_value, r_state));
                }
            }
            Ok(all_results)
        }

        Expr::Get => Ok(vec![(state, state)]),

        Expr::Put(e) => {
            let results = eval(e, env, state)?;
            Ok(results.into_iter().map(|(v, _)| (v, v)).collect())
        }

        Expr::Fail(msg) => Err(msg.clone()),

        Expr::Choose(left, right) => {
            let left_results = eval(left, env, state)?;
            let right_results = eval(right, env, state)?;
            Ok([left_results, right_results].concat())
        }
    }
}

// すべての効果を処理して結果を返す
//
// Reml の handle ... do ... do ... に相当するが、
// Rust では Result 型と Vec で手動管理。
fn run_with_all_effects(expr: &Expr, env: &Env, init_state: i32) -> EffectResult {
    eval(expr, env, init_state)
}

// テストケース
fn example_expressions() -> Vec<(&'static str, Expr)> {
    vec![
        ("単純な加算", Expr::Add(
            Box::new(Expr::Lit(10)),
            Box::new(Expr::Lit(20)),
        )),
        ("乗算と除算", Expr::Div(
            Box::new(Expr::Mul(
                Box::new(Expr::Lit(6)),
                Box::new(Expr::Lit(7)),
            )),
            Box::new(Expr::Lit(2)),
        )),
        ("状態の取得", Expr::Add(
            Box::new(Expr::Get),
            Box::new(Expr::Lit(5)),
        )),
        ("状態の更新", Expr::Put(
            Box::new(Expr::Add(
                Box::new(Expr::Get),
                Box::new(Expr::Lit(1)),
            )),
        )),
        ("ゼロ除算エラー", Expr::Div(
            Box::new(Expr::Lit(10)),
            Box::new(Expr::Lit(0)),
        )),
        ("非決定的選択", Expr::Choose(
            Box::new(Expr::Lit(1)),
            Box::new(Expr::Lit(2)),
        )),
        ("複雑な例", Expr::Add(
            Box::new(Expr::Choose(
                Box::new(Expr::Lit(10)),
                Box::new(Expr::Lit(20)),
            )),
            Box::new(Expr::Put(
                Box::new(Expr::Add(
                    Box::new(Expr::Get),
                    Box::new(Expr::Lit(1)),
                )),
            )),
        )),
    ]
}

// テスト実行関数
fn run_examples() {
    let examples = example_expressions();
    let env = HashMap::new();
    let init_state = 0;

    for (name, expr) in examples {
        println!("--- {} ---", name);
        match run_with_all_effects(&expr, &env, init_state) {
            Ok(results) => {
                for (value, state) in results {
                    println!("  結果: {}, 状態: {}", value, state);
                }
            }
            Err(err) => {
                println!("  エラー: {}", err);
            }
        }
    }
}

// Reml との比較メモ:
//
// 1. **効果の表現**
//    Reml: effect State<S> { operation get() -> S; operation put(s: S) -> () }
//    Rust: type EffectResult = Result<Vec<(i32, i32)>, String>
//    - Reml は言語レベルで効果を定義
//    - Rust は Result と Vec で手動エンコード（ボイラープレートが多い）
//
// 2. **ハンドラーの実装**
//    Reml: handler state_handler<A>(init) for State<S> { ... }
//    Rust: eval 関数内で state を明示的に渡す
//    - Reml はハンドラーが宣言的で再利用可能
//    - Rust は手続き的でエラーハンドリングが煩雑
//
// 3. **非決定性の扱い**
//    Reml: choose_handler で分岐を自動収集
//    Rust: Vec<(value, state)> を手動で管理
//    - Reml は分岐が自然に追跡される
//    - Rust は明示的なベクタ操作が必要
//
// 4. **型安全性**
//    Reml: 効果が型レベルで追跡される
//    Rust: Result<T, E> で安全だが、効果の種類は追跡されない
//    - Reml の方が効果の型が明確
//
// 5. **可読性**
//    Reml: with State<Int>, Except<String>, Choose で効果が明確
//    Rust: Result 型と Vec の組み合わせが冗長
//    - Reml の方が効果の意図が分かりやすい
//
// 6. **所有権とライフタイム**
//    Reml: 効果システムが所有権を抽象化
//    Rust: Box<Expr> や clone() が頻出（パフォーマンス懸念）
//    - Reml の方が所有権の管理が不要
//
// 7. **エラーハンドリング**
//    Reml: perform Except.raise(msg) で効果として送出
//    Rust: ? 演算子で Result を伝播
//    - どちらも簡潔だが、Reml の方が効果の合成が柔軟
//
// **結論**:
// Rust の Result 型は安全だが、複雑な効果の合成では煩雑になる。
// Reml の代数的効果システムはより宣言的で、効果の種類が型レベルで追跡される。
// 特に状態管理と非決定性の組み合わせで、Reml の方が記述性に優れる。

// テスト実行例（main 関数）
// fn main() {
//     run_examples();
// }