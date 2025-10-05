defmodule JsonExtended do
  @moduledoc """
  JSON拡張版：コメント・トレーリングカンマ対応。

  標準JSONからの拡張点：
  1. コメント対応（`//` 行コメント、`/* */` ブロックコメント）
  2. トレーリングカンマ許可（配列・オブジェクトの最後の要素の後）
  3. より詳細なエラーメッセージ

  実用的な設定ファイル形式として：
  - `package.json` 風の設定ファイル
  - `.babelrc`, `.eslintrc` など開発ツールの設定
  - VS Code の `settings.json`
  """

  @type json_value ::
          :null
          | boolean()
          | number()
          | String.t()
          | [json_value()]
          | %{String.t() => json_value()}

  @type parse_error :: {:error, String.t()}

  # パース状態
  defmodule State do
    defstruct [:input, :pos]
  end

  @doc """
  JSON拡張文字列をパース。
  """
  @spec parse(String.t()) :: {:ok, json_value()} | parse_error()
  def parse(input) do
    state = %State{input: input, pos: 0}

    with {:ok, state} <- skip_whitespace_and_comments(state),
         {:ok, value, state} <- parse_value(state),
         {:ok, _state} <- skip_whitespace_and_comments(state),
         true <- state.pos >= String.length(state.input) do
      {:ok, value}
    else
      {:error, _} = err -> err
      false -> {:error, "入力の終端に到達していません"}
    end
  end

  # 空白とコメントをスキップ
  defp skip_whitespace_and_comments(state) do
    case skip_ws(state) do
      %State{pos: pos, input: input} = new_state ->
        cond do
          pos >= String.length(input) ->
            {:ok, new_state}

          String.slice(input, pos, 2) == "//" ->
            skip_whitespace_and_comments(skip_line_comment(new_state))

          String.slice(input, pos, 2) == "/*" ->
            case skip_block_comment(new_state) do
              {:ok, state_after} -> skip_whitespace_and_comments(state_after)
              err -> err
            end

          true ->
            {:ok, new_state}
        end
    end
  end

  # 空白文字をスキップ
  defp skip_ws(%State{input: input, pos: pos} = state) do
    len = String.length(input)

    new_pos =
      Enum.reduce_while(pos..(len - 1), pos, fn i, _acc ->
        ch = String.at(input, i)

        if ch in [" ", "\n", "\t", "\r"] do
          {:cont, i + 1}
        else
          {:halt, i}
        end
      end)

    %State{state | pos: new_pos}
  end

  # 行コメントをスキップ
  defp skip_line_comment(%State{input: input, pos: pos} = state) do
    # "//" をスキップ
    new_pos = pos + 2
    len = String.length(input)

    # 改行まで進む
    final_pos =
      Enum.reduce_while(new_pos..(len - 1), new_pos, fn i, _acc ->
        if String.at(input, i) == "\n" do
          {:halt, i + 1}
        else
          {:cont, i + 1}
        end
      end)

    %State{state | pos: final_pos}
  end

  # ブロックコメントをスキップ
  defp skip_block_comment(%State{input: input, pos: pos} = state) do
    # "/*" をスキップ
    new_pos = pos + 2
    len = String.length(input)

    # "*/" を探す
    case find_block_end(input, new_pos, len) do
      {:ok, end_pos} -> {:ok, %State{state | pos: end_pos}}
      :error -> {:error, "ブロックコメントが閉じられていません"}
    end
  end

  defp find_block_end(input, pos, len) when pos + 1 < len do
    if String.slice(input, pos, 2) == "*/" do
      {:ok, pos + 2}
    else
      find_block_end(input, pos + 1, len)
    end
  end

  defp find_block_end(_input, _pos, _len), do: :error

  # 値のパース
  defp parse_value(state) do
    with {:ok, state} <- skip_whitespace_and_comments(state) do
      %State{input: input, pos: pos} = state
      len = String.length(input)

      cond do
        pos >= len ->
          {:error, "予期しないEOF"}

        String.slice(input, pos, 4) == "null" ->
          {:ok, :null, %State{state | pos: pos + 4}}

        String.slice(input, pos, 4) == "true" ->
          {:ok, true, %State{state | pos: pos + 4}}

        String.slice(input, pos, 5) == "false" ->
          {:ok, false, %State{state | pos: pos + 5}}

        String.at(input, pos) == "\"" ->
          parse_string(state)

        String.at(input, pos) == "[" ->
          parse_array(state)

        String.at(input, pos) == "{" ->
          parse_object(state)

        String.at(input, pos) in ["-", "0", "1", "2", "3", "4", "5", "6", "7", "8", "9"] ->
          parse_number(state)

        true ->
          {:error, "不正な値"}
      end
    end
  end

  # 文字列リテラルのパース
  defp parse_string(%State{input: input, pos: pos} = state) do
    # 開始の " をスキップ
    new_pos = pos + 1
    len = String.length(input)

    case find_string_end(input, new_pos, len, "") do
      {:ok, str, end_pos} ->
        {:ok, str, %State{state | pos: end_pos}}

      :error ->
        {:error, "文字列が閉じられていません"}
    end
  end

  defp find_string_end(input, pos, len, acc) when pos < len do
    ch = String.at(input, pos)

    case ch do
      "\"" -> {:ok, acc, pos + 1}
      "\\" when pos + 1 < len ->
        next_ch = String.at(input, pos + 1)
        escaped = case next_ch do
          "n" -> "\n"
          "t" -> "\t"
          "r" -> "\r"
          "\\" -> "\\"
          "\"" -> "\""
          _ -> next_ch
        end
        find_string_end(input, pos + 2, len, acc <> escaped)
      _ ->
        find_string_end(input, pos + 1, len, acc <> ch)
    end
  end

  defp find_string_end(_input, _pos, _len, _acc), do: :error

  # 数値のパース
  defp parse_number(%State{input: input, pos: pos} = state) do
    len = String.length(input)

    end_pos =
      Enum.reduce_while(pos..(len - 1), pos, fn i, _acc ->
        ch = String.at(input, i)

        if ch in ["-", "+", ".", "e", "E"] or (ch >= "0" and ch <= "9") do
          {:cont, i + 1}
        else
          {:halt, i}
        end
      end)

    num_str = String.slice(input, pos, end_pos - pos)

    case parse_num_string(num_str) do
      {:ok, num} -> {:ok, num, %State{state | pos: end_pos}}
      :error -> {:error, "不正な数値: #{num_str}"}
    end
  end

  defp parse_num_string(str) do
    cond do
      String.contains?(str, ".") or String.contains?(str, "e") or String.contains?(str, "E") ->
        case Float.parse(str) do
          {num, ""} -> {:ok, num}
          _ -> :error
        end

      true ->
        case Integer.parse(str) do
          {num, ""} -> {:ok, num}
          _ -> :error
        end
    end
  end

  # 配列のパース（トレーリングカンマ対応）
  defp parse_array(%State{input: input, pos: pos} = state) do
    # "[" をスキップ
    new_state = %State{state | pos: pos + 1}

    with {:ok, new_state} <- skip_whitespace_and_comments(new_state) do
      # 空配列チェック
      if String.at(input, new_state.pos) == "]" do
        {:ok, [], %State{new_state | pos: new_state.pos + 1}}
      else
        parse_array_elements(new_state, [])
      end
    end
  end

  defp parse_array_elements(state, acc) do
    with {:ok, value, state} <- parse_value(state),
         {:ok, state} <- skip_whitespace_and_comments(state) do
      new_acc = acc ++ [value]
      ch = String.at(state.input, state.pos)

      case ch do
        "," ->
          state = %State{state | pos: state.pos + 1}
          {:ok, state} = skip_whitespace_and_comments(state)

          # トレーリングカンマチェック
          if String.at(state.input, state.pos) == "]" do
            {:ok, new_acc, %State{state | pos: state.pos + 1}}
          else
            parse_array_elements(state, new_acc)
          end

        "]" ->
          {:ok, new_acc, %State{state | pos: state.pos + 1}}

        _ ->
          {:error, "配列要素の後には ',' または ']' が必要です"}
      end
    end
  end

  # オブジェクトのパース（トレーリングカンマ対応）
  defp parse_object(%State{input: input, pos: pos} = state) do
    # "{" をスキップ
    new_state = %State{state | pos: pos + 1}

    with {:ok, new_state} <- skip_whitespace_and_comments(new_state) do
      # 空オブジェクトチェック
      if String.at(input, new_state.pos) == "}" do
        {:ok, %{}, %State{new_state | pos: new_state.pos + 1}}
      else
        parse_object_pairs(new_state, %{})
      end
    end
  end

  defp parse_object_pairs(state, acc) do
    with {:ok, key, state} <- parse_string(state),
         {:ok, state} <- skip_whitespace_and_comments(state),
         {:ok, state} <- expect_char(state, ":"),
         {:ok, state} <- skip_whitespace_and_comments(state),
         {:ok, value, state} <- parse_value(state),
         {:ok, state} <- skip_whitespace_and_comments(state) do
      new_acc = Map.put(acc, key, value)
      ch = String.at(state.input, state.pos)

      case ch do
        "," ->
          state = %State{state | pos: state.pos + 1}
          {:ok, state} = skip_whitespace_and_comments(state)

          # トレーリングカンマチェック
          if String.at(state.input, state.pos) == "}" do
            {:ok, new_acc, %State{state | pos: state.pos + 1}}
          else
            parse_object_pairs(state, new_acc)
          end

        "}" ->
          {:ok, new_acc, %State{state | pos: state.pos + 1}}

        _ ->
          {:error, "オブジェクト要素の後には ',' または '}' が必要です"}
      end
    end
  end

  # 特定の文字を期待
  defp expect_char(%State{input: input, pos: pos} = state, expected) do
    if String.at(input, pos) == expected do
      {:ok, %State{state | pos: pos + 1}}
    else
      {:error, "'#{expected}' が必要です"}
    end
  end

  @doc """
  JSON値を文字列にレンダリング（検証用）。
  """
  @spec render_to_string(json_value(), integer()) :: String.t()
  def render_to_string(value, indent_level \\ 0)

  def render_to_string(:null, _), do: "null"
  def render_to_string(true, _), do: "true"
  def render_to_string(false, _), do: "false"
  def render_to_string(num, _) when is_number(num), do: to_string(num)
  def render_to_string(str, _) when is_binary(str), do: "\"#{str}\""

  def render_to_string(list, indent_level) when is_list(list) do
    if list == [] do
      "[]"
    else
      indent = String.duplicate("  ", indent_level)
      next_indent = String.duplicate("  ", indent_level + 1)

      items =
        list
        |> Enum.map(&"#{next_indent}#{render_to_string(&1, indent_level + 1)}")
        |> Enum.join(",\n")

      "[\n#{items}\n#{indent}]"
    end
  end

  def render_to_string(map, indent_level) when is_map(map) do
    if map == %{} do
      "{}"
    else
      indent = String.duplicate("  ", indent_level)
      next_indent = String.duplicate("  ", indent_level + 1)

      pairs =
        map
        |> Enum.map(fn {key, val} ->
          "#{next_indent}\"#{key}\": #{render_to_string(val, indent_level + 1)}"
        end)
        |> Enum.join(",\n")

      "{\n#{pairs}\n#{indent}}"
    end
  end

  @doc """
  拡張機能のテスト例。
  """
  def test_extended_json do
    test_cases = [
      {"コメント対応", """
      {
        // これは行コメント
        "name": "test",
        /* これは
           ブロックコメント */
        "version": "1.0"
      }
      """},
      {"トレーリングカンマ", """
      {
        "items": [
          1,
          2,
          3,
        ],
        "config": {
          "debug": true,
          "port": 8080,
        }
      }
      """},
      {"複雑な例", """
      {
        // パッケージ情報
        "name": "my-project",
        "version": "0.1.0",

        /* 依存関係 */
        "dependencies": {
          "core": "1.0",
          "utils": "0.5",
        },

        // スクリプト
        "scripts": {
          "build": "reml build",
          "test": "reml test",
        },
      }
      """}
    ]

    Enum.each(test_cases, fn {name, json_str} ->
      IO.puts("--- #{name} ---")

      case parse(json_str) do
        {:ok, value} ->
          IO.puts("パース成功:")
          IO.puts(render_to_string(value, 0))

        {:error, err} ->
          IO.puts("パースエラー: #{err}")
      end

      IO.puts("")
    end)
  end
end