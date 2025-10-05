# TOML風設定ファイルパーサー：キーバリューペアとテーブルを扱う題材。
#
# 対応する構文（TOML v1.0.0準拠の簡易版）：
# - キーバリューペア: `key = "value"`
# - テーブル: `[section]`
# - 配列テーブル: `[[array_section]]`
# - データ型: 文字列、整数、浮動小数点、真偽値、配列、インラインテーブル
# - コメント: `# comment`
#
# Elixirの特徴：
# - パターンマッチとパイプライン演算子で見通しの良い実装
# - 再帰とアキュムレータパターンで状態管理
# - with構文によるエラー処理の連鎖

defmodule TomlParser do
  @moduledoc """
  TOML風パーサー：簡易版TOMLサブセットをパース。
  """

  defmodule TomlValue do
    @type t ::
      {:string, String.t()} |
      {:integer, integer()} |
      {:float, float()} |
      {:boolean, boolean()} |
      {:array, [t()]} |
      {:inline_table, %{String.t() => t()}}
  end

  defmodule TomlDocument do
    @type t :: %{
      root: %{String.t() => TomlValue.t()},
      tables: %{[String.t()] => %{String.t() => TomlValue.t()}}
    }
  end

  defmodule ParseError do
    defexception [:message, :position]
  end

  # パース結果型
  @type parse_result(t) :: {:ok, t, String.t()} | {:error, String.t()}

  # --- 基本パーサー ---

  defp skip_whitespace(input) do
    input
    |> String.trim_leading()
  end

  defp skip_comment(input) do
    if String.starts_with?(input, "#") do
      case String.split(input, "\n", parts: 2) do
        [_comment, rest] -> rest
        [_comment] -> ""
      end
    else
      input
    end
  end

  defp skip_whitespace_and_comments(input) do
    input
    |> skip_whitespace()
    |> skip_comment()
    |> then(fn rest ->
      if rest != input do
        skip_whitespace_and_comments(rest)
      else
        rest
      end
    end)
  end

  defp expect_string(input, expected) do
    if String.starts_with?(input, expected) do
      {:ok, expected, String.slice(input, String.length(expected)..-1//1)}
    else
      {:error, "Expected '#{expected}'"}
    end
  end

  # --- キー名のパース ---

  defp parse_bare_key(input) do
    {key, rest} =
      String.graphemes(input)
      |> Enum.reduce_while({"", input}, fn
        c, {acc, rest} when c in ["a".."z"] or c in ["A".."Z"] or c in ["0".."9"] or c == "-" or c == "_" ->
          {:cont, {acc <> c, String.slice(rest, 1..-1//1)}}
        _, {acc, rest} ->
          {:halt, {acc, rest}}
      end)

    if key == "" do
      {:error, "Expected key"}
    else
      {:ok, key, rest}
    end
  end

  defp parse_quoted_key(input) do
    with {:ok, _, rest} <- expect_string(input, "\""),
         {key, rest} <- take_until_unescaped(rest, "\""),
         {:ok, _, rest} <- expect_string(rest, "\"") do
      {:ok, key, rest}
    end
  end

  defp take_until_unescaped(input, delimiter) do
    take_until_unescaped_impl(input, delimiter, "")
  end

  defp take_until_unescaped_impl(input, delimiter, acc) do
    cond do
      String.starts_with?(input, "\\#{delimiter}") ->
        take_until_unescaped_impl(
          String.slice(input, 2..-1//1),
          delimiter,
          acc <> delimiter
        )
      String.starts_with?(input, delimiter) ->
        {acc, input}
      String.length(input) > 0 ->
        {first, rest} = String.split_at(input, 1)
        take_until_unescaped_impl(rest, delimiter, acc <> first)
      true ->
        {acc, ""}
    end
  end

  defp parse_key(input) do
    cond do
      String.starts_with?(input, "\"") -> parse_quoted_key(input)
      true -> parse_bare_key(input)
    end
  end

  defp parse_key_path(input) do
    parse_key_path_impl(input, [])
  end

  defp parse_key_path_impl(input, acc) do
    input = skip_whitespace(input)
    with {:ok, key, rest} <- parse_key(input) do
      rest = skip_whitespace(rest)
      if String.starts_with?(rest, ".") do
        rest = String.slice(rest, 1..-1//1)
        parse_key_path_impl(rest, acc ++ [key])
      else
        {:ok, acc ++ [key], rest}
      end
    end
  end

  # --- 値のパース ---

  defp parse_string_value(input) do
    cond do
      String.starts_with?(input, "\"\"\"") ->
        # 複数行基本文字列
        with {:ok, _, rest} <- expect_string(input, "\"\"\""),
             {content, rest} <- take_until_unescaped(rest, "\"\"\""),
             {:ok, _, rest} <- expect_string(rest, "\"\"\"") do
          {:ok, {:string, content}, rest}
        end
      String.starts_with?(input, "'''") ->
        # 複数行リテラル文字列
        with {:ok, _, rest} <- expect_string(input, "'''"),
             {content, rest} <- take_until(rest, "'''"),
             {:ok, _, rest} <- expect_string(rest, "'''") do
          {:ok, {:string, content}, rest}
        end
      String.starts_with?(input, "'") ->
        # リテラル文字列
        with {:ok, _, rest} <- expect_string(input, "'"),
             {content, rest} <- take_until(rest, "'"),
             {:ok, _, rest} <- expect_string(rest, "'") do
          {:ok, {:string, content}, rest}
        end
      String.starts_with?(input, "\"") ->
        # 基本文字列
        with {:ok, _, rest} <- expect_string(input, "\""),
             {content, rest} <- take_until_unescaped(rest, "\""),
             {:ok, _, rest} <- expect_string(rest, "\"") do
          {:ok, {:string, content}, rest}
        end
      true ->
        {:error, "Expected string"}
    end
  end

  defp take_until(input, delimiter) do
    case String.split(input, delimiter, parts: 2) do
      [before, _after] -> {before, delimiter <> _after}
      [before] -> {before, ""}
    end
  end

  defp parse_integer_value(input) do
    sign = if String.starts_with?(input, "-"), do: "-", else: ""
    rest = if sign == "-", do: String.slice(input, 1..-1//1), else: input

    {digits, rest} =
      String.graphemes(rest)
      |> Enum.reduce_while({"", rest}, fn
        c, {acc, rest} when c in ["0".."9"] or c == "_" ->
          if c == "_" do
            {:cont, {acc, String.slice(rest, 1..-1//1)}}
          else
            {:cont, {acc <> c, String.slice(rest, 1..-1//1)}}
          end
        _, {acc, rest} ->
          {:halt, {acc, rest}}
      end)

    if digits == "" do
      {:error, "Expected integer"}
    else
      case Integer.parse(sign <> digits) do
        {n, _} -> {:ok, {:integer, n}, rest}
        :error -> {:error, "Invalid integer"}
      end
    end
  end

  defp parse_float_value(input) do
    sign = if String.starts_with?(input, "-"), do: "-", else: ""
    rest = if sign == "-", do: String.slice(input, 1..-1//1), else: input

    {num_str, rest} =
      String.graphemes(rest)
      |> Enum.reduce_while({"", rest}, fn
        c, {acc, rest} when c in ["0".."9"] or c == "." or c == "_" or c == "e" or c == "E" or c == "+" or c == "-" ->
          if c == "_" do
            {:cont, {acc, String.slice(rest, 1..-1//1)}}
          else
            {:cont, {acc <> c, String.slice(rest, 1..-1//1)}}
          end
        _, {acc, rest} ->
          {:halt, {acc, rest}}
      end)

    if num_str == "" or not String.contains?(num_str, ".") do
      {:error, "Expected float"}
    else
      case Float.parse(sign <> num_str) do
        {f, _} -> {:ok, {:float, f}, rest}
        :error -> {:error, "Invalid float"}
      end
    end
  end

  defp parse_boolean_value(input) do
    cond do
      String.starts_with?(input, "true") ->
        {:ok, {:boolean, true}, String.slice(input, 4..-1//1)}
      String.starts_with?(input, "false") ->
        {:ok, {:boolean, false}, String.slice(input, 5..-1//1)}
      true ->
        {:error, "Expected boolean"}
    end
  end

  defp parse_array_value(input) do
    with {:ok, _, rest} <- expect_string(input, "[") do
      rest = skip_whitespace_and_comments(rest)
      parse_array_elements(rest, [])
    end
  end

  defp parse_array_elements(input, acc) do
    input = skip_whitespace_and_comments(input)
    cond do
      String.starts_with?(input, "]") ->
        {:ok, {:array, Enum.reverse(acc)}, String.slice(input, 1..-1//1)}
      acc == [] or String.starts_with?(input, ",") ->
        rest = if acc != [] do
          String.slice(input, 1..-1//1)
        else
          input
        end
        rest = skip_whitespace_and_comments(rest)
        if String.starts_with?(rest, "]") do
          {:ok, {:array, Enum.reverse(acc)}, String.slice(rest, 1..-1//1)}
        else
          with {:ok, value, rest} <- parse_value(rest) do
            parse_array_elements(rest, [value | acc])
          end
        end
      true ->
        {:error, "Expected ',' or ']'"}
    end
  end

  defp parse_inline_table(input) do
    with {:ok, _, rest} <- expect_string(input, "{") do
      rest = skip_whitespace_and_comments(rest)
      parse_inline_table_entries(rest, [])
    end
  end

  defp parse_inline_table_entries(input, acc) do
    input = skip_whitespace_and_comments(input)
    cond do
      String.starts_with?(input, "}") ->
        {:ok, {:inline_table, Map.new(Enum.reverse(acc))}, String.slice(input, 1..-1//1)}
      acc == [] or String.starts_with?(input, ",") ->
        rest = if acc != [] do
          String.slice(input, 1..-1//1)
        else
          input
        end
        rest = skip_whitespace_and_comments(rest)
        if String.starts_with?(rest, "}") do
          {:ok, {:inline_table, Map.new(Enum.reverse(acc))}, String.slice(rest, 1..-1//1)}
        else
          with {:ok, key, rest} <- parse_key(rest),
               rest = skip_whitespace(rest),
               {:ok, _, rest} <- expect_string(rest, "="),
               rest = skip_whitespace_and_comments(rest),
               {:ok, value, rest} <- parse_value(rest) do
            parse_inline_table_entries(rest, [{key, value} | acc])
          end
        end
      true ->
        {:error, "Expected ',' or '}'"}
    end
  end

  defp parse_value(input) do
    input = skip_whitespace_and_comments(input)
    cond do
      String.starts_with?(input, "\"") or String.starts_with?(input, "'") ->
        parse_string_value(input)
      String.starts_with?(input, "true") or String.starts_with?(input, "false") ->
        parse_boolean_value(input)
      String.starts_with?(input, "[") ->
        parse_array_value(input)
      String.starts_with?(input, "{") ->
        parse_inline_table(input)
      true ->
        # 数値（整数または浮動小数点）
        case parse_float_value(input) do
          {:ok, _, _} = result -> result
          {:error, _} -> parse_integer_value(input)
        end
    end
  end

  # --- キーバリューペアのパース ---

  defp parse_key_value_pair(input) do
    input = skip_whitespace_and_comments(input)
    with {:ok, path, rest} <- parse_key_path(input),
         rest = skip_whitespace(rest),
         {:ok, _, rest} <- expect_string(rest, "="),
         rest = skip_whitespace_and_comments(rest),
         {:ok, value, rest} <- parse_value(rest) do
      {:ok, {:key_value, path, value}, rest}
    end
  end

  # --- テーブルヘッダーのパース ---

  defp parse_table_header(input) do
    input = skip_whitespace_and_comments(input)
    cond do
      String.starts_with?(input, "[[") ->
        with {:ok, _, rest} <- expect_string(input, "[["),
             rest = skip_whitespace(rest),
             {:ok, path, rest} <- parse_key_path(rest),
             rest = skip_whitespace(rest),
             {:ok, _, rest} <- expect_string(rest, "]]") do
          {:ok, {:array_table, path}, rest}
        end
      String.starts_with?(input, "[") ->
        with {:ok, _, rest} <- expect_string(input, "["),
             rest = skip_whitespace(rest),
             {:ok, path, rest} <- parse_key_path(rest),
             rest = skip_whitespace(rest),
             {:ok, _, rest} <- expect_string(rest, "]") do
          {:ok, {:table, path}, rest}
        end
      true ->
        {:error, "Expected table header"}
    end
  end

  # --- ドキュメント要素のパース ---

  defp parse_document_element(input) do
    input = skip_whitespace_and_comments(input)
    if input == "" do
      {:error, "End of input"}
    else
      cond do
        String.starts_with?(input, "[") ->
          parse_table_header(input)
        true ->
          parse_key_value_pair(input)
      end
    end
  end

  defp skip_newline(input) do
    cond do
      String.starts_with?(input, "\r\n") -> String.slice(input, 2..-1//1)
      String.starts_with?(input, "\n") -> String.slice(input, 1..-1//1)
      String.starts_with?(input, "\r") -> String.slice(input, 1..-1//1)
      true -> input
    end
  end

  defp parse_document_elements(input, acc) do
    input = skip_whitespace_and_comments(input)
    if input == "" do
      {:ok, Enum.reverse(acc)}
    else
      case parse_document_element(input) do
        {:ok, elem, rest} ->
          rest = skip_newline(rest)
          parse_document_elements(rest, [elem | acc])
        {:error, "End of input"} ->
          {:ok, Enum.reverse(acc)}
        {:error, msg} ->
          {:error, msg}
      end
    end
  end

  # --- ドキュメント構築 ---

  defp insert_nested(table, [key], value) do
    Map.put(table, key, value)
  end

  defp insert_nested(table, [key | rest], value) do
    nested = case Map.get(table, key) do
      {:inline_table, t} -> t
      nil -> %{}
      _ -> %{}
    end
    updated_nested = insert_nested(nested, rest, value)
    Map.put(table, key, {:inline_table, updated_nested})
  end

  defp build_document(elements) do
    Enum.reduce(elements, {[], %{}, %{}}, fn elem, {current_table, root, tables} ->
      case elem do
        {:table, path} ->
          tables = if not Map.has_key?(tables, path) do
            Map.put(tables, path, %{})
          else
            tables
          end
          {path, root, tables}

        {:array_table, path} ->
          # 簡易実装では通常テーブルと同じ扱い
          tables = if not Map.has_key?(tables, path) do
            Map.put(tables, path, %{})
          else
            tables
          end
          {path, root, tables}

        {:key_value, path, value} ->
          if current_table == [] do
            # ルートテーブルに追加
            root = insert_nested(root, path, value)
            {current_table, root, tables}
          else
            # 現在のテーブルに追加
            table = Map.get(tables, current_table, %{})
            updated_table = insert_nested(table, path, value)
            tables = Map.put(tables, current_table, updated_table)
            {current_table, root, tables}
          end
      end
    end)
  end

  # --- パブリックAPI ---

  @doc """
  TOML文字列をパース。
  """
  def parse(input) do
    case parse_document_elements(input, []) do
      {:ok, elements} ->
        {_current_table, root, tables} = build_document(elements)
        {:ok, %{root: root, tables: tables}}
      {:error, msg} ->
        {:error, msg}
    end
  end

  # --- レンダリング（検証用） ---

  @doc """
  TOMLドキュメントを文字列にレンダリング。
  """
  def render_to_string(doc) do
    output = render_table(doc.root, [])

    table_output = doc.tables
    |> Enum.map(fn {path, table} ->
      "\n[#{Enum.join(path, ".")}]\n" <> render_table(table, [])
    end)
    |> Enum.join("")

    output <> table_output
  end

  defp render_table(table, prefix) do
    table
    |> Enum.map(fn {key, value} ->
      full_key = if prefix == [] do
        key
      else
        Enum.join(prefix ++ [key], ".")
      end

      case value do
        {:inline_table, nested} ->
          render_table(nested, prefix ++ [key])
        _ ->
          "#{full_key} = #{render_value(value)}\n"
      end
    end)
    |> Enum.join("")
  end

  defp render_value({:string, s}), do: "\"#{s}\""
  defp render_value({:integer, n}), do: Integer.to_string(n)
  defp render_value({:float, f}), do: Float.to_string(f)
  defp render_value({:boolean, true}), do: "true"
  defp render_value({:boolean, false}), do: "false"
  defp render_value({:array, items}) do
    items_str = items
    |> Enum.map(&render_value/1)
    |> Enum.join(", ")
    "[#{items_str}]"
  end
  defp render_value({:inline_table, entries}) do
    entries_str = entries
    |> Enum.map(fn {k, v} -> "#{k} = #{render_value(v)}" end)
    |> Enum.join(", ")
    "{ #{entries_str} }"
  end

  # --- テスト ---

  @doc """
  テスト例を実行。
  """
  def test_examples do
    example_toml = """
# Reml パッケージ設定

[package]
name = "my_project"
version = "0.1.0"
authors = ["Author Name"]

[dependencies]
core = "1.0"

[dev-dependencies]
test_framework = "0.5"

[[plugins]]
name = "system"
version = "1.0"

[[plugins]]
name = "memory"
version = "1.0"
"""

    IO.puts("--- reml.toml 風設定のパース ---")
    case parse(example_toml) do
      {:ok, doc} ->
        IO.puts("パース成功:")
        IO.puts(render_to_string(doc))
      {:error, err} ->
        IO.puts("パースエラー: #{err}")
    end
  end
end

# テスト実行
TomlParser.test_examples()

# Elixirの特徴：
#
# 1. **パターンマッチとwith構文**
#    - エラー処理の連鎖がwith構文で明快
#    - パターンマッチによる分岐が自然
#
# 2. **再帰とアキュムレータ**
#    - 配列やテーブルのパースは末尾再帰で実装
#    - アキュムレータパターンで結果を蓄積
#
# 3. **Mapデータ構造**
#    - ネストしたテーブルはMapで表現
#    - 動的なキー追加が容易
#
# 4. **課題**
#    - 手書きパーサーのため、エラーメッセージの質が実装依存
#    - バックトラックの実装が煩雑
#
# Remlとの比較：
# - Remlはパーサーコンビネーターライブラリによる高レベル抽象化
# - Elixirは手書きパーサーでより明示的な制御が必要
# - Remlのcut/commitによるエラー位置特定がより正確