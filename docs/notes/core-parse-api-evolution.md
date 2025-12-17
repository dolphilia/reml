# Core Parse API Evolution メモ

## 2025-12-17: `commit` のレイヤ決定（Cut/Commit の表面整理）

### 結論
- `cut` / `cut_here` は **コア（最小公理系）**に残す。
- `commit(p)` は **派生（derived）の関数糖衣**として `Core.Parse` に提供し、意味論は `cut(p)` と同一とする。
- メソッド糖衣は `p.cut()` を正とし、`p.commit()` は追加しない。

### 判断理由
- **最小公理系を太らせない**: `commit` は意味論上 `cut` と同一であり、コアへ追加すると「概念の追加ではなく別名の追加」になりやすい（2.2 の方針と衝突）。
- **読みやすさ（意図の強調）**: `Parse.or(left, Parse.commit(right))` の形は「右枝はバックトラック境界として確定する」という意図が読み取りやすい。Spec.Core の基準ケース `examples/spec_core/chapter2/parser_core/core-parse-or-commit-ok.reml` と整合する。
- **用語の揺れを抑える**: `cut` / `cut_here` / `commit` の同時提供は必須だが、メソッド糖衣まで増やすと `p.cut()` と `p.commit()` が併存し、スタイルが分裂しやすい。既存の `.cut()`（2.1 ミニ例）を正として固定する。
- **仕様の一貫性**: `commit` は「別概念」ではなく「別名（糖衣）」として扱い、`Reply{consumed, committed}` と `or` の分岐規則（2.1/2.2/2.5）に影響を与えない。

### 反映先
- `docs/spec/2-2-core-combinator.md`（派生API一覧に `commit` を追記、実務指針で `cut`/`commit` の関係を明記）
- `docs/plans/core-parse-improvement/1-0-cut-commit-plan.md`（WS1 Step2 の決定として明文化）

## Phase 2-5 Step2 Core_parse シグネチャ草案（2025-12-04）

### 背景
- `PARSER-003` Step1 で抽出した Menhir 規則とコアコンビネーター対応表を基に、仕様どおりの `Core_parse` モジュール構成を決める必要があった[^parser003-step1].  
- 仕様 2.1/2.2/2.6 の契約では、`Parser<T>` が `State` を介して `Reply` を返し、`rule` により安定した `ParserId` を付与することが前提となる[^spec-parser-type][^spec-core-comb].  
- Phase 2-5 の RunConfig 導入（`PARSER-002` Step6）で `parser_driver` と `Core_parse_lex` のフックが整ったため、コンビネーター層から `RunConfig.extensions`・診断状態へアクセスする方法も合わせて定義する。

### 公開シグネチャ案

```ocaml
module Core_parse : sig
  module Id : sig
    type t
    val namespace : t -> string
    val name : t -> string
    val ordinal : t -> int
    val fingerprint : t -> int64
    val origin : t -> [ `Static | `Dynamic ]
    val of_static : namespace:string -> name:string -> t
    val of_dynamic : namespace:string -> name:string -> t
  end

  module State : sig
    type t
    val input : t -> Core_input.t
    val config : t -> Parser_run_config.t
    val diag : t -> Parser_diag_state.t
    val menhir_checkpoint : t -> Parser.MenhirInterpreter.checkpoint
    val with_input : t -> Core_input.t -> t
    val with_diag : t -> Parser_diag_state.t -> t
    val with_checkpoint :
      t -> Parser.MenhirInterpreter.checkpoint -> t
  end

  module Reply : sig
    type 'a t =
      | Ok of {
          id : Id.t;
          value : 'a;
          span : Diagnostic.span;
          consumed : bool;
        }
      | Err of {
          id : Id.t option;
          error : Diagnostic.parse_error;
          consumed : bool;
          committed : bool;
        }

    val map : ('a -> 'b) -> 'a t -> 'b t
  end

  type 'a parser = State.t -> Reply.'a t * State.t

  val rule :
    ?doc:string ->
    namespace:string ->
    name:string ->
    'a parser ->
    'a parser

  val label :
    printable:string ->
    'a parser ->
    'a parser

  val cut : 'a parser -> 'a parser
  val cut_here : unit parser
  val attempt : 'a parser -> 'a parser

  val recover :
    id:Id.t ->
    until:(unit parser) ->
    with_:(State.t -> Reply.'a t * State.t) ->
    'a parser ->
    'a parser

  module Builder : sig
    val return : 'a -> 'a parser
    val bind : 'a parser -> ('a -> 'b parser) -> 'b parser
    val map : 'a parser -> f:('a -> 'b) -> 'b parser
  end

  module Registry : sig
    type entry = {
      namespace : string;
      name : string;
      ordinal : int;
      fingerprint : int64;
      origin : [ `Static | `Dynamic ];
    }

    val static : entry list
    val lookup : namespace:string -> name:string -> entry option
    val ensure : namespace:string -> name:string -> Id.t
    val register_dynamic : namespace:string -> name:string -> entry
  end
end
```

- `State` は `Input`・`RunConfig`・診断状態・Menhir チェックポイントを保持し、Step3 のブリッジ層で包む想定。  
- `Reply` は仕様どおり `consumed` / `committed` を持ち、`rule`/`recover` が割り当てる `id` を保持する。  
- `recover` は同期トークンを `until` から受け取り、補完値生成を `with_` に委譲するカリー化されたシグネチャに統一する。  
- `Registry.ensure` は `rule` の内部で利用し、`namespace`（Menhir 非終端名）＋`name`（論理名）に対して `Id.t` を返す。静的登録済みの中央値は `static` に保持し、未登録の場合は `register_dynamic` で後段の Packrat/監査向けに記録する。

### ParserId 割当戦略
- 静的領域は `core_parse_id_registry.ml`（自動生成ファイル）に保持し、Step1 で整理した 15 コアコンビネーター＋主要非終端を `ordinal = 0-4095` に割り当てる。生成スクリプトでは `namespace:name` を `Digestif.xxhash64`（既存依存あり）でハッシュ化し `fingerprint` に保存、ビルド時に重複を検査する。  
- 動的領域は `ordinal >= 0x1000` を開始点とし、`Registry.register_dynamic` が `Hashtbl` で重複検査しつつ採番する。プラグインやテスト専用の `rule` はこの経路を利用し、`origin = \`Dynamic` を返す。  
- `Id.of_static` は静的表への存在確認を要求し、見つからない場合は `Invalid_argument` を送出してビルド時に気付けるようにする。`Id.of_dynamic` は `Registry.register_dynamic` を呼び出し、後続の監査ログに `origin` を残すことで Phase 2-7 の追跡に備える。  
- Packrat メモ化キーは `(Id.fingerprint, byte_off)` を 128bit ペアとして保持し、RunConfig で Packrat が無効な場合でも `fingerprint` を診断トレースに埋め込むことで再現性を確保する。

### State / RunConfig / 診断連携
- `State.config` で `parser_driver` が受け取った `RunConfig` を閲覧し、`rule` / `recover` / `trace` コンビネーターから `extensions["lex"]`・`["recover"]`・`["stream"]` を参照できるフックにする。  
- `State.with_input` / `with_checkpoint` により Menhir チェックポイントと Core.Parse `Input` を同期させ、`attempt` が必要な巻き戻しで `consumed=false` を再現する。  
- `Reply.Err` が `committed=true` の場合は `Parser_diag_state` の `record_committed` を呼び出す設計とし、`cut`/`cut_here` は状態に `committed=true` のフラグを書き込む薄いユーティリティになる。  
- `recover` は `RunConfig.Recover` シム（`PARSER-002`）を参照して同期トークン一覧をロードし、成功時には `Parser_diag_state.record_recovery` を呼び出すためのコールバックを同梱する。

### TODO
- Step3 で `parser_driver`／Menhir ブリッジを `State` ラッパーに切り替え、`rule`／`label`／`cut` 呼び出し箇所に `Registry.ensure` を挿入する。  
- Packrat PoC で `(Id.fingerprint, byte_off)` テーブルのプロトタイプを実装し、`RunConfig.packrat=true` 時の性能測定を `0-3-audit-and-metrics.md` に追加する。  
- `core_parse_id_registry.ml` の自動生成スクリプトと CI チェックを整備し、静的 ID と Step1 マトリクスの乖離を検知できるようにする。

## Phase 2-5 Step4 Packrat・回復・Capability 統合設計（2025-12-12）

### Packrat キャッシュ方針
- キャッシュキーは `Cache_key = { id : Id.t; offset : int }` とし、`offset` は入力バイト位置で管理する。`Id.fingerprint` を 64bit 値として保持し、キー比較を高速化する。  
- `State` に `packrat : Packrat_cache.t option ref` を追加し、`RunConfig.packrat` が有効な場合のみ `Some cache` を挿入する。PoC では `None` を維持し、Step5 で `Packrat_cache` の実装（`find` / `store` / `invalidate`）を導入する。  
- `module Packrat_cache` は以下の最小構成で導入する想定:
  ```ocaml
  module Packrat_cache : sig
    type 'a entry = {
      reply : 'a Reply.t;
      state_snapshot : State.snapshot;
    }

    type t

    val create : unit -> t
    val find :
      t -> key:Cache_key.t -> 'a entry option
    val store :
      t -> key:Cache_key.t -> entry:'a entry -> unit
    val invalidate_namespace :
      t -> namespace:string -> unit
  end
  ```
  `State.snapshot` は入力カーソルと診断スナップショット（`Parser_diag_state.farthest_snapshot`）を含む予定。  
- Packrat 有効時は `Core_parse.rule` で `Cache_key` を組み立て、`Reply.Ok` の場合は消費バイト数を次回ヒット時に検証する。`RunConfig.packrat=false` の場合は既存ロジックをそのまま利用する。

### 回復・同期トークンの取り扱い
- `RunConfig.Recover.of_run_config`（compiler/ocaml/src/parser_run_config.ml:240）で取得した `sync_tokens` を `State.recover_config` へ保持する。  
- `Core_parse.recover` は `sync_tokens` を `parser_expectation.collect` にも提供し、同期トークンが適用された場合は `Parser_diag_state.record_recovery` を呼び出して監査ログに `recover.sync_token` を記録する。  
- `RunConfig.Recover.emit_notes` が有効なときは `Diagnostic.extensions["recover.notes"] = true` を出力するフローを追加し、CLI/LSP 表示で同期トークンを提示できるようにする（実装は Phase 2-7 へ TODO）。

### 複数 Capability の維持
- `RunConfig.Effects.required_capabilities`（compiler/ocaml/src/parser_run_config.ml:320）と `Diagnostic` の監査メタデータ（compiler/ocaml/src/diagnostic.ml:846-896）を突合し、Packrat 経路でも `effect.capabilities[*]` と `effect.stage.*` を失わないよう `Reply.Err` に `effect_metadata` フィールドを追加する案を整理。  
- キャッシュヒット時は `Reply` に含めた `effect_metadata` をそのまま返却し、再評価を避ける。ただし Stage が外部要因で変化した場合を検知するため、`Cache_key` に `effects_digest` を含める検討を TODO として残した。

### フォローアップ
- `Packrat_cache` 実装とメトリクス（`parser.packrat_cache_hit_ratio` / `parser.packrat_entry_count`）追加を Step5 で扱う。  
- `recover` 同期トークンのテストケースを LSP フィクスチャへ追加し、CLI ゴールデンにも `recover.notes` 出力を反映させる。  
- Stage/Capability 情報が欠落した診断を検出する CI ルールを Phase 2-7 で導入する（`collect-iterator-audit-metrics.py --require-success` に新規チェックを追加）。

## Phase 2-5 Step6 ドキュメント同期と引き継ぎ（2025-12-24）

- `docs/spec/2-2-core-combinator.md` に `Core_parse` 進捗脚注を追加し、OCaml 実装が公開した `rule`/`label`/`cut`/Packrat 指標が仕様に反映されたことを明文化した。`docs/guides/plugin-authoring.md` と `docs/guides/core-parse-streaming.md` にはコンビネーター利用例と RunConfig 共有手順を追記し、CLI/LSP/ストリーミングで同じ設定を再現できるガイドを整備。  
- リポジトリ索引として `README.md` と `docs/plans/bootstrap-roadmap/2-5-proposals/README.md` を更新し、PARSER-003 の進捗が一目で把握できるようリンクとタイムスタンプを追加。`docs/plans/bootstrap-roadmap/2-5-review-log.md` 2025-12-24 エントリに作業ログを記録。  
- テレメトリ統合と Menhir 置換方針は未決定のため、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に TODO を移送。Packrat 指標と `parser.core.rule.*` メタデータを活用した監査強化を Phase 2-7 で評価する。

## TODO: Rust Lex/Streaming/Plugin 連携 {#todo-rust-lex-streaming-plugin}

- Rust ランタイムには `Parser<T>` / Packrat / 期待集合生成が導入されたが、`RunConfig.extensions["lex"]` プロファイルの共有やコメント/トリビア消費、`Core.Parse.Streaming` / `Core.Parse.Plugin` との橋渡しは未実装。4.1 Core.Parse 計画 Phase 6 のフォローアップとして、Lex ブリッジと Plugin Capability 連携の設計案を立案する。  
- `docs/guides/core-parse-streaming.md` と `docs/guides/plugin-authoring.md` に暫定の非対応メモを追加済み。実装着手後は同ガイドと `docs/spec/2-0-parser-api-overview.md` / `2-2-core-combinator.md` の脚注を更新し、RunConfig 共有キーや診断メタデータの差分を明文化する。  
- Streaming Runner の PoC を Rust で復元する際は、Packrat キャッシュ共有と `Recover` 同期トークンが欠落しないかを micro テストで検証し、欠落箇所を `docs/plans/bootstrap-roadmap/4-1-core-parse-combinator-plan.md` へ逆流させる。  
- Phase 12 で追加した回帰シナリオ（`CH2-PARSE-901` autoWhitespace/Layout、`CH2-PARSE-902` ParserProfile JSON）向けに Rust 側サンプル・expected を作成する。`RunConfig.extensions["lex"].layout_profile` 未設定時のフォールバックと `profile_output` 書き出し失敗が診断に影響しないことを CLI/LSP/Streaming で確認する。

## Phase 4-1 Phase11 Plugin/Streaming/OpBuilder 連携メモ

- Plugin 経由で Core.Parse を呼び出す際の安全策として、`RuntimeBridgeAuditSpec`（`docs/spec/3-8-core-runtime-capability.md`）で要求される `bridge.stage.*` / `effect.capabilities[*]` を `register_parser` 経路に転写するチェックリストを `docs/guides/plugin-authoring.md` に追加。Stage 不一致や署名乖離は `@dsl_export` → Manifest → CLI 検証（`reml plugin verify` 想定）で検出し、プラグイン固有の RunConfig を同時に共有する方針を明文化した。
- Streaming との接続では、`ParserId`・Packrat・期待集合を共有したまま `Parser<T>` を `StreamDriver` へ渡す変換手順（lex/layout/autoWhitespace/優先度テーブルの共有を必須とする）を `docs/guides/core-parse-streaming.md#94-parser-から-streamingparser-への変換指針（phase-11）` に追記。Rust 実装欠落分は `#todo-rust-lex-streaming-plugin` へ残し、実装開始時のギャップ一覧として利用する。
- OpBuilder と autoWhitespace/演算子優先度ビルダーの上書き経路を RunConfig (`extensions["parse"].operator_table`) で統一する方針を記録。CLI/OpBuilder DSL からの上書きがプラグイン側設定と衝突した場合に備え、`phase4-scenario-matrix.csv` で該当シナリオを追加し回帰監視する案を Phase 12 へ引き継ぐ。

---

[^parser003-step1]: `docs/plans/bootstrap-roadmap/2-5-proposals/PARSER-003-proposal.md` Step1 実施記録。Menhir 規則と 15 コアコンビネーターの対応表・欠落メタデータを整理。
[^spec-parser-type]: `docs/spec/2-1-parser-type.md` §A〜§D。`Parser<T>` の意味論と `Reply` の 4 状態を定義。
[^spec-core-comb]: `docs/spec/2-2-core-combinator.md` §A〜§C。`rule`/`label`/`cut`/`recover` の契約と Packrat/診断要件を記述。
