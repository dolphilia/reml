package main

import (
	"fmt"
	"strconv"
	"strings"
)

// Expr はLisp式を表すAST
type Expr interface {
	expr()
}

type Number struct {
	Value float64
}

func (n Number) expr() {}

type Symbol struct {
	Name string
}

func (s Symbol) expr() {}

type List struct {
	Items []Expr
}

func (l List) expr() {}

// Value は評価結果の値
type Value interface {
	value()
}

type NumValue struct {
	Val float64
}

func (n NumValue) value() {}

type LambdaValue struct {
	Params []string
	Body   Expr
	Env    map[string]Value
}

func (l LambdaValue) value() {}

type BuiltinFunc func([]Value) (Value, error)

type BuiltinValue struct {
	Fn BuiltinFunc
}

func (b BuiltinValue) value() {}

// トークン化
func tokenize(source string) []string {
	source = strings.ReplaceAll(source, "(", " ( ")
	source = strings.ReplaceAll(source, ")", " ) ")
	tokens := strings.Fields(source)
	return tokens
}

// パース
func parseExpr(tokens []string, index int) (Expr, int, error) {
	if index >= len(tokens) {
		return nil, index, fmt.Errorf("入力が空です")
	}

	token := tokens[index]
	switch token {
	case "(":
		return parseList(tokens, index+1)
	case ")":
		return nil, index, fmt.Errorf("対応しない閉じ括弧です")
	default:
		return parseAtom(token), index + 1, nil
	}
}

func parseList(tokens []string, index int) (Expr, int, error) {
	items := []Expr{}
	for index < len(tokens) {
		if tokens[index] == ")" {
			return List{Items: items}, index + 1, nil
		}
		expr, next, err := parseExpr(tokens, index)
		if err != nil {
			return nil, index, err
		}
		items = append(items, expr)
		index = next
	}
	return nil, index, fmt.Errorf("リストが閉じられていません")
}

func parseAtom(token string) Expr {
	if num, err := strconv.ParseFloat(token, 64); err == nil {
		return Number{Value: num}
	}
	return Symbol{Name: token}
}

// 評価
func evalExpr(expr Expr, env map[string]Value) (Value, error) {
	switch e := expr.(type) {
	case Number:
		return NumValue{Val: e.Value}, nil

	case Symbol:
		if val, ok := env[e.Name]; ok {
			return val, nil
		}
		return nil, fmt.Errorf("未定義シンボル: %s", e.Name)

	case List:
		if len(e.Items) == 0 {
			return nil, fmt.Errorf("空の式は評価できません")
		}

		// 特殊形式: lambda
		if sym, ok := e.Items[0].(Symbol); ok && sym.Name == "lambda" {
			if len(e.Items) != 3 {
				return nil, fmt.Errorf("lambda は (lambda (params...) body) の形式です")
			}
			paramsList, ok := e.Items[1].(List)
			if !ok {
				return nil, fmt.Errorf("lambda のパラメータはリストである必要があります")
			}
			params := []string{}
			for _, p := range paramsList.Items {
				if s, ok := p.(Symbol); ok {
					params = append(params, s.Name)
				} else {
					return nil, fmt.Errorf("lambda のパラメータはシンボルである必要があります")
				}
			}
			// 環境をキャプチャ
			captured := make(map[string]Value)
			for k, v := range env {
				captured[k] = v
			}
			return LambdaValue{Params: params, Body: e.Items[2], Env: captured}, nil
		}

		// 関数呼び出し
		callee, err := evalExpr(e.Items[0], env)
		if err != nil {
			return nil, err
		}

		args := []Value{}
		for _, argExpr := range e.Items[1:] {
			argVal, err := evalExpr(argExpr, env)
			if err != nil {
				return nil, err
			}
			args = append(args, argVal)
		}

		return apply(callee, args)
	}

	return nil, fmt.Errorf("未知の式型")
}

func apply(callee Value, args []Value) (Value, error) {
	switch c := callee.(type) {
	case BuiltinValue:
		return c.Fn(args)

	case LambdaValue:
		if len(c.Params) != len(args) {
			return nil, fmt.Errorf("引数の数が一致しません: 期待 %d, 実際 %d", len(c.Params), len(args))
		}
		// 新しい環境を作成
		newEnv := make(map[string]Value)
		for k, v := range c.Env {
			newEnv[k] = v
		}
		for i, param := range c.Params {
			newEnv[param] = args[i]
		}
		return evalExpr(c.Body, newEnv)

	case NumValue:
		return nil, fmt.Errorf("数値を関数適用できません")

	default:
		return nil, fmt.Errorf("適用できない値です")
	}
}

// デフォルト環境
func defaultEnv() map[string]Value {
	env := make(map[string]Value)

	env["+"] = BuiltinValue{Fn: numericOp(func(a, b float64) (float64, error) {
		return a + b, nil
	})}

	env["-"] = BuiltinValue{Fn: numericOp(func(a, b float64) (float64, error) {
		return a - b, nil
	})}

	env["*"] = BuiltinValue{Fn: numericOp(func(a, b float64) (float64, error) {
		return a * b, nil
	})}

	env["/"] = BuiltinValue{Fn: numericOp(func(a, b float64) (float64, error) {
		if b == 0 {
			return 0, fmt.Errorf("0 で割れません")
		}
		return a / b, nil
	})}

	return env
}

func numericOp(op func(float64, float64) (float64, error)) BuiltinFunc {
	return func(args []Value) (Value, error) {
		if len(args) != 2 {
			return nil, fmt.Errorf("2 引数で呼び出してください")
		}
		lhs, ok1 := args[0].(NumValue)
		rhs, ok2 := args[1].(NumValue)
		if !ok1 || !ok2 {
			return nil, fmt.Errorf("数値以外を演算できません")
		}
		result, err := op(lhs.Val, rhs.Val)
		if err != nil {
			return nil, err
		}
		return NumValue{Val: result}, nil
	}
}

// エントリーポイント
func Eval(source string) (Value, error) {
	tokens := tokenize(source)
	expr, rest, err := parseExpr(tokens, 0)
	if err != nil {
		return nil, err
	}
	if rest != len(tokens) {
		return nil, fmt.Errorf("未消費トークンがあります")
	}
	env := defaultEnv()
	return evalExpr(expr, env)
}

// テスト例
func main() {
	testCases := []string{
		"(+ 40 2)",
		"(* 6 7)",
		"(/ 84 2)",
		"(- 50 8)",
	}

	for _, tc := range testCases {
		result, err := Eval(tc)
		if err != nil {
			fmt.Printf("エラー: %s => %v\n", tc, err)
		} else {
			if num, ok := result.(NumValue); ok {
				fmt.Printf("%s => %.0f\n", tc, num.Val)
			} else {
				fmt.Printf("%s => %v\n", tc, result)
			}
		}
	}
}
