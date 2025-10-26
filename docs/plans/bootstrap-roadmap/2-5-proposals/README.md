# Phase 2-5 修正計画カタログ

このディレクトリは Phase 2-5「仕様差分是正」（`../2-5-spec-drift-remediation.md`）で扱う修正計画の置き場です。計画を参照・更新する際は以下の方針を守ってください。

- **前提資料の確認**: `../../spec/0-1-project-purpose.md` と `../2-0-phase2-stabilization.md` を参照し、優先度と成果物の期待値を再確認する。
- **差分管理**: 各計画の実装状況や脚注追加・更新時には関連仕様（`docs/spec/`）と `README.md`（リポジトリ索引）を同時に更新する。
- **記録保持**: 重要な判断・保留事項は計画内の「残課題」または `docs/notes/` 配下の関連ノートへ記録し、追跡可能な状態を維持する。

## 目次とハイライト

### 診断ドメイン（DIAG）
- [DIAG-001 修正計画](./DIAG-001-proposal.md): `Severity = {Error, Warning, Info, Hint}` を導入して Chapter 3（`docs/spec/3-6-core-diagnostics-audit.md`）との整合を回復。
- [DIAG-002 修正計画](./DIAG-002-proposal.md): `Diagnostic.audit` と `timestamp` を必須化し、Builder/Legacy 経路の棚卸し・JSON スキーマ更新・`collect-iterator-audit-metrics.py` のゲート強化を通じて `ffi_bridge.audit_pass_rate` / `iterator.stage.audit_pass_rate` を 1.0 に引き上げる。
- [DIAG-003 修正計画](./DIAG-003-proposal.md): `DiagnosticDomain` を効果・プラグイン・LSP など仕様準拠の語彙へ拡張し、監査ログ分析を改善。

### 効果システム（EFFECT）
- [EFFECT-001 修正計画](./EFFECT-001-proposal.md): `mut`/`io`/`ffi`/`unsafe` などのタグ検出を強化し、残余効果診断を Chapter 1-3 と一致させる。
- [EFFECT-002 修正計画](./EFFECT-002-proposal.md): `perform`/`handle` を含む効果操作 PoC の方針を明確化し、`Σ_before`/`Σ_after` の検証を可能にする。
- [EFFECT-003 修正計画](./EFFECT-003-proposal.md): 複数 Capability を解析・監査へ出力する仕組みを整備し、Stage 契約（`docs/spec/3-8-core-runtime-capability.md`）との齟齬を是正。

### エラー回復（ERR）
- [ERR-001 修正計画](./ERR-001-proposal.md): Menhir の期待集合を `ExpectationSummary` に反映させ、`docs/spec/2-5-error.md` で定義された期待値提示を実現。
- [ERR-002 修正計画](./ERR-002-proposal.md): `Parse.recover` の同期トークンと FixIt を導入し、CLI/LSP での自動修正と診断補助を整備。

### 実行戦略（EXEC）
- [EXEC-001 修正計画](./EXEC-001-proposal.md): `run_stream`/`resume` を備えたストリーミング実行 PoC を構築し、`docs/spec/2-6-execution-strategy.md` の契約を検証。

### 字句解析（LEXER）
- [LEXER-001 修正計画](./LEXER-001-proposal.md): Unicode 識別子プロファイル導入までの暫定対応を明文化し、DSL/プラグイン計画と共有。
- [LEXER-002 修正計画](./LEXER-002-proposal.md): `Core.Parse.Lex` ユーティリティを抽出し、字句設定 (`RunConfig.extensions["lex"]`) を仕様準拠に整備。

### 構文解析（PARSER）
- [PARSER-001 修正計画](./PARSER-001-proposal.md): `ParseResult` シムを導入し、`Reply{consumed, committed}` と診断集約を再現。Week31 Day1-5 で `parser_driver` を段階的に差し替え、`parser.parse_result_consistency` / `parser.farthest_error_offset` を `0-3-audit-and-metrics.md` に登録して CI 監視する（実装済: `parser_driver.ml` シム化・`parser_diag_state.ml` 追加・`dune runtest tests` 成功・メトリクス/脚注/`scripts/validate-diagnostic-json.sh` の自動検証まで反映完了）。
- [PARSER-002 修正計画](./PARSER-002-proposal.md): `RunConfig` をランナーへ統合し、Packrat／recover／stream 設定を反映できるようにする。
- [PARSER-003 修正計画](./PARSER-003-proposal.md): 15 個のコアコンビネーターを OCaml 実装へ抽出し、`Core.Parse` API と DSL の互換性を確保。

### 構文仕様（SYNTAX）
- [SYNTAX-001 修正計画](./SYNTAX-001-proposal.md): Unicode 識別子制約の暫定状態を仕様脚注で明示し、Phase 2-7 の対応計画を共有。
- [SYNTAX-002 修正計画](./SYNTAX-002-proposal.md): `use` 文の多段ネストを AST に反映し、Chapter 1 のサンプル通過を保証。
- [SYNTAX-003 修正計画](./SYNTAX-003-proposal.md): 効果構文（`perform`/`handle`）の実装ステージを明確化し、Formal BNF との乖離を是正。

### 型システム（TYPE）
- [TYPE-001 修正計画](./TYPE-001-proposal.md): 値制限と効果タグ連携を復元し、副作用を持つ束縛の多相化を防止。
- [TYPE-002 修正計画](./TYPE-002-proposal.md): 効果行を型表現へ統合するロードマップを策定し、型と効果の一体管理を再構築。
- [TYPE-003 修正計画](./TYPE-003-proposal.md): 型クラス辞書渡しを Core IR へ復元し、監査ログへの Capability 情報出力を再開。（2025-10-30 更新: Typer／Core IR／CI メトリクス整備まで完了。2025-10-31 追記: Stage 逆引き・辞書付き診断ゴールデン・ドキュメント整備まで完了。）

## 着手順序ガイド
| 時期と順序 | 対象計画 | 目的と前提関係 |
|------------|----------|----------------|
| Phase 2-5 開始直後（Week31 前半） | PARSER-001, TYPE-003, DIAG-002 | パーサ基盤・型クラス監査・監査ログ必須化を最初に整備し、以降の差分検証を可能にする |
| Phase 2-5 前半（Week31 後半〜Week32） | EFFECT-001, DIAG-001, SYNTAX-002, ERR-001 | 効果タグと Severity を拡張し、`use` ネスト・期待集合のギャップを早期に解消する |
| Phase 2-5 中盤（Week32〜Week33） | PARSER-002, LEXER-002, DIAG-003, EFFECT-003, TYPE-001 | RunConfig/lex シムと複数 Capability を整備し、値制限復元を可能にする |
| Phase 2-5 後半（Week33〜Week34） | PARSER-003, EXEC-001, ERR-002 | コアコンビネーター抽出後にストリーミング PoC と FixIt 拡張を実装し、ランナー整合を仕上げる |
| Phase 2-5 クロージング〜Phase 2-7 準備 | LEXER-001, SYNTAX-001, SYNTAX-003, EFFECT-002, TYPE-002 | Unicode・効果構文・効果行は脚注整備とロードマップ策定を Phase 2-5 で行い、Phase 2-7 以降で本実装する |

## 運用メモ
- 新しい計画を追加する際は、ドメイン別セクションに箇条書きを追加し、関連仕様とメトリクスを併記する。
- 計画のステータス更新（完了・棚上げ等）は本文と併せてここにも反映し、Phase 2-5 全体の進捗を一目で把握できるようにする。
- 大幅な構造更新やファイル移動を行った場合は `docs-migrations.log` と `README.md`（リポジトリ索引）を忘れずに追記する。
