# PARSER-002 RunConfig 導入ロードマップ計画

## 1. 背景と症状
- Phase 2-5 の修正計画では「RunConfig/lex シムと複数 Capability を整備し、値制限復元を可能にする」ことが Critical/High 項目として掲げられており（docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md:133-149）、本計画で RunConfig 基盤を整備することが前提になっている。  
- 仕様は `RunConfig` に `require_eof` / `packrat` / `left_recursion` / `trace` / `merge_warnings` / `locale` と `extensions` を定義し（docs/spec/2-1-parser-type.md:90-188、docs/spec/2-6-execution-strategy.md:35-110）、`extensions["lex"]` / `["recover"]` / `["stream"]` などを通じて CLI・LSP・DSL が同じ設定で動作することを求めている。  
- 現行の `parser_driver` は `require_eof` と `legacy_result` だけを保持するローカルレコードに依存しており（compiler/ocaml/src/parser_driver.ml:6-15）、Packrat/左再帰/trace/extension の値がランナーへ到達しない。  
- `PARSER-001` で `ParseResult` シムと診断集約は導入済みだが、RunConfig を渡せないため `LEXER-002` の lex シム、`EFFECT-003` の複数 Capability 解析、`TYPE-001` の値制限復元が一貫した設定で検証できない。  
- CLI/LSP/テストで RunConfig を生成する仕組みが無く、`0-3-audit-and-metrics.md` に計測するべき RunConfig 系メトリクスも未登録のままである。これにより Phase 2-5 以降の差分監視や Phase 3 の Self-host 移行で必要なエビデンスが欠落している。

## 2. Before / After
### Before
- `parser_driver.run` は `run_config = { require_eof; legacy_result }` だけを受け取り、その他のスイッチを無視する。  
- `RunConfig.extensions` を参照する層が存在せず、`extensions["lex"]` や `["recover"]` を設定しても字句シム・回復戦略・ストリーミング設定へ共有できない。  
- CLI/LSP/テストが RunConfig を構築するヘルパを持たず、設定値を記録・可視化するメトリクスも存在しない。

### After
- `Run_config`（仮称）モジュールを新設し、仕様と同じフィールドを持つ不変レコードと `with_extension` ヘルパを提供する。  
- `parser_driver.run` / `run_partial` / `parse` を `Run_config.t` 受け取りに切り替え、`require_eof` / `packrat` / `left_recursion` / `trace` / `merge_warnings` / `locale` / `legacy_result` を `DiagState` と Menhir ドライバへ橋渡しする。  
- `extensions["lex"]`・`["recover"]`・`["stream"]`・`["config"]` の値を `LEXER-002`、`ERR-002`、`EXEC-001` が再利用できるようにするため、`parser_driver` と新設シムでイミュータブルな共有構造を用意する。  
- CLI/LSP/テストに RunConfig ビルダを追加し、設定を JSON・監査ログ・メトリクスに記録できるようにする。  
- `0-3-audit-and-metrics.md` に RunConfig 系メトリクス（例: `parser.runconfig_switch_coverage`, `parser.extensions_lex_profile_pass_rate`, `parser.extensions_stream_handshake`）を登録して CI 監視へ組み込む。

## 3. 影響範囲と検証
- **コード**: `compiler/ocaml/src/parser_driver.ml`, `parser_diag_state.ml`, `parser_expectation.ml`、新設 `compiler/ocaml/src/parser_run_config.{ml,mli}`（予定）、`core_parse_lex`（LEXER-002）など。  
- **ツール/クライアント**: `compiler/ocaml/src/main.ml`（CLI）、`tooling/lsp/` 系のパーサ呼び出し、`tooling/ci/collect-iterator-audit-metrics.py`、`scripts/validate-diagnostic-json.sh`。  
- **ドキュメント**: `docs/spec/2-1-parser-type.md`・`docs/spec/2-6-execution-strategy.md` に移行脚注を追加/更新し、`docs/plans/bootstrap-roadmap/2-5-review-log.md`・`docs/notes/core-parser-migration.md`（未作成なら新規）へ進捗を記録する。Phase 2-8 で作成した `reports/spec-audit/diffs/SYNTAX-003-ch1-rust-gap.md` を参照し、effect handler/operation の受理状況と RunConfig 共有の関係を脚注で追跡する。[^syntax003-gap]  
- **テスト/メトリクス**: `compiler/ocaml/tests/run_config_tests.ml`（新設）、既存パーサー/診断テストの RunConfig 差し替え、`0-3-audit-and-metrics.md` への RunConfig 指標登録、`reports/diagnostic-format-regression.md` に RunConfig 切替シナリオを追加。  
- **検証手段**: `dune runtest parser`、`scripts/validate-diagnostic-json.sh`、`tooling/ci/collect-iterator-audit-metrics.py --track parser.runconfig_*`、CLI/LSP 経路の手動確認と JSON フィクスチャ比較。

## 4. フォローアップ
- Packrat/左再帰・コンビネータ抽出は `PARSER-003` に委譲しつつ、RunConfig スイッチを `PARSER-003` が利用できるよう先行で注入する。  
- RunConfig シム導入後は CLI フラグ・LSP 設定ファイル・ガイド類（`docs/guides/core-parse-streaming.md`, `docs/guides/plugin-authoring.md`）を更新し、ユーザーが仕様準拠の設定を選択できるようにする。  
- `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` と連携して RunConfig/lex/stream 設定のダッシュボード化と監査指標整備を Phase 2-7 へ接続する。  
- `TYPE-001` 値制限・`EFFECT-003` 複数 Capability 実装が RunConfig 経由で設定を取得できるかレビューし、必要なデータ共有（例: `extensions["effects"]` / `["runtime"]`）を追加する。  
- RunConfig 導入状況を `docs/plans/bootstrap-roadmap/2-5-review-log.md` に段階記録し、Phase 3 での Reml 実装移植作業に備えて `docs/notes/core-parser-migration.md` へまとめる。

## 5. 実施ステップ（Week32〜Week33）
### Step 0: 仕様・実装調査（Week32 Day1）
- `docs/spec/2-1-parser-type.md`・`docs/spec/2-6-execution-strategy.md`・`docs/spec/2-3-lexer.md` を読み込み、RunConfig フィールドと既定値、`extensions` の標準ネームスペースを整理したマッピング表を作成する。  
- 現行 `parser_driver`・CLI (`compiler/ocaml/src/main.ml`)・LSP 呼び出しの RunConfig/parse API を棚卸しし、既存テストやスクリプトが `parse` を直接呼び出している箇所を洗い出す。  
- Menhir の `Parser.MenhirInterpreter` が提供するチェックポイント API で Packrat/左再帰シムを挿入する際の制約を調査し、必要なフック（memo table・seed-growing）を `PARSER-003` と共有できるようメモする。  
- 調査結果と課題を `docs/plans/bootstrap-roadmap/2-5-review-log.md` に `PARSER-002` エントリとして追記し、Phase 2-7・Phase 3 へ渡す TODO をマークする。

#### 調査サマリ（2025-11-18）
- RunConfig の仕様項目・既定値と OCaml 実装の差分を整理し、表1に記載した。仕様は `docs/spec/2-1-parser-type.md:92-175`・`docs/spec/2-6-execution-strategy.md:60-107` を一次資料とした。  
- 標準拡張ネームスペース（`lex`/`config`/`recover`/`stream`/`lsp` 等）の期待値を表2にまとめ、CLI/LSP/テストで未反映であることを確認した。  
- `Parser_driver` 経由の呼び出し経路（CLI・各種テスト・補助 API）を列挙し、RunConfig 非導入によるギャップを特定した（詳細は「API/呼び出し経路棚卸し」節）。  
- Menhir チェックポイント API を再確認し、Packrat/左再帰シム投入時に追加で必要となるメモ化フックと診断伝播ポイントを整理した。

##### 表1: RunConfig 項目マッピング
| 項目 | 仕様の既定値・参照 | 現行 OCaml 実装 | 差分/課題 |
| --- | --- | --- | --- |
| `require_eof` | `false`（docs/spec/2-1-parser-type.md:96） | `default_run_config.require_eof = true`、`parse` 系は常に `true` を使用（compiler/ocaml/src/parser_driver.ml:11,205） | 仕様と既定値が逆転。`RunConfig` 導入時に互換モードを明示切替する必要あり。 |
| `packrat` | `false`（docs/spec/2-1-parser-type.md:97） | フィールド未実装、Packrat メモ化なし | `Run_config` 型新設とメモテーブル管理が未着手。`PARSER-003` へのインターフェイスを準備する必要。 |
| `left_recursion` | `"auto"`（docs/spec/2-1-parser-type.md:98、docs/spec/2-6-execution-strategy.md:62-66） | 未実装 | Packrat 有効時のみ動作させる仕様。設定値と警告の取り扱いを決める必要。 |
| `trace` | `false`（docs/spec/2-1-parser-type.md:99） | 設定項目なし。CLI 側の `opts.trace` は CLI ログ用途で、パーサーへ伝播していない（compiler/ocaml/src/main.ml:586-617） | `SpanTrace` 収集やメトリクス連携のスイッチを RunConfig 化する必要。 |
| `merge_warnings` | `true`（docs/spec/2-1-parser-type.md:100） | 常に期待まとめを行う挙動のみ実装（parser_diag_state.ml 全体） | OFF 切替時の警告個別出力が未対応。状態保持ロジックにフラグ追加が必要。 |
| `legacy_result` | `false`（docs/spec/2-1-parser-type.md:101） | `parse` と `parse_string` は互換モード専用（legacy_result=true）で公開（compiler/ocaml/src/parser_driver.ml:12,205-214） | 新 RunConfig 導入時に legacy API を包むラッパーを別扱いにする必要。 |
| `locale` | `None`（環境変数フォールバック、docs/spec/2-1-parser-type.md:102-124） | `Diagnostic.Builder` 既定値のみ。RunConfig からの伝播経路なし | CLI/LSP でのロケール指定と診断整合の配線が未着手。 |
| `extensions` | `Map<Str, Any>`、推奨キーは表2（docs/spec/2-1-parser-type.md:164-175） | 未実装。RunConfig 型自体が存在せず共有コンテナも不在 | `lex`/`config`/`recover`/`stream` 等のネームスペースを扱う抽象化が必要。 |

##### 表2: 標準 extensions ネームスペース整理
| キー | 仕様での目的・参照 | 現行実装 | フォローアップ |
| --- | --- | --- | --- |
| `\"lex\"` | 字句シム共有（docs/spec/2-1-parser-type.md:170、docs/spec/2-3-lexer.md:255-267） | フィールドなし。`Lexer` 側も共有プロファイル未連携 | `LEXER-002` が `Run_config` から取得できるようイミュータブル構造を準備。 |
| `\"config\"` | コンフィグ互換モード共有（docs/spec/2-1-parser-type.md:171、docs/spec/2-3-lexer.md:255-267） | 未実装。CLI/LSP 設定も RunConfig へ反映されない | `ConfigTriviaProfile` 等を格納する API 設計が必要。 |
| `\"recover\"` | 回復シンクトークン/notes 共有（docs/spec/2-1-parser-type.md:172、docs/spec/2-6-execution-strategy.md:62-66） | 未実装。`Parser_diag_state` は固定挙動 | `ERR-002` で利用できる構造体を RunConfig へ格納。 |
| `\"stream\"` | ストリーミング継続共有（docs/spec/2-1-parser-type.md:173、docs/spec/2-6-execution-strategy.md:68-74） | 未実装。`run_partial` もスタブのまま | `EXEC-001` 向けに checkpoint/resume 情報を出し入れできるプレースホルダを要整備。 |
| `\"lsp\"` | IDE 設定共有（docs/spec/2-1-parser-type.md:174、docs/guides/ai-integration.md 等） | LSP 実装が `RunConfig` を持たないため未使用 | LSP 側設定ローダと RunConfig 構築ヘルパの設計が必要。 |
| `\"runtime\"` / `\"effects\"` / `\"target\"` | Capability/Stage・ターゲット情報（docs/spec/2-1-parser-type.md:124、docs/spec/2-6-execution-strategy.md:76-105、docs/spec/3-8-core-runtime-capability.md:264） | `Diagnostic.extensions` に断片的な情報があるが RunConfig 未連携 | `EFFECT-003`・`TYPE-001` と合意したキー構造を RunConfig に集約する。 |

##### API/呼び出し経路棚卸し
- `parser_driver` は `type run_config = { require_eof; legacy_result }` のみ保有し、RunConfig 相当の構造が存在しない（compiler/ocaml/src/parser_driver.ml:6-13）。  
- CLI は `Parser_driver.parse` を直接呼び出し、RunConfig を組み立てるヘルパが存在しない（compiler/ocaml/src/main.ml:612）。  
- テストは `Parser_driver.parse` / `parse_string` を広範に利用しており（例: compiler/ocaml/tests/test_parser.ml:10、test_type_inference.ml:18）、RunConfig 切替時に共通ビルダーが必要。  
- `run_partial` は `require_eof=false` を上書きするだけのスタブで、`rest` も常に `None`（compiler/ocaml/src/parser_driver.ml:172-175）。ストリーミング拡張と連携していない。  
- `scripts/validate-diagnostic-json.sh` や `tooling/ci/collect-iterator-audit-metrics.py` は RunConfig 値を追跡しておらず、メトリクス登録未実施（前者は CLI 出力 JSON を検証するのみ）。

##### Menhir チェックポイントと Packrat/左再帰シム観点
- `Parser.MenhirInterpreter` は `INCREMENTAL_ENGINE` を公開しており、`I.InputNeeded`・`I.Shifting`・`I.HandlingError` 分岐を `parser_driver` の `loop` で直接処理している（compiler/ocaml/src/parser_driver.ml:133-166）。ここにメモ化フックを挿入する必要がある。  
- Packrat を実装するためには `(ParserId, byte_off)` キーで `Reply` をキャッシュする仕様（docs/spec/2-1-parser-type.md:108-111）に沿ったメモテーブルを `Run_config` 側で初期化し、`I.offer` 前後でヒット判定するフックを追加する必要がある。  
- 左再帰シムは Packrat 有効時のみ許可され、`left_recursion="auto"` の解釈とメモテーブルの「評価中」フラグをループへ組み込む必要がある（docs/spec/2-6-execution-strategy.md:62-66,171-188）。  
- `trace` と `merge_warnings` の切替は `Parser_diag_state` の `record_diagnostic` 処理と `SpanTrace` 収集位置で分岐する想定。RunConfig フィールドが無い現状では常にトレース非収集・警告集約となっており、スイッチ追加で副作用を制御する必要がある。

### Step 1: RunConfig 型設計とドキュメント同期（Week32 Day1-2）
- `compiler/ocaml/src/parser_run_config.{ml,mli}`（仮称）を新設し、仕様準拠の `type t`・`type extensions`・`with_extension`・`find_extension` 等の API を設計する。`Map.Make(String)` を利用した不変マップで実装し、`RunConfigExtensions` の値は `Run_config.value`（`Bool` / `Int` / `String` / `Parser_id` 等）として表現する。  
- `RunConfig.default` および `RunConfig.Legacy.bridge` を定義し、旧 API (`parse`/`parse_string`) へ互換を提供する。  
- `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` §6.2 に参照脚注を追加し、RunConfig 移行のステップとメトリクス登録予定を明記する準備を行う。  
- `docs/spec/2-1-parser-type.md` と `docs/spec/2-6-execution-strategy.md` に「OCaml 実装の移行ステータス」脚注を追加/更新し、今回の開発範囲（RunConfig 型と CLI/LSP 連携）が Phase 2-5 中盤であることを記載する。

> 2025-11-18 更新: Step 1 完了。`compiler/ocaml/src/parser_run_config.{ml,mli}` を追加し、`with_extension` / `find_extension` / `Legacy.bridge` など仕様準拠の API を整備した。`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` §6.3 へ進捗脚注を追記し、`docs/spec/2-1-parser-type.md` / `docs/spec/2-6-execution-strategy.md` に OCaml 実装状況を明記。後続ステップ（Step 2 以降）でドライバ適用・メトリクス登録を行う前提条件が揃った。

### Step 2: parser_driver への RunConfig 導入（Week32 Day2-3）
- `parser_driver.run` のシグネチャを `?config:Run_config.t -> Lexing.lexbuf -> parse_result` へ変更し、`DiagState` に `trace`・`merge_warnings`・`locale` の設定を渡す。  
- `packrat` と `left_recursion` は当面シム層として `PARSER-003` が実装するメモ化/seed-growing フックへ委譲する準備を行い、未実装の場合は診断または `Parser_diag_state.record_warning` で追跡できるようガードを入れる。  
- `RunConfig.require_eof` を `ParseResult.recovered` の評価に合わせて処理し、`extensions["config"]` が EOF 設定を上書きする場合の優先順位を明文化する。  
- `trace=true` の時だけ `Parser_diag_state` に `SpanTrace` を保持し、既定ではコストゼロになることをユニットテストで保証する。  
- `merge_warnings=false` の場合は回復診断をすべて出力するよう `Parser_diag_state` を調整する。  
- `locale` を `Diagnostic.Builder` 初期化と `PrettyOptions` の既定値へ伝播させる下準備を `DIAG-003` と共有する。

> 2025-11-19 更新: Step 2 完了。`parser_driver` が `Run_config.t` を直接受け取り、`trace`/`merge_warnings`/`locale` を `Parser_diag_state.create` に伝播できるよう更新した。`packrat` と `left_recursion` は未実装フラグを `Warning` 診断として記録し、`extensions["config"].require_eof` を優先して未消費入力をエラー化するガードを追加。`SpanTrace` は `trace=true` のときのみルートスパンを収集し、既定モードでは追加コストゼロであることを確認した（ユニットテストは Step 5 で拡充予定）。

### Step 3: extensions/lex/recover/stream シム構築（Week32 Day3-4）
- `RunConfig.extensions["lex"]` を取得して `LEXER-002` が導入する `Core.Parse.Lex` シムへ渡す手続きを設計し、設定が無い場合は `ConfigTriviaProfile::strict_json` を既定にする。  
- `extensions["recover"]` の `sync_tokens` / `notes` を `Parser_diag_state` が利用できるようにし、`ERR-002` の FixIt 実装と連携するための API を整備する。  
- `extensions["stream"]` / `["config"]` を `EXEC-001`（run_stream PoC）へ受け渡すための placeholder を用意し、現時点では `None` の場合に警告を発しないようガードする。  
- 拡張キーが未登録の場合は `Run_config.lookup_extension` が `None` を返すだけに留め、将来の Stage 監査と衝突しないよう `EFFECT-003` とログの粒度を調整する。

> 2025-11-20 更新: Step 3 完了。`parser_run_config` に `Config`/`Lex`/`Recover`/`Stream` サブモジュールを追加し、`extensions["lex"]` は `profile` 未指定時に `ConfigTriviaProfile::strict_json` を返すシムを提供、`ParserId` は将来の Packrat 向けに `int option` として保持するよう整備した。`extensions["recover"]` の `sync_tokens`/`notes` は `Parser_diag_state.create` へ伝播するようになり、`recover.notes=false` でも診断警告を抑制できる。`extensions["stream"]` は checkpoint/resume ヒントを `Run_config.Stream.t` で保持し、未設定時は空のプレースホルダで扱う。`compiler/ocaml/src/dune` に `parser_run_config` を追加し `dune build` を通過することを確認。

### Step 4: クライアント・測定基盤の対応（Week32 Day4）
- CLI オプション (`Cli.Options`) から RunConfig を構築するヘルパを追加し、`--require-eof` / `--packrat` / `--left-recursion` / `--trace` / `--no-merge-warnings` など Phase 2 で予定していたフラグを再確認して実装する。未実装のフラグは TODO として `docs/notes/core-parser-migration.md` に記録する。  
- LSP トランスポート層で RunConfig を生成する処理を追加し、既存の設定ファイル（`tooling/lsp/config/*.json`）を `extensions["lex"]` 等に反映させる。  
- `compiler/ocaml/tests/` のテストヘルパを更新し、RunConfig を明示的に渡す API（`Test_support.with_run_config` 等）を用意する。  
- `reports/diagnostic-format-regression.md` に RunConfig 切替シナリオを追加し、CLI/LSP の JSON が設定値に追従することを比較できるようにする。

> 2025-11-21 更新: Step 4 完了。`compiler/ocaml/src/cli/options.ml` に `Cli.Options.to_run_config` を追加し、`--require-eof` / `--packrat` / `--left-recursion` / `--no-merge-warnings` を経由して RunConfig を構築できるようにした。CLI 既定値は仕様に合わせて `require_eof=false` へ更新し、従来挙動は `--require-eof` で復元できる。LSP 向けには `tooling/lsp/run_config_loader.ml` と `tooling/lsp/config/default.json` を作成し、`extensions["lex"|"recover"|"stream"]` を設定ファイルから再現するロードパスを定義した。テスト支援ライブラリに `compiler/ocaml/tests/support/test_support.ml` を追加し、`Test_support.parse_string` / `Test_support.with_run_config` を `test_parser.ml`・`test_type_inference.ml` へ展開。RunConfig 移行時の未完タスクは `docs/notes/core-parser-migration.md` に TODO として記録した。

### Step 5: テスト・検証・メトリクス定着（Week32 Day4-5）
- `compiler/ocaml/tests/run_config_tests.ml` を作成し、`require_eof`・`merge_warnings` の挙動、`trace` ON/OFF の `SpanTrace` 収集、`extensions["lex"]` による空白共有、`legacy_result=true` での互換性をパラメトリックテストで確認する。  
- `0-3-audit-and-metrics.md` に `parser.runconfig_switch_coverage`（packrat/left_recursion/trace/merge_warnings のテスト網羅率）と `parser.runconfig_extension_pass_rate`（lex/recover/stream の設定伝搬率）を追加し、`collect-iterator-audit-metrics.py` が新メトリクスを収集できるように改修する。  
- `scripts/validate-diagnostic-json.sh` に RunConfig 設定を含むサンプル（`parser-runconfig-packrat.json` 等）を追加し、AJV 検証で設定値が JSON に記録されることを確かめる。  
- CLI・LSP 双方で RunConfig を通したパースを実行し、`diagnostic_schema.validation_pass` が 1.0 を維持することを確認する。

> 2025-11-22 更新: Step 5 完了。`compiler/ocaml/tests/run_config_tests.ml` を追加して `require_eof` ガード・`merge_warnings` 重複処理・`trace` SpanTrace・`extensions["lex"]` デコード・Legacy ブリッジを検証した。RunConfig 専用ゴールデン（`compiler/ocaml/tests/golden/diagnostics/parser/parser-runconfig-packrat.json.golden`）を整備し、`tooling/ci/collect-iterator-audit-metrics.py` に `parser.runconfig_switch_coverage` / `parser.runconfig_extension_pass_rate` を集計する処理を追加。`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` へ新指標を登録し、`--require-success` で RunConfig 監視が必須化された。

### Step 6: 共有とレビュー記録（Week32 Day5）
- 実装結果を `docs/plans/bootstrap-roadmap/2-5-review-log.md` に記録し、完了条件（RunConfig フィールド実装、メトリクス更新、CLI/LSP 連携）を明確にする。  
- `docs/spec/2-1-parser-type.md`・`docs/spec/2-6-execution-strategy.md`・`docs/guides/core-parse-streaming.md` に移行脚注と利用例を追記し、仕様との整合を示す。  
- `docs/notes/core-parser-migration.md`（新設予定）に RunConfig 移行ステップと今後の課題（Packrat 実装、ストリーミング連携）を整理し、Phase 3 の self-host 作業へ渡す。

> 2025-11-24 更新: Step 6 完了。`docs/plans/bootstrap-roadmap/2-5-review-log.md` に Day6 エントリを追加し、RunConfig 共有手順と残課題を整理した。`docs/spec/2-1-parser-type.md` と `docs/spec/2-6-execution-strategy.md` に CLI/LSP 共有脚注と利用例を追記し、`docs/guides/core-parse-streaming.md` §9 へ RunConfig 連携ワークフローを掲載。`docs/notes/core-parser-migration.md` で Step1〜6 の完了状況とフォローアップ先（`PARSER-003`/`LEXER-002`/`EXEC-001`）を一覧化した。

## 6. 依存関係と連携
- **PARSER-001**: `ParseResult` シムを導入済み。RunConfig を渡すために `parser_driver` API 変更が前提。  
- **LEXER-002**: `extensions["lex"]` で共有する字句設定を利用するため、RunConfig へのアクセス関数と互換メトリクスを調整する。  
- **EFFECT-003**: 複数 Capability 情報を診断へ出力する際に `RunConfig.extensions["runtime"]` などを参照する可能性があるため、拡張のキー解決ルールを共有する。  
- **TYPE-001**: 値制限復元時に RunConfig の設定（特に `legacy_result` と `extensions["effects"]`）が影響する可能性を確認し、必要なフックを提供する。  
- **EXEC-001**: ストリーミング PoC が `extensions["stream"]` を利用するため、RunConfig 導入時に placeholder と API を提供しておく。  
- **Phase 2-7**: RunConfig 関連の監査ダッシュボード更新・CLI テキスト出力刷新を Phase 2-7 チームへ引き継ぐ。

## 7. 残課題
- Packrat/左再帰の実装詳細は `PARSER-003` に委ねる必要があり、Menhir ランナーでメモ化を実現するための検討（LR(1) → GLR/Packrat シム化）が未完。  
- `RunConfig.locale` と `Diagnostic` 出力のロケール連携は `DIAG-003` の判断待ちであり、既定値とフォールバック方針を共有する必要がある。  
- CLI/LSP に追加する RunConfig フラグの UX（名称・互換性）と設定ファイルへの保存形式を確定する。  
- `extensions` に格納する値の型安全性（`ParserId` や `ConfigTriviaProfile` のシリアライズ方法）を決定し、誤設定時の診断をどう報告するか決める必要がある。  
- 大規模入力での RunConfig 導入によるメモリ・性能影響を継続的に測定し、Phase 2-6 以降へフィードバックする。

[^syntax003-gap]: `reports/spec-audit/diffs/SYNTAX-003-ch1-rust-gap.md`（2025-11-18 更新）。effect handler 受理と `TraceEvent::ExprEnter` 拡張の証跡を整理。
