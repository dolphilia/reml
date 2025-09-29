defmodule Pl0 do
  @moduledoc """
  PL/0 風サブセットの逐次実行器。式は整数演算、ループは `while` のみを扱う。
  """

  @type op :: :add | :sub | :mul | :div
  @type expr :: {:number, integer()} | {:var, String.t()} | {:bin, op(), expr(), expr()}
  @type stmt :: {:assign, String.t(), expr()} | {:while, expr(), [stmt()]} | {:write, expr()}

  @type runtime :: %{vars: %{optional(String.t()) => integer()}, output: [integer()]}
  @type result :: {:ok, runtime()} | {:error, String.t()}

  @spec exec([stmt()]) :: result()
  def exec(program) do
    Enum.reduce_while(program, {:ok, %{vars: %{}, output: []}}, fn stmt, {:ok, rt} ->
      case exec_stmt(stmt, rt) do
        {:ok, next} -> {:cont, {:ok, next}}
        {:error, reason} -> {:halt, {:error, reason}}
      end
    end)
  end

  defp exec_stmt({:assign, name, expr}, rt) do
    with {:ok, value} <- eval_expr(expr, rt.vars) do
      {:ok, %{rt | vars: Map.put(rt.vars, name, value)}}
    end
  end

  defp exec_stmt({:write, expr}, rt) do
    with {:ok, value} <- eval_expr(expr, rt.vars) do
      {:ok, %{rt | output: rt.output ++ [value]}}
    end
  end

  defp exec_stmt({:while, cond, body}, rt), do: exec_while(cond, body, rt)

  defp exec_while(cond, body, rt) do
    with {:ok, value} <- eval_expr(cond, rt.vars) do
      if value == 0 do
        {:ok, rt}
      else
        case Enum.reduce_while(body, {:ok, rt}, fn stmt, {:ok, state} ->
               case exec_stmt(stmt, state) do
                 {:ok, next} -> {:cont, {:ok, next}}
                 {:error, reason} -> {:halt, {:error, reason}}
               end
             end) do
          {:ok, new_state} -> exec_while(cond, body, new_state)
          {:error, reason} -> {:error, reason}
        end
      end
    end
  end

  defp eval_expr({:number, value}, _vars), do: {:ok, value}

  defp eval_expr({:var, name}, vars) do
    case Map.fetch(vars, name) do
      {:ok, value} -> {:ok, value}
      :error -> {:error, "未定義変数: #{name}"}
    end
  end

  defp eval_expr({:bin, op, lhs, rhs}, vars) do
    with {:ok, l} <- eval_expr(lhs, vars),
         {:ok, r} <- eval_expr(rhs, vars) do
      apply_op(op, l, r)
    end
  end

  defp apply_op(:add, a, b), do: {:ok, a + b}
  defp apply_op(:sub, a, b), do: {:ok, a - b}
  defp apply_op(:mul, a, b), do: {:ok, a * b}
  defp apply_op(:div, _a, 0), do: {:error, "0 で割れません"}
  defp apply_op(:div, a, b), do: {:ok, div(a, b)}
end
