# YAML風パーサー：インデント管理が重要な題材。
#
# 対応する構文（簡易版）：
# - スカラー値: 文字列、数値、真偽値、null
# - リスト: `- item1`
# - マップ: `key: value`
# - ネストしたインデント構造
#
# Elixirの特徴：
# - パターンマッチとパイプライン演算子で見通しの良い実装
# - 再帰とアキュムレータパターンでインデント管理
# - with構文によるエラー処理の連鎖

defmodule YamlParser do
  @moduledoc """
  YAML風パーサー：簡易版YAMLサブセットをパース。
  """

  defmodule YamlValue do
    @type t :: {:scalar, String.t()} | {:list, [t()]} | {:map, %{String.t() => t()}} | :null
  end

  defmodule ParseError do
    defexception [:message, :position]
  end

  # パース結果型
  @type parse_result(t) :: {:ok, t, String.t()} | {:error, String.t()}

  # --- 基本パーサー ---

  defp consume_while(input, predicate) do
    {taken, rest} = String.split_at(input, String.length(input) - String.length(String.trim_leading(input, predicate)))
    {taken, rest}
  end

  defp hspace(input) do
    {_spaces, rest} = consume_while(input, fn c -> c == ?\s or c == ?\t end)
    {:ok, nil, rest}
  end

  defp expect_string(input, expected) do
    if String.starts_with?(input, expected) do
      {:ok, expected, String.slice(input, String.length(expected)..-1)}
    else
      {:error, "Expected '#{expected}'"}
    end
  end

  defp newline(input) do
    cond do
      String.starts_with?(input, "\r\n") -> {:ok, nil, String.slice(input, 2..-1)}
      String.starts_with?(input, "\n") -> {:ok, nil, String.slice(input, 1..-1)}
      String.starts_with?(input, "\r") -> {:ok, nil, String.slice(input, 1..-1)}
      true -> {:error, "Expected newline"}
    end
  end

  # インデント検証
  defp expect_indent(input, level) do
    {spaces, rest} = take_spaces(input)
    actual = String.length(spaces)
    if actual == level do
      {:ok, nil, rest}
    else
      {:error, "インデント不一致: 期待 #{level}, 実際 #{actual}"}
    end
  end

  defp take_spaces(input) do
    Enum.reduce_while(String.graphemes(input), {"", input}, fn
      " ", {acc, rest} -> {:cont, {acc <> " ", String.slice(rest, 1..-1)}}
      _, {acc, rest} -> {:halt, {acc, rest}}
    end)
  end

  # --- スカラー値パーサー ---

  defp parse_scalar(input) do
    cond do
      String.starts_with?(input, "null") ->
        {:ok, :null, String.slice(input, 4..-1)}
      String.starts_with?(input, "~") ->
        {:ok, :null, String.slice(input, 1..-1)}
      String.starts_with?(input, "true") ->
        {:ok, {:scalar, "true"}, String.slice(input, 4..-1)}
      String.starts_with?(input, "false") ->
        {:ok, {:scalar, "false"}, String.slice(input, 5..-1)}
      true ->
        # 文字列（引用符なし：行末まで）
        case String.split(input, "\n", parts: 2) do
          [line, rest] ->
            trimmed = String.trim(line)
            if trimmed != "" do
              {:ok, {:scalar, trimmed}, rest}
            else
              {:error, "Empty scalar"}
            end
          [line] ->
            trimmed = String.trim(line)
            if trimmed != "" do
              {:ok, {:scalar, trimmed}, ""}
            else
              {:error, "Empty scalar"}
            end
        end
    end
  end

  # --- リストパーサー ---

  defp parse_list_item(input, indent) do
    with {:ok, _, rest} <- expect_indent(input, indent),
         {:ok, _, rest} <- expect_string(rest, "-"),
         {:ok, _, rest} <- hspace(rest),
         {:ok, value, rest} <- parse_value(rest, indent + 2) do
      {:ok, value, rest}
    end
  end

  defp parse_list(input, indent) do
    parse_list_items(input, indent, [])
  end

  defp parse_list_items(input, indent, acc) do
    case parse_list_item(input, indent) do
      {:ok, item, rest} ->
        rest = skip_optional_newline(rest)
        parse_list_items(rest, indent, [item | acc])
      {:error, _} ->
        if acc == [] do
          {:error, "Expected at least one list item"}
        else
          {:ok, {:list, Enum.reverse(acc)}, input}
        end
    end
  end

  defp skip_optional_newline(input) do
    case newline(input) do
      {:ok, _, rest} -> rest
      {:error, _} -> input
    end
  end

  # --- マップパーサー ---

  defp parse_map_entry(input, indent) do
    with {:ok, _, rest} <- expect_indent(input, indent),
         {key_line, rest} <- take_until(rest, ":"),
         key = String.trim(key_line),
         {:ok, _, rest} <- expect_string(rest, ":"),
         {:ok, _, rest} <- hspace(rest) do
      # 値が同じ行にあるか、次の行にネストされているか
      case parse_value(rest, indent) do
        {:ok, value, rest} ->
          {:ok, {key, value}, rest}
        {:error, _} ->
          # 次の行にネストされた値
          with {:ok, _, rest} <- newline(rest),
               {:ok, value, rest} <- parse_value(rest, indent + 2) do
            {:ok, {key, value}, rest}
          end
      end
    end
  end

  defp take_until(input, delimiter) do
    case String.split(input, delimiter, parts: 2) do
      [before, after] -> {before, after}
      [before] -> {before, ""}
    end
  end

  defp parse_map(input, indent) do
    parse_map_entries(input, indent, [])
  end

  defp parse_map_entries(input, indent, acc) do
    case parse_map_entry(input, indent) do
      {:ok, {key, value}, rest} ->
        rest = skip_optional_newline(rest)
        parse_map_entries(rest, indent, [{key, value} | acc])
      {:error, _} ->
        if acc == [] do
          {:error, "Expected at least one map entry"}
        else
          {:ok, {:map, Map.new(Enum.reverse(acc))}, input}
        end
    end
  end

  # --- 値パーサー（再帰的） ---

  defp parse_value(input, indent) do
    cond do
      # リスト
      String.contains?(input, "-") ->
        case parse_list(input, indent) do
          {:ok, _, _} = result -> result
          {:error, _} -> parse_map_or_scalar(input, indent)
        end
      # マップまたはスカラー
      true -> parse_map_or_scalar(input, indent)
    end
  end

  defp parse_map_or_scalar(input, indent) do
    case parse_map(input, indent) do
      {:ok, _, _} = result -> result
      {:error, _} -> parse_scalar(input)
    end
  end

  # --- ドキュメントパーサー ---

  defp skip_blank_lines(input) do
    input
    |> String.split("\n")
    |> Enum.drop_while(&(String.trim(&1) == ""))
    |> Enum.join("\n")
  end

  @doc """
  YAML文字列をパース。
  """
  def parse(input) do
    input = skip_blank_lines(input)
    case parse_value(input, 0) do
      {:ok, doc, _rest} -> {:ok, doc}
      {:error, msg} -> {:error, msg}
    end
  end

  # --- レンダリング（検証用） ---

  @doc """
  YAML値を文字列にレンダリング。
  """
  def render_to_string(doc) do
    render_value(doc, 0)
  end

  defp render_value({:scalar, s}, _indent), do: s
  defp render_value(:null, _indent), do: "null"
  defp render_value({:list, items}, indent) do
    indent_str = String.duplicate(" ", indent)
    items
    |> Enum.map(fn item ->
      "#{indent_str}- #{render_value(item, indent + 2)}"
    end)
    |> Enum.join("\n")
  end
  defp render_value({:map, entries}, indent) do
    indent_str = String.duplicate(" ", indent)
    entries
    |> Enum.map(fn {key, val} ->
      case val do
        {:scalar, _} -> "#{indent_str}#{key}: #{render_value(val, 0)}"
        :null -> "#{indent_str}#{key}: #{render_value(val, 0)}"
        _ -> "#{indent_str}#{key}:\n#{render_value(val, indent + 2)}"
      end
    end)
    |> Enum.join("\n")
  end

  # --- テスト ---

  @doc """
  テスト例を実行。
  """
  def test_examples do
    examples = [
      {"simple_scalar", "hello"},
      {"simple_list", "- item1\n- item2\n- item3"},
      {"simple_map", "key1: value1\nkey2: value2"},
      {"nested_map", "parent:\n  child1: value1\n  child2: value2"},
      {"nested_list", "items:\n  - item1\n  - item2"},
      {"mixed", "name: John\nage: 30\nhobbies:\n  - reading\n  - coding"}
    ]

    Enum.each(examples, fn {name, yaml_str} ->
      IO.puts("--- #{name} ---")
      case parse(yaml_str) do
        {:ok, doc} ->
          IO.puts("パース成功:")
          IO.puts(render_to_string(doc))
        {:error, err} ->
          IO.puts("パースエラー: #{err}")
      end
      IO.puts("")
    end)
  end
end

# テスト実行
YamlParser.test_examples()

# Elixirの特徴：
#
# 1. **パターンマッチとwith構文**
#    - エラー処理の連鎖がwith構文で明快
#    - パターンマッチによる分岐が自然
#
# 2. **再帰とアキュムレータ**
#    - リストやマップのパースは末尾再帰で実装
#    - アキュムレータパターンで結果を蓄積
#
# 3. **パイプライン演算子**
#    - レンダリング処理などでパイプラインが活躍
#    - 関数合成が読みやすい
#
# 4. **課題**
#    - 状態管理が関数引数渡しになり煩雑になりがち
#    - エラーメッセージの位置情報が不足
#
# Remlとの比較：
# - Remlはパーサーコンビネーターライブラリによる高レベル抽象化
# - Elixirは手書きパーサーでより明示的な制御が必要