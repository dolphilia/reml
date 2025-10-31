# Core Parser Migration メモ

## Phase 2-5 RunConfig 移行サマリ（2025-11-24）
- Step1: `parser_run_config.{ml,mli}` を導入し、仕様と同じフィールド／拡張 API を実装（`compiler/ocaml/src/parser_run_config.ml`）。
- Step2: `parser_driver` と `Parser_diag_state` を `RunConfig` 受け取りへ移行し、CLI・テストが新 API を利用する準備を完了。
- Step3: CLI (`compiler/ocaml/src/main.ml`) とユニットテストで共通ビルダーを使用、監査メタデータ出力を `parser.runconfig.*` 系に統一。
- Step4: LSP の `run_config_loader` を整備し、`extensions["lex"|"recover"|"stream"]` を `run_stream` へ伝播。設定ファイル（`tooling/lsp/config/default.json`）を共有。
- Step5: `run_config_tests.ml` と `parser-runconfig-packrat.json.golden` を追加し、`collect-iterator-audit-metrics.py` で `parser.runconfig_switch_coverage` / `parser.runconfig_extension_pass_rate` を集計。
- Step6: 仕様脚注（`docs/spec/2-1-parser-type.md`、`docs/spec/2-6-execution-strategy.md`）とガイド（`docs/guides/core-parse-streaming.md`）へ RunConfig 共有手順を反映し、レビュー記録（`docs/plans/bootstrap-roadmap/2-5-review-log.md`）へ Day6 エントリを追加。

## Phase 2-5 Core コンビネーター棚卸し（2025-11-01）
- Step1: Menhir 規則と仕様コアコンビネーターの対応を整理し、欠落メタデータを洗い出した。

### コアコンビネーター現況
- **`ok`**  
  - 仕様: 非消費で成功し値を返す（`docs/spec/2-2-core-combinator.md:15`）。  
  - Menhir現状: 空産出を使って `None` や `[]` を返す規則が多用される（例: `module_header_opt`、`use_decl_list`、`arg_list_opt`。`compiler/ocaml/src/parser.mly:727`, `compiler/ocaml/src/parser.mly:735`, `compiler/ocaml/src/parser.mly:1255`）。  
  - ギャップ: 成功時に `ParserId` やトレースを残す仕組みがなく、`ok` の適用箇所を走行時に識別できない。
- **`fail`**  
  - 仕様: 非消費で即時失敗し、期待集合は空（`docs/spec/2-2-core-combinator.md:17`）。  
  - Menhir現状: `Parser.MenhirInterpreter` の `HandlingError`／`Rejected` 分岐で一律に診断を生成しており、任意メッセージ付きの失敗を仕込む API が存在しない（`compiler/ocaml/src/parser_driver.ml:206-237`）。  
  - ギャップ: `fail` 相当を呼び出しても診断ドメインや期待集合を細かく制御できず、仕様の `fail(msg)` をそのまま表現できない。
- **`eof`**  
  - 仕様: 入力末尾のみ成功するゼロ幅パーサ（`docs/spec/2-2-core-combinator.md:18`）。  
  - Menhir現状: `compilation_unit` 規則で `EOF` トークンを要求し（`compiler/ocaml/src/parser.mly:719-724`）、解析後に `RunConfig.require_eof` を別途チェックしている（`compiler/ocaml/src/parser_driver.ml:241-258`）。  
  - ギャップ: `eof` の判定とエラーメッセージ生成が分散しており、コンビネーター層に集約するためのフックがない。
- **`rule`**  
  - 仕様: 規則に名前と `ParserId` を付与して Packrat と診断で利用（`docs/spec/2-2-core-combinator.md:19`）。  
  - Menhir現状: 非終端名は存在するが安定 ID として公開されず、`record_span_trace` も `compilation_unit` 固定のラベルしか持たない（`compiler/ocaml/src/parser.mly:1174`、`compiler/ocaml/src/parser_driver.ml:219-223`）。  
  - ギャップ: `rule` が要求する ID 付与・トレース・Packrat キー生成のいずれも未実装。
- **`label`**  
  - 仕様: 失敗時に期待名を差し替える（`docs/spec/2-2-core-combinator.md:20`）。  
  - Menhir現状: 期待集合はトークン／非終端名から自動生成され、`label` に相当する上書きメカニズムがない（`compiler/ocaml/src/parser_expectation.ml:139-175`）。  
  - ギャップ: ヒューマンリーダブルな期待名を与える手段がなく、仕様の `label`/`expect` 系 API を再現できない。
- **`then`**  
  - 仕様: 2 つのパーサを直列結合しタプルで返す（`docs/spec/2-2-core-combinator.md:29`）。  
  - Menhir現状: 逐次的な規則展開で同様の効果を得ている（例: `compilation_unit`。`compiler/ocaml/src/parser.mly:719-724`）。  
  - ギャップ: 直列結合の結果を明示的な `Reply` に変換する層がなく、`then` が要求する `consumed/committed` の規約を露出できていない。
- **`andThen`**  
  - 仕様: 成功結果に基づき次のパーサを決めるモナディック結合（`docs/spec/2-2-core-combinator.md:30`）。  
  - Menhir現状: 後続規則は静的に決まり、動的にパーサを差し替える手段がない。必要に応じて新しい非終端を作るしかなく、`andThen` の抽象性を欠いている（`compiler/ocaml/src/parser.mly:1174-1191`）。  
  - ギャップ: 実行時にパーサを合成する API が無いため、`andThen` 導入にはシム層でのラッパー実装が必須。
- **`skipL`**  
  - 仕様: 左側の結果を捨て右側を返す（`docs/spec/2-2-core-combinator.md:31`）。  
  - Menhir現状: アクションで先行トークンを無視する形で実現（例: `LPAREN; e = expr; RPAREN { e }`。`compiler/ocaml/src/parser.mly:1204-1211`）。  
  - ギャップ: 捨てたトークンを診断やトレースに残す仕組みが無いため、`skipL` を導入してもメタデータが欠落する。
- **`skipR`**  
  - 仕様: 右側の結果を捨て左側を返す（`docs/spec/2-2-core-combinator.md:32`）。  
  - Menhir現状: 右項を無視する場合は自前で値を返しているが、どこでも同じパターンを再利用できる抽象がない（例: `module_header_opt` で `MODULE` を破棄し `Some` を返す。`compiler/ocaml/src/parser.mly:727-732`）。  
  - ギャップ: `skipR` の呼び出し位置を追跡するメタデータや共通実装が存在せず、仕様の糖衣と差がある。
- **`or`**  
  - 仕様: 左優先の選択で消費済みなら分岐打ち切り（`docs/spec/2-2-core-combinator.md:33-35`）。  
  - Menhir現状: 代替規則として記述され、LR 解析の特性上自動的に左優先になる（`compiler/ocaml/src/parser.mly:1174-1191`）。  
  - ギャップ: `consumed`/`committed` フラグを露出する API が無いため、`or` の失敗統合ルールをコンビネーター層で評価できない。
- **`choice`**  
  - 仕様: `or` をリストに拡張した総合集合（`docs/spec/2-2-core-combinator.md:33-35`）。  
  - Menhir現状: 多分岐規則で近似されるが、分岐集合をデータとして扱う箇所は存在しない（`compiler/ocaml/src/parser.mly:1266-1320`）。  
  - ギャップ: 分岐ごとのメタデータや `choice` のソース識別を保持する仕組みがなく、Packrat/診断に必要な識別子を提供できない。
- **`map`**  
  - 仕様: 解析結果を純粋関数で変換（`docs/spec/2-2-core-combinator.md:45`）。  
  - Menhir現状: 各規則で `make_expr` などのアクションを実行しており実質的に `map` と同等（`compiler/ocaml/src/parser.mly:1194-1231`）。  
  - ギャップ: `map` が要求する純粋性・効果タグ検証が実装されておらず、`map` 呼出しを特定できない。
- **`cut`**  
  - 仕様: 以降の失敗を `committed=true` に変換（`docs/spec/2-2-core-combinator.md:46`）。  
  - Menhir現状: `parser_driver` で `committed` フラグを管理しているが、どこでも `true` に更新されていない（`compiler/ocaml/src/parser_driver.ml:185-223`）。  
  - ギャップ: `cut` を差し込むフックが皆無で、`ParserId` と組み合わせた診断強化ができない。
- **`attempt`**  
  - 仕様: 失敗時に消費を巻き戻し空失敗化（`docs/spec/2-2-core-combinator.md:48`）。  
  - Menhir現状: LR 解析のためバックトラックが無く、`attempt` 相当の挙動を制御する手段が提供されていない。  
  - ギャップ: `attempt` を実装するにはシム層でメモ化と `consumed` フラグの書き換えを導入する必要がある。
- **`recover`**  
  - 仕様: 指定トークンまで読み飛ばし診断を残して継続（`docs/spec/2-2-core-combinator.md:49-50`）。  
  - Menhir現状: `Parser_diag_state` に `recover_config` と `record_recovery` が用意されているが、呼び出し元が存在しない（`compiler/ocaml/src/parser_diag_state.ml:24-63`、`compiler/ocaml/src/parser_driver.ml:187-205`）。  
  - ギャップ: 同期トークンの指定や診断拡張を行うエントリポイントが欠如しており、RunConfig の `recover` 拡張も実装未着手。

### 欠落メタデータとリスク
- `ParserId`／`rule` 名称が生成されないため Packrat・トレース・監査が不可。  
- `committed` フラグが常に `false` のため、`cut` や `attempt` の契約を検証できない。  
- `recover` シナリオを記録する仕組みが未着手で、`parser.recover` Capability を名乗れない。  
- `label` や `choice` に紐付く期待名が静的文字列のみで、仕様要求の人間可読メッセージを提供できていない。


## TODO: RunConfig フラグの未実装点
- [ ] `--packrat` / `RunConfig.packrat` は警告のみで実装待ち。`PARSER-003` のメモ化シム導入後に CLI/LSP 両方で挙動を検証する。
- [ ] `--left-recursion=<mode>` の `on/auto` はシムが未着手のため警告を伴う。左再帰テーブル構築手順を `docs/plans/bootstrap-roadmap/2-5-proposals/PARSER-003-proposal.md` と同期する。
- [ ] LSP 側の設定ファイル読み込み処理はドラフト段階。`tooling/lsp/config/default.json` を更新した場合は CLI 側の `extensions["config"]` と突合し、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` にフォローアップを残す。

## 参考リンク
- `docs/plans/bootstrap-roadmap/2-5-proposals/PARSER-002-proposal.md`
- `docs/plans/bootstrap-roadmap/2-5-proposals/LEXER-002-proposal.md`
- `docs/plans/bootstrap-roadmap/2-5-review-log.md`
