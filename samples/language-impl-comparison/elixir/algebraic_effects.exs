# 代数的効果を使うミニ言語 - Elixir 版
# Reml との比較: プロセスベースの効果エミュレーション

defmodule AlgebraicEffects do
  @moduledoc """
  代数的効果を Elixir のプロセスメッセージングでエミュレートする実装。

  Reml の代数的効果システムとの対比：
  - Reml: 言語レベルで effect/handler をサポート
  - Elixir: プロセス + GenServer でステートマシンとして実装
  - 効果は {:effect, :name, args} メッセージとして表現
  """

  # ミニ言語の式定義
  defmodule Expr do
    defstruct [:type, :args]

    def lit(n), do: %Expr{type: :lit, args: [n]}
    def var(name), do: %Expr{type: :var, args: [name]}
    def add(left, right), do: %Expr{type: :add, args: [left, right]}
    def mul(left, right), do: %Expr{type: :mul, args: [left, right]}
    def divide(left, right), do: %Expr{type: :div, args: [left, right]}
    def get(), do: %Expr{type: :get, args: []}
    def put(expr), do: %Expr{type: :put, args: [expr]}
    def fail(msg), do: %Expr{type: :fail, args: [msg]}
    def choose(left, right), do: %Expr{type: :choose, args: [left, right]}
  end

  # 効果ハンドラーのプロトコル
  defmodule EffectHandler do
    @doc """
    効果を処理する GenServer ベースのハンドラー。

    State: 可変状態を管理
    Except: 例外を捕捉して Result に変換
    Choose: 非決定的選択をリストで収集
    """

    use GenServer

    # クライアント API

    def start_link(init_state) do
      GenServer.start_link(__MODULE__, %{state: init_state, results: []})
    end

    def get_state(pid) do
      GenServer.call(pid, :get)
    end

    def put_state(pid, new_state) do
      GenServer.call(pid, {:put, new_state})
    end

    def raise_error(pid, msg) do
      GenServer.call(pid, {:raise, msg})
    end

    def choose(pid, left, right) do
      GenServer.call(pid, {:choose, left, right})
    end

    def get_final_state(pid) do
      GenServer.call(pid, :final_state)
    end

    # サーバーコールバック

    @impl true
    def init(initial_state) do
      {:ok, initial_state}
    end

    @impl true
    def handle_call(:get, _from, state) do
      {:reply, {:ok, state.state}, state}
    end

    @impl true
    def handle_call({:put, new_state}, _from, state) do
      {:reply, :ok, %{state | state: new_state}}
    end

    @impl true
    def handle_call({:raise, msg}, _from, state) do
      {:reply, {:error, msg}, state}
    end

    @impl true
    def handle_call({:choose, left, right}, _from, state) do
      # 非決定的選択: 両方の結果を収集
      {:reply, {:choices, [left, right]}, state}
    end

    @impl true
    def handle_call(:final_state, _from, state) do
      {:reply, state, state}
    end
  end

  # 式の評価関数（効果を持つ）
  @doc """
  式を評価する。効果は handler プロセス経由で処理される。

  Reml との違い:
  - Reml: perform Except.raise(msg) で効果を送出
  - Elixir: GenServer.call(handler, {:raise, msg}) でメッセージング
  """
  def eval(expr, env, handler) do
    case expr.type do
      :lit ->
        [n] = expr.args
        {:ok, n}

      :var ->
        [name] = expr.args
        case Enum.find(env, fn {k, _v} -> k == name end) do
          {_k, v} -> {:ok, v}
          nil -> EffectHandler.raise_error(handler, "未定義変数: #{name}")
        end

      :add ->
        [left, right] = expr.args
        with {:ok, l} <- eval(left, env, handler),
             {:ok, r} <- eval(right, env, handler) do
          {:ok, l + r}
        end

      :mul ->
        [left, right] = expr.args
        with {:ok, l} <- eval(left, env, handler),
             {:ok, r} <- eval(right, env, handler) do
          {:ok, l * r}
        end

      :div ->
        [left, right] = expr.args
        with {:ok, l} <- eval(left, env, handler),
             {:ok, r} <- eval(right, env, handler) do
          if r == 0 do
            EffectHandler.raise_error(handler, "ゼロ除算")
          else
            {:ok, div(l, r)}
          end
        end

      :get ->
        EffectHandler.get_state(handler)

      :put ->
        [e] = expr.args
        with {:ok, v} <- eval(e, env, handler),
             :ok <- EffectHandler.put_state(handler, v) do
          {:ok, v}
        end

      :fail ->
        [msg] = expr.args
        EffectHandler.raise_error(handler, msg)

      :choose ->
        [left, right] = expr.args
        with {:ok, l} <- eval(left, env, handler),
             {:ok, r} <- eval(right, env, handler) do
          {:choices, [l, r]}
        end
    end
  end

  # すべての効果をハンドル（State + Except + Choose）
  @doc """
  すべての効果を処理して結果を返す。

  Reml の handle ... do ... do ... に相当するが、
  Elixir ではプロセス起動 → 評価 → 結果収集の手順が必要。
  """
  def run_with_all_effects(expr, env, init_state) do
    {:ok, handler} = EffectHandler.start_link(init_state)

    result = case eval(expr, env, handler) do
      {:ok, value} ->
        final = EffectHandler.get_final_state(handler)
        {:ok, [{value, final.state}]}

      {:error, msg} ->
        {:error, msg}

      {:choices, values} ->
        final = EffectHandler.get_final_state(handler)
        results = Enum.map(values, fn v -> {v, final.state} end)
        {:ok, results}
    end

    GenServer.stop(handler)
    result
  end

  # テストケース
  def example_expressions do
    [
      {"単純な加算", Expr.add(Expr.lit(10), Expr.lit(20))},
      {"乗算と除算", Expr.divide(Expr.mul(Expr.lit(6), Expr.lit(7)), Expr.lit(2))},
      {"状態の取得", Expr.add(Expr.get(), Expr.lit(5))},
      {"状態の更新", Expr.put(Expr.add(Expr.get(), Expr.lit(1)))},
      {"ゼロ除算エラー", Expr.divide(Expr.lit(10), Expr.lit(0))},
      {"非決定的選択", Expr.choose(Expr.lit(1), Expr.lit(2))},
      {"複雑な例", Expr.add(
        Expr.choose(Expr.lit(10), Expr.lit(20)),
        Expr.put(Expr.add(Expr.get(), Expr.lit(1)))
      )}
    ]
  end

  # テスト実行
  def run_examples do
    examples = example_expressions()
    env = []
    init_state = 0

    Enum.each(examples, fn {name, expr} ->
      IO.puts("--- #{name} ---")
      case run_with_all_effects(expr, env, init_state) do
        {:ok, results} ->
          Enum.each(results, fn {value, state} ->
            IO.puts("  結果: #{value}, 状態: #{state}")
          end)

        {:error, err} ->
          IO.puts("  エラー: #{err}")
      end
    end)
  end
end

# Reml との比較メモ:
#
# 1. **効果の表現**
#    Reml: 言語レベルの effect/handler 構文
#    Elixir: プロセスメッセージング（GenServer）でエミュレート
#    - Reml はコンパイラが効果を追跡・最適化
#    - Elixir は実行時にプロセス間通信が発生（オーバーヘッド大）
#
# 2. **ハンドラーの合成**
#    Reml: handle state_handler(init) do ... で宣言的
#    Elixir: プロセス起動 → 評価 → 停止の手続き的フロー
#    - Reml の方がシンプルで理解しやすい
#
# 3. **非決定性の扱い**
#    Reml: choose_handler でリストを自動生成
#    Elixir: {:choices, [left, right]} を手動で管理
#    - Reml はハンドラーが自動で分岐を収集
#    - Elixir は明示的なパターンマッチが必要
#
# 4. **型安全性**
#    Reml: 効果が型レベルで追跡される
#    Elixir: 動的型付けのため実行時エラーのリスク
#
# 5. **パフォーマンス**
#    Reml: 効果はコンパイル時に最適化可能
#    Elixir: プロセス通信のオーバーヘッドが常に発生
#
# **結論**:
# Elixir は並行性に強いが、代数的効果の表現力では Reml に劣る。
# プロセスモデルは効果のエミュレーションに使えるが、
# 型安全性・パフォーマンス・可読性の面で Reml の方が優れている。

# テスト実行例
# AlgebraicEffects.run_examples()