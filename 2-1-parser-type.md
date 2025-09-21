# 2.1 パーサ型

> 目標：**小さく強いコア**で、**高品質エラー**と**実用性能（ゼロコピー・Packrat/左再帰対応）**を両立。
> 原則：**純粋（副作用なし）**・**Unicode前提**・**デフォルト安全**。
> スコープ：パーサの**型**・**入出力モデル**・**実行時状態**・**コミット/消費の意味論**を確定します（詳細なエラー統合は *2.5* で掘り下げ）。

---

## A. 主要型

```reml
// コア：パーサは Input を読み、成功/失敗と残り入力を返す純関数
type Parser<T> = fn(&mut State) -> Reply<T>

// 実行結果（consumed/committed の2ビットを明示）
type Reply<T> =
  | Ok(value: T, rest: Input, span: Span, consumed: Bool)
  | Err(error: ParseError, consumed: Bool, committed: Bool)

// ランナーが外部へ返す“エラー不可能”結果（AST + 診断）
type ParseResult<T> = {
  value: Option<T>,                 // 成功時は値、失敗時は None
  span: Option<Span>,               // 値が存在する場合の全体スパン
  diagnostics: List<Diagnostic>,    // 2.5 で定義される診断の列
  recovered: Bool,                  // recover 等で補完した場合 true
  legacy_error: Option<ParseError>  // 互換モード用（cfg.legacy_result=true）
}

// 実行状態（不変入力 + 可変の解析状態）
type State = {
  input: Input,                // 現在の入力ビュー（不変データの参照＋オフセット）
  config: RunConfig,           // 実行設定（Packrat 等）
  memo: MemoTable,             // Packrat/左再帰用メモテーブル
  diag: DiagState,             // 最遠エラー等の集約
  trace: TraceState            // 追跡（オフ既定）
}
```

**ポイント**

* `Reply` は **4状態**を表現可能：
  `Ok(consumed=false/true)` / `Err(consumed=false/true, committed=false/true)`
  → `or` の分岐可否や `cut` の挙動を**分岐なし**で実装できる（Parsec 流の *empty/consumed* + *commit*）。
* `span` は **そのパーサが消費した範囲**（`Ok` のみ）。ノード単位の位置取りに使う。
* `ParseResult<T>` は **常に AST と Diagnostic の組**を返し、「値がないが診断が得られる」ケース（recover 後など）も扱う。旧来の `Result<(T, Span), ParseError>` は `RunConfig.legacy_result=true` で再利用できるが非推奨。

---

## B. 入力モデル `Input`

```reml
type Input = {
  source: SourceId,        // ファイル/文字列単位の識別子
  bytes: Bytes,            // UTF-8 本体（共有参照/COW）
  byte_off: usize,         // 現在の先頭（バイト）
  line: usize,             // 現在の行番号（1-origin）
  column: usize,           // 現在の列（拡張書記素基準、1-origin）
  // 境界キャッシュ（必要時だけ構築、ビュー間で共有）
  cp_index: Option<CpIndex>,    // コードポイント境界表
  g_index: Option<GraphemeIndex>// グラフェム境界表
}
```

* **不変ビュー**：`Input` は参照共有の **ゼロコピー**スライス。`rest` は **オフセットを進めた写像**のみ。
* **位置**は 1.4 の文字モデルに準拠（行=LF 正規化、列=グラフェム）。
* `mark()/rewind()` は `Input` の**スナップショット**で安価に取れる（バックトラックに使用）。

---

## C. スパンとトレース

```reml
type Span = {
  source: SourceId,
  byte_start: usize, byte_end: usize,
  line_start: usize, col_start: usize,
  line_end: usize,   col_end: usize
}

// 成功断片の履歴（IDE/可視化目的）。既定は OFF。
type SpanTrace = List<(name: String, span: Span)>
```

* 既定では **成功スパンのみ**保持（軽量）。
* `.spanned()` コンビネータで **「値 + Span」** を得る（AST への位置付与に使う）。
* `SpanTrace` は実行時 `RunConfig.trace = On` のときのみ収集（オーバーヘッド遮断）。

---

## D. 実行設定 `RunConfig` とメモ

```reml
type RunConfig = {
  require_eof: Bool = false,            // 全消費を要求（parse_all 相当）
  packrat: Bool = false,                // Packrat メモ化を明示的に有効化
  left_recursion: "off" | "on" | "auto" = "auto",
  trace: Bool = false,
  merge_warnings: Bool = true,
  legacy_result: Bool = false          // 旧 API (`Result<(T, Span), ParseError>`) 互換
}

type ParserId = u32  // ルール毎に安定ID（rule()/label() が付与）
type MemoKey  = (ParserId, usize /*byte_off*/)
type MemoVal<T> = Reply<T>  // Ok/Err ごと丸ごとキャッシュ
type MemoTable = Map<MemoKey, Any>  // 実装上は型消去（内部用）
```

* **RunConfig の主な項目**
  - `require_eof` で余剰入力を許可するかを選択。
  - `packrat` と `left_recursion` は Packrat メモ化と seed-growing 左再帰の利用可否を制御。
  - `trace` は `SpanTrace` 収集を有効化し、診断に詳細な履歴を残す。
  - `merge_warnings` は連続する回復警告を集約してノイズを抑制する。
  - `legacy_result` は旧 API (`Result<(T, Span), ParseError>`) を返す互換モード（移行期間限定）。
* `rule(name, p)` が **ParserId とラベル**を付与し、Packrat と診断に使う。

---

## E. コミットと消費の意味論

* `consumed`：**入力を1バイト以上前進**したか。
* `committed`：`cut` 境界を**越えた**とマーク（消費の有無に関わらず）。

**合成の基本規則（抜粋）**

* `p.or(q)`：

  * `p` が `Err(consumed=true, _ )` または `Err(_, committed=true)` → **q を試さない**。
  * `p` が `Err(consumed=false, committed=false)` → **q を試す**。
* `p.then(q)`：

  * `p` が `Ok(consumed=*)` → `q` へ続行（`consumed` は合成：`p||q`）
  * `p` が `Err` → そのまま `Err`。
* `cut`：以降で失敗したら **`committed=true`** を返す（期待集合は 2.5 参照）。
* `label("x", p)`：`p` の期待名を `"x"` に差し替え（エラー統合で優先）。

> この規則で **`try` 相当**は不要：`cut` を使わず書けば *empty エラー* として `or` に落ちる。必要なら `recover` を使う。

---

## F. 失敗表現（最小要素：2.5 と両立）

```reml
type ParseError = {
  at: Span,                           // 失敗位置（最狭）
  expected: Set<Expectation>,         // 期待集合（トークン/ラベル/EOF 等）
  context: List<Label>,               // 直近の label からの文脈
  committed: Bool,                    // cut を越えた失敗か
  notes: List<String>                 // 補助（回復やヒント）
}
```

* **Ok/Err に `consumed/committed` を分離**したことで、エラー統合（最遠位置の採用・期待セットの和/差）を**一意に定義**できる（詳細は *2.5*）。

---

## G. ランナー API（外部からの呼び出し）

```reml
fn run<T>(p: Parser<T>, src: String, cfg: RunConfig = {}) -> ParseResult<T>
// AST と診断を常に返す。cfg.require_eof=true なら余剰入力は Diagnostic として報告。

fn run_partial<T>(p: Parser<T>, src: String, cfg: RunConfig = {}) -> ParseResultWithRest<T>
// 部分パース：残り Input を `rest` に格納し、result.diagnostics も一緒に返す。

type ParseResultWithRest<T> = {
  result: ParseResult<T>,
  rest: Option<Input>
}

* `ParseResult` は成功/失敗にかかわらず診断を含むため、IDE や CI でのフィードバックが一貫する。
* `ParseResultWithRest` は REPL や差分適用で再利用しやすいよう、未消費入力を同梱する。
* `src` は `Input.bytes` へ参照共有され、コピーを発生させない。文字位置は 1.4 節の Unicode モデルに従う。

> ストリーミング処理や継続再開、バックプレッシャ制御などの高度なランナーは `Core.Parse.Streaming` 拡張（別途定義）で提供します。コア仕様ではバッチ実行と部分パースのみを扱います。
---

## H. 代数則（使用者向けの直観）

* **純度**：`Parser<T>` は参照透過（同じ `State` → 同じ `Reply`）。
* **Functor**：`map` は恒等・合成を保つ。
* **Applicative/Monadic**：`then/andThen` は結合律を満たす（エラー統合規則の範囲で）。
* **`or` の単位**：`fail("x")` は空失敗（`consumed=false, committed=false`）。
* **`cut`**：`label("x", cut(p))` で「ここから先は x を期待」を強制。

---

## I. プラグイン連携の位置付け

Reml コアの `Core.Parse` はプラグイン登録 API を持ちません。DSL 拡張や capability 管理が必要な場合は、別途提供されるプラグインガイド（`guides/DSL-plugin.md`）と関連拡張ライブラリを利用してください。これにより、コア API は小さく安定したまま、プロジェクト固有の拡張点を opt-in で追加できます。

---

## J. メモリと性能（実装規約）

* **Input**：COW/RC・SSO（短文字列インライン）・部分文字列は親バッファ参照。
* **Span**：必要最小を保持。`SpanTrace` は OFF 既定。
* **Packrat**：

  * キーは `(ParserId, byte_off)`、値は `Reply<T>`。
  * LRU/リングで上限を設け、巨大入力でのメモリ爆発を回避。
* **左再帰**：`left_recursion=true` のとき、既知の **種別変換法**（seed-growing）を使用（ルールに `ParserId` が必須）。
* **ステップ上限**：必要に応じて実装側が安全弁を設ける（診断には直近のルール列を含めることを推奨）。

### J-4. 拡張（Core.Async への導線）

非同期ランナーやバックプレッシャ制御を含むストリーミング実行はコア仕様の対象外です。必要に応じて `Core.Parse.Streaming` と `Core.Async` 系の拡張ライブラリを読み込み、ここで定義した `Parser` の意味論と互換な形で実装してください。

---

## K. ミニ例（意味論の確認）

```reml
// トークン
let sym = |s: Str| rule("sym(" + s + ")", Lex.symbol(sc, s))

// 式: atom ('*' atom)*
let atom: Parser<i64> =
  rule("atom",
    (Lex.int(10).map(|n| n)                      // Ok(..., consumed=true)
     .or(sym("(").then(expr).then(sym(")")).map(|(_,v,_)| v))  // 括弧
     .or(label("number or '('", fail())))        // 空失敗 → or が次を試す
  )

let term: Parser<i64> =
  rule("term",
    atom.andThen( many( sym("*").cut().then(atom) ) )
        .map(|(h, tail)| tail.fold(h, |a, (_,b)| a * b))
  )
// '*' の直後に cut → 以降の err は committed=true になり、
// `atom or (...)` に戻らず “ここは '*' の右項が必要” と報告される。
```

---

## K. 仕様チェックリスト

* [ ] `Reply` は **Ok/Err × consumed/committed** を表現（4状態）。
* [ ] `Input` は UTF-8/COW、行=LF正規化、列=グラフェム、**ゼロコピー**。
* [ ] `Span` は**開始/終了の行列＋バイト**を保持。
* [ ] `run / run_partial` の外部 API を定義（`require_eof` などバッチ実行に必要な選択肢のみ）。
* [ ] `RunConfig` で **Packrat/左再帰/トレース**を切替。
* [ ] `rule(name, p)` で **ParserId/ラベル**を付与（Packrat & 診断）。
* [ ] `or/then/cut/label` の**合成規則**を確定。
* [ ] メモ上限・ステップ上限の**安全弁**を持つ。
* [ ] 文字モデル（1.4）と**列=グラフェム**で位置整合。
* [ ] すべて**純関数**（1.3 効果）— 外界作用は `Parser` の外で扱う。

---

### まとめ

* `Parser<T> = fn(&mut State) -> Reply<T>` による**最小核**に、

  * **consumed/committed** の2ビット、
  * **ゼロコピー入力と正確な Span**、
  * **Packrat/左再帰/トレース**の *ON/OFF* を備え、
* **書きやすさ**（`cut/label` が直観的）、**読みやすさ**（`rule` 命名・位置情報）、**エラー品質**（期待集合×最遠位置）、**性能**（メモ化・ゼロコピー）を同時に満たします。

この 2.1 を土台に、次は **2.2 コア・コンビネータ**で API の最小公理系を詰めましょう。
