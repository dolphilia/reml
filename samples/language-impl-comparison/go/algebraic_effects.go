package main

import (
	"fmt"
)

// 代数的効果を使うミニ言語 - Go版
// Reml との比較: インターフェースとエラーハンドリングによる効果のエミュレーション

// Expr はミニ言語の式
type EffExpr interface {
	effExpr()
}

type LitExpr struct {
	Value int
}

func (l LitExpr) effExpr() {}

type VarExpr2 struct {
	Name string
}

func (v VarExpr2) effExpr() {}

type AddExpr struct {
	Left  EffExpr
	Right EffExpr
}

func (a AddExpr) effExpr() {}

type MulExpr struct {
	Left  EffExpr
	Right EffExpr
}

func (m MulExpr) effExpr() {}

type DivExpr struct {
	Left  EffExpr
	Right EffExpr
}

func (d DivExpr) effExpr() {}

type GetExpr struct{}

func (g GetExpr) effExpr() {}

type PutExpr struct {
	Expr EffExpr
}

func (p PutExpr) effExpr() {}

type FailExpr struct {
	Message string
}

func (f FailExpr) effExpr() {}

type ChooseExpr struct {
	Left  EffExpr
	Right EffExpr
}

func (c ChooseExpr) effExpr() {}

// Result は効果の結果を表す型
// State<Int> × Except<String> × Choose を表現
type Result struct {
	Value int
	State int
}

// Eval は式を評価（効果を持つ）
//
// Reml の perform に相当する操作を手動で記述：
// - State: state を引数で渡して結果と共に返す
// - Except: error で表現
// - Choose: []Result で複数の結果を収集
func Eval(expr EffExpr, env map[string]int, state int) ([]Result, error) {
	switch e := expr.(type) {
	case LitExpr:
		return []Result{{Value: e.Value, State: state}}, nil

	case VarExpr2:
		if val, ok := env[e.Name]; ok {
			return []Result{{Value: val, State: state}}, nil
		}
		return nil, fmt.Errorf("未定義変数: %s", e.Name)

	case AddExpr:
		leftResults, err := Eval(e.Left, env, state)
		if err != nil {
			return nil, err
		}
		allResults := []Result{}
		for _, lRes := range leftResults {
			rightResults, err := Eval(e.Right, env, lRes.State)
			if err != nil {
				return nil, err
			}
			for _, rRes := range rightResults {
				allResults = append(allResults, Result{
					Value: lRes.Value + rRes.Value,
					State: rRes.State,
				})
			}
		}
		return allResults, nil

	case MulExpr:
		leftResults, err := Eval(e.Left, env, state)
		if err != nil {
			return nil, err
		}
		allResults := []Result{}
		for _, lRes := range leftResults {
			rightResults, err := Eval(e.Right, env, lRes.State)
			if err != nil {
				return nil, err
			}
			for _, rRes := range rightResults {
				allResults = append(allResults, Result{
					Value: lRes.Value * rRes.Value,
					State: rRes.State,
				})
			}
		}
		return allResults, nil

	case DivExpr:
		leftResults, err := Eval(e.Left, env, state)
		if err != nil {
			return nil, err
		}
		allResults := []Result{}
		for _, lRes := range leftResults {
			rightResults, err := Eval(e.Right, env, lRes.State)
			if err != nil {
				return nil, err
			}
			for _, rRes := range rightResults {
				if rRes.Value == 0 {
					return nil, fmt.Errorf("ゼロ除算")
				}
				allResults = append(allResults, Result{
					Value: lRes.Value / rRes.Value,
					State: rRes.State,
				})
			}
		}
		return allResults, nil

	case GetExpr:
		return []Result{{Value: state, State: state}}, nil

	case PutExpr:
		results, err := Eval(e.Expr, env, state)
		if err != nil {
			return nil, err
		}
		newResults := []Result{}
		for _, res := range results {
			newResults = append(newResults, Result{Value: res.Value, State: res.Value})
		}
		return newResults, nil

	case FailExpr:
		return nil, fmt.Errorf(e.Message)

	case ChooseExpr:
		leftResults, err := Eval(e.Left, env, state)
		if err != nil {
			return nil, err
		}
		rightResults, err := Eval(e.Right, env, state)
		if err != nil {
			return nil, err
		}
		return append(leftResults, rightResults...), nil

	default:
		return nil, fmt.Errorf("未知の式型")
	}
}

// RunWithAllEffects はすべての効果を処理して結果を返す
//
// Reml の handle ... do ... do ... に相当するが、
// Go では error と []Result で手動管理
func RunWithAllEffects(expr EffExpr, env map[string]int, initState int) ([]Result, error) {
	return Eval(expr, env, initState)
}

// テストケース
func exampleExpressions() []struct {
	name string
	expr EffExpr
} {
	return []struct {
		name string
		expr EffExpr
	}{
		{"単純な加算", AddExpr{
			Left:  LitExpr{Value: 10},
			Right: LitExpr{Value: 20},
		}},
		{"乗算と除算", DivExpr{
			Left: MulExpr{
				Left:  LitExpr{Value: 6},
				Right: LitExpr{Value: 7},
			},
			Right: LitExpr{Value: 2},
		}},
		{"状態の取得", AddExpr{
			Left:  GetExpr{},
			Right: LitExpr{Value: 5},
		}},
		{"状態の更新", PutExpr{
			Expr: AddExpr{
				Left:  GetExpr{},
				Right: LitExpr{Value: 1},
			},
		}},
		{"ゼロ除算エラー", DivExpr{
			Left:  LitExpr{Value: 10},
			Right: LitExpr{Value: 0},
		}},
		{"非決定的選択", ChooseExpr{
			Left:  LitExpr{Value: 1},
			Right: LitExpr{Value: 2},
		}},
		{"複雑な例", AddExpr{
			Left: ChooseExpr{
				Left:  LitExpr{Value: 10},
				Right: LitExpr{Value: 20},
			},
			Right: PutExpr{
				Expr: AddExpr{
					Left:  GetExpr{},
					Right: LitExpr{Value: 1},
				},
			},
		}},
	}
}

// テスト実行
func runExamples() {
	examples := exampleExpressions()
	env := make(map[string]int)
	initState := 0

	for _, ex := range examples {
		fmt.Printf("--- %s ---\n", ex.name)
		results, err := RunWithAllEffects(ex.expr, env, initState)
		if err != nil {
			fmt.Printf("  エラー: %s\n", err)
		} else {
			for _, res := range results {
				fmt.Printf("  結果: %d, 状態: %d\n", res.Value, res.State)
			}
		}
	}
}

// Reml との比較メモ:
//
// 1. **効果の表現**
//    Reml: effect State<S> { operation get() -> S; operation put(s: S) -> () }
//    Go: type Result struct { Value int; State int }; []Result と error
//    - Reml は言語レベルで効果を定義
//    - Go は構造体とスライスで手動エンコード（ボイラープレートが多い）
//
// 2. **ハンドラーの実装**
//    Reml: handler state_handler<A>(init) for State<S> { ... }
//    Go: Eval 関数内で state を明示的に渡す
//    - Reml はハンドラーが宣言的で再利用可能
//    - Go は手続き的でエラーハンドリングが煩雑
//
// 3. **非決定性の扱い**
//    Reml: choose_handler で分岐を自動収集
//    Go: []Result を手動で管理
//    - Reml は分岐が自然に追跡される
//    - Go は明示的なスライス操作が必要
//
// 4. **型安全性**
//    Reml: 効果が型レベルで追跡される
//    Go: error で安全だが、効果の種類は追跡されない
//    - Reml の方が効果の型が明確
//
// 5. **可読性**
//    Reml: with State<Int>, Except<String>, Choose で効果が明確
//    Go: error と []Result の組み合わせが冗長
//    - Reml の方が効果の意図が分かりやすい
//
// 6. **メモリ管理**
//    Reml: 効果システムがメモリを抽象化
//    Go: スライスの append による再割り当てが頻出
//    - Reml の方がメモリ管理が不要
//
// 7. **エラーハンドリング**
//    Reml: perform Except.raise(msg) で効果として送出
//    Go: error を返して if err != nil でチェック
//    - どちらも明示的だが、Reml の方が効果の合成が柔軟
//
// **結論**:
// Go の error は明示的で安全だが、複雑な効果の合成では冗長になる。
// Reml の代数的効果システムはより宣言的で、効果の種類が型レベルで追跡される。
// 特に状態管理と非決定性の組み合わせで、Reml の方が記述性に優れる。

func main() {
	runExamples()
}
