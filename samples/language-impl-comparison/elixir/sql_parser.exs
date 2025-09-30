#!/usr/bin/env elixir

# 簡易SQL Parser
# SELECT, WHERE, JOIN, ORDER BY など基本的な構文のみ対応

defmodule SQLParser do
  # AST定義
  defmodule Query do
    defstruct [:type, :columns, :from, :where, :joins, :order_by]
  end

  defmodule Column do
    defstruct [:type, :expr, :alias]
  end

  defmodule TableRef do
    defstruct [:table, :alias]
  end

  defmodule Join do
    defstruct [:join_type, :table, :on_condition]
  end

  defmodule Expr do
    defstruct [:type, :value, :left, :right, :op, :name, :args, :table, :column]
  end

  # パーサーコンビネーター
  defmodule Parser do
    defstruct [:input, :pos]

    def new(input), do: %Parser{input: input, pos: 0}

    def current(%Parser{input: input, pos: pos}) when pos >= String.length(input), do: nil
    def current(%Parser{input: input, pos: pos}), do: String.at(input, pos)

    def advance(%Parser{} = p, n \\ 1), do: %{p | pos: p.pos + n}

    def skip_whitespace(%Parser{} = p) do
      skip_while(p, fn c -> c in [?\s, ?\t, ?\n, ?\r] end)
    end

    def skip_while(%Parser{} = p, pred) do
      case current(p) do
        nil -> p
        c when pred.(c) -> skip_while(advance(p), pred)
        _ -> p
      end
    end

    def peek_string(%Parser{input: input, pos: pos}, str) do
      String.slice(input, pos, String.length(str)) == str
    end

    def consume_string(%Parser{} = p, str) do
      if peek_string(p, str) do
        {:ok, str, advance(p, String.length(str))}
      else
        {:error, "Expected '#{str}'"}
      end
    end

    def consume_keyword(%Parser{} = p, kw) do
      p = skip_whitespace(p)
      len = String.length(kw)

      if peek_string(p, kw) do
        # キーワード後に英数字が続かないことを確認
        next_p = advance(p, len)
        case current(next_p) do
          c when c in ?a..?z or c in ?A..?Z or c in ?0..?9 or c == ?_ ->
            {:error, "Keyword boundary"}
          _ ->
            {:ok, kw, skip_whitespace(next_p)}
        end
      else
        # 大文字小文字を区別しない比較
        slice = String.slice(p.input, p.pos, len)
        if String.downcase(slice) == String.downcase(kw) do
          next_p = advance(p, len)
          case current(next_p) do
            c when c in ?a..?z or c in ?A..?Z or c in ?0..?9 or c == ?_ ->
              {:error, "Keyword boundary"}
            _ ->
              {:ok, kw, skip_whitespace(next_p)}
          end
        else
          {:error, "Expected keyword '#{kw}'"}
        end
      end
    end

    def consume_symbol(%Parser{} = p, sym) do
      p = skip_whitespace(p)
      case consume_string(p, sym) do
        {:ok, _, p2} -> {:ok, sym, skip_whitespace(p2)}
        err -> err
      end
    end

    def parse_identifier(%Parser{} = p) do
      p = skip_whitespace(p)
      case current(p) do
        c when c in ?a..?z or c in ?A..?Z or c == ?_ ->
          {name, p2} = collect_identifier(advance(p), [c])
          name_str = to_string(Enum.reverse(name))

          # 予約語チェック
          reserved = ~w(select from where join inner left right full on and or not like order by asc desc null true false)
          if String.downcase(name_str) in reserved do
            {:error, "Reserved word '#{name_str}' cannot be used as identifier"}
          else
            {:ok, name_str, skip_whitespace(p2)}
          end
        _ ->
          {:error, "Expected identifier"}
      end
    end

    defp collect_identifier(%Parser{} = p, acc) do
      case current(p) do
        c when c in ?a..?z or c in ?A..?Z or c in ?0..?9 or c == ?_ ->
          collect_identifier(advance(p), [c | acc])
        _ ->
          {acc, p}
      end
    end

    def parse_integer(%Parser{} = p) do
      p = skip_whitespace(p)
      case current(p) do
        c when c in ?0..?9 ->
          {digits, p2} = collect_digits(advance(p), [c])
          num = digits |> Enum.reverse() |> to_string() |> String.to_integer()
          {:ok, num, skip_whitespace(p2)}
        _ ->
          {:error, "Expected integer"}
      end
    end

    defp collect_digits(%Parser{} = p, acc) do
      case current(p) do
        c when c in ?0..?9 -> collect_digits(advance(p), [c | acc])
        _ -> {acc, p}
      end
    end

    def parse_string_literal(%Parser{} = p) do
      p = skip_whitespace(p)
      case current(p) do
        ?' ->
          collect_string(advance(p), [])
        _ ->
          {:error, "Expected string literal"}
      end
    end

    defp collect_string(%Parser{} = p, acc) do
      case current(p) do
        nil -> {:error, "Unclosed string"}
        ?' -> {:ok, acc |> Enum.reverse() |> to_string(), skip_whitespace(advance(p))}
        c -> collect_string(advance(p), [c | acc])
      end
    end
  end

  # SQL パーサー本体
  def parse(input) do
    p = Parser.new(input) |> Parser.skip_whitespace()
    case parse_select_query(p) do
      {:ok, query, p2} ->
        p2 = Parser.skip_whitespace(p2)
        # オプションのセミコロン
        p2 = case Parser.consume_symbol(p2, ";") do
          {:ok, _, p3} -> p3
          _ -> p2
        end
        p2 = Parser.skip_whitespace(p2)
        if p2.pos >= String.length(p2.input) do
          {:ok, query}
        else
          {:error, "Unexpected input at position #{p2.pos}"}
        end
      err -> err
    end
  end

  defp parse_select_query(%Parser{} = p) do
    with {:ok, _, p} <- Parser.consume_keyword(p, "select"),
         {:ok, columns, p} <- parse_column_list(p),
         {:ok, _, p} <- Parser.consume_keyword(p, "from"),
         {:ok, from, p} <- parse_table_ref(p),
         {:ok, joins, p} <- parse_joins(p, []),
         {:ok, where_clause, p} <- parse_optional_where(p),
         {:ok, order_by, p} <- parse_optional_order_by(p) do
      query = %Query{
        type: :select,
        columns: columns,
        from: from,
        where: where_clause,
        joins: joins,
        order_by: order_by
      }
      {:ok, query, p}
    end
  end

  defp parse_column_list(%Parser{} = p) do
    case Parser.consume_symbol(p, "*") do
      {:ok, _, p} -> {:ok, [%Column{type: :all}], p}
      _ -> parse_column_exprs(p, [])
    end
  end

  defp parse_column_exprs(%Parser{} = p, acc) do
    case parse_expr(p) do
      {:ok, expr, p} ->
        # オプションの AS alias
        {alias, p} = case Parser.consume_keyword(p, "as") do
          {:ok, _, p2} ->
            case Parser.parse_identifier(p2) do
              {:ok, name, p3} -> {name, p3}
              _ -> {nil, p}
            end
          _ ->
            # AS なしのエイリアス
            case Parser.parse_identifier(p) do
              {:ok, name, p2} -> {name, p2}
              _ -> {nil, p}
            end
        end

        col = %Column{type: :expr, expr: expr, alias: alias}

        case Parser.consume_symbol(p, ",") do
          {:ok, _, p2} -> parse_column_exprs(p2, [col | acc])
          _ -> {:ok, Enum.reverse([col | acc]), p}
        end
      err ->
        if acc == [], do: err, else: {:ok, Enum.reverse(acc), p}
    end
  end

  defp parse_table_ref(%Parser{} = p) do
    case Parser.parse_identifier(p) do
      {:ok, table, p} ->
        # オプションの AS alias
        {alias, p} = case Parser.consume_keyword(p, "as") do
          {:ok, _, p2} ->
            case Parser.parse_identifier(p2) do
              {:ok, name, p3} -> {name, p3}
              _ -> {nil, p}
            end
          _ ->
            case Parser.parse_identifier(p) do
              {:ok, name, p2} -> {name, p2}
              _ -> {nil, p}
            end
        end
        {:ok, %TableRef{table: table, alias: alias}, p}
      err -> err
    end
  end

  defp parse_joins(%Parser{} = p, acc) do
    case parse_join_clause(p) do
      {:ok, join, p} -> parse_joins(p, [join | acc])
      _ -> {:ok, Enum.reverse(acc), p}
    end
  end

  defp parse_join_clause(%Parser{} = p) do
    # JOIN タイプの判定
    join_type = cond do
      match?({:ok, _, _}, Parser.consume_keyword(p, "inner")) ->
        {:ok, _, p} = Parser.consume_keyword(p, "inner")
        {:ok, _, p} = Parser.consume_keyword(p, "join")
        {:inner, p}
      match?({:ok, _, _}, Parser.consume_keyword(p, "left")) ->
        {:ok, _, p} = Parser.consume_keyword(p, "left")
        {:ok, _, p} = Parser.consume_keyword(p, "join")
        {:left, p}
      match?({:ok, _, _}, Parser.consume_keyword(p, "right")) ->
        {:ok, _, p} = Parser.consume_keyword(p, "right")
        {:ok, _, p} = Parser.consume_keyword(p, "join")
        {:right, p}
      match?({:ok, _, _}, Parser.consume_keyword(p, "full")) ->
        {:ok, _, p} = Parser.consume_keyword(p, "full")
        {:ok, _, p} = Parser.consume_keyword(p, "join")
        {:full, p}
      match?({:ok, _, _}, Parser.consume_keyword(p, "join")) ->
        {:ok, _, p} = Parser.consume_keyword(p, "join")
        {:inner, p}
      true ->
        nil
    end

    case join_type do
      {type, p} ->
        with {:ok, table, p} <- parse_table_ref(p),
             {:ok, _, p} <- Parser.consume_keyword(p, "on"),
             {:ok, condition, p} <- parse_expr(p) do
          {:ok, %Join{join_type: type, table: table, on_condition: condition}, p}
        end
      nil ->
        {:error, "Expected JOIN clause"}
    end
  end

  defp parse_optional_where(%Parser{} = p) do
    case Parser.consume_keyword(p, "where") do
      {:ok, _, p} -> parse_expr(p)
      _ -> {:ok, nil, p}
    end
  end

  defp parse_optional_order_by(%Parser{} = p) do
    case Parser.consume_keyword(p, "order") do
      {:ok, _, p} ->
        case Parser.consume_keyword(p, "by") do
          {:ok, _, p} -> parse_order_by_list(p, [])
          err -> err
        end
      _ ->
        {:ok, nil, p}
    end
  end

  defp parse_order_by_list(%Parser{} = p, acc) do
    case parse_expr(p) do
      {:ok, expr, p} ->
        # オプションの ASC/DESC
        {dir, p} = cond do
          match?({:ok, _, _}, Parser.consume_keyword(p, "asc")) ->
            {:ok, _, p2} = Parser.consume_keyword(p, "asc")
            {:asc, p2}
          match?({:ok, _, _}, Parser.consume_keyword(p, "desc")) ->
            {:ok, _, p2} = Parser.consume_keyword(p, "desc")
            {:desc, p2}
          true ->
            {:asc, p}
        end

        item = {expr, dir}

        case Parser.consume_symbol(p, ",") do
          {:ok, _, p2} -> parse_order_by_list(p2, [item | acc])
          _ -> {:ok, Enum.reverse([item | acc]), p}
        end
      err ->
        if acc == [], do: err, else: {:ok, Enum.reverse(acc), p}
    end
  end

  # 式パーサー（演算子優先度対応）
  defp parse_expr(%Parser{} = p), do: parse_or_expr(p)

  defp parse_or_expr(%Parser{} = p) do
    with {:ok, left, p} <- parse_and_expr(p) do
      parse_or_expr_cont(p, left)
    end
  end

  defp parse_or_expr_cont(%Parser{} = p, left) do
    case Parser.consume_keyword(p, "or") do
      {:ok, _, p} ->
        case parse_and_expr(p) do
          {:ok, right, p} ->
            expr = %Expr{type: :binary_op, op: :or, left: left, right: right}
            parse_or_expr_cont(p, expr)
          err -> err
        end
      _ ->
        {:ok, left, p}
    end
  end

  defp parse_and_expr(%Parser{} = p) do
    with {:ok, left, p} <- parse_comparison_expr(p) do
      parse_and_expr_cont(p, left)
    end
  end

  defp parse_and_expr_cont(%Parser{} = p, left) do
    case Parser.consume_keyword(p, "and") do
      {:ok, _, p} ->
        case parse_comparison_expr(p) do
          {:ok, right, p} ->
            expr = %Expr{type: :binary_op, op: :and, left: left, right: right}
            parse_and_expr_cont(p, expr)
          err -> err
        end
      _ ->
        {:ok, left, p}
    end
  end

  defp parse_comparison_expr(%Parser{} = p) do
    with {:ok, left, p} <- parse_additive_expr(p) do
      parse_comparison_expr_cont(p, left)
    end
  end

  defp parse_comparison_expr_cont(%Parser{} = p, left) do
    p = Parser.skip_whitespace(p)
    cond do
      Parser.peek_string(p, "<=") ->
        {:ok, _, p} = Parser.consume_symbol(p, "<=")
        {:ok, right, p} = parse_additive_expr(p)
        {:ok, %Expr{type: :binary_op, op: :le, left: left, right: right}, p}

      Parser.peek_string(p, ">=") ->
        {:ok, _, p} = Parser.consume_symbol(p, ">=")
        {:ok, right, p} = parse_additive_expr(p)
        {:ok, %Expr{type: :binary_op, op: :ge, left: left, right: right}, p}

      Parser.peek_string(p, "<>") ->
        {:ok, _, p} = Parser.consume_symbol(p, "<>")
        {:ok, right, p} = parse_additive_expr(p)
        {:ok, %Expr{type: :binary_op, op: :ne, left: left, right: right}, p}

      Parser.peek_string(p, "!=") ->
        {:ok, _, p} = Parser.consume_symbol(p, "!=")
        {:ok, right, p} = parse_additive_expr(p)
        {:ok, %Expr{type: :binary_op, op: :ne, left: left, right: right}, p}

      Parser.peek_string(p, "=") ->
        {:ok, _, p} = Parser.consume_symbol(p, "=")
        {:ok, right, p} = parse_additive_expr(p)
        {:ok, %Expr{type: :binary_op, op: :eq, left: left, right: right}, p}

      Parser.peek_string(p, "<") ->
        {:ok, _, p} = Parser.consume_symbol(p, "<")
        {:ok, right, p} = parse_additive_expr(p)
        {:ok, %Expr{type: :binary_op, op: :lt, left: left, right: right}, p}

      Parser.peek_string(p, ">") ->
        {:ok, _, p} = Parser.consume_symbol(p, ">")
        {:ok, right, p} = parse_additive_expr(p)
        {:ok, %Expr{type: :binary_op, op: :gt, left: left, right: right}, p}

      match?({:ok, _, _}, Parser.consume_keyword(p, "like")) ->
        {:ok, _, p} = Parser.consume_keyword(p, "like")
        {:ok, right, p} = parse_additive_expr(p)
        {:ok, %Expr{type: :binary_op, op: :like, left: left, right: right}, p}

      true ->
        {:ok, left, p}
    end
  end

  defp parse_additive_expr(%Parser{} = p) do
    with {:ok, left, p} <- parse_multiplicative_expr(p) do
      parse_additive_expr_cont(p, left)
    end
  end

  defp parse_additive_expr_cont(%Parser{} = p, left) do
    p = Parser.skip_whitespace(p)
    cond do
      Parser.peek_string(p, "+") ->
        {:ok, _, p} = Parser.consume_symbol(p, "+")
        case parse_multiplicative_expr(p) do
          {:ok, right, p} ->
            expr = %Expr{type: :binary_op, op: :add, left: left, right: right}
            parse_additive_expr_cont(p, expr)
          err -> err
        end

      Parser.peek_string(p, "-") ->
        {:ok, _, p} = Parser.consume_symbol(p, "-")
        case parse_multiplicative_expr(p) do
          {:ok, right, p} ->
            expr = %Expr{type: :binary_op, op: :sub, left: left, right: right}
            parse_additive_expr_cont(p, expr)
          err -> err
        end

      true ->
        {:ok, left, p}
    end
  end

  defp parse_multiplicative_expr(%Parser{} = p) do
    with {:ok, left, p} <- parse_postfix_expr(p) do
      parse_multiplicative_expr_cont(p, left)
    end
  end

  defp parse_multiplicative_expr_cont(%Parser{} = p, left) do
    p = Parser.skip_whitespace(p)
    cond do
      Parser.peek_string(p, "*") ->
        {:ok, _, p} = Parser.consume_symbol(p, "*")
        case parse_postfix_expr(p) do
          {:ok, right, p} ->
            expr = %Expr{type: :binary_op, op: :mul, left: left, right: right}
            parse_multiplicative_expr_cont(p, expr)
          err -> err
        end

      Parser.peek_string(p, "/") ->
        {:ok, _, p} = Parser.consume_symbol(p, "/")
        case parse_postfix_expr(p) do
          {:ok, right, p} ->
            expr = %Expr{type: :binary_op, op: :div, left: left, right: right}
            parse_multiplicative_expr_cont(p, expr)
          err -> err
        end

      Parser.peek_string(p, "%") ->
        {:ok, _, p} = Parser.consume_symbol(p, "%")
        case parse_postfix_expr(p) do
          {:ok, right, p} ->
            expr = %Expr{type: :binary_op, op: :mod, left: left, right: right}
            parse_multiplicative_expr_cont(p, expr)
          err -> err
        end

      true ->
        {:ok, left, p}
    end
  end

  defp parse_postfix_expr(%Parser{} = p) do
    with {:ok, expr, p} <- parse_unary_expr(p) do
      parse_postfix_expr_cont(p, expr)
    end
  end

  defp parse_postfix_expr_cont(%Parser{} = p, expr) do
    case Parser.consume_keyword(p, "is") do
      {:ok, _, p} ->
        case Parser.consume_keyword(p, "not") do
          {:ok, _, p} ->
            case Parser.consume_keyword(p, "null") do
              {:ok, _, p} ->
                new_expr = %Expr{type: :unary_op, op: :is_not_null, expr: expr}
                parse_postfix_expr_cont(p, new_expr)
              _ ->
                {:ok, expr, p}
            end
          _ ->
            case Parser.consume_keyword(p, "null") do
              {:ok, _, p} ->
                new_expr = %Expr{type: :unary_op, op: :is_null, expr: expr}
                parse_postfix_expr_cont(p, new_expr)
              _ ->
                {:ok, expr, p}
            end
        end
      _ ->
        {:ok, expr, p}
    end
  end

  defp parse_unary_expr(%Parser{} = p) do
    case Parser.consume_keyword(p, "not") do
      {:ok, _, p} ->
        case parse_unary_expr(p) do
          {:ok, expr, p} ->
            {:ok, %Expr{type: :unary_op, op: :not, expr: expr}, p}
          err -> err
        end
      _ ->
        parse_atom(p)
    end
  end

  defp parse_atom(%Parser{} = p) do
    p = Parser.skip_whitespace(p)
    cond do
      # 括弧
      Parser.peek_string(p, "(") ->
        {:ok, _, p} = Parser.consume_symbol(p, "(")
        {:ok, expr, p} = parse_expr(p)
        {:ok, _, p} = Parser.consume_symbol(p, ")")
        {:ok, %Expr{type: :parenthesized, expr: expr}, p}

      # NULL
      match?({:ok, _, _}, Parser.consume_keyword(p, "null")) ->
        {:ok, _, p} = Parser.consume_keyword(p, "null")
        {:ok, %Expr{type: :literal, value: {:null, nil}}, p}

      # TRUE
      match?({:ok, _, _}, Parser.consume_keyword(p, "true")) ->
        {:ok, _, p} = Parser.consume_keyword(p, "true")
        {:ok, %Expr{type: :literal, value: {:bool, true}}, p}

      # FALSE
      match?({:ok, _, _}, Parser.consume_keyword(p, "false")) ->
        {:ok, _, p} = Parser.consume_keyword(p, "false")
        {:ok, %Expr{type: :literal, value: {:bool, false}}, p}

      # 文字列リテラル
      Parser.current(p) == ?' ->
        case Parser.parse_string_literal(p) do
          {:ok, str, p} -> {:ok, %Expr{type: :literal, value: {:string, str}}, p}
          err -> err
        end

      # 数値
      Parser.current(p) in ?0..?9 ->
        case Parser.parse_integer(p) do
          {:ok, num, p} -> {:ok, %Expr{type: :literal, value: {:int, num}}, p}
          err -> err
        end

      # 識別子または関数呼び出し
      true ->
        case Parser.parse_identifier(p) do
          {:ok, name, p} ->
            # 関数呼び出しの確認
            case Parser.consume_symbol(p, "(") do
              {:ok, _, p} ->
                case parse_function_args(p, []) do
                  {:ok, args, p} ->
                    {:ok, _, p} = Parser.consume_symbol(p, ")")
                    {:ok, %Expr{type: :function_call, name: name, args: args}, p}
                  err -> err
                end
              _ ->
                # カラム参照（qualified または simple）
                case Parser.consume_symbol(p, ".") do
                  {:ok, _, p} ->
                    case Parser.parse_identifier(p) do
                      {:ok, col, p} ->
                        {:ok, %Expr{type: :qualified_column, table: name, column: col}, p}
                      err -> err
                    end
                  _ ->
                    {:ok, %Expr{type: :column, name: name}, p}
                end
            end
          err -> err
        end
    end
  end

  defp parse_function_args(%Parser{} = p, acc) do
    p = Parser.skip_whitespace(p)
    # 空の引数リスト
    if Parser.peek_string(p, ")") do
      {:ok, Enum.reverse(acc), p}
    else
      case parse_expr(p) do
        {:ok, expr, p} ->
          case Parser.consume_symbol(p, ",") do
            {:ok, _, p} -> parse_function_args(p, [expr | acc])
            _ -> {:ok, Enum.reverse([expr | acc]), p}
          end
        err ->
          if acc == [], do: {:ok, [], p}, else: err
      end
    end
  end

  # レンダリング（検証用）
  def render(%Query{type: :select, columns: columns, from: from, where: where, joins: joins, order_by: order_by}) do
    cols_str = render_columns(columns)
    from_str = "FROM #{from.table}" <> if(from.alias, do: " AS #{from.alias}", else: "")

    joins_str = joins
    |> Enum.map(fn j ->
      join_type = case j.join_type do
        :inner -> "INNER JOIN"
        :left -> "LEFT JOIN"
        :right -> "RIGHT JOIN"
        :full -> "FULL JOIN"
      end
      "#{join_type} #{j.table.table} ON #{render_expr(j.on_condition)}"
    end)
    |> Enum.join(" ")

    where_str = if where, do: " WHERE #{render_expr(where)}", else: ""

    order_str = if order_by do
      cols = order_by
      |> Enum.map(fn {e, dir} ->
        dir_str = if dir == :asc, do: "ASC", else: "DESC"
        "#{render_expr(e)} #{dir_str}"
      end)
      |> Enum.join(", ")
      " ORDER BY #{cols}"
    else
      ""
    end

    "SELECT #{cols_str} #{from_str} #{joins_str}#{where_str}#{order_str}"
  end

  defp render_columns(columns) do
    columns
    |> Enum.map(fn
      %Column{type: :all} -> "*"
      %Column{type: :expr, expr: e, alias: a} ->
        render_expr(e) <> if(a, do: " AS #{a}", else: "")
    end)
    |> Enum.join(", ")
  end

  defp render_expr(%Expr{type: :literal, value: {type, val}}) do
    case type do
      :int -> "#{val}"
      :string -> "'#{val}'"
      :bool -> if val, do: "TRUE", else: "FALSE"
      :null -> "NULL"
    end
  end
  defp render_expr(%Expr{type: :column, name: name}), do: name
  defp render_expr(%Expr{type: :qualified_column, table: t, column: c}), do: "#{t}.#{c}"
  defp render_expr(%Expr{type: :binary_op, op: op, left: l, right: r}) do
    op_str = case op do
      :add -> "+" | :sub -> "-" | :mul -> "*" | :div -> "/" | :mod -> "%"
      :eq -> "=" | :ne -> "<>" | :lt -> "<" | :le -> "<=" | :gt -> ">" | :ge -> ">="
      :and -> "AND" | :or -> "OR" | :like -> "LIKE"
    end
    "(#{render_expr(l)} #{op_str} #{render_expr(r)})"
  end
  defp render_expr(%Expr{type: :unary_op, op: op, expr: e}) do
    case op do
      :not -> "NOT #{render_expr(e)}"
      :is_null -> "#{render_expr(e)} IS NULL"
      :is_not_null -> "#{render_expr(e)} IS NOT NULL"
    end
  end
  defp render_expr(%Expr{type: :function_call, name: name, args: args}) do
    args_str = args |> Enum.map(&render_expr/1) |> Enum.join(", ")
    "#{name}(#{args_str})"
  end
  defp render_expr(%Expr{type: :parenthesized, expr: e}), do: "(#{render_expr(e)})"
end

# テスト
test_cases = [
  "SELECT * FROM users",
  "SELECT name, age FROM users WHERE age > 18",
  "SELECT u.name, o.total FROM users u INNER JOIN orders o ON u.id = o.user_id",
  "SELECT name FROM users WHERE active = true ORDER BY name ASC",
  "SELECT COUNT(*) FROM users WHERE created_at > '2024-01-01'"
]

IO.puts("=== SQL Parser Test ===\n")
for sql <- test_cases do
  IO.puts("Input: #{sql}")
  case SQLParser.parse(sql) do
    {:ok, query} ->
      IO.puts("Parsed: OK")
      IO.puts("Rendered: #{SQLParser.render(query)}")
    {:error, msg} ->
      IO.puts("Error: #{msg}")
  end
  IO.puts("")
end