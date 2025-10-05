defmodule JsonParser do
  @moduledoc """
  JSON 文字列を再帰下降で解析するミニ実装。
  戻り値は `{:ok, value}` もしくは `{:error, reason}`。
  """

  @type json ::
          :null
          | {:bool, boolean()}
          | {:number, float()}
          | {:string, String.t()}
          | {:array, [json()]}
          | {:object, %{optional(String.t()) => json()}}

  @type result :: {:ok, json()} | {:error, String.t()}

  @spec parse(String.t()) :: result
  def parse(source) do
    with {:ok, value, rest} <- parse_value(source),
         true <- String.trim(rest) == "" do
      {:ok, value}
    else
      false -> {:error, "未消費文字が残っています"}
      {:error, reason} -> {:error, reason}
    end
  end

  defp parse_value(source) do
    trimmed = skip_ws(source)

    case trimmed do
      <<"null", rest::binary>> -> {:ok, :null, rest}
      <<"true", rest::binary>> -> {:ok, {:bool, true}, rest}
      <<"false", rest::binary>> -> {:ok, {:bool, false}, rest}
      <<?", rest::binary>> -> parse_string(rest)
      <<?[ , rest::binary>> -> parse_array(rest)
      <<?{ , rest::binary>> -> parse_object(rest)
      <<?- , _::binary>> -> parse_number(trimmed)
      <<digit, _::binary>> when digit in ?0..?9 -> parse_number(trimmed)
      <<>> -> {:error, "入力が途中で終了しました"}
      _ -> {:error, "想定外の文字列です"}
    end
  end

  defp parse_array(source), do: array_values(skip_ws(source), [])

  defp array_values(<<?] , rest::binary>>, acc), do: {:ok, {:array, Enum.reverse(acc)}, rest}

  defp array_values(<<>>, _acc), do: {:error, "配列が途中で終了しました"}

  defp array_values(source, acc) do
    with {:ok, value, rest} <- parse_value(source),
         trimmed <- skip_ws(rest) do
      case trimmed do
        <<?, , more::binary>> -> array_values(more, [value | acc])
        <<?] , more::binary>> -> {:ok, {:array, Enum.reverse([value | acc])}, more}
        <<>> -> {:error, "配列が途中で終了しました"}
        _ -> {:error, "配列内の区切りが不正です"}
      end
    end
  end

  defp parse_object(source), do: object_members(skip_ws(source), %{})

  defp object_members(<<?} , rest::binary>>, acc), do: {:ok, {:object, acc}, rest}
  defp object_members(<<>>, _acc), do: {:error, "オブジェクトが途中で終了しました"}

  defp object_members(source, acc) do
    with <<?", rest::binary>> <- source,
         {:ok, key, after_key} <- parse_string(rest),
         trimmed <- skip_ws(after_key),
         <<?: , after_colon::binary>> <- trimmed,
         {:ok, value, after_value} <- parse_value(after_colon) do
      case skip_ws(after_value) do
        <<?, , more::binary>> -> object_members(more, Map.put(acc, key, value))
        <<?} , more::binary>> -> {:ok, {:object, Map.put(acc, key, value)}, more}
        <<>> -> {:error, "オブジェクトが途中で終了しました"}
        _ -> {:error, "オブジェクト内の区切りが不正です"}
      end
    else
      :error -> {:error, "キー文字列を期待しました"}
      _ -> {:error, "オブジェクトの構造が不正です"}
    end
  end

  defp parse_number(source) do
    {token, rest} = take_number(source, "")

    case Float.parse(token) do
      {number, ""} -> {:ok, {:number, number}, rest}
      _ -> {:error, "数値の解釈に失敗しました"}
    end
  end

  defp take_number(<<char, rest::binary>>, acc) when char in '0123456789-+eE.' do
    take_number(rest, acc <> <<char>>)
  end

  defp take_number(source, acc), do: {acc, source}

  defp parse_string(source), do: string_chars(source, [])

  defp string_chars(<<>>, _acc), do: {:error, "文字列が閉じていません"}
  defp string_chars(<<?" , rest::binary>>, acc), do: {:ok, Enum.reverse(acc) |> List.to_string(), rest}

  defp string_chars(<<?\" , rest::binary>>, acc) do
    case rest do
      <<?" , more::binary>> -> string_chars(more, [?" | acc])
      <<?\ , more::binary>> -> string_chars(more, [?\ | acc])
      <<?/ , more::binary>> -> string_chars(more, [?/ | acc])
      <<?b , more::binary>> -> string_chars(more, [?\b | acc])
      <<?f , more::binary>> -> string_chars(more, [?\f | acc])
      <<?n , more::binary>> -> string_chars(more, [?\n | acc])
      <<?r , more::binary>> -> string_chars(more, [?\r | acc])
      <<?t , more::binary>> -> string_chars(more, [?\t | acc])
      <<?u, a, b, c, d, more::binary>> ->
        with {:ok, codepoint} <- take_unicode(a, b, c, d) do
          string_chars(more, [codepoint | acc])
        end
      _ -> {:error, "不正なエスケープシーケンスです"}
    end
  end

  defp string_chars(<<char, rest::binary>>, acc), do: string_chars(rest, [char | acc])

  defp take_unicode(a, b, c, d) do
    hex = <<a, b, c, d>>

    case Integer.parse(hex, 16) do
      {code, ""} -> {:ok, code}
      _ -> {:error, "Unicode エスケープが不正です"}
    end
  end

  defp skip_ws(<<c, rest::binary>>) when c in ' \t\n\r', do: skip_ws(rest)
  defp skip_ws(rest), do: rest
end
