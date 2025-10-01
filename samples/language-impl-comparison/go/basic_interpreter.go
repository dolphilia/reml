package main

import (
	"fmt"
	"strings"
)

// Value は実行時の値
type Value interface {
	value()
}

type NumberValue struct {
	Val float64
}

func (n NumberValue) value() {}

type StringValue struct {
	Val string
}

func (s StringValue) value() {}

type ArrayValue struct {
	Elements []Value
}

func (a ArrayValue) value() {}

// Statement は文
type Statement interface {
	stmt()
}

type LetStmt struct {
	Var  string
	Expr Expr
}

func (l LetStmt) stmt() {}

type PrintStmt struct {
	Exprs []Expr
}

func (p PrintStmt) stmt() {}

type IfStmt struct {
	Cond      Expr
	ThenBlock []Statement
	ElseBlock []Statement
}

func (i IfStmt) stmt() {}

type ForStmt struct {
	Var   string
	Start Expr
	End   Expr
	Step  Expr
	Body  []Statement
}

func (f ForStmt) stmt() {}

type WhileStmt struct {
	Cond Expr
	Body []Statement
}

func (w WhileStmt) stmt() {}

type GotoStmt struct {
	Line int
}

func (g GotoStmt) stmt() {}

type GosubStmt struct {
	Line int
}

func (g GosubStmt) stmt() {}

type ReturnStmt struct{}

func (r ReturnStmt) stmt() {}

type DimStmt struct {
	Var  string
	Size Expr
}

func (d DimStmt) stmt() {}

type EndStmt struct{}

func (e EndStmt) stmt() {}

// Expr は式
type Expr interface {
	expr()
}

type NumberExpr struct {
	Val float64
}

func (n NumberExpr) expr() {}

type StringExpr struct {
	Val string
}

func (s StringExpr) expr() {}

type VariableExpr struct {
	Name string
}

func (v VariableExpr) expr() {}

type ArrayAccessExpr struct {
	Var   string
	Index Expr
}

func (a ArrayAccessExpr) expr() {}

type BinOpExpr struct {
	Op    BinOperator
	Left  Expr
	Right Expr
}

func (b BinOpExpr) expr() {}

type UnaryOpExpr struct {
	Op      UnaryOperator
	Operand Expr
}

func (u UnaryOpExpr) expr() {}

// Operators
type BinOperator int

const (
	OpAdd BinOperator = iota
	OpSub
	OpMul
	OpDiv
	OpEq
	OpNe
	OpLt
	OpLe
	OpGt
	OpGe
	OpAnd
	OpOr
)

type UnaryOperator int

const (
	OpNeg UnaryOperator = iota
	OpNot
)

// Program
type ProgramLine struct {
	Line int
	Stmt Statement
}

type Program []ProgramLine

// Runtime state
type RuntimeState struct {
	Env       map[string]Value
	CallStack []int
	Output    []string
}

// Errors
type RuntimeError struct {
	Message string
}

func (e RuntimeError) Error() string {
	return e.Message
}

// Run executes a Basic program
func Run(program Program) ([]string, error) {
	state := RuntimeState{
		Env:       make(map[string]Value),
		CallStack: []int{},
		Output:    []string{},
	}

	return executeProgram(program, 0, state)
}

func executeProgram(program Program, pc int, state RuntimeState) ([]string, error) {
	if pc >= len(program) {
		return state.Output, nil
	}

	stmt := program[pc].Stmt

	switch s := stmt.(type) {
	case EndStmt:
		return state.Output, nil

	case LetStmt:
		val, err := evalExpr(s.Expr, state.Env)
		if err != nil {
			return nil, err
		}
		state.Env[s.Var] = val
		return executeProgram(program, pc+1, state)

	case PrintStmt:
		var parts []string
		for _, expr := range s.Exprs {
			val, err := evalExpr(expr, state.Env)
			if err != nil {
				return nil, err
			}
			parts = append(parts, valueToString(val))
		}
		state.Output = append(state.Output, strings.Join(parts, " "))
		return executeProgram(program, pc+1, state)

	case IfStmt:
		condVal, err := evalExpr(s.Cond, state.Env)
		if err != nil {
			return nil, err
		}
		var branch []Statement
		if isTruthy(condVal) {
			branch = s.ThenBlock
		} else {
			branch = s.ElseBlock
		}
		newState, err := executeBlock(branch, state)
		if err != nil {
			return nil, err
		}
		return executeProgram(program, pc+1, newState)

	case ForStmt:
		startVal, err := evalExpr(s.Start, state.Env)
		if err != nil {
			return nil, err
		}
		endVal, err := evalExpr(s.End, state.Env)
		if err != nil {
			return nil, err
		}
		stepVal, err := evalExpr(s.Step, state.Env)
		if err != nil {
			return nil, err
		}
		return executeForLoop(s.Var, startVal, endVal, stepVal, s.Body, program, pc, state)

	case WhileStmt:
		return executeWhileLoop(s.Cond, s.Body, program, pc, state)

	case GotoStmt:
		newPc, err := findLine(program, s.Line)
		if err != nil {
			return nil, err
		}
		return executeProgram(program, newPc, state)

	case GosubStmt:
		newPc, err := findLine(program, s.Line)
		if err != nil {
			return nil, err
		}
		state.CallStack = append(state.CallStack, pc+1)
		return executeProgram(program, newPc, state)

	case ReturnStmt:
		if len(state.CallStack) == 0 {
			return nil, RuntimeError{"スタックアンダーフロー"}
		}
		returnPc := state.CallStack[len(state.CallStack)-1]
		state.CallStack = state.CallStack[:len(state.CallStack)-1]
		return executeProgram(program, returnPc, state)

	case DimStmt:
		sizeVal, err := evalExpr(s.Size, state.Env)
		if err != nil {
			return nil, err
		}
		if num, ok := sizeVal.(NumberValue); ok {
			size := int(num.Val)
			array := make([]Value, size)
			for i := range array {
				array[i] = NumberValue{0.0}
			}
			state.Env[s.Var] = ArrayValue{array}
			return executeProgram(program, pc+1, state)
		}
		return nil, RuntimeError{"型不一致"}

	default:
		return nil, RuntimeError{"未知の文"}
	}
}

func executeBlock(block []Statement, state RuntimeState) (RuntimeState, error) {
	for _, stmt := range block {
		newState, err := executeSingleStatement(stmt, state)
		if err != nil {
			return state, err
		}
		state = newState
	}
	return state, nil
}

func executeSingleStatement(stmt Statement, state RuntimeState) (RuntimeState, error) {
	switch s := stmt.(type) {
	case LetStmt:
		val, err := evalExpr(s.Expr, state.Env)
		if err != nil {
			return state, err
		}
		state.Env[s.Var] = val
		return state, nil

	case PrintStmt:
		var parts []string
		for _, expr := range s.Exprs {
			val, err := evalExpr(expr, state.Env)
			if err != nil {
				return state, err
			}
			parts = append(parts, valueToString(val))
		}
		state.Output = append(state.Output, strings.Join(parts, " "))
		return state, nil

	default:
		return state, nil
	}
}

func executeForLoop(
	varName string,
	start, end, step Value,
	body []Statement,
	program Program,
	pc int,
	state RuntimeState,
) ([]string, error) {
	startNum, ok1 := start.(NumberValue)
	endNum, ok2 := end.(NumberValue)
	stepNum, ok3 := step.(NumberValue)

	if !ok1 || !ok2 || !ok3 {
		return nil, RuntimeError{"FOR ループには数値が必要です"}
	}

	return forLoopHelper(varName, startNum.Val, endNum.Val, stepNum.Val, body, program, pc, state)
}

func forLoopHelper(
	varName string,
	current, end, step float64,
	body []Statement,
	program Program,
	pc int,
	state RuntimeState,
) ([]string, error) {
	if (step > 0.0 && current > end) || (step < 0.0 && current < end) {
		return executeProgram(program, pc+1, state)
	}

	state.Env[varName] = NumberValue{current}
	newState, err := executeBlock(body, state)
	if err != nil {
		return nil, err
	}

	return forLoopHelper(varName, current+step, end, step, body, program, pc, newState)
}

func executeWhileLoop(
	cond Expr,
	body []Statement,
	program Program,
	pc int,
	state RuntimeState,
) ([]string, error) {
	condVal, err := evalExpr(cond, state.Env)
	if err != nil {
		return nil, err
	}

	if isTruthy(condVal) {
		newState, err := executeBlock(body, state)
		if err != nil {
			return nil, err
		}
		return executeWhileLoop(cond, body, program, pc, newState)
	}

	return executeProgram(program, pc+1, state)
}

func evalExpr(expr Expr, env map[string]Value) (Value, error) {
	switch e := expr.(type) {
	case NumberExpr:
		return NumberValue{e.Val}, nil

	case StringExpr:
		return StringValue{e.Val}, nil

	case VariableExpr:
		val, ok := env[e.Name]
		if !ok {
			return nil, RuntimeError{fmt.Sprintf("未定義変数: %s", e.Name)}
		}
		return val, nil

	case ArrayAccessExpr:
		arrVal, ok := env[e.Var]
		if !ok {
			return nil, RuntimeError{fmt.Sprintf("未定義変数: %s", e.Var)}
		}
		arr, ok := arrVal.(ArrayValue)
		if !ok {
			return nil, RuntimeError{"配列ではありません"}
		}
		idxVal, err := evalExpr(e.Index, env)
		if err != nil {
			return nil, err
		}
		idx, ok := idxVal.(NumberValue)
		if !ok {
			return nil, RuntimeError{"インデックスは数値である必要があります"}
		}
		i := int(idx.Val)
		if i < 0 || i >= len(arr.Elements) {
			return nil, RuntimeError{"インデックス範囲外"}
		}
		return arr.Elements[i], nil

	case BinOpExpr:
		left, err := evalExpr(e.Left, env)
		if err != nil {
			return nil, err
		}
		right, err := evalExpr(e.Right, env)
		if err != nil {
			return nil, err
		}
		return evalBinOp(e.Op, left, right)

	case UnaryOpExpr:
		operand, err := evalExpr(e.Operand, env)
		if err != nil {
			return nil, err
		}
		return evalUnaryOp(e.Op, operand)

	default:
		return nil, RuntimeError{"未知の式"}
	}
}

func evalBinOp(op BinOperator, left, right Value) (Value, error) {
	l, ok1 := left.(NumberValue)
	r, ok2 := right.(NumberValue)

	switch op {
	case OpAdd:
		if ok1 && ok2 {
			return NumberValue{l.Val + r.Val}, nil
		}
	case OpSub:
		if ok1 && ok2 {
			return NumberValue{l.Val - r.Val}, nil
		}
	case OpMul:
		if ok1 && ok2 {
			return NumberValue{l.Val * r.Val}, nil
		}
	case OpDiv:
		if ok1 && ok2 {
			if r.Val == 0.0 {
				return nil, RuntimeError{"0で割ることはできません"}
			}
			return NumberValue{l.Val / r.Val}, nil
		}
	case OpEq:
		if ok1 && ok2 {
			if l.Val == r.Val {
				return NumberValue{1.0}, nil
			}
			return NumberValue{0.0}, nil
		}
	case OpNe:
		if ok1 && ok2 {
			if l.Val != r.Val {
				return NumberValue{1.0}, nil
			}
			return NumberValue{0.0}, nil
		}
	case OpLt:
		if ok1 && ok2 {
			if l.Val < r.Val {
				return NumberValue{1.0}, nil
			}
			return NumberValue{0.0}, nil
		}
	case OpLe:
		if ok1 && ok2 {
			if l.Val <= r.Val {
				return NumberValue{1.0}, nil
			}
			return NumberValue{0.0}, nil
		}
	case OpGt:
		if ok1 && ok2 {
			if l.Val > r.Val {
				return NumberValue{1.0}, nil
			}
			return NumberValue{0.0}, nil
		}
	case OpGe:
		if ok1 && ok2 {
			if l.Val >= r.Val {
				return NumberValue{1.0}, nil
			}
			return NumberValue{0.0}, nil
		}
	case OpAnd:
		if isTruthy(left) && isTruthy(right) {
			return NumberValue{1.0}, nil
		}
		return NumberValue{0.0}, nil
	case OpOr:
		if isTruthy(left) || isTruthy(right) {
			return NumberValue{1.0}, nil
		}
		return NumberValue{0.0}, nil
	}

	return nil, RuntimeError{"型不一致"}
}

func evalUnaryOp(op UnaryOperator, operand Value) (Value, error) {
	switch op {
	case OpNeg:
		if num, ok := operand.(NumberValue); ok {
			return NumberValue{-num.Val}, nil
		}
	case OpNot:
		if isTruthy(operand) {
			return NumberValue{0.0}, nil
		}
		return NumberValue{1.0}, nil
	}

	return nil, RuntimeError{"型不一致"}
}

func isTruthy(value Value) bool {
	switch v := value.(type) {
	case NumberValue:
		return v.Val != 0.0
	case StringValue:
		return v.Val != ""
	case ArrayValue:
		return len(v.Elements) > 0
	default:
		return false
	}
}

func valueToString(value Value) string {
	switch v := value.(type) {
	case NumberValue:
		return fmt.Sprintf("%g", v.Val)
	case StringValue:
		return v.Val
	case ArrayValue:
		return "[Array]"
	default:
		return "?"
	}
}

func findLine(program Program, target int) (int, error) {
	for i, line := range program {
		if line.Line == target {
			return i, nil
		}
	}
	return 0, RuntimeError{fmt.Sprintf("未定義ラベル: %d", target)}
}

// テスト例
func main() {
	program := Program{
		{10, LetStmt{"x", NumberExpr{0.0}}},
		{20, LetStmt{"x", BinOpExpr{
			Op:    OpAdd,
			Left:  VariableExpr{"x"},
			Right: NumberExpr{1.0},
		}}},
		{30, PrintStmt{[]Expr{VariableExpr{"x"}}}},
		{40, IfStmt{
			Cond: BinOpExpr{
				Op:    OpLt,
				Left:  VariableExpr{"x"},
				Right: NumberExpr{5.0},
			},
			ThenBlock: []Statement{GotoStmt{20}},
			ElseBlock: []Statement{},
		}},
		{50, EndStmt{}},
	}

	result, err := Run(program)
	if err != nil {
		fmt.Printf("エラー: %v\n", err)
		return
	}

	for _, line := range result {
		fmt.Println(line)
	}
}
