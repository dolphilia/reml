# 2.1 パーサ型

> 目標：**小さく強いコア**で、**高品質エラー**と**実用性能（ゼロコピー・Packrat/左再帰ガード）**を両立。
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
* `ParseResult.recovered` は `recover` による同期・継続が 1 回でも発生した場合に true となる（2-5 §E、2-6 §B-2-2）。Build/CI で `extensions["recover"].mode="off"` の場合は recover が発火しないため、既定では false のままになる。


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
  legacy_result: Bool = false,         // 旧 API (`Result<(T, Span), ParseError>`) 互換
  locale: Option<Locale> = None,      // 診断・Pretty 表示のロケール
  extensions: RunConfigExtensions = {} // モジュール毎の拡張設定
}

type RunConfigExtensions = Map<Str, Any>

type ParserId = u32  // ルール毎に安定ID（rule()/label() が付与）
type MemoKey  = (ParserId, usize /*byte_off*/)
type MemoVal<T> = Reply<T>  // Ok/Err ごと丸ごとキャッシュ
type MemoTable = Map<MemoKey, Any>  // 実装上は型消去（内部用）
```

* **RunConfig の主な項目**
  - `require_eof` で余剰入力を許可するかを選択。
  - `packrat` と `left_recursion` は Packrat メモ化と seed-growing 左再帰ガードの利用可否を制御。左再帰文法の直接記述は想定しない。
  - `trace` は `SpanTrace` 収集を有効化し、診断に詳細な履歴を残す。
  - `merge_warnings` は連続する回復警告を集約してノイズを抑制する。
  - `legacy_result` は旧 API (`Result<(T, Span), ParseError>`) を返す互換モード（移行期間限定）。
  - `locale` は **呼び出し元指定 → 環境変数（`REML_LOCALE` / `LANG`）→ 既定値 (`Locale::EN_US`)** の優先順位で決定する。
    解決したロケールは `PrettyOptions.locale` と `PrettyOptions.expectation_locale` の既定値に同期され、`PrettyOptions` 側で
    個別指定がある場合はそちらを尊重する。CLI・LSP などフロントエンドは `RunConfig.locale` に値を渡す際、明示指定が無く
    環境変数も欠落していれば **初回のみ「ロケール未指定」警告を出して英語 UI へフォールバック**する。
  - `extensions` は LSP 連携・シンタックスハイライト・監査ログ・GC など、各拡張モジュールが提供する設定をネームスペース付きで保持する。例：`extensions["lsp"]`（LSP ガイド参照）、`extensions["runtime"]`（Core.Runtime 草案参照）。Phase 2-5 では `extensions["effects"].value_restriction_mode`（`"strict"` / `"legacy"`）と `extensions["effects"].max_handler_depth` を定義し、CLI の `--value-restriction={strict|legacy}`／`--legacy-value-restriction` スイッチや Typer の値制限判定と同期させる。【P:docs/plans/bootstrap-roadmap/2-5-proposals/TYPE-001-proposal.md†L52-L154】【R:docs/plans/bootstrap-roadmap/2-5-review-log.md†L22-L38】


`extensions` の既定ネームスペース（推奨）

| key | 用途 | 参照 |
| --- | --- | --- |
| `"effects"` | `value_restriction_mode: "strict"|"legacy"`, `max_handler_depth: Int` | 1-2 §C.3, 1-3 §B, 2-6 §B-2, 3-6 §10 |
| `"lsp"` | LSP/IDE 連携の挙動・シンタックスハイライト設定 | guides/lsp-integration.md |
| `"stream"` | ストリーミングランナー設定（`enabled`, `chunk_size`, `flow`, `stats` 等） | guides/core-parse-streaming.md §10, docs/spec/2-7-core-parse-streaming.md |
| `"runtime"` | GC・監査・メトリクスなど実行時基盤の設定 | 3-8-core-runtime-capability.md, guides/runtime-bridges.md |
| `"logging"` | 構造化ログ・フォーマット設定（例：`format = "json"`） | guides/lsp-integration.md, guides/config-cli.md |
| `"i18n"` | ロケール検出・未翻訳メッセージの記録、翻訳カタログのホットリロード | guides/lsp-integration.md |
| その他 | プロジェクト固有の拡張。キー重複を避けるため固有 prefix を推奨 | - |
* `rule(name, p)` が **ParserId とラベル**を付与し、Packrat と診断に使う。
* ストリーミング解析で CLI/LSP の統計を同期させるには、`extensions["stream"] = { "enabled": true, "stats": true, ... }` のように `stats` フラグを明示する。`stats = true` のときは `stream_meta` テレメトリが PublishDiagnostics / JSON レポートに追加され、`collect-iterator-audit-metrics.py --section streaming` のカバレッジ判定が有効になる。

### D-1. `RunConfig` ユーティリティ {#runconfig-utilities}

```reml
impl RunConfig {
  fn with_extension(self, key: Str, update: fn(Map<Str, Any>) -> Map<Str, Any>) -> RunConfig = todo
}
```

* `with_extension` は `extensions` の該当エントリを**イミュータブルに更新**する。`update` には既存値（未登録時は空マップ）を受け取り、新しい `Map<Str, Any>` を返すクロージャを渡す。戻り値は新しい `RunConfig` であり、元の設定は変更されない。
* 典型例：字句設定を共有するため `cfg.with_extension("lex", |map| map.insert("space", Any::from(space.space_id())))` のように呼び出し、CLI/LSP が同じ空白パーサ（`ParserId`）を再構成できるようにする。【参照: 2-3-lexer.md §L-4】
* `update` 内で `Any` に `ParserId` や `ConfigTriviaProfile` など具体型を格納し、取り出し側で型チェックを行う。これにより 0-1 §1.2 の安全性（型崩壊の防止）を担保する。
* ミュータブル更新が必要な場合は `RunConfig` を `mut` で受け取り、`cfg = cfg.with_extension(...)` の形で差し替える。`RunConfig` 自体は `Copy` ではないため、所有権移動と再代入が発生する点に留意する。

#### 利用例（CLI/LSP 共通設定） {#runconfig-cli-lsp-example}

```reml
fn configure(parser: Parser<Any>, file: Path, project_id: ProjectId) -> () = {
  let base = RunConfig{};
  let shared =
    base
      .with_extension("lex", |map| {
        map.insert("profile", Any::from(ConfigTriviaProfile::strict_json));
        map
      })
      .with_extension("recover", |map| {
        map.insert("mode", Any::from("collect"));
        map.insert("sync_tokens", Any::from(Set::from([";", "\n"])));
        map.insert("max_diagnostics", Any::from(64));
        map.insert("max_resync_bytes", Any::from(4096));
        map.insert("max_recoveries", Any::from(128));
        map.insert("notes", Any::from(true));
        map
      })
      .with_extension("stream", |map| {
        map.insert("resume_hint", Any::from(DemandHint{
          min_bytes: 256,
          preferred_bytes: Some(1024),
          frame_boundary: None
        }));
        map
      });

  Core.CLI.parse_file(parser, file, shared);
  Core.LSP.Parser.attach(project_id, parser, shared);
}
```

* CLI/LSP が同じ `RunConfig` を受け取ることで、字句設定・回復戦略・ストリーミングヒントが一致する。`collect-iterator-audit-metrics.py` はこの設定を JSON・監査ログから読み取り、`parser.runconfig_extension_pass_rate` を算出する。
* `RunConfig` を再利用しつつ、必要な経路だけ追加の `with_extension` をチェーンさせることで、DSL プラグインなど拡張モジュールが独自設定を注入できる（Phase 2-7 `EXEC-001` 参照）。

### D-2. 公式スイッチと既定値 {#runconfig-official-switches}

| 項目 | 目的 | 既定値 | 0-1 との整合 |
| --- | --- | --- | --- |
| `require_eof` | 入力全体の消費を強制し、潜在的な設定ミスを即時検出する。 | `false` | 2.2（明確な診断）: 余剰入力を `Diagnostic` 化して早期警告。 |
| `packrat` | 線形時間を保証するためにメモ化を常時有効化するかを切り替える。 | `false` | 1.1（性能）: 大規模入力での O(n) 維持に寄与。 |
| `left_recursion` | `"off" | "on" | "auto"` の 3 段階で seed-growing 左再帰ガードを適用。 | `"auto"` | 1.1（性能）: 混入時の安全弁として必要箇所のみ左再帰処理を有効化。 |
| `trace` | `SpanTrace` を収集し、IDE/CLI で解析過程を可視化。 | `false` | 2.2（診断の透明性）: 必要時のみ情報を開示し過剰負荷を回避。 |
| `merge_warnings` | 回復警告 (`recover`) の連続発生を集約。 | `true` | 2.2: ノイズを抑えつつ要点を共有。 |
| `legacy_result` | 旧 API (`Result<(T, Span), ParseError>`) を返す互換スイッチ。 | `false` | 3.2（エコシステム連携）: 移行期間の後方互換を確保。 |
| `locale` | 診断表示のロケールを明示指定。 | `None` | 2.2: 翻訳済み診断を利用しつつ既定を英語にフォールバック。 |

コメント・互換設定・ストリーミングなど追加の挙動は `extensions` を通して opt-in で提供し、コアの性能特性（0-1 §1.1）と単純さ（0-1 §2.1）を損なわないようにする。

### D-3. 標準拡張ネームスペース {#runconfig-extension-namespaces}

以下のエントリは Reml 仕様で予約されており、IDE・CLI・プラグインが相互運用する際の契約を提供する。

| key | 代表キー | 役割 | 参照 |
| --- | --- | --- | --- |
| `"lex"` | `space_id: ParserId`, `profile: ConfigTriviaProfile` | `Core.Parse.Lex` による空白・コメント処理を共有し、手書きのスキップ処理を排除する。 | 2-3 §L-4, 3-7 §1.5 |
| `"config"` | `compat: ConfigCompatibility`, `trivia: ConfigTriviaProfile` | 設定ファイル互換モード（コメント許容・トレーリングカンマ等）と診断メタを一元化。 | 3-7 §1.5, 3-6 §2.4 |
| `"recover"` | `mode: "off"|"collect"`, `sync_tokens: Set<TokenClass>`, `max_diagnostics: Int`, `max_resync_bytes: Int`, `max_recoveries: Int`, `notes: Bool` | `recover` の有効/無効（Build/CI と IDE/LSP の切替）、同期トークン集合、上限制御を共有し、CLI/LSP/ストリーミングが同じ復旧戦略を再現できるようにする。 | 2-5 §E, 2-6 §B-2-2, 3-6 §2.2 |
| `"stream"` | `checkpoint: Option<Span>`, `resume_hint: DemandHint` | ストリーミング実行（`run_stream`）で保持した継続情報をバッチランナーへ橋渡しする。 | 2-6 §B-2-3, 2-7 |
| `"lsp"` | `syntax_highlight: Bool`, `semantic_tokens: Bool` | LSP 拡張でのトークン生成・トレース同期。 | guides/lsp-integration.md |

`lex` と `config` を組み合わせることで、コメント・空白・互換モードの情報を `RunConfig` に集約し、サンプル群が独自のスキップ処理を持たずに済む。これにより、0-1 §1.1 が求める性能と 0-1 §2.2 の診断一貫性を同時に満たす。

---

## E. コミットと消費の意味論


* `consumed`：**入力を1バイト以上前進**したか。
* `committed`：`cut` 境界を**越えた**とマーク（消費の有無に関わらず）。ゼロ幅の `cut_here()` でも `committed=true` になり、`or` の右枝を試さない。

**合成の基本規則（抜粋）**

* `p.or(q)`：

  * `p` が `Err(consumed=true, _ )` または `Err(_, committed=true)` → **q を試さない**。
  * `p` が `Err(consumed=false, committed=false)` → **q を試す**。
* `p.then(q)`：

  * `p` が `Ok(consumed=*)` → `q` へ続行（`consumed` は合成：`p||q`）
* `p` が `Err` → そのまま `Err`。
* `cut`：以降で失敗したら **`committed=true`** を返す（期待集合は 2.5 B-5 を参照し、cut 境界で親の期待を破棄して再構築する）。
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
fn run<T>(p: Parser<T>, src: String, cfg: RunConfig = {}) -> ParseResult<T> = todo
// AST と診断を常に返す。cfg.require_eof=true なら余剰入力は Diagnostic として報告。

fn run_partial<T>(p: Parser<T>, src: String, cfg: RunConfig = {}) -> ParseResultWithRest<T> = todo
// 部分パース：残り Input を `rest` に格納し、result.diagnostics も一緒に返す。

type ParseResultWithRest<T> = {
  result: ParseResult<T>,
  rest: Option<Input>
}
```

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

Reml コアの `Core.Parse` はプラグイン登録 API を持ちません。DSL 拡張や capability 管理が必要な場合は、オプションモジュール `Core.Parse.Plugin`（[5-7](5-7-core-parse-plugin.md) 参照）を読み込み、そこで提供される `register_plugin`/`register_capability` 等を介して機能を注入してください。運用指針やテンプレートは `../guides/dsl/DSL-plugin.md` を参照にします。これにより、コア API は小さく安定したまま、プロジェクト固有の拡張点を opt-in で追加できます。

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
