package main

import (
	"fmt"
)

// Stmt はPL/0風言語の文
type Stmt interface {
	stmt()
}

type AssignStmt struct {
	Name string
	Expr Expr
}

func (a AssignStmt) stmt() {}

type WhileStmt struct {
	Cond Expr
	Body []Stmt
}

func (w WhileStmt) stmt() {}

type WriteStmt struct {
	Expr Expr
}

func (w WriteStmt) stmt() {}

// Expr は式
type Expr interface {
	expr()
}

type NumberExpr struct {
	Value int64
}

func (n NumberExpr) expr() {}

type VarExpr struct {
	Name string
}

func (v VarExpr) expr() {}

type BinaryExpr struct {
	Op  Op
	Lhs Expr
	Rhs Expr
}

func (b BinaryExpr) expr() {}

// Op は二項演算子
type Op int

const (
	OpAdd Op = iota
	OpSub
	OpMul
	OpDiv
)

// Runtime は実行時環境
type Runtime struct {
	Vars   map[string]int64
	Output []int64
}

func NewRuntime() *Runtime {
	return &Runtime{
		Vars:   make(map[string]int64),
		Output: []int64{},
	}
}

// Exec はプログラムを実行
func Exec(program []Stmt) (*Runtime, error) {
	runtime := NewRuntime()
	for _, stmt := range program {
		if err := execStmt(stmt, runtime); err != nil {
			return nil, err
		}
	}
	return runtime, nil
}

func execStmt(stmt Stmt, runtime *Runtime) error {
	switch s := stmt.(type) {
	case AssignStmt:
		value, err := evalExpr(s.Expr, runtime.Vars)
		if err != nil {
			return err
		}
		runtime.Vars[s.Name] = value

	case WhileStmt:
		for {
			cond, err := evalExpr(s.Cond, runtime.Vars)
			if err != nil {
				return err
			}
			if cond == 0 {
				break
			}
			for _, inner := range s.Body {
				if err := execStmt(inner, runtime); err != nil {
					return err
				}
			}
		}

	case WriteStmt:
		value, err := evalExpr(s.Expr, runtime.Vars)
		if err != nil {
			return err
		}
		runtime.Output = append(runtime.Output, value)

	default:
		return fmt.Errorf("未知の文型")
	}

	return nil
}

func evalExpr(expr Expr, vars map[string]int64) (int64, error) {
	switch e := expr.(type) {
	case NumberExpr:
		return e.Value, nil

	case VarExpr:
		if val, ok := vars[e.Name]; ok {
			return val, nil
		}
		return 0, fmt.Errorf("未定義変数: %s", e.Name)

	case BinaryExpr:
		lhs, err := evalExpr(e.Lhs, vars)
		if err != nil {
			return 0, err
		}
		rhs, err := evalExpr(e.Rhs, vars)
		if err != nil {
			return 0, err
		}

		switch e.Op {
		case OpAdd:
			return lhs + rhs, nil
		case OpSub:
			return lhs - rhs, nil
		case OpMul:
			return lhs * rhs, nil
		case OpDiv:
			if rhs == 0 {
				return 0, fmt.Errorf("ゼロ除算")
			}
			return lhs / rhs, nil
		default:
			return 0, fmt.Errorf("未知の演算子")
		}

	default:
		return 0, fmt.Errorf("未知の式型")
	}
}

// テスト例
func main() {
	// プログラム例:
	// x := 3
	// while x do
	//   write x
	//   x := x - 1
	program := []Stmt{
		AssignStmt{
			Name: "x",
			Expr: NumberExpr{Value: 3},
		},
		WhileStmt{
			Cond: VarExpr{Name: "x"},
			Body: []Stmt{
				WriteStmt{Expr: VarExpr{Name: "x"}},
				AssignStmt{
					Name: "x",
					Expr: BinaryExpr{
						Op:  OpSub,
						Lhs: VarExpr{Name: "x"},
						Rhs: NumberExpr{Value: 1},
					},
				},
			},
		},
	}

	runtime, err := Exec(program)
	if err != nil {
		fmt.Printf("実行エラー: %v\n", err)
		return
	}

	fmt.Printf("出力: %v\n", runtime.Output)
	fmt.Printf("変数: %v\n", runtime.Vars)
}
