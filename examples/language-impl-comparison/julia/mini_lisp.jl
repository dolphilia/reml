# ミニ Lisp 評価機 (Julia)
# Julia の特徴: 多重ディスパッチ、メタプログラミング、動的型・静的型のハイブリッド

# S式の定義
abstract type Expr end

struct Num <: Expr
    value::Int
end

struct Sym <: Expr
    name::String
end

struct List <: Expr
    items::Vector{Expr}
end

# 値
abstract type Value end

struct VNum <: Value
    value::Int
end

struct VSym <: Value
    name::String
end

struct VList <: Value
    items::Vector{Value}
end

struct VFunc <: Value
    func::Function
end

struct VNil <: Value end

# 環境
const Env = Dict{String, Value}

# エラー型
struct EvalError <: Exception
    msg::String
end

# S式の文字列化（多重ディスパッチ）
expr_to_string(e::Num) = string(e.value)
expr_to_string(e::Sym) = e.name
expr_to_string(e::List) = "(" * join(map(expr_to_string, e.items), " ") * ")"

# 値の文字列化（多重ディスパッチ）
value_to_string(v::VNum) = string(v.value)
value_to_string(v::VSym) = v.name
value_to_string(v::VList) = "(" * join(map(value_to_string, v.items), " ") * ")"
value_to_string(v::VFunc) = "<function>"
value_to_string(v::VNil) = "nil"

# 簡易パーサー（トークン化）
function tokenize(input::String)::Vector{String}
    replaced = replace(input, "(" => " ( ", ")" => " ) ")
    filter(!isempty, split(replaced))
end

# パース
function parse_expr(tokens::Vector{String})::Tuple{Expr, Vector{String}}
    if isempty(tokens)
        throw(EvalError("Unexpected EOF"))
    end

    token = tokens[1]
    rest = tokens[2:end]

    if token == "("
        return parse_list(rest, Expr[])
    elseif token == ")"
        throw(EvalError("Unexpected ')'"))
    else
        # 数値か記号か判定
        num = tryparse(Int, token)
        if num !== nothing
            return (Num(num), rest)
        else
            return (Sym(token), rest)
        end
    end
end

function parse_list(tokens::Vector{String}, acc::Vector{Expr})::Tuple{Expr, Vector{String}}
    if isempty(tokens)
        throw(EvalError("Unclosed '('"))
    end

    if tokens[1] == ")"
        return (List(acc), tokens[2:end])
    end

    expr, rest = parse_expr(tokens)
    return parse_list(rest, vcat(acc, [expr]))
end

# トップレベルパース
function parse(input::String)::Expr
    tokens = tokenize(input)
    expr, rest = parse_expr(tokens)
    if !isempty(rest)
        throw(EvalError("Extra tokens after expression"))
    end
    return expr
end

# 真偽値判定
is_truthy(v::VNil) = false
is_truthy(v::VNum) = v.value != 0
is_truthy(v::Value) = true

# 式を値に変換（quote用、多重ディスパッチ）
expr_to_value(e::Num) = VNum(e.value)
expr_to_value(e::Sym) = VSym(e.name)
expr_to_value(e::List) = VList(map(expr_to_value, e.items))

# パラメータ名抽出
function extract_param_names(params::Vector{Expr})::Vector{String}
    result = String[]
    for param in params
        if !(param isa Sym)
            throw(EvalError("Lambda parameters must be symbols"))
        end
        push!(result, param.name)
    end
    return result
end

# パラメータ束縛
function bind_params(env::Env, params::Vector{String}, args::Vector{Value})::Env
    if length(params) != length(args)
        throw(EvalError("Argument count mismatch"))
    end

    new_env = copy(env)
    for (name, value) in zip(params, args)
        new_env[name] = value
    end
    return new_env
end

# リスト評価
function eval_list(env::Env, exprs::Vector{Expr})::Vector{Value}
    map(e -> eval_expr(env, e), exprs)
end

# 数値抽出
function extract_numbers(values::Vector{Value})::Vector{Int}
    nums = Int[]
    for v in values
        if !(v isa VNum)
            throw(EvalError("Expected number"))
        end
        push!(nums, v.value)
    end
    return nums
end

# 算術演算
function eval_arithmetic(env::Env, args::Vector{Expr}, op::Function)::Value
    values = eval_list(env, args)
    nums = extract_numbers(values)

    if isempty(nums)
        throw(EvalError("Arithmetic requires at least one argument"))
    end

    result = nums[1]
    for i in 2:length(nums)
        result = op(result, nums[i])
    end
    return VNum(result)
end

# 比較演算
function eval_comparison(env::Env, args::Vector{Expr}, op::Function)::Value
    values = eval_list(env, args)
    nums = extract_numbers(values)

    if length(nums) != 2
        throw(EvalError("Comparison requires exactly 2 arguments"))
    end

    return op(nums[1], nums[2]) ? VNum(1) : VNum(0)
end

# 関数適用
function apply_func(func::Value, args::Vector{Value})::Value
    if !(func isa VFunc)
        throw(EvalError("Not a function"))
    end
    return func.func(args)
end

# 評価（多重ディスパッチを活用）
function eval_expr(env::Env, expr::Num)::Value
    return VNum(expr.value)
end

function eval_expr(env::Env, expr::Sym)::Value
    if haskey(env, expr.name)
        return env[expr.name]
    else
        throw(EvalError("Unbound variable: $(expr.name)"))
    end
end

function eval_expr(env::Env, expr::List)::Value
    items = expr.items

    # 空リスト
    if isempty(items)
        return VNil()
    end

    # 特殊形式: quote
    if length(items) >= 2 && items[1] isa Sym && items[1].name == "quote"
        return expr_to_value(items[2])
    end

    # 特殊形式: if
    if length(items) == 4 && items[1] isa Sym && items[1].name == "if"
        cond_val = eval_expr(env, items[2])
        if is_truthy(cond_val)
            return eval_expr(env, items[3])
        else
            return eval_expr(env, items[4])
        end
    end

    # 特殊形式: define
    if length(items) == 3 && items[1] isa Sym && items[1].name == "define"
        if !(items[2] isa Sym)
            throw(EvalError("define requires a symbol"))
        end
        value = eval_expr(env, items[3])
        # Juliaでは環境の破壊的更新が可能だが、ここでは値を返すのみ
        return value
    end

    # 特殊形式: lambda
    if length(items) == 3 && items[1] isa Sym && items[1].name == "lambda"
        if !(items[2] isa List)
            throw(EvalError("lambda requires parameter list"))
        end

        param_names = extract_param_names(items[2].items)
        body = items[3]

        # クロージャを作成（Juliaの強力なクロージャサポート）
        return VFunc(function(args::Vector{Value})
            new_env = bind_params(env, param_names, args)
            return eval_expr(new_env, body)
        end)
    end

    # 組み込み演算子
    if items[1] isa Sym
        op = items[1].name
        args = items[2:end]

        if op == "+"
            return eval_arithmetic(env, args, +)
        elseif op == "-"
            return eval_arithmetic(env, args, -)
        elseif op == "*"
            return eval_arithmetic(env, args, *)
        elseif op == "="
            return eval_comparison(env, args, ==)
        elseif op == "<"
            return eval_comparison(env, args, <)
        end
    end

    # 関数適用
    func = eval_expr(env, items[1])
    args = eval_list(env, items[2:end])
    return apply_func(func, args)
end

# 初期環境
function initial_env()::Env
    return Dict(
        "nil" => VNil(),
        "t" => VNum(1)
    )
end

# テスト実行
function test(env::Env, input::String, expected::String)
    try
        expr = parse(input)
        value = eval_expr(env, expr)
        result = value_to_string(value)

        if result == expected
            println("PASS: $input = $result")
        else
            println("FAIL: $input = $result (expected: $expected)")
        end
    catch e
        if e isa EvalError
            println("ERROR: $input -> $(e.msg)")
        else
            println("ERROR: $input -> $e")
        end
    end
end

# メイン関数
function main()
    println("=== Mini Lisp Evaluator (Julia) ===")

    env = initial_env()

    # 基本的な式
    test(env, "42", "42")
    test(env, "(+ 1 2 3)", "6")
    test(env, "(- 10 3)", "7")
    test(env, "(* 2 3 4)", "24")

    # 比較
    test(env, "(= 5 5)", "1")
    test(env, "(< 3 5)", "1")

    # quote
    test(env, "(quote (1 2 3))", "(1 2 3)")

    # if式
    test(env, "(if 1 10 20)", "10")
    test(env, "(if 0 10 20)", "20")

    # lambda（Juliaの強力なクロージャ）
    test(env, "((lambda (x) (+ x 1)) 5)", "6")
    test(env, "((lambda (x y) (* x y)) 3 4)", "12")

    println("\nAll tests completed.")
end

# スクリプトとして実行された場合
if abspath(PROGRAM_FILE) == @__FILE__
    main()
end

#=
設計ノート:

このJulia実装は、科学計算・数値計算に強い動的言語でのLisp評価機を示しています。

主な特徴:

1. **多重ディスパッチ（Multiple Dispatch）**
   - Juliaの核心的機能
   - 型による関数オーバーロード:
     ```julia
     eval_expr(env::Env, expr::Num)::Value
     eval_expr(env::Env, expr::Sym)::Value
     eval_expr(env::Env, expr::List)::Value
     ```
   - 実行時に最適な実装を選択

2. **型システム**
   - 抽象型: `abstract type Expr end`
   - 具象型: `struct Num <: Expr`
   - 型アノテーション: `::Int`, `::String`
   - 型推論も強力（省略可能）

3. **パフォーマンス**
   - JIT コンパイル（LLVM）
   - 型安定性により高速化
   - C/Fortran並みの性能を達成可能

4. **メタプログラミング**
   - マクロシステム（`@macro`）
   - この実装では未使用だが、Lisp的なコード生成が可能
   - `quote ... end` でコードをデータとして扱える

5. **クロージャ**
   ```julia
   VFunc(function(args::Vector{Value})
       new_env = bind_params(env, param_names, args)
       return eval_expr(new_env, body)
   end)
   ```
   - envとbodyをキャプチャした完全なクロージャ
   - 高階関数の自然な実装

6. **エラーハンドリング**
   - 例外ベース（`throw`, `try-catch`）
   - カスタム例外型: `struct EvalError <: Exception`

7. **ベクトル操作**
   - `Vector{Expr}`, `Vector{Value}`
   - 効率的な配列操作
   - `map`, `filter`, `zip` 等の高階関数

他言語との比較:

**Gleam（BEAM VM）**:
- 不変データ構造（必須）
- Result型（明示的）
- 並行処理に強い

**Idris 2（依存型）**:
- 型レベル証明
- 全域性チェック
- コンパイル時検証

**Julia（科学計算）**:
- 可変データ構造（選択可）
- 例外ベース
- 数値計算に強い
- 多重ディスパッチ

実用上の拡張:

- **型安定性の向上**: `@code_warntype` で検証
- **SIMD最適化**: `@simd` マクロ
- **並列化**: `@threads`, `@distributed`
- **C/Fortranとの連携**: `ccall`
- **GPU計算**: CUDA.jl, AMDGPU.jl

Juliaは「2言語問題」（プロトタイプはPython、本番はC++）を解決するため設計され、
高レベル記述と高性能を両立させています。この実装では基本的な機能のみですが、
多重ディスパッチによる拡張性の高さを示しています。
=#
