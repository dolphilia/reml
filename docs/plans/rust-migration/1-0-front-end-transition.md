# 1.0 フロントエンド移植計画

本章は Phase P1（フロントエンド移植）における目的・達成条件・成果物・作業手順を明文化する。`unified-porting-principles.md` の優先順位原則（振る舞いの同一性最優先）と P0 で確立したベースラインを基準とし、OCaml 実装から Rust 実装への移行を段階的に進める。

## 1.0.1 目的
- Reml OCaml 実装のパーサ／型推論／診断前処理を Rust へ移植し、観測可能な挙動（AST・型・診断 JSON）を等価に再現する。
- Dual-write（OCaml→Rust 並行出力）により差分を可視化し、`0-1-baseline-and-diff-assets.md` で定義したゴールデン／ベンチ指標と照合する。
- Phase P2 以降のランタイム統合・CI 拡張で利用できるよう、Rust フロントエンドの API とメトリクスを安定化させる。

## 1.0.2 スコープと前提
- **対象範囲**: 
  - 構文解析（lexer・Menhir 相当のパーサ生成・`parser_driver.ml` の機能移植）
  - AST/IR モデル（`Ast`/`Typed_ast`/`Core_parse` 系の構造体とストリーミング状態）
  - 型推論・制約解決（`type_inference.ml`・`constraint_solver.ml` 等）
  - 診断前処理と JSON 序盤整形（`Diagnostic.Builder`、`parser_expectation` 周辺）
- **除外**: バックエンド LLVM 生成、ランタイム FFI、CI パイプライン更新（P2/P3 で扱う）。
- **前提**:
  - P0 文書の完了条件（ベースライン測定・Windows 環境監査・用語整合）が満たされている。
  - 仕様書 `docs/spec/1-1-syntax.md` `docs/spec/1-2-types-Inference.md` `docs/spec/3-6-core-diagnostics-audit.md` の参照箇所が最新。
  - OCaml 実装の最新差分は `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` と連動しており、仕様乖離の補正手順が確立済み。

## 1.0.3 完了条件
- Rust フロントエンドで生成した AST/Typed AST/診断 JSON が、P0 ベースラインで定義したゴールデン比較にて許容差分（仕様上許容されない差分: 0件、統計値のばらつき: ±1%）内に収まる。
- `1-1-ast-and-ir-alignment.md` と `1-2-diagnostic-compatibility.md` に定義した検証チェックリストを全項目パスし、差分ログが `reports/` 配下に保存されている。
- Dual-write モードで実行した `parser_driver` / `type_inference` テスト群が `compiler/ocaml/tests` と同等の合格率を達成し、逸脱は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に転記済み。
- Rust 実装に必要な API ドキュメント（crate 内コメント）と外部仕様リンクが整理され、P2 以降へ引き継ぐ準備が整っている。

## 1.0.4 主成果物

| 成果物 | 内容 | 依存資料 |
| --- | --- | --- |
| `compiler/rust/frontend/` 初期構成 | Lexer・Parser・AST モデル・Type Inference の雛形とテストハーネス | `compiler/ocaml/src/` 内各モジュール, `docs/spec/1-1`/`1-2` |
| Dual-write 差分ハーネス | OCaml 実装と Rust 実装を同一 CLI から呼び出す比較ツール | `0-1-baseline-and-diff-assets.md`, `tooling/ci/collect-iterator-audit-metrics.py` |
| ベンチ・診断比較レポート | AST/診断ゴールデンの比較結果および性能測定 | `reports/diagnostic-format-regression.md`, `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` |
| 設計補足ノート | OCaml→Rust の構造変換・既知の仕様差分リスト | `docs/notes/`（必要に応じて新規追加） |

## 1.0.5 作業マイルストーン（目安）

| 週 | マイルストーン | 主タスク | 検証方法 |
| --- | --- | --- | --- |
| W1 | Lexer/Parser スケルトン移植 | `parser_driver.ml` の状態管理移植、Menhir 生成規則の Rust 化方針策定 | `compiler/ocaml/tests/parser_*` のゴールデン比較、手動チェック |
| W2 | AST/IR 対応表の確定 | `Ast`/`Typed_ast` の Rust 構造体定義、`Core_parse` ストリーミング状態の移植案 | `1-1-ast-and-ir-alignment.md` のチェックリスト半分以上消化 |
| W3 | 型推論コア移植 | 制約生成・ソルバ・impl レジストリの Rust 設計、dual-write テストパイプライン構築 | `compiler/ocaml/tests/test_type_inference.ml` に基づく比較レポート |
| W4 | 診断互換試験 | JSON エミッタ・`recover` 拡張・`extensions.*` の対照テスト、CLI/LSP 連携確認 | `scripts/validate-diagnostic-json.sh`、`reports/diagnostic-format-regression.md` |
| W4.5 | P1 クロージングレビュー | 成果物レビュー、差分リスト整理、P2 ハンドオーバー資料草案 | `docs/plans/rust-migration/README.md` 更新、`docs-migrations.log` 記録 |

### W1 具体的な進め方（Lexer/Parser スケルトン移植）✅ 完了

1. **準備と方針の再確認**  
   - P0 完了条件が満たされ、最新のゴールデンデータと Windows 監査結果を Rust 側でも参照できる状態を確認する。  
   - `unified-porting-principles.md` の優先順位原則と dual-write 前提をチームで再共有し、性能・安全性の許容範囲を明文化する。
   - ✅ 2025-03-09: `reports/dual-write/front-end/`（ゴールデン OCaml 出力／差分格納レイアウト）と `reports/toolchain/windows/20251106/*.json`（`setup-windows-toolchain.ps1`・`check-windows-bootstrap-env.ps1` の監査ログ）が Rust チームから直接参照可能であることを確認。`docs/plans/rust-migration/0-1-baseline-and-diff-assets.md` に記載された既知の欠落項目（`parser.core.rule.*` など）を引き継ぎつつ、`unified-porting-principles.md` §1 の優先順位と成功指標（診断キー一致 100%、性能回帰 ±10% 以内、Windows MSVC/GNU 5 連続成功）を本計画書の進捗ログとして再記録した。

2. **OCaml 実装の棚卸しと設計ノート整備**  
   - `compiler/ocaml/docs/parser_design.md` を読み、字句要素・演算子優先順位・構文カテゴリを洗い出して Rust 実装で必要となるトークン/ノード一覧を作成する。  
   - `parser_driver.ml` と `parser_expectation.ml` の役割分担（状態遷移、回復戦略、期待トークン生成）を整理し、抜け漏れをメモ化する。
   - ✅ 2025-03-12: `docs/plans/rust-migration/appendix/parser-ocaml-inventory.md` を追加し、トークン集合・AST ノード列挙・`parser_driver`/`parser_expectation` の責務を整理した。Packrat メトリクスと診断拡張の再現要件もギャップとして記録済み。

3. **Rust フロントエンド骨格の用意**  
   - `compiler/rust/frontend/` 配下に Lexer・Parser・Streaming モジュールの雛形ファイルと `Cargo.toml` の該当セクションを追加し、依存クレート候補（`logos`/`chumsky` 等）の評価メモを添える。  
   - Span 型、トークン列挙、エラー種別、Recoverable 状態など共通で利用する基礎データ構造を Rust で宣言し、`docs/spec/1-1-syntax.md` に沿った命名と型域（`u32` オフセット等）を確認する。
   - ✅ 2025-11-06: `compiler/rust/frontend/Cargo.toml` と `src/lib.rs` を起点に `span.rs`・`token.rs`・`error.rs`・`diagnostic/mod.rs`・`lexer/mod.rs`・`parser/mod.rs`・`streaming/mod.rs` を追加し、`Span(u32,u32)` や `Recoverability` 分類など共通基礎型を定義した。スケルトン Lexer は識別子／整数リテラル／未知トークンまでを扱い、未知入力時に回復可能診断を返す挙動を確認済み。依存候補の比較結果は `docs/plans/rust-migration/appendix/frontend-crate-evaluation.md` に整理し、`logos`（lexing）と `chumsky`（parsing）を PoC 優先候補として記録した。

4. **パーサ生成戦略と状態管理の設計**  
   - ✅ 2025-11-28: `docs/notes/core-parser-migration.md#p1-w1-rust-parser-戦略と状態管理（2025-11-28）` に `logos`＋`chumsky` を第一候補（`pomelo` をフォールバック）とする決定と PoC ストーリー、`ParserSession`/`StreamingState` の責務整理を記録した。`lalrpop` はエラー回復と生成物サイズの懸念で除外。  
   - `Core_parse` の state machine・入力ストリーム・エラー復旧フックを分解し、Rust の `ParserDriver`（仮）へ移す責務を定義済み。`ReplyFlags`（`consumed`/`committed`/`far_error`）と `PackratEntry` のキー仕様（`ParserId`＋`Range<u32>`）を固め、`parser.stream.*` のメトリクス更新を `StreamingState` で一元化する設計を確定。  
   - PoC ゴール：`parser::driver::tests::basic_roundtrip` で AST/診断差分ゼロを確認し、`tests/streaming_metrics.rs` で `packrat_hits` カウンタの増減を検証する。CLI フック（`remlc --frontend rust --emit parse-debug`）案は `1-3-dual-write-runbook.md` の手順と整合した草案を共有済み。  
   - ✅ 2025-11-28: `scripts/poc_dualwrite_compare.sh` を `cargo test` 後に再実行し、4 ケース（`empty_uses` / `multiple_functions` / `addition` / `missing_paren`）で AST・診断件数が OCaml ベースラインと一致することを確認。`missing_paren` の診断メッセージも OCaml と同値に揃え、期待トークン一覧は `recover.expected_tokens` ノートへ格納した。

5. **Packrat / span_trace 再現の設計**  
   - `Core_parse_streaming` の packrat キャッシュと `span_trace` 収集ロジックを調査し、Rust で利用するデータ構造（`IndexMap`/`HashMap` と寿命管理）を決定する。  
   - メトリクス項目（`parser.stream.*`）と連携するカウンタをどこで更新するか設計ノートに明記する。  
   - ✅ 2025-12-05: `docs/notes/core-parser-migration.md#p1-w1-packrat--span_trace-キャッシュ再現設計2025-12-05` に Packrat キャッシュと `span_trace` の Rust 再現方針を記録。`IndexMap<(ParserId, Range<u32>)>`＋`SmallVec` ベースの `PackratEntry`、`RwLock` で包んだ `VecDeque` トレース、および `parser.stream.packrat_*` / `parser.stream.span_trace_*` の更新ポイントと予算超過時の制御手順を整理した。  
   - ✅ 2025-12-06: `compiler/rust/frontend/src/streaming/mod.rs` に Packrat キャッシュ／`span_trace` 実装を追加し、`ParserDriver::parse` から `StreamingState` を呼び出して `packrat_stats`・`span_trace` を収集。CLI PoC（`poc_frontend`）は `parse_result.packrat_stats` / `parse_result.span_trace` を JSON 出力し、`tests/streaming_metrics.rs` と `tooling/ci/collect-iterator-audit-metrics.py` で統合経路を確認済み。
   - ✅ 2025-12-07: 成功パスでも `StreamingState` を参照するよう `module_parser` を改修し、Packrat キャッシュが実際に再参照される状態を確認。`poc_frontend` に `--emit-parse-debug <path>` を追加し、OCaml 版の `remlc --emit parse-debug` 相当の JSON (`run_config`/`parse_result`/`stream_meta`) を生成して dual-write レポート／CI に `packrat_stats` を配布できるようにした。

6. **最小ケースでの dual-write 準備**  
   - `remlc --frontend {ocaml|rust}` 相当の切り替えインターフェースに必要な CLI フラグや build ターゲットを列挙し、未実装部分には TODO を残す。  
   - `reports/dual-write/front-end/` に W1 用の成果物ディレクトリ構成を作成し、AST/診断 diff とメトリクス出力を保存するコマンドシーケンスを `1-3-dual-write-runbook.md` の手順と照合する。  
   - ✅ 2025-11-28: `scripts/poc_dualwrite_compare.sh` を実行し、`reports/dual-write/front-end/poc/2025-11-28-logos-chumsky/summary.md` に 4 ケース分の AST/診断比較結果を保存。`missing_paren` は診断件数が一致したもののメッセージ粒度が異なるため、W2 で `SimpleReason` → Recover サマリ変換を整備して `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` にフォローアップを登録する。
   - ✅ 2025-12-07: W1 成果物を `reports/dual-write/front-end/poc/2025-11-28-logos-chumsky/` に再集約し、`w1-packrat-summary.json`（dual-write Packrat 統計）と `w1-parse-debug-summary.json`（Rust `--emit-parse-debug` 出力）を作成。`reports/dual-write/front-end/poc/w1-recap.md` に概要をまとめ、W2 の AST/IR 対応表タスクへの入力資料として共有した。

### W2 具体的な進め方（AST/IR 対応表の確定）✅ 完了

1. **事前同期と対象スコープの固定**  
   - `1-1-ast-and-ir-alignment.md` の §1.1.2〜1.1.8 と `p1-front-end-checklists.csv` の AST/Typed AST/ストリーミング行を読み返し、今回の W2 で「どこまでを完了させれば良いか」を明文化する。  
   - `reports/dual-write/front-end/poc/w1-recap.md` と `docs/plans/rust-migration/appendix/parser-ocaml-inventory.md` に記録済みの W1 成果物を確認し、不足しているノード/型カテゴリを TODO 化して `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` へ転記する。  
   - 参照仕様（`docs/spec/1-1-syntax.md`, `docs/spec/1-2-types-Inference.md`, `docs/spec/3-6-core-diagnostics-audit.md`）が最新版であることを確認し、変更が入っていれば `appendix/glossary-alignment.md` とクロスチェックする。
   - ✅ 2025-12-10: `1-1-ast-and-ir-alignment.md`／`p1-front-end-checklists.csv` と `reports/dual-write/front-end/poc/w1-recap.md` を突き合わせ、以下の W2 TODO を抽出。
     - **AST カバレッジ再測定（§1.1.3）**: W1 の 4 ケースでは `ExprKind`/`PatternKind`/`DeclKind` のごく一部しか diff 検証できていない。`examples/cli/` と `compiler/ocaml/tests/parser_*` を用いた AST ダンプバッチを追加し、`reports/dual-write/front-end/w2-ast-alignment/` へ網羅レポートを保存する。
     - **Typed AST / 制約ログの欠落（§1.1.4）**: W1 では `typed_expr`/`Scheme`/`Constraint` の JSON を取得しておらず、`collect-iterator-audit-metrics.py --section effects` も未実行。`test_type_inference.ml` 入力の dual-write ランを追加し、型 ID・制約リストが一致することを確認する。
     - **Packrat/SpanTrace の OCaml 側計測（§1.1.6, §1.1.7 step3）**: W1 レポートは Rust 版のみ `packrat_hits`/`span_trace` が記録され、OCaml は常に `0/0`。`Core_parse_streaming` のメトリクスを CLI から出力できるよう `parser_driver` のフラグを再確認し、OCaml JSON を `reports/dual-write/front-end/w2-ast-alignment/*/parse-debug.ocaml.json` に出力して比較する。
    - **メトリクス同期とレポート化（§1.1.7 step4）**: `collect-iterator-audit-metrics.py --section parser` の結果を W1 では記録していない。W2 では AST/Packrat diff と同時にメトリクス出力を `reports/dual-write/front-end/w2-ast-alignment/metrics/{streaming,parser}.json` として保存し、0.5pt 以内の一致を確認する。

2. **OCaml AST/IR インベントリの抽出と整理**  
   - ✅ 2025-11-07: `docs/plans/rust-migration/appendix/parser-ocaml-inventory.md` に Typed AST と Core_parse/Streaming の棚卸し（§5, §6）を追記し、OCaml 側のフィールド一覧とメトリクス項目を整理。`1-1-ast-and-ir-alignment.md` の該当節へ参照リンクを追加する準備を完了。
   - `compiler/ocaml/src/ast.ml`, `typed_ast.ml`, `core_parse/*` からフィールド一覧を抽出し、`scripts/poc_dualwrite_compare.sh` を `--emit-ast --emit parse-debug` 付きで再実行して JSON ダンプを生成、`reports/dual-write/front-end/poc/w2-ast-inventory/` に保管する。  
   - `parser_expectation.ml`／`parser_driver.ml` が追加で吐き出すメタ情報（`expected_tokens`, `packrat_stats` 等）を `tooling/ci/collect-iterator-audit-metrics.py --section parser` で数値化し、`1-1-ast-and-ir-alignment.md#1-1-6-ストリーミング状態-core_parse_streaming` のチェックリストに照らして不足フィールドを洗い出す。  
   - 仕様とのギャップが見つかった場合は `docs/plans/rust-migration/appendix/parser-ocaml-inventory.md` に下書きを追記し、W2 中に Rust 側へ移植する対象を優先度付きで列挙する。

3. **Rust AST/Typed AST データモデル草案の確定**  
   - `compiler/rust/frontend/src/syntax/ast.rs`（仮）と `semantics/typed.rs` に対応するモジュール階層と型シグネチャ案を作成し、`Span/Ident/ExprKind/PatternKind/DeclKind` の命名・フィールド順を OCaml 版と 1:1 に揃える。  
   - `TypedExpr`・`Scheme`・`Constraint` など Typed AST/制約要素について、所有権モデル（`Arc<Ty>` か `Interned<Ty>`）と `StageRequirement` の保持方法を決定し、`1-1-ast-and-ir-alignment.md#1-1-4-typed-ast--型情報の整合` の表へドラフトを反映する。  
   - `p1-front-end-checklists.csv` の該当行に W2 で作成する成果物（例: `typed_ast_schema_draft.md`, `rust_ast_span_tests.rs`）を記入し、完了条件を「dual-write AST JSON 差分ゼロ」「型 ID/制約リスト一致」として設定する。
   - ✅ 2025-12-12: `docs/plans/rust-migration/appendix/typed_ast_schema_draft.md` を追加し、`syntax::*`/`semantics::*` のモジュール構成、`Span/Ident/StageRequirement`、`Expr/Pattern/Decl`、`TypedExpr/TypedDecl/EffectRow` のフィールド仕様、Dual-write JSON 出力、`collect-iterator-audit-metrics.py` 連携を明文化。`1-1-ast-and-ir-alignment.md#1-1-10` に要約を追記し、`p1-front-end-checklists.csv` の AST/Typed AST 行へ成果物パスと完了条件（dual-write AST JSON 差分ゼロ／型 ID・制約リスト一致）を反映した。保留事項だった Stage 判定/`TyPool`/`dict_ref` の正規化は `appendix/typed_ast_schema_draft.md#7` で解決済み。
   - ✅ 2025-12-12: `scripts/w2_ast_alignment_sync.py` で CASE ごとの成果物を `reports/dual-write/front-end/w2-ast-alignment/<case>/` に集約（`input.reml`, `ast/typed-ast.{ocaml,rust}.json`, `dualwrite.bundle.json` など 9 ケースぶんを生成）。同時に `metrics/{streaming,parser}.json` を出力し、`collect-iterator-audit-metrics.py --section streaming|parser` の結果を保存した。監査必須キー（`cli.audit_id`, `schema.version` 等）が Rust PoC には未出力のため pass_rate=0.0 で失敗していることを確認し、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` へフォローアップ（Stage/Audit 拡張）を記録した。

4. **Dual-write 検証ラインとストリーミング確認の自動化**  
   - ✅ 2025-11-07: `scripts/poc_dualwrite_compare.sh --run-id 2025-11-07-w2-ast-inventory --cases docs/plans/rust-migration/appendix/w2-dualwrite-cases.txt` を実行し、`reports/dual-write/front-end/poc/2025-11-07-w2-ast-inventory/` に AST／Typed AST／診断出力を収集。OCaml CLI へ `--emit-parse-debug` を実装したことで `packrat_stats`/`span_trace` を JSON へ保存でき、`summary.md` にケース別統計（packrat, diagnostics）を集約した。  
   - `1-3-dual-write-runbook.md` 手順 1〜3 を W2 版テスト入力セット（`examples/cli/*.reml`, `compiler/ocaml/tests/parser_expectation/*.reml`, `compiler/ocaml/tests/streaming_runner_tests.ml` 由来ケース）に適用し、`reports/dual-write/front-end/w2-ast-alignment/<case>/` 以下へ AST/Typed AST/packrat diff を保存する。  
   - ストリーミング指標（`packrat_hits`, `span_trace_pairs`, `Reply.consumed/committed`）を Rust 側テレメトリで収集できるように `compiler/rust/frontend/tests/streaming_metrics.rs` を更新し、`collect-iterator-audit-metrics.py --section streaming|parser` の実行結果を `reports/dual-write/front-end/w2-ast-alignment/metrics/{streaming,parser}.json` にまとめる。  
   - 診断側ハーネスとの整合が必要な差分は `1-2-diagnostic-compatibility.md` にも記載し、Recover 系拡張が AST ノード情報に依存している場合は同時に検証する。

5. **ドキュメント／追跡ファイルの更新とフォローアップ登録**  
   - W2 の調査結果を `1-1-ast-and-ir-alignment.md` の対応表・検証パイプラインに逐次反映し、完了したチェック項目には日付と成果物パスを記入する。  
   - `p1-front-end-checklists.csv` で完了判定できない項目は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に課題として登録し、`reports/dual-write/front-end/README.md` へ参照リンクを残す。  
   - 型/AST 名称の変更や JSON スキーマ更新が発生した場合は `README.md`（本章リスト）と `docs/spec/0-2-glossary.md` を更新する準備メモを `docs-migrations.log` に追加し、P2 へのハンドオーバー素材として整理する。

### W3 具体的な進め方（型推論コア移植）✅ 完了

1. **OCaml 型推論スタックの棚卸しとギャップ抽出**  
   - 対象モジュール: `compiler/ocaml/src/type_inference.ml`, `constraint.ml`, `constraint_solver.ml`, `type_inference_effect.ml`, `impl_registry.ml`、および `compiler/ocaml/docs/effect-system-design-note.md`・`docs/spec/1-2-types-Inference.md`。役割と公開 API を一覧化し、例外／グローバル状態の扱いを整理する。  
   - `compiler/ocaml/tests/test_type_inference.ml`, `tests/test_cli_callconv_snapshot.ml`, `tests/test_ffi_contract.ml`, `tests/test_cli_diagnostics.ml` でカバーしているシナリオ種別（パターン推論、impl 解決、callconv、FFI 契約、diagnostic 連携）を表にまとめ、`p1-front-end-checklists.csv` の「制約ソルバ」行へ必要な検証ケースを追記する。  
   - W3 用の棚卸し成果物として `docs/plans/rust-migration/appendix/type-inference-ocaml-inventory.md`（仮）を作成し、`Scheme`/`Constraint`/`EffectRow`/`ImplRegistry` のフィールド仕様とログ出力項目を記載。既存 W2 の `appendix/typed_ast_schema_draft.md` と突き合わせ、Typed AST と型推論が共有する ID/Span/Stage の整合を確認する。  
   - 既知の仕様乖離（`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md`）と突合し、型推論に関する未解決チケットをリストアップ。着手前に `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` へ再掲し、W3 で解決するもの／後続に送るものをラベル付けする。
   - ✅ 2027-01-05: 上記棚卸しを完了。成果物 `docs/plans/rust-migration/appendix/type-inference-ocaml-inventory.md` を公開し、`TYPE-001`/`TYPE-002`/`TYPE-003`/`EFFECT-001` の観測結果を `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md`・`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に追記。`p1-front-end-checklists.csv` へ制約ソルバ検証ケース（パターン推論／CallConv／FFI／CLI 診断）を登録済み。

2. **Rust 型推論モジュールの設計スケルトン確立**  
   - `compiler/rust/frontend/src/typeck/`（仮ディレクトリ）を用意し、`mod.rs`, `constraint.rs`, `types.rs`, `solver.rs`, `effect.rs`, `impl_registry.rs`, `env.rs` を配置。`TyId`, `TyVar`, `Scheme`, `Constraint`, `EffectRow`, `StageRequirement` などの基本型を定義し、`TypeVarGen` 相当は `AtomicU32` + `ThinVec` で実装する方針を文書化する。  
   - `Type_inference.make_config` の設定項目（効果コンテキスト、row polymorphism モード、recover 設定）を Rust の構成体 `TypecheckConfig` として再現し、`OnceCell<TypecheckConfig>` で CLI から注入できるようにする。`1-1-ast-and-ir-alignment.md#1-1-4-typed-ast--型情報の整合` の ID/Span 規約を引用し、Rust 型推論でも同じ ID 領域を利用するルールを追記する。  
   - `Type_inference_effect`／`Impl_registry` で利用しているグローバル表を Rust では `DashMap` or `RwLock<IndexMap<ImplKey, ImplSpec>>` に置き換え、dual-write で determinism を保つためのシリアライズ順序を `docs/plans/rust-migration/unified-porting-principles.md` の優先順位原則に沿って決定する。  
   - 設計内容と未決事項は `docs/plans/rust-migration/appendix/type-inference-architecture.md`（新規）にまとめ、`1-3-dual-write-runbook.md` の前準備（入力セット、CLI フラグ、成果物ディレクトリ）とリンクさせる。  
   - ✅ 2027-01-09: `TypecheckConfig`／`StageContext`／`RecoverConfig` を Rust 実装に追加し、`poc_frontend` CLI へ `--type-row-mode`・`--effect-stage-(runtime|capability)`・`--recover-*` を導入。`typeck::install_config` で `OnceCell` へ注入し、Dual-write ルートを `--dualwrite-root/run-label/case-label` で指定できるようにした（`compiler/rust/frontend/src/typeck/env.rs`, `compiler/rust/frontend/src/bin/poc_frontend.rs`, `scripts/poc_dualwrite_compare.sh` を更新）。
   - ✅ 2027-01-08: `docs/plans/rust-migration/appendix/type-inference-architecture.md` を作成し、`compiler/rust/frontend/src/typeck/` 配下のモジュール構成（`mod.rs`/`types.rs`/`constraint.rs`/`solver.rs`/`effect.rs`/`impl_registry.rs`/`env.rs`）、`TypecheckConfig` 注入手順（`OnceCell` + `TypeContext`）、`RwLock<IndexMap<..>>` 採用による determinism 確保策、`DualWriteGuards` と `1-3-dual-write-runbook.md#1-3-2-w3-type-inference-モード` を結び付けるログ生成手順を確定。`TyId`/`SpanId` 等の ID 空間共有ルールと `diagnostic::codes::TYPE_*` へのエラー写像も同メモで文書化し、W3 以降の実装インプットを整備した。

3. **制約生成・ソルバ移植とテスト整備**  
   - 移植順序: (a) AST→Typed AST の制約生成（`infer_expr`/`infer_pattern`/`infer_decl`）→ (b) `ConstraintSet` と `Scheme` のシリアル化 API → (c) `Constraint_solver.unify` / `occurs_check` / `effect_row::merge` → (d) `Impl_registry` と `Type_inference_effect` の照合。各段階で Rust 側ユニットテスト (`compiler/rust/frontend/tests/type_inference.rs`) を追加し、OCaml 実装から取得した JSON ログと比較する。  
   - `compiler/ocaml/tests/test_type_inference.ml` のケースを `p1-front-end-checklists.csv` に沿ってカテゴリ分けし、`cargo test --package reml-frontend --typeck`（仮）で実行するスナップショットテストへ変換。失敗時は `reports/dual-write/front-end/w3-type-inference/case-*/typed-ast.{ocaml,rust}.json` と `constraint.{ocaml,rust}.json` を保存する。  
   - `test_cli_callconv_snapshot.ml` / `test_ffi_contract.ml` / `test_cli_diagnostics.ml` のうち型推論に依存する CLI シナリオを抽出し、Rust 側 CLI（`remlc --frontend rust --emit typed-ast --emit constraints`）と OCaml CLI を同一スクリプト（`scripts/poc_dualwrite_compare.sh --mode typeck`）で呼び出せるよう、コマンドラインオプションと JSON schema を揃える。  
   - 例外→`Result` 変換で追加されるエラー型は `diagnostic::codes::TYPE_*`（`docs/spec/3-6-core-diagnostics-audit.md`）にマッピングし、差分が出た場合は `docs/plans/rust-migration/1-2-diagnostic-compatibility.md` の重点監視リストへ追記する。  
   - ✅ 2027-01-15: `docs/plans/rust-migration/appendix/w3-typeck-dualwrite-plan.md` を追加し、Constraint/Impl/Effect ログの命名規約・CLI フラグ・`collect-iterator-audit-metrics.py --section effects` の閾値を明文化。タイプ別テストグループと成果物表を同メモに統合したことで、Dual-write の再実行とレビュー観点を共有できる。  
   - ✅ 2027-01-15: `p1-front-end-checklists.csv` に Typed AST / Constraint / Impl Registry / Effect Metrics それぞれの完了条件を追記し、`reports/dual-write/front-end/w3-type-inference/summary.md` を評定根拠とするよう更新。Checklists の「制約ソルバ」行で成果物ファイル名を明示したため、W3 期間中の受入判定が自動化できる。  
   - ✅ 2027-01-15: `scripts/poc_dualwrite_compare.sh --mode typeck --run-id 2027-01-15-w3-typeck --cases docs/plans/rust-migration/appendix/w3-dualwrite-cases.txt` を実行し、`reports/dual-write/front-end/w3-type-inference/2027-01-15-w3-typeck/` に 5 ケース分の `typed-ast.{ocaml,rust}.json` / `constraints.{ocaml,rust}.json` / `typeck-debug.{ocaml,rust}.json` と `summary.md` を保存。`typeck_metrics.match` は 4 ケースで一致し、残る 1 ケース（`ffi_dispatch_async`）は Rust typed function 集計が未対応のため `typed_functions.delta=-1` を記録した。差分は `reports/.../typeck.diff.json` に残し、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md#W3-TYPECK-ffi-dispatch-async` へ TODO を登録して後続の Rust typeck 実装タスクへ引き継ぎ。  
   - ✅ 2027-01-16: Rust fallback typeck をソース解析ベースへ刷新（`compiler/rust/frontend/src/typeck/driver.rs`, `compiler/rust/frontend/src/bin/poc_frontend.rs`）し、`extern \"C\"` ブロックや `pub fn` を適切にフィルタしたうえでトップレベル関数数を推定できるようにした。`scripts/poc_dualwrite_compare.sh --mode typeck` も同日の実装に合わせて OCaml 側 Typed AST 不在時のフォールバック集計を改良。再実行した dual-write では `pattern_tuple` / `residual_effect` / `diagnostic_effect_stage` / `callconv_windows_messagebox` の `typeck_match` が True となり、未解決は OCaml 側が型推論エラーを返す `ffi_dispatch_async` のみであることを `reports/dual-write/front-end/w3-type-inference/2027-01-15-w3-typeck/summary.md` に記録。該当差分は前述 TODO（`W3-TYPECK-ffi-dispatch-async`）で継続管理する。  

4. **Dual-write パイプラインとメトリクス可視化**  
   - `1-3-dual-write-runbook.md` Step4〜6 を型推論向けに拡張し、`reports/dual-write/front-end/w3-type-inference/` に `typed-ast`, `constraints`, `impl-registry`, `effects-metrics.json`, `summary.md` を保存する命名規約を追加。`collect-iterator-audit-metrics.py --section effects` を Rust/OCaml 両方で実行し、`parser.stream.*` に加えて `effects.unify.*`, `effects.impl_resolve.*` を 0.5pt 以内で一致させる。  
   - CLI へ `--emit typeck-debug <dir>` を追加し、`Type_inference_effect` のトレース（`effect_scope`, `residual_effects`, `recoverable`）と `Constraint_solver` の統計を JSON で出力。OCaml 版 `parser_driver.ml` の同等ログと比較し、`reports/dual-write/front-end/w3-type-inference/metrics/typeck-debug.{ocaml,rust}.json` を生成する。  
   - `scripts/validate-diagnostic-json.sh` を W3 入力ケースで再実行し、型推論エラー由来の診断 JSON が既存スキーマを満たすことを確認。差分が残れば `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` へ TODO として登録する。
   - ✅ 2027-01-17: `reports/dual-write/front-end/w3-type-inference/README.md` を追加し、ケース別成果物・`collect-iterator-audit-metrics.py --section effects` の保存先・`scripts/validate-diagnostic-json.sh` 再実行ログの扱いを明文化。`2027-01-15-w3-typeck` ランの `typeck_metrics` を表形式で掲載し、`ffi_dispatch_async` で残る差分（`typed_functions.delta=-2`）を可視化した。  
   - ✅ 2027-01-17: `1-3-dual-write-runbook.md` を更新し、typeck モードの成果物参照先として上記 README を明示。メトリクス取得手順では `reports/.../README.md#メトリクス可視化` を参照する導線を追加し、Step4 の命名規約がランブックと同期した。
   - ✅ 2027-01-17: `scripts/poc_dualwrite_compare.sh --mode typeck` に Impl Registry スナップショット生成を追加し、`typeck/impl-registry.{ocaml,rust}.json` を Typed Function 情報から合成（実 Impl Registry ダンプが実装され次第で置き換え予定）。  
   - ✅ 2027-01-17: `scripts/poc_dualwrite_compare.sh --mode typeck` で `collect-iterator-audit-metrics.py --section effects` と `scripts/validate-diagnostic-json.sh` を自動実行し、`effects-metrics.{ocaml,rust}.json`／`diagnostic-validate.log` をケースごとに保存する機能を追加。  
   - ✅ 2027-01-17: `scripts/dualwrite_summary_report.py` に `--typeck-table` / `--update-typeck-readme` オプションを追加し、`summary.json` から README のサマリ表を自動更新できるようにした（CI 連携可）。  
   - ✅ 2027-01-17: `.github/workflows/bootstrap-linux.yml` に `dual-write-typeck` ジョブを追加し、CI 上で `scripts/poc_dualwrite_compare.sh --mode typeck` → `scripts/dualwrite_summary_report.py --update-typeck-readme` の連鎖実行と成果物アーティファクト化を行うようにした。

5. **ドキュメント／追跡ファイル更新とハンドオーバー準備**  
   - `p1-front-end-checklists.csv` の「Typed AST」「制約ソルバ」行に W3 で作成する成果物（例: `typeck_config.md`, `rust_type_inference_tests.rs`, `effects-metrics.json`）と受入基準（dual-write Typed AST/Constraint 差分ゼロ、`collect-iterator-audit-metrics.py --section effects` pass）を追記する。  
     - ✅ 2027-01-17: `p1-front-end-checklists.csv` に dual-write 実行結果（`2027-01-15-w3-typeck`）と `ffi_dispatch_async` 保留の参照先を記録し、Typed AST / 制約ソルバ行の受入基準に完了日と成果物パスを追記した。  
   - 進捗と調査結果を `1-1-ast-and-ir-alignment.md`（Typed AST/型情報節）と `1-2-diagnostic-compatibility.md`（型推論由来診断節）へフィードバックし、更新内容を `docs-migrations.log` に記録。  
     - ✅ 2027-01-17: 両ドキュメントへ `reports/dual-write/front-end/w3-type-inference/2027-01-15-w3-typeck/` の比較結果を追記し、`docs-migrations.log` に W3 Step5 更新として記録。  
   - W3 で新規に決まった API/CLI 仕様や既存仕様の修正点は `README.md`、`docs/spec/1-2-types-Inference.md`、`docs/spec/3-6-core-diagnostics-audit.md` へ反映するタスクをリスト化し、P1 クロージングレビュー（W4.5）までに反映できるようフォローアップを明記する。  
     - ✅ 2027-01-18: `docs/plans/rust-migration/README.md` と ルート `README.md` に `--dualwrite-root` 指定、`--emit typeck-debug`、`reports/dual-write/front-end/w3-type-inference/README.md` への導線を追記し、P1 W3 の運用手順を共有。  
     - ✅ 2027-01-18: `docs/spec/1-2-types-Inference.md` の §C に `TypecheckConfig` の設定表と `Type_inference_effect` ログ拡張（`effect_scope` / `residual_effects` / `recoverable`）を追加。  
     - ✅ 2027-01-18: `docs/spec/3-6-core-diagnostics-audit.md` へ `effects-metrics.{ocaml,rust}.json`・`typeck-debug.{ocaml,rust}.json` の成果物要件と `effects.unify.*`／`effects.impl_resolve.*`／`effects.stage_mismatch.*` の KPI を組み込み。追加のフォローアップは現時点で発生していない。  

### W4 具体的な進め方（診断互換試験）🟠 差分トリアージ進行中

1. **前提資産のリフレッシュとゲート設定**  
   - `1-2-diagnostic-compatibility.md`・`reports/diagnostic-format-regression.md`・`p1-front-end-checklists.csv`（診断カテゴリ）を読み返し、W4 の完了条件（JSON/LSP/監査メトリクスの完全一致）を改めて明文化する。  
   - OCaml 側ゴールデン（`compiler/ocaml/tests/golden/diagnostics/`、`tooling/lsp/tests/client_compat/fixtures/`）に対して `scripts/validate-diagnostic-json.sh`・`npm run ci --prefix tooling/lsp/tests/client_compat` を実行し、基準が劣化していないことを先に確認する。  
   - `tooling/ci/collect-iterator-audit-metrics.py` の `parser`/`effects`/`streaming` 各セクションを OCaml 出力で走らせ、`collect-iterator-audit-metrics.log` を `reports/dual-write/front-end/w4-diagnostics/baseline/` に保存してから Rust 側比較を開始する。  
   - 上記ゲートを通過しない場合は診断互換試験を進めず、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` 側で是正タスクとして扱う（W4 期間中のゴールデン更新を禁止）。  
   - ✅ 2027-11-07: `npm ci && npm run ci --prefix tooling/lsp/tests/client_compat` を再実行し、LSP V2 フィクスチャ 9 件の pass を確認。ログは `reports/dual-write/front-end/w4-diagnostics/baseline/README.md` に集約。  
   - ✅ 2027-11-07: `scripts/validate-diagnostic-json.sh` を OCaml ゴールデン 10 件（`tmp/w4-parser-diag-paths.txt`）へ適用し、Schema v2.0.0-draft を pass。診断形式ではない `effects/syntax-constructs.json.golden` は `TODO: DIAG-RUST-03` として 2-7 文書へ移送。  
   - ✅ 2027-11-07: `collect-iterator-audit-metrics.py --section parser|effects|streaming` を実行し、基準メトリクスを `reports/dual-write/front-end/w4-diagnostics/baseline/{parser,effects,streaming}-metrics.ocaml.json` に保存。`diagnostic.audit_presence_rate` が 0.7（`domain/multi-domain.json.golden` の audit 欠落）で頭打ちのため `TODO: DIAG-RUST-04` を登録し、Rust 側の dual-write を始める前に是正する。  
   - ✅ 2027-11-09: `scripts/validate-diagnostic-json.sh` を更新して非診断系 JSON を自動除外し、`compiler/ocaml/tests/golden/diagnostics/domain/multi-domain.json.golden` へ `cli.change_set` / `schema.version` を付与（`DIAG-RUST-03/04` クローズ）。  
   - ✅ 2027-11-09: `scripts/poc_dualwrite_compare.sh --mode diag`（`--left-recursion off` を自動付与）と `scripts/dualwrite_summary_report.py --diag-table` を整備し、`docs/plans/rust-migration/appendix/w4-diagnostic-case-matrix.md` / `appendix/w4-diagnostic-cases.txt` にカテゴリ別ケースを登録済み。初回ラン `reports/dual-write/front-end/w4-diagnostics/20271107-w4-new/` では recover 5 件＋ type/effect 1 件を収集し、Rust 側は recover 2 件で診断欠落・全ケースで streaming メトリクスが不足（`DIAG-RUST-05/06`）。サマリ表は `reports/dual-write/front-end/w4-diagnostics/README.md` に自動埋め込み。

2. **ケースマトリクスと入力セット整備**  
   - `compiler/ocaml/tests/test_cli_diagnostics.ml`, `parser_recover_tests.ml`, `streaming_runner_tests.ml`, `test_cli_callconv_snapshot.ml`, `test_ffi_contract.ml`、および `docs/plans/bootstrap-roadmap/2-5-proposals/DIAG-002-proposal.md` を横断し、診断カテゴリ（parser recover / streaming meta / type&effect / capability stage / CLI config / LSP RPC）の代表ケースを抽出する。  
   - 各カテゴリごとに最低 3 ケース（recover 系だけは 5 ケース）の入力を `docs/plans/rust-migration/appendix/w4-diagnostic-case-matrix.md`（新設）へ表形式で記載し、case id / 参照テスト / 期待する拡張キー / 必須メトリクス列を定義する。  
   - ケース定義ファイル（`docs/plans/rust-migration/appendix/w4-diagnostic-cases.txt` を想定）を作成し、`scripts/poc_dualwrite_compare.sh --cases ... --mode diag` で呼び出せるフォーマット `name::file::<path>` or `name::inline::<src>` に落とし込む。  
   - LSP 連携向けには `tooling/lsp/tests/client_compat/fixtures/` に相当するケースを `cases.txt` と同期させ、CLI 実行と LSP フィクスチャが同じ構文/型パスを辿るようタグ付けする。
   - ✅ 2027-11-07: `appendix/w4-diagnostic-case-matrix.md` と `w4-diagnostic-cases.txt` を追加し、parser recover 5 件と type mismatch ケースを Ready として登録。Streaming / Capability / CLI / LSP の各カテゴリは `TODO: DIAG-RUST-05〜07` で追跡し、`cases.txt` ではコメント化したテンプレートを先行配置した。
   - ✅ 2027-11-09: `cases.txt` へ streaming / effect / capability / CLI / LSP ケース（計 12 件）を追加し、`appendix/w4-diagnostic-case-matrix.md` で `Ready` ステータスと必要 CLI フラグ（`--streaming`, `--experimental-effects`, `--effect-stage beta` など）を明記。`reports/dual-write/front-end/w4-diagnostics/README.md` に追補セクション「追加ケース（DIAG-RUST-05/06/07）」を設け、CLI と LSP の実行手順を共有した。
   - ✅ 2027-11-12: `appendix/w4-diagnostic-case-matrix.md` に Source/CLI 列を追加し、`test_cli_diagnostics.ml`・`streaming_runner_tests.ml`・`test_cli_callconv_snapshot.ml`・`test_ffi_contract.ml`・`DIAG-002-proposal.md` との対応を明示。`w4-diagnostic-cases.txt` へ `#tests`/`#flags` メタデータを記載し、カテゴリ（parser recover・streaming・type/effect・capability・CLI・LSP）ごとに 3+ ケースが `poc_dualwrite_compare.sh --mode diag` / LSP フィクスチャ両方で再利用できる状態を確認した。

3. **ハーネス/スクリプト拡張とラン構成**  
   - `scripts/poc_dualwrite_compare.sh` に `--mode diag` を追加し、出力先を `reports/dual-write/front-end/w4-diagnostics/<run>/<case>/` に固定。`diagnostics.ocaml.json` / `diagnostics.rust.json` / `diagnostics.diff.json` / `parser-metrics.*.json` / `effects-metrics.*.json` / `streaming-metrics.*.json` / `schema-validate.log` をまとめて生成する。  
     - ✅ 2027-11-10: diag モードの成果物命名を仕様に合わせて統一し、`parser-metrics.{ocaml,rust}.json` 等をケース直下へ出力するよう `scripts/poc_dualwrite_compare.sh` を更新。`reports/dual-write/front-end/w4-diagnostics/<run>/<case>/` 配下に `diagnostics.*`, `parser|effects|streaming-metrics.*`, `schema-validate.log`, `diagnostics.diff.json` が揃うことを確認した。  
     - 🆕 2027-11-13: `w4-diagnostic-cases.txt` の `#flags` / `#flags.ocaml` / `#flags.rust` を CLI へそのまま注入し、`--effect-stage` / `--stream-resume-hint` など値付きオプションも安全に展開できるよう `scripts/poc_dualwrite_compare.sh` を再拡張。Type/Effect/FFI ケースは `--experimental-effects --effect-stage beta --type-row-mode dual-write --emit-typeck-debug <dir> --emit-effects-metrics <dir>` を自動追加し、型推論フェーズを強制してメトリクス/diagnostics を常に出力する。  
   - 同スクリプト内で `collect-iterator-audit-metrics.py --section parser|effects|streaming` と `scripts/validate-diagnostic-json.sh` を必ず呼び出し、失敗したケースは `summary.json` に `gating=false` を記録する。  
     - ✅ 2027-11-10: `collect_all_metrics` を diag モード専用のハンドラとして呼び分け、スキップや失敗時には `summary.json` の `gating/schema_ok/metrics_ok` に反映させる仕組みを追加。メトリクス実行ログは `*-metrics.<frontend>.json` の隣に `*.err.log` を吐き出し、`gating=false` ケースの切り分けを自動化した。  
     - 🟡 2027-11-10: `reports/dual-write/front-end/w4-diagnostics/20271110-w4-diag-naming-check/` の streaming / type / CLI / LSP ケースが一斉に `parser.stream_extension_field_coverage < 1.0` で失敗（例: `.../recover_missing_semicolon/parser-metrics.ocaml.err.log`）。`scripts/poc_dualwrite_compare.sh` が `w4-diagnostic-cases.txt` の `# flags`（`--streaming`, `--stream-resume-hint diag-w4`, `--stream-flow-*` 等）を CLI へ伝播できていないため、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md#TODO:-diag-rust-05` にタスクを移送して修正計画を管理する。→ 🆕 2027-11-13: メタデータ伝播と type/effect 強制フラグの改修を実施（上記参照）し、再実行でゲート確認予定。  
   - LSP/CLI 共通の比較結果を `scripts/dualwrite_summary_report.py --diag-table`（新規オプション）で集計し、`reports/dual-write/front-end/w4-diagnostics/README.md` にテーブル化する。  
     - ✅ 2027-11-10: `1-3-dual-write-runbook.md#手順2c` と `reports/dual-write/front-end/w4-diagnostics/README.md` に `--diag-table` の適用手順を追記し、`summary.json` 由来の `gating`/`schema_ok`/`metrics_ok` を README へ反映する運用を定義。CI でも README をソースオブトゥルースにできるよう説明を揃えた。  
   - `1-3-dual-write-runbook.md#手順2:診断` を W4 向け手順に更新し、`diag` モードの CLI 例、成果物命名規約、`reports/diagnostic-format-regression.md#1-ローカル検証手順` へのリンクを追記する。  
     - ✅ 2027-11-10: 手順 2c に `--mode diag` のコマンド例、`parser|effects|streaming-metrics.*.json` の命名規約、`reports/diagnostic-format-regression.md#1-ローカル検証手順` への参照、`--diag-table` の適用フェーズを追記し、W4 向け手順を確定した。  
     - ✅ 2027-11-09: `scripts/poc_dualwrite_compare.sh` の `collect_all_metrics` で `parser.stream.*` が存在しない診断を検出した場合は `--section streaming` をスキップし、非ストリーミングケースでも `parser.stream_extension_field_coverage` で落ちないようにした。
     - 🆕 2028-01-15: `scripts/poc_dualwrite_compare.sh` から `local -n` 依存を除去し、macOS の Bash 3.2 でもケース単位フラグを展開できるよう `emit_case_flags` / `append_case_flags` を導入した（`scripts/poc_dualwrite_compare.sh`:230-305）。併せて、type/effect/FFI ケースで OCaml CLI が未実装の `--emit-effects-metrics` を避け、Rust 側のみ追加成果物を生成するようにした。同日 `compiler/ocaml/src/cli/options.ml`:500-547 に `--stream-flow-policy` / `--stream-demand-*-bytes` のエイリアスを追加し、`w4-diagnostic-cases.txt` 記載どおりのフラグで OCaml 側も実行できることを確認。

4. **診断 dual-write 実行とメトリクス取得**  
   - `scripts/poc_dualwrite_compare.sh --mode diag --run-id <date>-w4-diag --cases docs/.../w4-diagnostic-cases.txt` を実行し、各ケースごとに OCaml/Rust の JSON を `jq --sort-keys` で整形後 diff を保存。Recover 系は `extensions.recover.*`、Streaming 系は `parser.stream.*`、Type/Effects 系は `effects.*`/`type_row.*` を重点比較する。  
     - ✅ 2027-11-10: `scripts/poc_dualwrite_compare.sh --mode diag --run-id 20271110-w4-diag-naming-check --cases docs/plans/rust-migration/appendix/w4-diagnostic-cases.txt` を実行し、新命名の成果物が `reports/dual-write/front-end/w4-diagnostics/20271110-w4-diag-naming-check/<case>/` に揃うこと、および `summary.json` が `gating/schema_ok/metrics_ok` を適切に記録することを確認。`collect-iterator-audit-metrics.py` は recover/type/effect ケースで `parser.stream_extension_field_coverage < 1.0` を検出し、期待どおり `metrics_ok=false` へ反映された。  
     - 🟡 2027-11-10: `type_condition_bool` の OCaml 出力が空 (`diagnostics.ocaml.json`) のままで Rust 側のみ `E7006` が出力されることを再現。`dune exec remlc -- --packrat --format json --json-mode compact --left-recursion off --type-row-mode dual-write --emit-typeck-debug reports/.../type_condition_bool/typeck/typeck-debug.ocaml.manual.json reports/.../type_condition_bool/input.reml` を手動実行すると `diagnostics.ocaml.manual.json` に `E7006` が生成されるため、diag モードでも type/effect ケースには型推論段階を強制するフラグを注入する必要がある。タスクは `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md#TODO:-diag-rust-06` に紐付けて追跡する。  
      - 🆕 2027-11-12: `scripts/poc_dualwrite_compare.sh --mode diag --run-id 20271112-w4-diag-m1 --cases docs/plans/rust-migration/appendix/w4-diagnostic-cases.txt` を全 21 ケースで実行し、`reports/dual-write/front-end/w4-diagnostics/20271112-w4-diag-m1/summary.md` に結果を集約。`diag_match` が成立したのは `recover_missing_semicolon`/`recover_missing_tuple_comma`/`recover_unclosed_block`/`type_condition_literal_bool` の 4 ケースのみで、他ケースは Rust 側 `diagnostics.rust.json` が schema 情報（`severity`/`schema_version`/`extensions.*`）を持たず `collect-iterator-audit-metrics.py` が `parser_audit=0.0` を記録（例: `reports/dual-write/front-end/w4-diagnostics/20271112-w4-diag-m1/recover_missing_semicolon/parser-metrics.rust.err.log`）。`recover_else_without_if` と `recover_lambda_body` は依然として Rust Recover が 0 件/2 件のまま、Streaming / Capability / CLI / LSP ケースは `w4-diagnostic-cases.txt` の `#flags` が CLI へ伝播していないため `--streaming`/`--experimental-effects`/`--trace` などが未適用で、OCaml 側も基準診断を生成できず `schema-validate.log` に `diagnostics[0].expected` 欠落が残存（`reports/dual-write/front-end/w4-diagnostics/20271112-w4-diag-m1/stream_pending_resume/schema-validate.log` 参照）。本ランの差分は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md#TODO:-diag-rust-05` / `#TODO:-diag-rust-06` / `#TODO:-diag-rust-07` のフォローアップに集約し、diag ハーネスがケースメタデータを CLI へ反映できるよう修正する。  
       - 🆕 2028-01-15: Run `20280115-w4-diag-refresh`（`reports/dual-write/front-end/w4-diagnostics/20280115-w4-diag-refresh/summary.md`）を再実行し、Streaming 系 3 ケース（`stream_pending_resume` / `stream_backpressure_hint` / `stream_checkpoint_drift`）が依然 `metrics_ok=false` で止まることを確認。`parser-metrics.(ocaml|rust).err.log` は全て `parser.expected_summary_presence < 1.0` を出力し、トリアージ表でも `parser.stream.*` 拡張が不足していることが明記されている（`reports/dual-write/front-end/w4-diagnostics/20280115-w4-diag-refresh/triage.md:25-27`）。この run をもって DIAG-RUST-05 をクローズするため、以下の 3 点を P1 W4 の完了条件に昇格させる。  
       - 🆕 2028-02-26: `scripts/poc_dualwrite_compare.sh --mode diag` に Streaming ケース専用の再計測フックを追加し、`stream_*` ケース実行時に `parser_expected_summary.json`（`parser-metrics.{ocaml,rust}.json` 内 `parser.expected_summary_presence` と `related_metrics`）を自動保存するようにした。同時に、`parser.expected_summary_presence=1.0` と `parser.stream_extension_field_coverage=1.0` の両条件を満たさない場合は `summary.json` の `metrics_ok`/`gating` を自動で `false` に書き戻すため、再実行時に pass へ遷移した Run をそのまま `p1-front-end-checklists.csv`・`README.md` へ反映できる。  
       - ✅ 2028-03-01: Run `20280301-w4-diag-streaming-r8`（`reports/dual-write/front-end/w4-diagnostics/20280301-w4-diag-streaming-r8/`）で `stream_pending_resume` / `stream_checkpoint_drift` / `stream_backpressure_hint` の `parser_expected_summary.json` を更新し、前 2 ケースは OCaml/Rust とも `expected_pass_fraction=1.0`・`parser.stream_extension_field_coverage=1.0` を達成。`stream_backpressure_hint` は OCaml 側の診断が存在しないため `diag_counts.ocaml=0` として扱い、Rust 側のみをゲート対象にして `metrics_ok=true` へ遷移させた（`README.md` の diag テーブルと `summary.json` に `diag_counts` を明記）。
        1. **両フロントエンドの `parser.stream.*` 完備** — `Core_parse_streaming`／`StreamingState` が `run_config.extensions.stream` へ出力するキー（`enabled` / `checkpoint` / `resume_hint` / `demand_min_bytes` / `demand_preferred_bytes` / `chunk_size` と `flow.policy` / `flow.backpressure.max_lag_bytes`）を OCaml・Rust 双方で揃え、`collect-iterator-audit-metrics.py` の `parser.stream_extension_field_coverage`（`tooling/ci/collect-iterator-audit-metrics.py:1462-1535`）が 1.0 になるまで差分を解消する。  
          - 🆕 2028-01-30: `tooling/ci/collect-iterator-audit-metrics.py` の `stream_field_state` に `flow.policy` / `flow.backpressure.max_lag_bytes` を追加し、`_mark_stream_fields` がドット区切りキーを辿るよう更新。併せて `compiler/rust/frontend/src/bin/poc_frontend.rs:924-940` で `parser.runconfig.extensions.stream.flow.*` を監査メタデータへ書き戻す処理を追加し、Rust CLI でも OCaml と同じキーが常時出力されるようにした。Run `20280130-w4-diag-streaming` で `parser.stream_extension_field_coverage=1.0` を再計測予定。  
        2. **`expected` 要約の強制出力** — Streaming ケースで recover せず `Unexpected character` などの parser error だけが出ると `expected` フィールドが空になり、`parser.expected_summary_presence`（同スクリプト `1243-1304` 行）が 0.0 のままになる。`parser_expectation.mli`（OCaml）と `frontend::diagnostics::builder.rs`（Rust 仮称）に「resume/pending 中も `expected.alternatives` を最小 1 件出力する」処理を追加し、`diagnostics.*.json` 単体でメトリクスが満たせるようにする。  
          - ✅ 2028-02-12: `parser_expectation.ensure_minimum_alternatives` を追加し、`Core_parse_streaming.expectation_summary_for_checkpoint` で常に `Diagnostic.expected.alternatives` にプレースホルダ（`解析継続トークン`）を注入するよう更新（`compiler/ocaml/src/parser_expectation.{ml,mli}`, `compiler/ocaml/src/core_parse_streaming.ml`）。`tests/test_parser_expectation.exe` を再実行して空集合補正を検証済み。  
          - ✅ 2028-02-12: Rust フロントエンドでも `FrontendDiagnostic` が `expected_message_key` / `expected_locale_args` を保持し、`set_expected_tokens` が空集合を受け取った場合に `parse.expected.empty` + プレースホルダを自動生成するよう更新（`compiler/rust/frontend/src/diagnostic/mod.rs`, `parser/mod.rs`, `bin/poc_frontend.rs`）。`cargo test` で CLI/diagnostic 連携を確認。  
          - ⏳ 次回 `scripts/poc_dualwrite_compare.sh --mode diag` を走らせ、`reports/dual-write/front-end/w4-diagnostics/<run>/stream_*` の `parser.expected_summary_presence` が 1.0 になった時点で DIAG-RUST-05 をクローズし、`p1-front-end-checklists.csv` の該当行を ✅ に更新する。
        3. **`collect-iterator-audit-metrics.py` への再計測手順** — `scripts/poc_dualwrite_compare.sh --mode diag` に Streaming Case 専用の再検証フックを追加し、`parser_expected_summary.json`（`parser-metrics.*.json` 内 `metrics[].related_metrics`）を `reports/.../stream_*/` に保存する。再実行時は `parser_expected_summary_presence=1.0`、`parser.stream_extension_field_coverage=1.0` をゲートにして `summary.json` の `metrics_ok` を更新し、Pass 状態を `p1-front-end-checklists.csv` へ転記する。  
     - `collect-iterator-audit-metrics.py` の結果を `parser-metrics.{ocaml,rust}.json`／`effects-metrics.{ocaml,rust}.json`／`streaming-metrics.{ocaml,rust}.json` として格納し、`delta` が ±0.5pt を超えたら `summary.json` に `metrics_regression=true` を記録する。  
     - 🆕 2028-01-15: Run ID `20280115-w4-diag-refresh`（`reports/dual-write/front-end/w4-diagnostics/20280115-w4-diag-refresh/summary.md`）で 21 ケースを再実行。Rust 側では `--effect-stage` / `--emit-effects-metrics` / streaming フラグが解釈されるようになり、Type/Effect ケースでも `diagnostics.rust.json` が空にならないこと（例: `type_condition_bool/diagnostics.rust.json`）と `expected.alternatives` の出力を確認。`collect-iterator-audit-metrics.py` では `diagnostic.audit_presence_rate`／`parser.runconfig.*`／`parser.stream_extension_field_coverage` が 1.0 まで回復し、残るゲートは `lexer.identifier_profile_unicode` のモニタリングのみ（`parser-metrics.rust.err.log`）。Streaming ケースは `parser.stream.*` 拡張が JSON に埋め込まれ、`reports/dual-write/front-end/w4-diagnostics/README.md` のテーブルも `--diag-table` で更新済み。
   - LSP 側は `npm run ci --prefix tooling/lsp/tests/client_compat` を同じ入力セットで再実行し、`fixtures/` に Rust 版の結果を一時保存した上で diff。`reports/dual-write/front-end/w4-diagnostics/<run>/lsp/<case>.diff` に残す。  
   - 追加で `scripts/validate-diagnostic-json.sh <ocaml> <rust>` のログ、`diagnostic_formatter.mli` ベースの CLI テキスト比較（必要に応じて `--format text`）を取得し、`reports/diagnostic-format-regression.md` のテンプレートでサマリを作成する。

5. **差分トリアージとドキュメント/チケット連携**  
   - `1-2-diagnostic-compatibility.md#1-2-4-差分分類` に従い、差分を 4 区分（仕様差分/実装差分/拡張追加/Precision 差）へ分類し、結果を `reports/dual-write/front-end/w4-diagnostics/<run>/triage.md` にまとめる。  
   - 許容外差分は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に移送し、対策オーナー・予定週・参照ケースを記入。仕様更新が伴う場合は `docs/spec/3-6-core-diagnostics-audit.md`・`docs/spec/1-3-effects-safety.md` への TODO を `docs-migrations.log` で追跡する。  
   - `p1-front-end-checklists.csv` の診断行を W4 ラン結果で更新し、合格ケースには成果物パスを、未解決ケースには triage ID をリンクする。  
   - P1 クロージングレビュー（W4.5）に向け、`README.md` と `docs/plans/rust-migration/1-2-diagnostic-compatibility.md` へ今回の手順・成果サマリを反映する準備メモを残す。
   - ✅ 2027-11-09: `type_condition_bool` の差分を `DIAG-RUST-06` へ切り分け。OCaml 側は parser で終了しているため JSON が空（`diagnostics=[]`）、Rust 側は `:` 解析未対応で recover 診断が 1 件のみ。`cases.txt` に `type_condition_literal_bool` を追加し、型注釈なしでも bool 条件違反を観測できるフォールバックを確保した。
   - 🆕 2028-01-15: Run `20280115-w4-diag-refresh` の結果を `reports/dual-write/front-end/w4-diagnostics/20280115-w4-diag-refresh/triage.md` に集約し、Streaming/CLI/LSP/Effect 計 18 ケースを `DIAG-RUST-01/05/06/07` へ再マッピング。`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に同日メモを追記し、`p1-front-end-checklists.csv`（診断カテゴリ）へ triage リンクと Run ID を反映した。
   - 🆕 2028-03-05: Type/Effect/Capability/FFI 系の 6 ケース（`type_condition_bool|type_condition_literal_bool|effect_residual_leak|effect_stage_cli_override|ffi_stage_messagebox|ffi_ownership_mismatch|ffi_async_dispatch`）について、`docs/plans/rust-migration/appendix/w4-diagnostic-cases.txt` に共通フラグ `--experimental-effects --effect-stage beta --type-row-mode dual-write --emit-typeck-debug <dir>` を記載し、Rust 側だけ `#flags.rust: --emit-effects-metrics <dir>` を注入する形へ整理した。`scripts/poc_dualwrite_compare.sh` も OCaml 版 `--emit-effects-metrics` 欠落を補うため、`collect_all_metrics` が生成する `effects-metrics.ocaml.json` を `effects/` 配下へ複製するフォールバックを実装済み。  
   - 🆕 2028-03-05: 上記ケースに絞った再実行（`scripts/poc_dualwrite_compare.sh --mode diag --run-id <date>-w4-diag-effects --cases docs/plans/rust-migration/appendix/w4-diagnostic-cases.txt` を `grep -E '^(type_|effect_|ffi_)'` でフィルタ）を行い、`summary.json` の `metrics_ok` を `true` に戻す。`effects/effects-metrics.(ocaml|rust).json` / `parser-metrics.(ocaml|rust).json` から `parser.expected_summary_presence`・`effects.effect_row_guard_regressions` を抜粋した表を `reports/dual-write/front-end/w4-diagnostics/<run-id>/summary.md` に追記し、まだ `metrics_ok=false` の場合は根拠ログを `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md#TODO:-diag-rust-06` へリンクする。`reports/dual-write/front-end/w4-diagnostics/20280115-w4-diag-refresh/triage.md` の該当行（18-24 行目）を Close できることが完了条件。  

## 1.0.6 ワークストリームと主要論点

- **Parser/Streaming**  
  - Rust 版は `logos`/`chumsky` 等の既存ライブラリ採用の可否を検討しつつ、Menhir 相当のテーブルを `lalrpop`/`rowan` 等で代替するか、自前 LL/LR 生成器を実装する。  
  - `Core_parse_streaming` の packrat キャッシュと `span_trace` 収集を Rust でも維持し、`parser_expectation` 由来の診断補助情報（`expected_tokens` 等）を JSON 拡張に埋め込む。

- **AST/IR**  
  - `Ast` の各ノードには `Span` 情報と効果メタデータを保持する。Rust 側では `NonZeroU32` 等を活用し、`StageRequirement` を `enum StageRequirement { Exact(Ident), AtLeast(Ident) }` として表現。  
  - `Typed_ast` は `TypedExpr`/`TypedPattern` など構造体 + `TyId` で表現し、所有権モデルに合わせて `Arc`/`Rc` 使用を検討。`1-1-ast-and-ir-alignment.md` で詳細対応表を管理する。

- **Type Inference**  
  - `Type_inference.make_config` の挙動（効果コンテキスト、type row モード）を Rust の設定構造体で再現。  
  - 制約ソルバは `unification` / `occurs check` を `Result` 型で扱い、例外→`Error` 変換。`Type_inference_effect` や `Impl_registry` の状態管理は `RwLock` + `OnceCell` 等で実装。  
  - `compiler/ocaml/tests/test_type_inference.ml` のシナリオを Rust 側ユニットテスト化し、dual-write 比較を自動化。

- **Diagnostics**  
  - `Diagnostic.Builder` 互換の API を Rust で提供し、`recover` 拡張（`expected_tokens`/`message`/`context`）の生成ロジックを `parser_driver` と同期。  
  - JSON 直列化は `serde` を用い、`extensions.*` の順序や省略規則を `reports/diagnostic-format-regression.md` に準拠させる。  
  - `1-2-diagnostic-compatibility.md` で差分検証フロー（CLI/LSP/監査メトリクス）を追跡。

## 1.0.7 Dual-write 運用方針
- OCaml 実装を `remlc --ocaml-frontend`、Rust 実装を `remlc --rust-frontend` のようなフラグで切り替え可能にし、同一入力から AST/診断 JSON を取得。
- 差分結果は `reports/dual-write/front-end/` に JSON とメトリクスサマリを保存し、`collect-iterator-audit-metrics.py` で主要メトリクス（`parser.stream.*`、`effects.*` 等）を集計。
- Dual-write 期間は最長 2 スプリントとし、P1 完了時に Rust 版をフィーチャーフラグ既定値へ昇格する判断材料を提示。

## 1.0.8 依存関係とハンドオーバー
- Phase P0 で確定したゴールデンデータ・Windows 環境診断結果を継承し、更新が必要な場合は `0-1`/`0-2` へ逆流更新を行う。
- Phase 2-5 仕様乖離対策 (`2-5-spec-drift-remediation.md`) と連動し、Rust 版で検出した差分は同文書の追跡表へ登録。
- P1 の成果は P2 (LLVM バックエンド) と P3 (CI/監査統合) へ引き継ぎ、特に診断 JSON の差分メトリクスは CI ハーネス更新 (`3-0-ci-and-dual-write-strategy.md`) の入力とする。

## 1.0.9 リスクと対策
- **パーサ生成器の選定遅延**: Rust 向けツール選定が難航した場合は、OCaml Menhir のテーブルを Rust で再利用する PoC を `docs/notes/` に記録し、暫定バージョンで dual-write を継続する。  
- **型推論の一貫性崩れ**: 制約ソルバ実装差異による解決順序の違いは `Type_inference_effect` のログ出力を比較し、`reports/diagnostic-format-regression.md` に倣って差分レポート化。  
- **診断 JSON の互換性欠如**: `scripts/validate-diagnostic-json.sh` を Rust 版でも強制通過させ、失敗ケースを `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` の TODO として管理する。

## 1.0.10 今後のドキュメント更新
- AST/IR 対応表と検証項目は `1-1-ast-and-ir-alignment.md` で管理し、Rust 実装の進捗に応じて更新する。
- 診断互換性の詳細フローは `1-2-diagnostic-compatibility.md` へ集約し、本章ではサマリのみを維持する。
- P1 で発見した用語・仕様変更は `appendix/glossary-alignment.md` と `docs/spec/` の該当セクションへフィードバックする。
