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

### W1 具体的な進め方（Lexer/Parser スケルトン移植）

1. **準備と方針の再確認**  
   - P0 完了条件が満たされ、最新のゴールデンデータと Windows 監査結果を Rust 側でも参照できる状態を確認する。  
   - `unified-porting-principles.md` の優先順位原則と dual-write 前提をチームで再共有し、性能・安全性の許容範囲を明文化する。
   - ✅ 2025-03-09: `reports/dual-write/front-end/`（ゴールデン OCaml 出力／差分格納レイアウト）と `reports/toolchain/windows/20251106/*.json`（`setup-windows-toolchain.ps1`・`check-windows-bootstrap-env.ps1` の監査ログ）が Rust チームから直接参照可能であることを確認。`docs/plans/rust-migration/0-1-baseline-and-diff-assets.md` に記載された既知の欠落項目（`parser.core.rule.*` など）を引き継ぎつつ、`unified-porting-principles.md` §1 の優先順位と成功指標（診断キー一致 100%、性能回帰 ±10% 以内、Windows MSVC/GNU 5 連続成功）を本計画書の進捗ログとして再記録した。

2. **OCaml 実装の棚卸しと設計ノート整備**  
   - `compiler/ocaml/docs/parser_design.md` を読み、字句要素・演算子優先順位・構文カテゴリを洗い出して Rust 実装で必要となるトークン/ノード一覧を作成する。  
   - `parser_driver.ml` と `parser_expectation.ml` の役割分担（状態遷移、回復戦略、期待トークン生成）を整理し、抜け漏れをメモ化する。

3. **Rust フロントエンド骨格の用意**  
   - `compiler/rust/frontend/` 配下に Lexer・Parser・Streaming モジュールの雛形ファイルと `Cargo.toml` の該当セクションを追加し、依存クレート候補（`logos`/`chumsky` 等）の評価メモを添える。  
   - Span 型、トークン列挙、エラー種別、Recoverable 状態など共通で利用する基礎データ構造を Rust で宣言し、`docs/spec/1-1-syntax.md` に沿った命名と型域（`u32` オフセット等）を確認する。

4. **パーサ生成戦略と状態管理の設計**  
   - Menhir 相当の構文解析を Rust でどう再現するか（既存ジェネレータ活用か自前 LL/LR 実装か）を比較し、選定理由と PoC 計画を `docs/notes/` に記録する。  
   - `Core_parse` の state machine・入力ストリーム・エラー復旧フックを分解し、Rust の `ParserDriver`（仮）に移す責務を定義する。

5. **Packrat / span_trace 再現の設計**  
   - `Core_parse_streaming` の packrat キャッシュと `span_trace` 収集ロジックを調査し、Rust で利用するデータ構造（`IndexMap`/`HashMap` と寿命管理）を決定する。  
   - メトリクス項目（`parser.stream.*`）と連携するカウンタをどこで更新するか設計ノートに明記する。

6. **最小ケースでの dual-write 準備**  
   - `remlc --frontend {ocaml|rust}` 相当の切り替えインターフェースに必要な CLI フラグや build ターゲットを列挙し、未実装部分には TODO を残す。  
   - `reports/dual-write/front-end/` に W1 用の成果物ディレクトリ構成を作成し、AST/診断 diff とメトリクス出力を保存するコマンドシーケンスを `1-3-dual-write-runbook.md` の手順と照合する。

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
