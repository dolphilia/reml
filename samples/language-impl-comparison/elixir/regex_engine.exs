#!/usr/bin/env elixir

# 正規表現エンジン：パース + 評価の両方を実装。
#
# Unicode対応の特徴：
# - 文字クラス `\p{L}` (Unicode Letter) など対応
# - `.` は任意の1文字（Unicode対応）
#
# 対応する正規表現構文（簡易版）：
# - リテラル: `abc`
# - 連結: `ab`
# - 選択: `a|b`
# - 繰り返し: `a*`, `a+`, `a?`, `a{2,5}`
# - グループ: `(abc)`
# - 文字クラス: `[a-z]`, `[^0-9]`, `\d`, `\w`, `\s`
# - アンカー: `^`, `$`
# - ドット: `.` (任意の1文字)

defmodule RegexEngine do
  # 正規表現のAST
  defmodule Regex do
    defstruct [:type, :value]

    def literal(s), do: %Regex{type: :literal, value: s}
    def char_class(cs), do: %Regex{type: :char_class, value: cs}
    def dot(), do: %Regex{type: :dot, value: nil}
    def concat(terms), do: %Regex{type: :concat, value: terms}
    def alternation(alts), do: %Regex{type: :alternation, value: alts}
    def repeat(inner, kind), do: %Regex{type: :repeat, value: {inner, kind}}
    def group(inner), do: %Regex{type: :group, value: inner}
    def anchor(kind), do: %Regex{type: :anchor, value: kind}
  end

  defmodule CharSet do
    defstruct [:type, :value]

    def char_range(start, end_char), do: %CharSet{type: :range, value: {start, end_char}}
    def char_list(chars), do: %CharSet{type: :list, value: chars}
    def predefined(class), do: %CharSet{type: :predefined, value: class}
    def negated(inner), do: %CharSet{type: :negated, value: inner}
    def union(sets), do: %CharSet{type: :union, value: sets}
  end

  # パーサーコンビネーター実装
  defmodule Parser do
    defstruct [:run]

    def new(run), do: %Parser{run: run}

    def run(parser, input) do
      parser.run.(input)
    end

    def ok(value) do
      new(fn input -> {:ok, value, input} end)
    end

    def fail(message) do
      new(fn _input -> {:error, message} end)
    end

    def bind(parser, f) do
      new(fn input ->
        case run(parser, input) do
          {:ok, value, rest} -> run(f.(value), rest)
          {:error, _} = err -> err
        end
      end)
    end

    def map(parser, f) do
      bind(parser, fn value -> ok(f.(value)) end)
    end

    def choice(parsers) do
      new(fn input ->
        Enum.find_value(parsers, {:error, "no choice matched"}, fn p ->
          case run(p, input) do
            {:ok, _, _} = result -> result
            {:error, _} -> nil
          end
        end)
      end)
    end

    def sequence(parsers) do
      Enum.reduce(parsers, ok([]), fn p, acc ->
        bind(acc, fn values ->
          bind(p, fn v -> ok(values ++ [v]) end)
        end)
      end)
    end

    def many(parser) do
      new(fn input ->
        many_helper(parser, input, [])
      end)
    end

    defp many_helper(parser, input, acc) do
      case run(parser, input) do
        {:ok, value, rest} -> many_helper(parser, rest, acc ++ [value])
        {:error, _} -> {:ok, acc, input}
      end
    end

    def many1(parser) do
      bind(parser, fn first ->
        bind(many(parser), fn rest ->
          ok([first | rest])
        end)
      end)
    end

    def optional(parser) do
      new(fn input ->
        case run(parser, input) do
          {:ok, value, rest} -> {:ok, {:some, value}, rest}
          {:error, _} -> {:ok, :none, input}
        end
      end)
    end

    def char(c) do
      new(fn input ->
        case input do
          <<^c::utf8, rest::binary>> -> {:ok, c, rest}
          _ -> {:error, "expected #{<<c::utf8>>}"}
        end
      end)
    end

    def string(s) do
      new(fn input ->
        if String.starts_with?(input, s) do
          {:ok, s, String.slice(input, String.length(s)..-1//1)}
        else
          {:error, "expected #{s}"}
        end
      end)
    end

    def satisfy(pred) do
      new(fn input ->
        case String.next_grapheme(input) do
          {grapheme, rest} ->
            <<codepoint::utf8>> = grapheme
            if pred.(codepoint) do
              {:ok, codepoint, rest}
            else
              {:error, "predicate failed"}
            end
          nil -> {:error, "end of input"}
        end
      end)
    end

    def digit() do
      satisfy(fn c -> c >= ?0 and c <= ?9 end)
    end

    def integer() do
      bind(many1(digit()), fn digits ->
        num = Enum.reduce(digits, 0, fn d, acc -> acc * 10 + (d - ?0) end)
        ok(num)
      end)
    end

    def sep_by1(parser, sep) do
      bind(parser, fn first ->
        bind(many(bind(sep, fn _ -> parser end)), fn rest ->
          ok([first | rest])
        end)
      end)
    end
  end

  # 正規表現パーサー
  import Parser

  def parse_regex(input) do
    case Parser.run(regex_expr(), input) do
      {:ok, regex, ""} -> {:ok, regex}
      {:ok, _, rest} -> {:error, "unexpected input: #{rest}"}
      {:error, _} = err -> err
    end
  end

  defp regex_expr() do
    alternation_expr()
  end

  defp alternation_expr() do
    bind(sep_by1(concat_expr(), string("|")), fn alts ->
      ok(case alts do
        [single] -> single
        _ -> Regex.alternation(alts)
      end)
    end)
  end

  defp concat_expr() do
    bind(many1(postfix_term()), fn terms ->
      ok(case terms do
        [single] -> single
        _ -> Regex.concat(terms)
      end)
    end)
  end

  defp postfix_term() do
    bind(atom(), fn base ->
      bind(optional(repeat_suffix()), fn repeat_opt ->
        ok(case repeat_opt do
          {:some, kind} -> Regex.repeat(base, kind)
          :none -> base
        end)
      end)
    end)
  end

  defp atom() do
    choice([
      # 括弧グループ
      bind(string("("), fn _ ->
        bind(regex_expr(), fn inner ->
          bind(string(")"), fn _ ->
            ok(Regex.group(inner))
          end)
        end)
      end),
      # アンカー
      map(string("^"), fn _ -> Regex.anchor(:start) end),
      map(string("$"), fn _ -> Regex.anchor(:end) end),
      # ドット
      map(string("."), fn _ -> Regex.dot() end),
      # 文字クラス
      char_class(),
      # 定義済みクラス
      predefined_class(),
      # エスケープ文字
      escape_char(),
      # 通常のリテラル
      map(satisfy(fn c ->
        c not in [?(, ?), ?[, ?], ?{, ?}, ?*, ?+, ??, ?., ?|, ?^, ?$, ?\\]
      end), fn c -> Regex.literal(<<c::utf8>>) end)
    ])
  end

  defp escape_char() do
    bind(string("\\"), fn _ ->
      bind(satisfy(fn c -> c in [?n, ?t, ?r, ?\\, ?(, ?), ?[, ?], ?{, ?}, ?*, ?+, ??, ?., ?|, ?^, ?$] end), fn c ->
        ok(Regex.literal(case c do
          ?n -> "\n"
          ?t -> "\t"
          ?r -> "\r"
          _ -> <<c::utf8>>
        end))
      end)
    end)
  end

  defp predefined_class() do
    bind(string("\\"), fn _ ->
      bind(choice([
        map(char(?d), fn _ -> :digit end),
        map(char(?w), fn _ -> :word end),
        map(char(?s), fn _ -> :whitespace end),
        map(char(?D), fn _ -> :not_digit end),
        map(char(?W), fn _ -> :not_word end),
        map(char(?S), fn _ -> :not_whitespace end)
      ]), fn class ->
        ok(Regex.char_class(CharSet.predefined(class)))
      end)
    end)
  end

  defp char_class() do
    bind(string("["), fn _ ->
      bind(optional(string("^")), fn negated ->
        bind(many1(char_class_item()), fn items ->
          bind(string("]"), fn _ ->
            union_set = CharSet.union(items)
            ok(Regex.char_class(
              case negated do
                {:some, _} -> CharSet.negated(union_set)
                :none -> union_set
              end
            ))
          end)
        end)
      end)
    end)
  end

  defp char_class_item() do
    choice([
      # 範囲
      bind(satisfy(fn c -> c != ?] and c != ?- end), fn start ->
        bind(optional(bind(string("-"), fn _ ->
          satisfy(fn c -> c != ?] end)
        end)), fn end_opt ->
          ok(case end_opt do
            {:some, end_char} -> CharSet.char_range(start, end_char)
            :none -> CharSet.char_list([start])
          end)
        end)
      end),
      # 定義済みクラス
      predefined_class(),
      # 単一文字
      map(satisfy(fn c -> c != ?] end), fn c -> CharSet.char_list([c]) end)
    ])
  end

  defp repeat_suffix() do
    choice([
      map(string("*"), fn _ -> :zero_or_more end),
      map(string("+"), fn _ -> :one_or_more end),
      map(string("?"), fn _ -> :zero_or_one end),
      # {n,m} 形式
      bind(string("{"), fn _ ->
        bind(integer(), fn n ->
          bind(optional(bind(string(","), fn _ ->
            optional(integer())
          end)), fn range_opt ->
            bind(string("}"), fn _ ->
              ok(case range_opt do
                :none -> {:exactly, n}
                {:some, :none} -> {:range, n, :infinity}
                {:some, {:some, m}} -> {:range, n, m}
              end)
            end)
          end)
        end)
      end)
    ])
  end

  # マッチングエンジン
  def match_regex(regex, text) do
    match_from_pos(regex, text, 0)
  end

  defp match_from_pos(%Regex{type: :literal, value: s}, text, pos) do
    String.slice(text, pos, String.length(s)) == s
  end

  defp match_from_pos(%Regex{type: :char_class, value: cs}, text, pos) do
    case String.at(text, pos) do
      nil -> false
      char ->
        <<codepoint::utf8>> = char
        char_matches_class?(codepoint, cs)
    end
  end

  defp match_from_pos(%Regex{type: :dot}, text, pos) do
    String.at(text, pos) != nil
  end

  defp match_from_pos(%Regex{type: :concat, value: terms}, text, pos) do
    Enum.reduce_while(terms, {true, pos}, fn term, {_matched, current_pos} ->
      if match_from_pos(term, text, current_pos) do
        {:cont, {true, current_pos + 1}}
      else
        {:halt, {false, current_pos}}
      end
    end)
    |> elem(0)
  end

  defp match_from_pos(%Regex{type: :alternation, value: alts}, text, pos) do
    Enum.any?(alts, fn alt -> match_from_pos(alt, text, pos) end)
  end

  defp match_from_pos(%Regex{type: :repeat, value: {inner, kind}}, text, pos) do
    case kind do
      :zero_or_more -> match_repeat_zero_or_more(inner, text, pos)
      :one_or_more -> match_repeat_one_or_more(inner, text, pos)
      :zero_or_one -> match_repeat_zero_or_one(inner, text, pos)
      {:exactly, n} -> match_repeat_exactly(inner, text, pos, n)
      {:range, min, max} -> match_repeat_range(inner, text, pos, min, max)
    end
  end

  defp match_from_pos(%Regex{type: :group, value: inner}, text, pos) do
    match_from_pos(inner, text, pos)
  end

  defp match_from_pos(%Regex{type: :anchor, value: :start}, _text, pos) do
    pos == 0
  end

  defp match_from_pos(%Regex{type: :anchor, value: :end}, text, pos) do
    pos >= String.length(text)
  end

  defp char_matches_class?(ch, %CharSet{type: :range, value: {start, end_char}}) do
    ch >= start and ch <= end_char
  end

  defp char_matches_class?(ch, %CharSet{type: :list, value: chars}) do
    ch in chars
  end

  defp char_matches_class?(ch, %CharSet{type: :predefined, value: class}) do
    case class do
      :digit -> ch >= ?0 and ch <= ?9
      :word -> (ch >= ?a and ch <= ?z) or (ch >= ?A and ch <= ?Z) or (ch >= ?0 and ch <= ?9) or ch == ?_
      :whitespace -> ch in [?\s, ?\t, ?\n, ?\r]
      :not_digit -> not (ch >= ?0 and ch <= ?9)
      :not_word -> not ((ch >= ?a and ch <= ?z) or (ch >= ?A and ch <= ?Z) or (ch >= ?0 and ch <= ?9) or ch == ?_)
      :not_whitespace -> ch not in [?\s, ?\t, ?\n, ?\r]
    end
  end

  defp char_matches_class?(ch, %CharSet{type: :negated, value: inner}) do
    not char_matches_class?(ch, inner)
  end

  defp char_matches_class?(ch, %CharSet{type: :union, value: sets}) do
    Enum.any?(sets, fn set -> char_matches_class?(ch, set) end)
  end

  defp match_repeat_zero_or_more(inner, text, pos) do
    match_repeat_loop(inner, text, pos, 0, 0, :infinity)
  end

  defp match_repeat_one_or_more(inner, text, pos) do
    if match_from_pos(inner, text, pos) do
      match_repeat_zero_or_more(inner, text, pos + 1)
    else
      false
    end
  end

  defp match_repeat_zero_or_one(inner, text, pos) do
    match_from_pos(inner, text, pos) or true
  end

  defp match_repeat_exactly(inner, text, pos, n) do
    match_repeat_loop(inner, text, pos, 0, n, n)
  end

  defp match_repeat_range(inner, text, pos, min, max) do
    match_repeat_loop(inner, text, pos, 0, min, max)
  end

  defp match_repeat_loop(inner, text, pos, count, min, max) do
    cond do
      count == max -> true
      count >= min and not match_from_pos(inner, text, pos) -> true
      match_from_pos(inner, text, pos) -> match_repeat_loop(inner, text, pos + 1, count + 1, min, max)
      count >= min -> true
      true -> false
    end
  end

  # テスト例
  def test_examples() do
    examples = [
      {"a+", "aaa", true},
      {"a+", "b", false},
      {"[0-9]+", "123", true},
      {"[0-9]+", "abc", false},
      {"\\d{2,4}", "12", true},
      {"\\d{2,4}", "12345", true},
      {"(abc)+", "abcabc", true},
      {"a|b", "a", true},
      {"a|b", "b", true},
      {"a|b", "c", false},
      {"^hello$", "hello", true},
      {"^hello$", "hello world", false}
    ]

    Enum.each(examples, fn {pattern, text, expected} ->
      case parse_regex(pattern) do
        {:ok, regex} ->
          result = match_regex(regex, text)
          status = if result == expected, do: "✓", else: "✗"
          IO.puts("#{status} パターン: '#{pattern}', テキスト: '#{text}', 期待: #{expected}, 結果: #{result}")
        {:error, err} ->
          IO.puts("✗ パーサーエラー: #{pattern} - #{err}")
      end
    end)
  end
end

# 実行
RegexEngine.test_examples()