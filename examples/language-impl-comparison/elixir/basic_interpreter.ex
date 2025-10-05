defmodule BasicInterpreter do
  @moduledoc """
  Basic言語インタープリタのElixir実装
  パターンマッチとパイプラインを活用した関数型スタイル
  """

  # ============================================================================
  # データ型定義
  # ============================================================================

  defmodule Value do
    @moduledoc "実行時の値"

    defstruct [:type, :data]

    def number(n), do: %Value{type: :number, data: n}
    def string(s), do: %Value{type: :string, data: s}
    def array(arr), do: %Value{type: :array, data: arr}

    def to_string(%Value{type: :number, data: n}), do: Float.to_string(n)
    def to_string(%Value{type: :string, data: s}), do: s
    def to_string(%Value{type: :array}), do: "[Array]"

    def truthy?(%Value{type: :number, data: n}), do: n != 0.0
    def truthy?(%Value{type: :string, data: s}), do: s != ""
    def truthy?(%Value{type: :array, data: arr}), do: length(arr) > 0
  end

  defmodule Statement do
    @moduledoc "文"

    defstruct [:type, :data]

    def let(var, expr), do: %Statement{type: :let, data: %{var: var, expr: expr}}
    def print(exprs), do: %Statement{type: :print, data: %{exprs: exprs}}
    def if_stmt(cond, then_block, else_block),
      do: %Statement{type: :if, data: %{cond: cond, then_block: then_block, else_block: else_block}}
    def for_stmt(var, start, end_val, step, body),
      do: %Statement{type: :for, data: %{var: var, start: start, end: end_val, step: step, body: body}}
    def while_stmt(cond, body), do: %Statement{type: :while, data: %{cond: cond, body: body}}
    def goto(line), do: %Statement{type: :goto, data: %{line: line}}
    def gosub(line), do: %Statement{type: :gosub, data: %{line: line}}
    def return_stmt(), do: %Statement{type: :return, data: %{}}
    def dim(var, size), do: %Statement{type: :dim, data: %{var: var, size: size}}
    def end_stmt(), do: %Statement{type: :end, data: %{}}
  end

  defmodule Expr do
    @moduledoc "式"

    defstruct [:type, :data]

    def number(n), do: %Expr{type: :number, data: %{val: n}}
    def string(s), do: %Expr{type: :string, data: %{val: s}}
    def variable(name), do: %Expr{type: :variable, data: %{name: name}}
    def array_access(var, index), do: %Expr{type: :array_access, data: %{var: var, index: index}}
    def binary_op(op, left, right), do: %Expr{type: :binary_op, data: %{op: op, left: left, right: right}}
    def unary_op(op, operand), do: %Expr{type: :unary_op, data: %{op: op, operand: operand}}
  end

  defmodule RuntimeState do
    @moduledoc "実行時の状態"

    defstruct env: %{}, call_stack: [], output: []

    def new(), do: %RuntimeState{}

    def add_output(%RuntimeState{output: output} = state, line) do
      %RuntimeState{state | output: output ++ [line]}
    end

    def set_var(%RuntimeState{env: env} = state, name, value) do
      %RuntimeState{state | env: Map.put(env, name, value)}
    end

    def get_var(%RuntimeState{env: env}, name) do
      Map.get(env, name)
    end

    def push_call(%RuntimeState{call_stack: stack} = state, pc) do
      %RuntimeState{state | call_stack: stack ++ [pc]}
    end

    def pop_call(%RuntimeState{call_stack: []} = state) do
      {:error, :stack_underflow, state}
    end

    def pop_call(%RuntimeState{call_stack: stack} = state) do
      {return_pc, new_stack} = List.pop_at(stack, -1)
      {:ok, return_pc, %RuntimeState{state | call_stack: new_stack}}
    end
  end

  # ============================================================================
  # プログラム実行
  # ============================================================================

  @doc "プログラムを実行してアウトプットを返す"
  def run(program) do
    case execute_program(program, 0, RuntimeState.new()) do
      {:ok, state} -> {:ok, state.output}
      {:error, reason} -> {:error, reason}
    end
  end

  # プログラムカウンタが範囲外 → 正常終了
  defp execute_program(program, pc, state) when pc >= length(program) do
    {:ok, state}
  end

  # 各文を実行
  defp execute_program(program, pc, state) do
    {_line_num, stmt} = Enum.at(program, pc)

    case stmt.type do
      :end ->
        {:ok, state}

      :let ->
        %{var: var, expr: expr} = stmt.data
        with {:ok, value} <- eval_expr(expr, state.env) do
          execute_program(program, pc + 1, RuntimeState.set_var(state, var, value))
        end

      :print ->
        %{exprs: exprs} = stmt.data
        with {:ok, values} <- eval_exprs(exprs, state.env) do
          text = values |> Enum.map(&Value.to_string/1) |> Enum.join(" ")
          execute_program(program, pc + 1, RuntimeState.add_output(state, text))
        end

      :if ->
        %{cond: cond, then_block: then_block, else_block: else_block} = stmt.data
        with {:ok, cond_val} <- eval_expr(cond, state.env) do
          branch = if Value.truthy?(cond_val), do: then_block, else: else_block
          with {:ok, new_state} <- execute_block(branch, state) do
            execute_program(program, pc + 1, new_state)
          end
        end

      :for ->
        %{var: var, start: start_expr, end: end_expr, step: step_expr, body: body} = stmt.data
        with {:ok, start_val} <- eval_expr(start_expr, state.env),
             {:ok, end_val} <- eval_expr(end_expr, state.env),
             {:ok, step_val} <- eval_expr(step_expr, state.env) do
          execute_for_loop(var, start_val, end_val, step_val, body, program, pc, state)
        end

      :while ->
        %{cond: cond, body: body} = stmt.data
        execute_while_loop(cond, body, program, pc, state)

      :goto ->
        %{line: target} = stmt.data
        case find_line(program, target) do
          {:ok, new_pc} -> execute_program(program, new_pc, state)
          error -> error
        end

      :gosub ->
        %{line: target} = stmt.data
        case find_line(program, target) do
          {:ok, new_pc} ->
            new_state = RuntimeState.push_call(state, pc + 1)
            execute_program(program, new_pc, new_state)
          error -> error
        end

      :return ->
        case RuntimeState.pop_call(state) do
          {:ok, return_pc, new_state} -> execute_program(program, return_pc, new_state)
          {:error, reason, _} -> {:error, reason}
        end

      :dim ->
        %{var: var, size: size_expr} = stmt.data
        with {:ok, %Value{type: :number, data: size}} <- eval_expr(size_expr, state.env) do
          array = List.duplicate(Value.number(0.0), trunc(size))
          execute_program(program, pc + 1, RuntimeState.set_var(state, var, Value.array(array)))
        else
          _ -> {:error, {:type_mismatch, "Number", "Other"}}
        end
    end
  end

  # ブロック実行（IF/FOR/WHILEの中身）
  defp execute_block([], state), do: {:ok, state}

  defp execute_block([stmt | rest], state) do
    case execute_single_statement(stmt, state) do
      {:ok, new_state} -> execute_block(rest, new_state)
      error -> error
    end
  end

  # 単一文の実行（ブロック内）
  defp execute_single_statement(stmt, state) do
    case stmt.type do
      :let ->
        %{var: var, expr: expr} = stmt.data
        with {:ok, value} <- eval_expr(expr, state.env) do
          {:ok, RuntimeState.set_var(state, var, value)}
        end

      :print ->
        %{exprs: exprs} = stmt.data
        with {:ok, values} <- eval_exprs(exprs, state.env) do
          text = values |> Enum.map(&Value.to_string/1) |> Enum.join(" ")
          {:ok, RuntimeState.add_output(state, text)}
        end

      _ ->
        {:ok, state}
    end
  end

  # FORループ
  defp execute_for_loop(var, %Value{type: :number, data: start}, %Value{type: :number, data: end_val},
                        %Value{type: :number, data: step}, body, program, pc, state) do
    for_loop_helper(var, start, end_val, step, body, program, pc, state)
  end

  defp execute_for_loop(_, _, _, _, _, _, _, _) do
    {:error, {:type_mismatch, "Number", "Other"}}
  end

  defp for_loop_helper(var, current, end_val, step, body, program, pc, state) do
    cond do
      (step > 0.0 and current > end_val) or (step < 0.0 and current < end_val) ->
        execute_program(program, pc + 1, state)

      true ->
        new_state = RuntimeState.set_var(state, var, Value.number(current))
        with {:ok, loop_state} <- execute_block(body, new_state) do
          for_loop_helper(var, current + step, end_val, step, body, program, pc, loop_state)
        end
    end
  end

  # WHILEループ
  defp execute_while_loop(cond, body, program, pc, state) do
    with {:ok, cond_val} <- eval_expr(cond, state.env) do
      if Value.truthy?(cond_val) do
        with {:ok, new_state} <- execute_block(body, state) do
          execute_while_loop(cond, body, program, pc, new_state)
        end
      else
        execute_program(program, pc + 1, state)
      end
    end
  end

  # ============================================================================
  # 式評価
  # ============================================================================

  defp eval_exprs(exprs, env) do
    exprs
    |> Enum.reduce_while({:ok, []}, fn expr, {:ok, acc} ->
      case eval_expr(expr, env) do
        {:ok, val} -> {:cont, {:ok, acc ++ [val]}}
        error -> {:halt, error}
      end
    end)
  end

  defp eval_expr(%Expr{type: :number, data: %{val: n}}, _env) do
    {:ok, Value.number(n)}
  end

  defp eval_expr(%Expr{type: :string, data: %{val: s}}, _env) do
    {:ok, Value.string(s)}
  end

  defp eval_expr(%Expr{type: :variable, data: %{name: name}}, env) do
    case Map.get(env, name) do
      nil -> {:error, {:undefined_variable, name}}
      val -> {:ok, val}
    end
  end

  defp eval_expr(%Expr{type: :array_access, data: %{var: var, index: index_expr}}, env) do
    case Map.get(env, var) do
      nil ->
        {:error, {:undefined_variable, var}}

      %Value{type: :array, data: arr} ->
        with {:ok, %Value{type: :number, data: idx}} <- eval_expr(index_expr, env) do
          index = trunc(idx)
          if index >= 0 and index < length(arr) do
            {:ok, Enum.at(arr, index)}
          else
            {:error, :index_out_of_bounds}
          end
        else
          _ -> {:error, {:type_mismatch, "Number", "Other"}}
        end

      _ ->
        {:error, {:type_mismatch, "Array", "Other"}}
    end
  end

  defp eval_expr(%Expr{type: :binary_op, data: %{op: op, left: left, right: right}}, env) do
    with {:ok, left_val} <- eval_expr(left, env),
         {:ok, right_val} <- eval_expr(right, env) do
      eval_binary_op(op, left_val, right_val)
    end
  end

  defp eval_expr(%Expr{type: :unary_op, data: %{op: op, operand: operand}}, env) do
    with {:ok, val} <- eval_expr(operand, env) do
      eval_unary_op(op, val)
    end
  end

  # 二項演算子評価
  defp eval_binary_op(:add, %Value{type: :number, data: l}, %Value{type: :number, data: r}) do
    {:ok, Value.number(l + r)}
  end

  defp eval_binary_op(:sub, %Value{type: :number, data: l}, %Value{type: :number, data: r}) do
    {:ok, Value.number(l - r)}
  end

  defp eval_binary_op(:mul, %Value{type: :number, data: l}, %Value{type: :number, data: r}) do
    {:ok, Value.number(l * r)}
  end

  defp eval_binary_op(:div, %Value{type: :number, data: _l}, %Value{type: :number, data: 0.0}) do
    {:error, :division_by_zero}
  end

  defp eval_binary_op(:div, %Value{type: :number, data: l}, %Value{type: :number, data: r}) do
    {:ok, Value.number(l / r)}
  end

  defp eval_binary_op(:eq, %Value{type: :number, data: l}, %Value{type: :number, data: r}) do
    {:ok, Value.number(if l == r, do: 1.0, else: 0.0)}
  end

  defp eval_binary_op(:ne, %Value{type: :number, data: l}, %Value{type: :number, data: r}) do
    {:ok, Value.number(if l != r, do: 1.0, else: 0.0)}
  end

  defp eval_binary_op(:lt, %Value{type: :number, data: l}, %Value{type: :number, data: r}) do
    {:ok, Value.number(if l < r, do: 1.0, else: 0.0)}
  end

  defp eval_binary_op(:le, %Value{type: :number, data: l}, %Value{type: :number, data: r}) do
    {:ok, Value.number(if l <= r, do: 1.0, else: 0.0)}
  end

  defp eval_binary_op(:gt, %Value{type: :number, data: l}, %Value{type: :number, data: r}) do
    {:ok, Value.number(if l > r, do: 1.0, else: 0.0)}
  end

  defp eval_binary_op(:ge, %Value{type: :number, data: l}, %Value{type: :number, data: r}) do
    {:ok, Value.number(if l >= r, do: 1.0, else: 0.0)}
  end

  defp eval_binary_op(:and, left, right) do
    result = if Value.truthy?(left) and Value.truthy?(right), do: 1.0, else: 0.0
    {:ok, Value.number(result)}
  end

  defp eval_binary_op(:or, left, right) do
    result = if Value.truthy?(left) or Value.truthy?(right), do: 1.0, else: 0.0
    {:ok, Value.number(result)}
  end

  defp eval_binary_op(_, _, _) do
    {:error, {:type_mismatch, "Number", "Other"}}
  end

  # 単項演算子評価
  defp eval_unary_op(:neg, %Value{type: :number, data: n}) do
    {:ok, Value.number(-n)}
  end

  defp eval_unary_op(:not, val) do
    result = if Value.truthy?(val), do: 0.0, else: 1.0
    {:ok, Value.number(result)}
  end

  defp eval_unary_op(_, _) do
    {:error, {:type_mismatch, "Number", "Other"}}
  end

  # ============================================================================
  # ユーティリティ
  # ============================================================================

  defp find_line(program, target) do
    case Enum.find_index(program, fn {line, _} -> line == target end) do
      nil -> {:error, {:undefined_label, target}}
      idx -> {:ok, idx}
    end
  end
end

# ============================================================================
# テスト例
# ============================================================================

defmodule BasicInterpreter.Example do
  alias BasicInterpreter.{Statement, Expr, Value}

  def run_example() do
    # 10 LET x = 0
    # 20 LET x = x + 1
    # 30 PRINT x
    # 40 IF x < 5 THEN GOTO 20
    # 50 END

    program = [
      {10, Statement.let("x", Expr.number(0.0))},
      {20, Statement.let("x", Expr.binary_op(:add, Expr.variable("x"), Expr.number(1.0)))},
      {30, Statement.print([Expr.variable("x")])},
      {40, Statement.if_stmt(
        Expr.binary_op(:lt, Expr.variable("x"), Expr.number(5.0)),
        [Statement.goto(20)],
        []
      )},
      {50, Statement.end_stmt()}
    ]

    case BasicInterpreter.run(program) do
      {:ok, output} ->
        IO.puts("実行結果:")
        Enum.each(output, &IO.puts/1)

      {:error, reason} ->
        IO.puts("エラー: #{inspect(reason)}")
    end
  end
end
