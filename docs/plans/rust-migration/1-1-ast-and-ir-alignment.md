# 1.1 AST / IR 対応表と検証手順

P1 フロントエンド移植の中核である AST（抽象構文木）および Typed AST / 中間表現 (IR) の対応関係を整理し、Rust 実装で互換性を保証するためのチェックリストを定義する。OCaml 実装 (`compiler/ocaml/src/ast.ml`, `typed_ast.ml`, `core_parse/*.ml`) から抽出した構造を基に、Rust 側で再現すべきデータモデルと検証手順をまとめる。

## 1.1.1 目的
- OCaml 実装と Rust 実装で AST/IR のデータ構造を 1:1 で対応付け、仕様書 `docs/spec/1-1-syntax.md` `docs/spec/1-2-types-Inference.md` の契約を保持する。
- Dual-write 比較時にフィールド差異を特定しやすくし、差分原因（仕様差分・実装差分）を切り分ける。
- バックエンド (P2) や CI (P3) が期待するデータフォーマットを前提に、Rust 側の API 設計を固定する。

## 1.1.2 モジュール分類と対応方針

| 分類 | OCaml モジュール/型 | Rust 側設計の方針 | 備考 |
| --- | --- | --- | --- |
| AST (構文木) | `Ast` (`expr`, `pattern`, `decl` 等) | `crate::syntax::ast::*`（構造体 + `enum`）。`Span` は `u32` オフセット + `NonZeroU32` などを使用 | Span はバイトオフセットを維持し、`StageRequirement` を `enum` 化 |
| Typed AST | `Typed_ast` (`typed_expr`, `typed_pattern` 等) | `crate::semantics::typed::*`。型情報は `Arc<Ty>` を想定 | `dict_ref` 等の参照は `usize` インデックス化を検討 |
| 型表現 | `Types`, `Type_env` | `crate::semantics::types::*`。代数的データ型を `enum` で再現 | 仕様 `1-2` の命名を優先 |
| 制約/ソルバ | `Constraint`, `Constraint_solver` | `crate::semantics::constraints::*`。`Result` で例外を排除 | 発散検知 (`occurs_check_failed`) を同名エラーで保持 |
| パーサ状態 | `Core_parse.State`, `Core_parse_streaming` | `crate::frontend::streaming::*`。Packrat キャッシュを `FxHashMap` 等で再現 | `span_trace` / `packrat_stats` の型を揃える |
| 診断補助 | `Parser_expectation`, `Diagnostic.Builder` | `crate::frontend::diagnostics::*` | `expected_tokens` 生成ロジックを Rust へ移植 |

## 1.1.3 AST ノード対応チェックリスト

| カテゴリ | OCaml 定義 | Rust での対応指針 | 検証方法 |
| --- | --- | --- | --- |
| 位置情報 | `type span = { start; end_ }` | `struct Span { start: u32, end_: u32 }`。`end_` は排他的終端 | AST JSON ダンプ比較（`--emit-ast --format json`） |
| 識別子 | `type ident = { name; span }` | `struct Ident { name: SmolStr, span: Span }`。`SmolStr` 採用を検討 | Dual-write で名前/Span が一致するか |
| 式ノード | `expr_kind` バリアント | `enum ExprKind`。`Pipe`, `PerformCall`, `Lambda` 等を同名再現 | OCaml AST を JSON シリアライズして diff |
| パターン | `pattern_kind` | `enum PatternKind`。`PatConstructor` など optional payload を `Vec` で表現 | 型推論テスト (`test_type_inference.ml`) で束縛一覧を比較 |
| 宣言 | `decl_kind` | `enum DeclKind`。`effect`/`handler` など PoC 項目も含む | `examples/cli/*.reml` をパースして比較 |
| 効果参照 | `effect_reference` | `struct EffectRef { path: Option<ModulePath>, ... }` | 効果構文テストで JSON diff |

### JSON フォーマット方針
- OCaml AST ダンプ（既存テストの `*.json.golden`）を基準に、Rust AST も `serde` で同形式に整形する。
- フィールド順序はキー名アルファベット順で統一し、`null`/未使用フィールドは省略する。省略規則は P0 `0-1-baseline-and-diff-assets.md` の規約に合わせる。
- 違いが許容されるケース（例: `Span` の終端値が UTF-8 サロゲートにより ±1 変動）は `tolerance.md`（必要なら新設）で根拠を明示する。

## 1.1.4 Typed AST / 型情報の整合

- OCaml 側の詳細なフィールド棚卸しは `docs/plans/rust-migration/appendix/parser-ocaml-inventory.md` §5 を参照（W2 で更新）。
- W2 の dual-write 出力（Typed AST 含むサマリ）は `reports/dual-write/front-end/poc/2025-11-07-w2-ast-inventory/summary.md` を起点に確認できる（OCaml 側は `remlc --emit-parse-debug` で Packrat/SpanTrace を JSON 化済み）。

- *2025-12-02 追記*: `docs/spec/1-2-types-Inference.md` §3.4 の `where Numeric<T>` 制約に合わせ、`core_numeric` feature で導入したトレイトを Rust フロントエンドの型推論テストに組み込む。`compiler/rust/frontend/tests/type_numeric.rs`（新設予定）で `Numeric` 制約を含む推論ケースを `type_equal` / `effects` ゴールデンとして保存し、本章の対応表からリンクする。

- `typed_expr` / `typed_pattern` の構造を Rust で `struct TypedExpr { kind, ty, span, dict_refs }` とする。型 (`ty`) は `Arc<Ty>` または `Interned<Ty>` を利用し、共有コストを抑える。
- `constrained_scheme` は `struct Scheme { ty: Ty, constraints: Vec<Constraint> }` とし、汎化 (`forall`) 情報は `SmallVec` 等で表現する。
- `Impl_registry` はスレッドセーフなレジストリ (`RwLock<HashMap<ImplId, ImplEntry>>`) で保持し、dual-write ではレジストリ内容を JSON 化して比較。Rust 側で未解決の場合は `pending_impls` としてログへ。
- *2027-01-05 追記*: 型推論スタック全体の棚卸しは `docs/plans/rust-migration/appendix/type-inference-ocaml-inventory.md` に集約し、`TypedExpr`/`Scheme`/`Constraint`/`ImplRegistry` のフィールド仕様・ログ出力・テストシナリオ（`test_type_inference.ml`, `test_cli_callconv_snapshot.ml`, `test_ffi_contract.ml`, `test_cli_diagnostics.ml`）を一覧化した。Rust 側の `typeck` 実装は本章の対応表と同メモを併用して整合を確認する。
- 効果分析 (`Type_inference_effect`) のラベル付け (`call_tag_prefixes`, `is_known_ffi_call` 等) は `HashSet<String>` で実装し、Rust 移行後も `collect-iterator-audit-metrics.py --section effects` の計測値が一致することを確認。

## 1.1.5 制約スナップショットとの連携
- `docs/plans/rust-migration/appendix/w3-typeck-dualwrite-plan.md` で規定した `typed-ast.{ocaml,rust}.json` / `constraints.{ocaml,rust}.json` / `impl-registry.{ocaml,rust}.json` のスキーマを Typed AST 対応表の付録として扱う。フィールド順序は AST/Typed AST と同じ「ID→Span→Payload」を徹底し、`Constraint` が参照する `TyId` を `"ty_id": <u32>` で明示する。
- `ConstraintBuilder`（Rust）と `Constraint.new_constraint`（OCaml）の対応を `TyId`/`SpanId` ベースで照合し、Dual-write では `reports/dual-write/front-end/w3-type-inference/<case>/` に保存された JSON を `diff -u` で直接比較する。`extensions.type_row` / `extensions.effect_row` の順序差は許容せず、差分が出た場合は `appendix/w3-typeck-dualwrite-plan.md#2-制約生成とソルバ移植の粒度` のフォローアップ欄へ記録する。
- Impl Registry は `IndexMap<ImplKey, ImplSpec>`（Rust）と `Ordered_table`（OCaml）で順序を固定し、Dual-write JSON の `entries` 配列が完全一致することを完了条件とする。順序を壊す変更が発生した場合は `p1-front-end-checklists.csv` の新規行（Impl Registry determinism）で検知できる。
- `effects-metrics.{ocaml,rust}.json` と `typeck-debug.{ocaml,rust}.json` は診断互換性（`1-2-diagnostic-compatibility.md`）の検証対象でもあるため、ここでは ID／Span／StageRequirement の対応のみを追跡し、詳細な診断差分は診断計画へ委譲する。
- *2027-01-17 進捗*: `scripts/poc_dualwrite_compare.sh --mode typeck --run-id 2027-01-15-w3-typeck` を用いた比較で `typed-ast.{ocaml,rust}.json` / `constraints.{ocaml,rust}.json` / `impl-registry.{ocaml,rust}.json` は 5 ケース中 4 ケースで完全一致した。残る `ffi_dispatch_async` は OCaml 側が型推論エラーを返したため、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md#W3-TYPECK-ffi-dispatch-async` に差分を移送済み。測定結果は `reports/dual-write/front-end/w3-type-inference/2027-01-15-w3-typeck/summary.md` に記録し、`p1-front-end-checklists.csv` の Typed AST / 制約ソルバ行へ完了日を反映した。

## 1.1.6 ストリーミング状態 (`Core_parse_streaming`)

- OCaml 実装で計測しているメトリクスと構造の棚卸しは `docs/plans/rust-migration/appendix/parser-ocaml-inventory.md` §6 に集約した。
- OCaml/Rust 双方の Packrat/SpanTrace 出力は `reports/dual-write/front-end/poc/2025-11-07-w2-ast-inventory/*.{ocaml|rust}.parse-debug.json` に保存済み。`ocaml_packrat_queries`/`hits` は CLI `--packrat --emit-parse-debug` で実測値を取得できる。

- `packrat_cache` は OCaml 版の `Parser_expectation.Packrat.t` と同じキー構造（入力オフセット → 期待値/結果）を保持。Rust では `IndexMap<usize, CacheEntry>` を仮採用し、`order` を維持する。
- `span_trace_pairs`（refined span の履歴）は `Vec<(Option<String>, Span)>` として保存し、回収ロジックを Rust へ移植。`parser/streaming_runner_tests.ml` のゴールデンが差分ゼロであることが完了条件。
- `Core_parse.Reply` の状態（`consumed`, `committed`）は Rust でも `enum Reply { Success { consumed: bool, ... }, RecoverableError { ... } }` の形で再現し、診断との結合は `1-2-diagnostic-compatibility.md` の手順に従う。

### 1.1.6.1 Streaming 拡張フィールドと `expected` 要約（DIAG-RUST-05）

- Run `20280115-w4-diag-refresh` では Streaming ケース 3 件すべてが `parser.expected_summary_presence < 1.0` で失敗し、`parser-metrics.(ocaml|rust).err.log` が共通して同エラーを出力した（例: `reports/dual-write/front-end/w4-diagnostics/20280115-w4-diag-refresh/stream_pending_resume/parser-metrics.ocaml.err.log`）。トリアージ結果も同じ課題を指摘している（`reports/dual-write/front-end/w4-diagnostics/20280115-w4-diag-refresh/triage.md:25-27`）。  
- `collect-iterator-audit-metrics.py` は `run_config.extensions.stream` 内の 6 キー（`enabled` / `checkpoint` / `resume_hint` / `demand_min_bytes` / `demand_preferred_bytes` / `chunk_size`）と `flow.policy` / `flow.backpressure.max_lag_bytes` を監視し、欠落時に `parser.stream_extension_field_coverage` を 0 にする（`tooling/ci/collect-iterator-audit-metrics.py:1462-1535`）。`Core_parse_streaming.snapshot` と Rust `StreamingState::snapshot` の両方でこの構造を組み立て、`parse-debug` → `diagnostics.*.json` → `audit_metadata.parser.runconfig.extensions.stream.*` の経路で lossless に伝搬させる。  
- 同スクリプト `1243-1304` 行では `expected.alternatives` の有無を `parser.expected_summary_presence` として集計する。Streaming Pending/Resume のように recover が走らないケースでも `Parser_expectation`（OCaml）と Rust `frontend::diagnostics::builder` が最小 1 件の `expected` 候補を生成するようにし、`parser_expected_tokens_avg >= 1` を保証する。  
- 検証は `scripts/poc_dualwrite_compare.sh --mode diag --run-id <date>-w4-diag-refresh` を再実行し、`parser-metrics.{ocaml,rust}.json` で `parser.stream_extension_field_coverage` と `parser.expected_summary_presence` が両方 1.0 になったことを確認する。パス後は `p1-front-end-checklists.csv` の Streaming 行と `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md#TODO:-DIAG-RUST-05` を更新してクローズを宣言する。

### 1.1.6.2 `ExpectedTokenCollector` と Recover 列挙順（DIAG-RUST-01）

- OCaml 版は `parser_expectation.ml` の `collect_expected` → `dedup_and_sort` → `humanize` で Menhir の `Terminal`/`Nonterminal` から `{ keyword | token | class | rule }` を整列させている。Rust 版でも `ExpectedKind::{Keyword, Token, Class, Rule}` を `#[repr(u8)]` で定義し、`IndexSet<ExpectedToken>`（`ExpectedToken { kind, repr, span, source }`）に投入した後 `kind` 優先で安定ソートする。  
- `Keyword` は予約語文字列、`Token` は記号リテラル（`"{"`, `"=>"` など）、`Class` は `identifier`, `literal`, `effect` など抽象カテゴリ、`Rule` は文法非終端（`lambda`, `tuple` 等）の説明文を格納する。`humanize` のテンプレートは `docs/spec/1-1-syntax.md` の構文名に合わせ、`Rule` だけ sentence case、日本語説明には括弧で英語原語を添える。  
- OCaml 側の `Expected_token.equal` 相当判定は `message_key=parse.expected` + `Span` で比較するため、Rust 側でも `ExpectedTokenCollector::finalize()` は `(message_key, span.start, span.end)` をキーにして `BTreeMap` へ格納し、重複診断があれば後勝ちで `expected_tokens` を差し替える。`recover_lambda_body` で 2 件出力されている既知差分はこの段階で 1 件へ収束させる。  
- ハーネス側では `scripts/poc_dualwrite_compare.sh --mode diag --emit-expected-tokens <dir>` を必須化し、`reports/dual-write/front-end/w4-diagnostics/<run>/<case>/expected_tokens.{ocaml,rust}.json` を比較する。`expected_tokens.diff.json` が空、かつ `parser.expected_summary_presence=1.0` を満たすまで `p1-front-end-checklists.csv` の Parser Recover 行を `Ready + Pass` へ更新しない。  
- `collect-iterator-audit-metrics.py --section parser` で新たに `expected_tokens_match`（`true/false`）を算出し、`parser-metrics.err.log` へ差分キーを列挙する。Run 単位のログは `docs/plans/rust-migration/appendix/w4-diagnostic-case-matrix.md` にリンクし、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md#TODO:-DIAG-RUST-01` と同期させる。

## 1.1.7 検証パイプライン

1. **AST ダンプ比較**: `remlc --frontend ocaml|rust --emit-ast --format json` を dual-write 実行し、`jq --sort-keys` で整形して差分を出す。差分が生じた場合は `reports/dual-write/front-end/ast-diff-report.md` に結果を集約する。
2. **Typed AST 比較**: 型推論後の `TypedExpr` を JSON/バイナリでダンプし、`type_equal` の結果と併せて比較。ソルバの解決順による順序差は `sorted` 正規化後に比較。
3. **Packrat/Span Trace 検証**: ストリーミングテストを dual-write 実行し、`packrat_stats`（ヒット/ミス件数）と `span_trace` を比較。差分が 5%以上の場合は `1-0-front-end-transition.md` のリスク項目へエスカレーション。
   - Rust 側は `cargo run --quiet --bin poc_frontend -- --emit-parse-debug reports/dual-write/front-end/latest/parse-debug.rust.json <input.reml>` を実行し、生成された `parse_result.packrat_stats` を `tooling/ci/collect-iterator-audit-metrics.py --source <file>` へ渡して OCaml 版と同じメトリクス経路で検証する。
4. **メトリクス同期**: `collect-iterator-audit-metrics.py --section parser --require-success` を Rust 版に対して実行し、OCaml 版との差分が `0.5pt` 以内に収まるか確認。

## 1.1.8 既知の差分と対応

- **Stage Requirement 注釈**: OCaml 版は `StageRequirement_annot` を `ident` ベースで保持している。Rust 版では `StageIdentId` (interned) にする可能性があるため、比較時に名称を正規化するフィルタを実装する。
- **効果参照の正規化**: `Effect_profile.canonicalize_module_path` の挙動差（`::` プレフィックスの扱い）が Rust 標準ライブラリで変化する可能性がある。比較前に `lowercase + trim` をかけ、スタイル差を抑制。
- **浮動小数リテラル**: OCaml は文字列のまま保持するが、Rust で `f64` に直列化する場合丸め誤差が発生する。文字列保存を基本とし、評価は別フェーズ（P2）で扱う。
- **Span のエンコーディング**: Windows 環境で改行コード（CRLF）差分が出る場合は、`0-2-windows-toolchain-audit.md` で定義した行末正規化を適用する。

## 1.1.9 ドキュメント連携
- 本文で決めた対応表は、Rust 実装の `README`（予定）や `docs/spec/1-5-formal-grammar-bnf.md` の脚注更新時に参照する。
- 新しい用語・型名が発生した場合は `appendix/glossary-alignment.md` に追記し、`docs/spec/0-2-glossary.md` との整合を確認する。
- P1 のレビュー結果は `docs/plans/rust-migration/1-0-front-end-transition.md` へフィードバックし、必要なら `docs/notes/core-library-outline.md` などの関連ノートへリンクを追加する。

## 1.1.10 Rust AST / Typed AST データモデル草案（W2）

- `docs/plans/rust-migration/appendix/typed_ast_schema_draft.md` に W2 で確定した Rust データモデルを集約した。`syntax::*` と `semantics::*` のモジュール階層、`Span/Ident/StageRequirement`、`Expr/Pattern/Decl`、`TypedExpr/TypedDecl`、`Scheme/Constraint/EffectRow` のフィールド構成を OCaml 版と 1:1 で表形式に整理している。  
- AST ルート (`AstModule`) には `packrat_stats`・`span_trace` を保持し、Dual-write JSON (`reports/dual-write/front-end/w2-ast-alignment/<case>/ast.{ocaml,rust}.json`) と `collect-iterator-audit-metrics.py --section parser` の両方から参照できるようにした。  
- Typed AST は `TyId`/`SchemeId`/`ConstraintId` などインデックス化された ID を採用し、`typed_expr.dict_refs` や `typed_decl.scheme` を JSON の別テーブルとしてダンプする。`--section effects` で抽出する `effects.row.len` / `effects.dict_refs` はこのテーブルから算出する。  
- `StageRequirement` は `enum StageRequirement { Exact(IdentId), AtLeast(IdentId) }` で統一し、AST では構文レベルの stage 註釈のみを保持、実行 Stage 判定は Typed AST で追加される `EffectMeta { stage: CapabilityStage, residual: EffectRow }` に集約する（W2-AST-001 完了）。`TyPool` は `index_vec::IndexVec<TyId, TyKind>` で実装し、`NonZeroU32` の `TyId` と 1:1 で対応させることで割り当て課金とシリアル化コストを抑える（W2-AST-002）。`dict_ref` は `dict_ref_table: Vec<DictRefEntry>` と `dict_ref_ids: SmallVec<[DictRefId; 2]>` の二段構成で直列化し、JSON では `{ "dict_ref_table": [...], "typed_expr": { "dict_refs": [id,...] } }` という参照構造を採用する（W2-AST-003）。  
- 本草案をもって `p1-front-end-checklists.csv` の AST／Typed AST 行は「成果物: typed_ast_schema_draft.md」「完了条件: dual-write AST JSON 差分ゼロ／型 ID・制約リスト一致」として更新し、W2 の達成範囲を固定した。`reports/dual-write/front-end/w2-ast-alignment/metrics/{streaming,parser}.json` には 9 ケース分のメトリクス結果を保存しており、現状は Rust PoC で `audit.*` / `run_config.*` の必須キーが欠落しているため pass_rate=0.0 になっている（差分調査タスクは `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` へ移送済み）。

## 1.1.11 P2 連携メモ（W4.5 ハンドオーバー）

`1-0-front-end-transition.md#w4.5-p1-クロージングレビューp2-ハンドオーバー準備` で整理した判定に基づき、AST/IR 観点の受け渡し内容を以下の通り整理する。

| P2 計画書 | 必須入力 (P1 成果物) | 受け渡しメモ |
| --- | --- | --- |
| `2-0-llvm-backend-plan.md` | `reports/dual-write/front-end/w3-type-inference/2027-01-15-w3-typeck/{typed-ast,constraints,impl-registry}.{ocaml,rust}.json` | LLVM 側で MIR→IR 生成を検証する際のゴールデン。`typed_ast_schema_draft.md` とセットで保存し、`w3-typeck` Run ID を CI の `LLVM backend` ジョブで必須入力にする。 |
| `2-1-runtime-integration.md` | `reports/dual-write/front-end/w4-diagnostics/20280418-w4-diag-effects-r3/ffi_*/typeck` | `effect.stage.*` / `bridge.stage.*` 欠落を既知課題として受領。P2 では StageAuditPayload の Rust 実装から修正するため、Run ID・ケース名を §2.1.7 に記載する。 |
| `2-2-adapter-layer-guidelines.md` | `reports/dual-write/front-end/w4-diagnostics/20280410-w4-diag-streaming-r21/stream_*/{parse-debug,expected_tokens}.json` | Streaming Flow 設定（`flow.policy`, `flow.backpressure.max_lag_bytes`, `runconfig.extensions.stream.*`）と Packrat/SpanTrace をアダプタ層の要件に組み込む。 |

### ハンドオーバー手順
1. 上表の Run ID を `P1_W4.5_frontend_handover/ast-ir/` ディレクトリにコピーし、`README.md` へ表形式で再掲する。`poc_frontend` CLI の再現コマンド（`cmd.json`）を添付し、P2 が `scripts/poc_dualwrite_compare.sh --mode {ast,typeck,diag}` を即再実行できる状態にする。
2. `p1-front-end-checklists.csv` に `HandedOver` 列を追加し、AST／Typed AST／Streaming の各行へ `W4.5` と Run ID を記録する。`Pending(W4.5)` の行（Streaming/TypeEffect/CLI）は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` の該当 TODO とリンク。
3. `docs/spec/1-1-syntax.md` と `docs/spec/1-2-types-Inference.md` に P2 参照用脚注を追加し、「Rust バックエンド移行で参照する AST/Typed AST ゴールデンは `w3-type-inference/2027-01-15-w3-typeck`」である旨を記載（別タスク）。この脚注番号を本節にも併記する。
4. `docs/plans/rust-migration/overview.md` と `README.md` に「P1 W4.5 → P2 ハンドオーバー」として本節の表を引用し、P2 側ドキュメントから逆リンクできるようにする。

### 未解決ポイント（P2 フォローアップ）
- Streaming 系 `ExpectedTokenCollector`（DIAG-RUST-05）は Rust 実装が未完成のため、P2 ではアダプタ層側で RunConfig / Packrat 情報を保持しながら検証する必要がある。`collect-iterator-audit-metrics.py --section streaming` の `expected_tokens_match` を P2 の性能ゲートに追加する。
- Type/Effect/FFI（DIAG-RUST-06）は Stage/Audit JSON と `typeck/typeck-debug` 書き出しを P2 のランタイム統合着手条件にする。`2-1-runtime-integration.md` では Stage 判定ログ (`AuditEnvelope.metadata.bridge.*`) を Rust FFI 層で生成する前提でスケジューリングする。
- CLI/LSP RunConfig（DIAG-RUST-07）は `RunConfigBuilder` をアダプタ層・バックエンド双方で参照する必要があるため、P2 開始時点で CLI/LSP 仕様の diff を `w4-diagnostics/20280430-w4-diag-cli-lsp/README.md` から参照する。
