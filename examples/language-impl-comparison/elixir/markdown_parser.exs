defmodule MarkdownParser do
  @moduledoc """
  Markdown風軽量マークアップパーサー - Elixir実装

  Unicode処理の注意点：
  - ElixirのStringはUTF-8バイナリで、graphemeクラスター単位の操作が可能
  - String.length/1 はgrapheme数を返す
  - byte_size/1 はバイト数を返す
  - String.codepoints/1 でコードポイント単位の操作も可能
  - Remlの3層モデル（Byte/Char/Grapheme）に近い柔軟性を持つ
  """

  @type inline ::
          {:text, String.t()}
          | {:strong, [inline()]}
          | {:emphasis, [inline()]}
          | {:code, String.t()}
          | {:link, [inline()], String.t()}
          | :line_break

  @type block ::
          {:heading, non_neg_integer(), [inline()]}
          | {:paragraph, [inline()]}
          | {:unordered_list, [[inline()]]}
          | {:ordered_list, [[inline()]]}
          | {:code_block, String.t() | nil, String.t()}
          | :horizontal_rule

  @type document :: [block()]

  @type parse_state :: %{
          input: String.t(),
          position: non_neg_integer()
        }

  @type parse_result(t) :: {:ok, t, parse_state()} | {:error, String.t()}

  @spec parse(String.t()) :: {:ok, document()} | {:error, String.t()}
  def parse(input) do
    initial_state = %{input: input, position: 0}
    parse_document(initial_state, [])
  end

  defp parse_document(state, blocks) do
    case parse_block(state) do
      {:ok, block, new_state} ->
        parse_document(new_state, [block | blocks])

      {:error, "EOF"} ->
        {:ok, Enum.reverse(blocks)}

      {:error, reason} ->
        {:error, reason}
    end
  end

  defp parse_block(state) do
    state = skip_blank_lines(state)

    if eof?(state) do
      {:error, "EOF"}
    else
      state = skip_hspace(state)

      case peek_char(state) do
        {:ok, ?#} -> parse_heading(state)
        {:ok, ?`} ->
          case match_string(state, "```") do
            {:ok, _} -> parse_code_block(state)
            :error -> parse_paragraph(state)
          end

        {:ok, c} when c in [?-, ?*, ?_] ->
          case parse_horizontal_rule(state) do
            {:ok, _block, _new_state} = result -> result
            {:error, _} -> parse_unordered_list(state)
          end

        {:ok, _} ->
          parse_paragraph(state)

        :error ->
          {:error, "EOF"}
      end
    end
  end

  defp parse_heading(state) do
    state = skip_hspace(state)
    {level, state} = count_hashes(state, 0)

    if level == 0 or level > 6 do
      {:error, "見出しレベルは1-6の範囲内である必要があります"}
    else
      state = skip_hspace(state)
      {text, state} = read_until_eol(state)
      state = consume_newline(state)
      inline = [{:text, String.trim(text)}]
      {:ok, {:heading, level, inline}, state}
    end
  end

  defp count_hashes(state, n) do
    case peek_char(state) do
      {:ok, ?#} -> count_hashes(advance_char(state), n + 1)
      _ -> {n, state}
    end
  end

  defp parse_horizontal_rule(state) do
    state = skip_hspace(state)
    {text, state} = read_until_eol(state)
    state = consume_newline(state)

    trimmed = String.trim(text)

    is_rule =
      (String.graphemes(trimmed) |> Enum.all?(&(&1 == "-")) and String.length(trimmed) >= 3) or
        (String.graphemes(trimmed) |> Enum.all?(&(&1 == "*")) and String.length(trimmed) >= 3) or
        (String.graphemes(trimmed) |> Enum.all?(&(&1 == "_")) and String.length(trimmed) >= 3)

    if is_rule do
      {:ok, :horizontal_rule, state}
    else
      {:error, "水平線として認識できません"}
    end
  end

  defp parse_code_block(state) do
    case match_string(state, "```") do
      :error ->
        {:error, "コードブロック開始が見つかりません"}

      {:ok, state} ->
        {lang_line, state} = read_until_eol(state)
        state = consume_newline(state)

        lang =
          case String.trim(lang_line) do
            "" -> nil
            trimmed -> trimmed
          end

        {code_lines, state} = read_code_lines(state, [])
        state = consume_newline(state)

        code = Enum.join(code_lines, "\n")
        {:ok, {:code_block, lang, code}, state}
    end
  end

  defp read_code_lines(state, acc) do
    case match_string(state, "```") do
      {:ok, state} ->
        {Enum.reverse(acc), state}

      :error ->
        if eof?(state) do
          {Enum.reverse(acc), state}
        else
          {line, state} = read_until_eol(state)
          state = consume_newline(state)
          read_code_lines(state, [line | acc])
        end
    end
  end

  defp parse_unordered_list(state) do
    {items, state} = parse_list_items(state, [])

    if Enum.empty?(items) do
      {:error, "リスト項目が見つかりません"}
    else
      {:ok, {:unordered_list, Enum.reverse(items)}, state}
    end
  end

  defp parse_list_items(state, acc) do
    state = skip_hspace(state)

    case peek_char(state) do
      {:ok, c} when c in [?-, ?*] ->
        state = advance_char(state)
        state = skip_hspace(state)
        {text, state} = read_until_eol(state)
        state = consume_newline(state)
        inline = [{:text, String.trim(text)}]
        parse_list_items(state, [inline | acc])

      _ ->
        {acc, state}
    end
  end

  defp parse_paragraph(state) do
    {lines, state} = read_paragraph_lines(state, [])
    text = lines |> Enum.join(" ") |> String.trim()
    inline = [{:text, text}]
    {:ok, {:paragraph, inline}, state}
  end

  defp read_paragraph_lines(state, acc) do
    if eof?(state) do
      {Enum.reverse(acc), state}
    else
      case peek_char(state) do
        {:ok, ?\n} ->
          state = advance_char(state)

          case peek_char(state) do
            {:ok, ?\n} -> {Enum.reverse(acc), state}
            _ -> read_paragraph_lines(state, ["" | acc])
          end

        {:ok, _} ->
          {line, state} = read_until_eol(state)
          state = consume_newline(state)
          read_paragraph_lines(state, [line | acc])

        :error ->
          {Enum.reverse(acc), state}
      end
    end
  end

  # パーサーユーティリティ

  defp peek_char(%{input: input, position: pos}) do
    case String.at(input, pos) do
      nil -> :error
      char -> {:ok, String.to_charlist(char) |> hd()}
    end
  end

  defp advance_char(%{position: pos} = state) do
    %{state | position: pos + 1}
  end

  defp match_string(state, target) do
    %{input: input, position: pos} = state
    remaining = String.slice(input, pos..-1//1)

    if String.starts_with?(remaining, target) do
      new_pos = pos + String.length(target)
      {:ok, %{state | position: new_pos}}
    else
      :error
    end
  end

  defp skip_hspace(state) do
    case peek_char(state) do
      {:ok, c} when c in [?\s, ?\t] ->
        skip_hspace(advance_char(state))

      _ ->
        state
    end
  end

  defp skip_blank_lines(state) do
    case peek_char(state) do
      {:ok, ?\n} -> skip_blank_lines(advance_char(state))
      _ -> state
    end
  end

  defp read_until_eol(%{input: input, position: pos} = state) do
    remaining = String.slice(input, pos..-1//1)

    case String.split(remaining, "\n", parts: 2) do
      [line, _rest] ->
        {line, %{state | position: pos + String.length(line)}}

      [line] ->
        {line, %{state | position: pos + String.length(line)}}
    end
  end

  defp consume_newline(state) do
    case peek_char(state) do
      {:ok, ?\n} -> advance_char(state)
      _ -> state
    end
  end

  defp eof?(%{input: input, position: pos}) do
    pos >= String.length(input)
  end

  # レンダリング

  @spec render_to_string(document()) :: String.t()
  def render_to_string(doc) do
    doc |> Enum.map(&render_block/1) |> Enum.join()
  end

  defp render_inline(inlines) do
    inlines
    |> Enum.map(fn
      {:text, text} -> text
      {:strong, inner} -> "**#{render_inline(inner)}**"
      {:emphasis, inner} -> "*#{render_inline(inner)}*"
      {:code, code} -> "`#{code}`"
      {:link, text, url} -> "[#{render_inline(text)}](#{url})"
      :line_break -> "\n"
    end)
    |> Enum.join()
  end

  defp render_block(block) do
    case block do
      {:heading, level, inline} ->
        prefix = String.duplicate("#", level)
        "#{prefix} #{render_inline(inline)}\n\n"

      {:paragraph, inline} ->
        "#{render_inline(inline)}\n\n"

      {:unordered_list, items} ->
        items_str =
          items
          |> Enum.map(fn item -> "- #{render_inline(item)}\n" end)
          |> Enum.join()

        "#{items_str}\n"

      {:ordered_list, items} ->
        items_str =
          items
          |> Enum.with_index(1)
          |> Enum.map(fn {item, i} -> "#{i}. #{render_inline(item)}\n" end)
          |> Enum.join()

        "#{items_str}\n"

      {:code_block, lang, code} ->
        lang_str = if lang, do: lang, else: ""
        "```#{lang_str}\n#{code}\n```\n\n"

      :horizontal_rule ->
        "---\n\n"
    end
  end
end

# Unicode 3層モデル比較：
#
# Elixirでは String はUTF-8バイナリで、標準で3つのレベルの操作が可能：
#
# - byte_size(str) → バイト数
# - String.length(str) → Grapheme（書記素クラスター）数
# - String.codepoints(str) → コードポイント単位のリスト
#
# 例：
# str = "🇯🇵"  # 国旗絵文字（2つのコードポイント、1つのgrapheme）
# byte_size(str) # => 8
# String.length(str) # => 1 (grapheme)
# String.codepoints(str) # => ["🇯", "🇵"]
#
# ElixirはRemlの3層モデルに近い明示性を持ち、
# デフォルトでgrapheme単位の操作が可能なため、絵文字や結合文字の扱いが自然。