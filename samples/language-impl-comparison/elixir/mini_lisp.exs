defmodule MiniLisp do
  @moduledoc """
  ミニ Lisp 評価機。S 式をトークン列に分割し、再帰下降で解析・評価する。
  """

  @type expr ::
          {:number, float()}
          | {:symbol, String.t()}
          | {:list, [expr()]}

  @type value ::
          {:number, float()}
          | {:lambda, [String.t()], expr(), env()}
          | {:builtin, ( [value()] -> result(value()) )}

  @type env :: %{optional(String.t()) => value()}
  @type result(t) :: {:ok, t} | {:error, String.t()}

  @spec eval(String.t()) :: result(value())
  def eval(source) do
    tokens = tokenize(source)

    with {:ok, {expr, []}} <- parse_expr(tokens),
         {:ok, value} <- eval_expr(expr, default_env()) do
      {:ok, value}
    else
      {:ok, {_expr, rest}} -> {:error, "未消費トークンがあります: #{Enum.join(rest, " ")}"}
      {:error, reason} -> {:error, reason}
    end
  end

  @spec tokenize(String.t()) :: [String.t()]
  def tokenize(source) do
    source
    |> String.replace("(", " ( ")
    |> String.replace(")", " ) ")
    |> String.split()
  end

  @spec parse_expr([String.t()]) :: result({expr(), [String.t()]})
  def parse_expr([]), do: {:error, "入力が空です"}

  def parse_expr(["(" | rest]), do: parse_list(rest, [])

  def parse_expr([")" | _]), do: {:error, "対応しない閉じ括弧です"}

  def parse_expr([token | rest]) do
    {:ok, {atom(token), rest}}
  end

  defp atom(token) do
    case Float.parse(token) do
      {number, ""} -> {:number, number}
      _ -> {:symbol, token}
    end
  end

  defp parse_list([], _acc), do: {:error, "リストが閉じられていません"}

  defp parse_list([")" | rest], acc), do: {:ok, {{:list, Enum.reverse(acc)}, rest}}

  defp parse_list(tokens, acc) do
    with {:ok, {expr, rest}} <- parse_expr(tokens) do
      parse_list(rest, [expr | acc])
    end
  end

  defp eval_expr({:number, n}, _env), do: {:ok, {:number, n}}

  defp eval_expr({:symbol, name}, env) do
    case Map.fetch(env, name) do
      {:ok, value} -> {:ok, value}
      :error -> {:error, "未定義シンボル: #{name}"}
    end
  end

  defp eval_expr({:list, []}, _env), do: {:error, "空の式は評価できません"}

  defp eval_expr({:list, [{:symbol, "lambda"}, {:list, params}, body]}, env) do
    with {:ok, names} <- ensure_symbols(params) do
      {:ok, {:lambda, names, body, env}}
    end
  end

  defp eval_expr({:list, [{:symbol, "if"}, cond, then_branch, else_branch]}, env) do
    with {:ok, {:number, cond_value}} <- eval_expr(cond, env) do
      if cond_value == 0 do
        eval_expr(else_branch, env)
      else
        eval_expr(then_branch, env)
      end
    end
  end

  defp eval_expr({:list, [head | tail]}, env) do
    with {:ok, fun} <- eval_expr(head, env),
         {:ok, args} <- eval_args(tail, env) do
      apply_fun(fun, args)
    end
  end

  defp eval_args(exprs, env) do
    exprs
    |> Enum.reduce_while({:ok, []}, fn expr, {:ok, acc} ->
      case eval_expr(expr, env) do
        {:ok, value} -> {:cont, {:ok, [value | acc]}}
        {:error, reason} -> {:halt, {:error, reason}}
      end
    end)
    |> case do
      {:ok, values} -> {:ok, Enum.reverse(values)}
      other -> other
    end
  end

  defp apply_fun({:builtin, fun}, args), do: fun.(args)

  defp apply_fun({:lambda, params, body, captured}, args) do
    if length(params) != length(args) do
      {:error, "引数の数が一致しません"}
    else
      new_bindings = Enum.zip(params, args) |> Map.new()
      merged = Map.merge(captured, new_bindings)
      eval_expr(body, merged)
    end
  end

  defp apply_fun({:number, _}, _), do: {:error, "数値は適用できません"}

  defp ensure_symbols(list) do
    list
    |> Enum.reduce_while({:ok, []}, fn
      {:symbol, name}, {:ok, acc} -> {:cont, {:ok, [name | acc]}}
      other, _ -> {:halt, {:error, "シンボルを期待しましたが #{inspect(other)} でした"}}
    end)
    |> case do
      {:ok, names} -> {:ok, Enum.reverse(names)}
      error -> error
    end
  end

  defp default_env do
    %{
      "+" => numeric2(&Kernel.+/2),
      "-" => numeric2(&Kernel.-/2),
      "*" => numeric2(&Kernel.*/2),
      "/" => numeric2(fn _a, 0 -> {:error, "0 で割れません"}; a, b -> {:ok, a / b} end),
      ">" => numeric2(fn a, b -> {:ok, if(a > b, do: 1.0, else: 0.0)} end),
      "=" => numeric2(fn a, b -> {:ok, if(a == b, do: 1.0, else: 0.0)} end)
    }
  end

  defp numeric2(fun) do
    {:builtin,
     fn
       [{:number, a}, {:number, b}] ->
         case fun.(a, b) do
           {:ok, result} -> {:ok, {:number, result}}
           {:error, reason} -> {:error, reason}
           result when is_number(result) -> {:ok, {:number, result}}
         end
       _ -> {:error, "数値 2 引数を期待します"}
     end}
  end
end
