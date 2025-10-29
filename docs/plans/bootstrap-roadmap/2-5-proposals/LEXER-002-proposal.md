# LEXER-002 Core.Parse.Lex API 抽出計画

## 1. 背景と症状
- 仕様は `Core.Parse.Lex` が空白・コメント・lexeme 等の共有ユーティリティを提供すると定義している（docs/spec/2-3-lexer.md 全体、特に §C〜G）。  
- 実装は `parser_driver` から直接 `Lexer.token` を呼び出し、`lexeme`/`symbol`/`config_trivia` などの高級 API や `RunConfig.extensions["lex"]` と連携する層が存在しない。  
- DSL や CLI/LSP が同じ空白処理を共有できず、仕様で求める `ParserId` / `ConfigTriviaProfile` の再利用が行えない。

## 2. Before / After
### Before
- 字句ユーティリティは非公開モジュールに散在し、構文パーサは Menhir 経由で直接トークン列を消費。  
- `RunConfig.extensions["lex"]` を設定しても利用されず、`lexeme` 相当の動作を実装が行っていない。

### After
- `compiler/ocaml/src/core_parse_lex.ml`（仮称）に `lexeme` / `symbol` / `config_trivia` など仕様準拠の関数群を実装し、`RunConfig.extensions["lex"]` から共有設定を読み込むシムを提供。  
- `parser_driver` をリファクタリングし、字句処理を `Core.Parse.Lex` 経由で行うよう段階移行する。  
- 仕様には現行制限の脚注を追加し、「OCaml 実装は Lex API の抽出を進行中」と明記する。

## 3. 影響範囲と検証
- **共有設定**: CLI/LSP/テストで `config_trivia` を利用し、同じ空白・コメントプロファイルが適用されるか検証。  
- **Packrat/ParserId**: `lexeme` が `rule` と連携し、`ParserId` を安定化させるかテスト（2-3 §B〜C）。  
- **メトリクス**: `0-3-audit-and-metrics.md` に `lexer.shared_profile_pass_rate` を追加し、`RunConfig.extensions["lex"]` の反映状況を監視。
- **単体テスト**: `compiler/ocaml/tests/core_parse_lex_tests.ml` を追加し、空白・コメント・カスタムトークンプロファイルを切り替えた際の `lexeme` / `symbol` 出力をゴールデン比較で検証する。

## 4. フォローアップ
- `PARSER-002`（RunConfig 導入）と連動し、字句設定を `RunConfig` から読み込む順序を調整する。  
- `docs/guides/core-parse-streaming.md` のサンプルを OCaml 実装で動かせるよう、Lex API を呼び出す例を追加する。  
- Unicode プロファイル（LEXER-001）の対応と並行して、字句 API のテストを整備する。
- `docs/notes/core-parse-streaming-todo.md` に Lex API 抽出の進捗を追記し、Streaming PoC（EXEC-001）との依存関係を明確化する。
- **タイミング**: Phase 2-5 の中盤で RunConfig シムと並行して着手し、EXEC-001 ストリーミング PoC を開始するまでに Lex API 抽出を完了させる。

## 5. 実装ステップと調査

### Step 0: 仕様・現行実装ギャップ調査（Week33 Day1）
- `docs/spec/2-3-lexer.md` のコア API（§B〜§L）と `docs/spec/2-1-parser-type.md` §D を読み込み、必要なインターフェースを一覧化して `compiler/ocaml/src/lexer.mll`・`parser_driver.ml` との乖離を洗い出す[^lex-spec-core]。
- `parser_run_config.Lex.Trivia_profile` と `ConfigTriviaProfile` の差分、`RunConfig.extensions["lex"]` の現状利用ポイントを表にまとめ、`docs/plans/bootstrap-roadmap/2-5-review-log.md` へ LEXER-002 の Day1 エントリとして記録する[^runconfig-lex]。
- `docs/notes/core-parse-streaming-todo.md` に調査メモを残し、`PARSER-002`/`PARSER-003`/`EXEC-001` との依存関係（共有 `ParserId`・`RunConfig` 伝播・Streaming API）を整理する。

#### 調査サマリ（2025-11-25）
- `docs/spec/2-3-lexer.md` が定義する `Core.Parse.Lex` API（`lexeme` / `symbol` / `config_trivia` 等）に対応する公開モジュールは現行実装に存在せず、`parser_driver` は `Lexer.token` を直接呼び出している（`compiler/ocaml/src/parser_driver.ml`）。これにより `RunConfig.extensions["lex"]` や `ParserId` 連携が遮断されている。
- `parser_run_config.Lex.Trivia_profile` は `ConfigTriviaProfile` と同等のフィールドを保持するものの、実際の字句処理へ渡されておらず、`space_id` や `profile` が `lexer.mll` で参照されない。`config_trivia` / `config_lexeme` / `config_symbol` に相当するユーティリティも未実装である。
- `lexer.mll` は ASCII ベースの空白・コメント・識別子判定に留まっており、仕様で必須とされる `shebang` 読み飛ばし・`hash_inline` コメント・`doc_comment` 収集・Unicode XID 対応が欠落している。
- 仕様との差分一覧と `RunConfig.extensions["lex"]` の現状利用調査を [`docs/plans/bootstrap-roadmap/2-5-review-log.md`](../2-5-review-log.md) の「LEXER-002 Day1」へ記録し、Streaming/RunConfig 計画との依存関係メモを `docs/notes/core-parse-streaming-todo.md` に追加した。

### Step 1: Core.Parse.Lex ベースモジュール設計（Week33 Day1-2）
- `compiler/ocaml/src/core_parse_lex.mli`（新規）に公開シグネチャ案を記述し、`lexeme`/`symbol`/`config_trivia`/`token` を最小構成として定義する。同時に `core_parse_lex.ml` へ未実装例外を置き、後続ステップで段階的に埋める。
- `docs/spec/2-2-core-combinator.md` と `PARSER-003` の計画を参照し、`type 'a parser` の実装方式（現状の Menhir シム vs 将来のコンビネーター層）を調査したうえで、`ParserId` 付与と互換になる API 設計メモを `docs/notes/core-parse-api-evolution.md`（必要なら新規）へ追記する。
- `token.ml` で定義済みのトークン集合と照合し、Lex API が生成する予定の値型（識別子・数値・コメントスキップなど）が現行 AST と齟齬を起こさないか確認する。

### Step 2: ConfigTriviaProfile と RunConfig 橋渡し（Week33 Day2）
- `parser_run_config.Lex.Trivia_profile` を `Core.Parse.Lex.Trivia`（仮名）にラップし、仕様の `ConfigTriviaProfile` 定数（`strict_json` ほか）へマッピングするユーティリティを実装する[^config-trivia]。
- `Core.Parse.Lex.config_trivia` / `config_lexeme` / `config_symbol` を `Run_config.Lex.of_run_config` と `Run_config.Config` から生成できるようにし、結果の `ParserId` を `Extensions["lex"].space_id` に格納する経路を設計する。
- 互換モード（shebang・hash_inline 等）を評価するため、`examples/` 以下の JSON/TOML サンプルを用いてトリビア設定の期待挙動を表形式でまとめ、`2-5-review-log.md` に添付する。

### Step 3: lexeme/symbol 系ユーティリティ実装（Week33 Day3）
- `Core.Parse.Lex.lexeme`/`symbol`/`leading`/`trim`/`token` を `Parser` コンビネータの構成要素として実装し、`ParserId` 生成・`Span` 付与・`RunConfig` からのスペース共有を行う。
- `lexer.mll` 内で担っている空白・コメント処理を `Core.Parse.Lex` に委譲するため、`skip_trivia` 相当の内部関数を抽出し、Menhir ランナーでもコメント処理が二重にならないように責務境界を定義する[^lexer-impl]。
- 仕様が求める診断情報（期待集合と farthest offset）に影響しないかを確認するため、`parser_diag_state` と `Parser_expectation` の更新ポイントを点検し、必要に応じて TODO を `docs/notes/core-parse-diagnostics-gap.md` へ登録する。

### Step 4: parser_driver / CLI 統合（Week33 Day3-4）
- `parser_driver.run` で `Run_config.Lex.of_run_config` を取得し、`Core.Parse.Lex` に橋渡しするコードパスを追加する。`extensions["lex"]` が未設定の場合は `ConfigTriviaProfile::strict_json` を既定とし、`space_id` が得られた際は `Extensions.with_namespace` で再格納する。
- `lexer.mll` が返す `Token` と `Core.Parse.Lex` が維持する `Span` / `ParserId` を同期させるため、`lexer.mll` に軽量なフック（例: `Core_parse_lex.Record.consume`）を挿入し、コメントスキップを統一的に計測可能にする。
- CLI / LSP 経路（`compiler/ocaml/src/main.ml`, `tooling/lsp/run_config_loader.ml`）が `extensions["lex"]` の `profile`・`space_id` を正しく設定することを再確認し、欠落時は Warning を出す実装 TODO を登録する。

### Step 5: テスト・メトリクス・性能確認（Week33 Day4-5）
- `compiler/ocaml/tests/core_parse_lex_tests.ml` を新設し、`strict_json` / `json_relaxed` / `toml_relaxed` 各プロフィールでの `lexeme` / `symbol` / `config_trivia` 動作をゴールデンで検証する。Packrat 向けの `ParserId` 安定性は `parser.runconfig_extension_pass_rate` と組み合わせて監視する。
- `tooling/ci/collect-iterator-audit-metrics.py` に `lexer.shared_profile_pass_rate` 指標を追加し、`0-3-audit-and-metrics.md`・`2-5-review-log.md` へ測定方法を明記する[^metrics-lex]。
- 大規模入力（10MB クラス）の字句性能を `scripts/benchmark.sh` または既存マイクロベンチで測定し、`docs/notes/lexer-performance-study.md`（必要なら新規）に比較データを残す。

### Step 6: ドキュメント反映とレビュー記録（Week33 Day5）
- `docs/spec/2-3-lexer.md` と `docs/spec/2-6-execution-strategy.md` に OCaml 実装の進捗脚注を追加し、`RunConfig` 経由で Lex API を共有できる状態になったことを明記する。`docs/guides/core-parse-streaming.md` のサンプルコードも新 API に合わせて更新する。
- `docs/plans/bootstrap-roadmap/2-5-review-log.md` に Step1〜6 の結果を追記し、未解決事項は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` へ引き継ぐ。
- 仕様差分計画本体（`../2-5-spec-drift-remediation.md`）の §6.2 脚注を更新し、LEXER-002 のステータス・残課題・メトリクス参照先をリンクする。

## 残課題
- `Core.Parse.Lex` と `Lexer.token` の責務分離（どこまでをシムで巻き取るか）について Parser チームと合意が必要。  
- コメント・空白処理を共有する際の性能影響（特に大型入力でのオーバーヘッド）を事前に評価したい。

[^lex-spec-core]:
    `docs/spec/2-3-lexer.md`（§B〜§L）と `docs/spec/2-1-parser-type.md` §D を突き合わせ、`lexeme`・`config_trivia`・`RunConfig.extensions` の契約を確認する。
[^runconfig-lex]:
    `docs/plans/bootstrap-roadmap/2-5-proposals/PARSER-002-proposal.md` で導入された `Run_config` シムと `parser_run_config.mli` の `Lex` モジュール、`docs/plans/bootstrap-roadmap/2-5-review-log.md` の Day6 記録を参照し、`extensions["lex"]` のハンドシェイク手順を整理する。
[^config-trivia]:
    `docs/spec/2-3-lexer.md` §G（ConfigTriviaProfile）と `docs/spec/3-7-core-config-data.md` §1.5、および `parser_run_config.ml` 内 `Lex.Trivia_profile` 実装を用いてプロフィール変換を設計する。
[^lexer-impl]:
    `compiler/ocaml/src/lexer.mll` の空白・コメント規則と `parser_driver.ml` の token 読み出しを調査し、`Core.Parse.Lex` へ責務移譲する際の影響範囲（位置情報・診断）を把握する。
[^metrics-lex]:
    `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の Parser 指標群と `tooling/ci/collect-iterator-audit-metrics.py` を更新し、Lex プロファイル共有率を CI 監視できるようにする。
