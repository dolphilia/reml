# 2-5 レビュー記録 — DIAG-002 Day1 調査

DIAG-002 の初期洗い出し結果を記録し、後続フェーズでの追跡に利用する。  
関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/DIAG-002-proposal.md`](./2-5-proposals/DIAG-002-proposal.md)

## 1. Diagnostic を直接構築している経路
| 種別 | ファイル:行 | 状態 | 想定対応 |
|------|-------------|------|----------|
| Legacy 変換 | `compiler/ocaml/src/diagnostic.ml:181` | `Diagnostic.Legacy.t` から `Diagnostic.t` をレコード直接構築。`audit = None` のまま返却され、`Legacy.audit_metadata` が空の場合は監査キーが欠落する。 | Week31 Day2 以降で `Diagnostic.Builder` 経由の移行パスを追加し、最低限 `Audit_envelope.empty_envelope` と `iso8601_timestamp` を強制する。既存のテストは Builder 経路へ切り替える。 |

## 2. 監査メタデータが不足する経路（`Diagnostic.Builder.create` → `Builder.build`）
| 優先度 | ファイル:行 | 出力チャネル | 現状 | 対応メモ |
|--------|-------------|--------------|--------|----------|
| 高 | `compiler/ocaml/src/llvm_gen/verify.ml:131` | `--verify-ir` 失敗時 (CLI) | `Builder.build` 直後の診断をそのまま `main.ml:597` から出力。`attach_audit` が呼ばれないため `cli.audit_id` / `cli.change_set` など `tooling/ci/collect-iterator-audit-metrics.py` が必須とするキーが欠落し、`ffi_bridge.audit_pass_rate` 集計で非準拠扱い。 | Day2 で `Verify.error_to_diagnostic` に `Diagnostic.set_audit_id` / `set_change_set` を注入するか、`main.ml` 側で再利用している `attach_audit` を適用する。 |
| 中 | `compiler/ocaml/src/diagnostic.ml:945` | `Parser_driver.process_lexer_error` | Builder 直後は監査メタデータが空だが、`main.ml:803` で `attach_audit` を通すため CLI/LSP 出力時点では `cli.audit_id` / `cli.change_set` が補完される。 | 現状維持でも仕様違反にはならないが、計測ログ用の `parser.*` 系キーを Builder 側で自動付与する改善案を検討。 |
| 中 | `compiler/ocaml/src/diagnostic.ml:950` | `Parser_driver.process_parser_error` | Lexer エラーと同じ挙動。`attach_audit` により最終的な監査キーは揃う。 | Parser 向けメタデータ自動化を Lexer と合わせて検討。 |
| 低 | `compiler/ocaml/tests/test_cli_diagnostics.ml:27` | CLI フォーマッタのゴールデン | テスト専用のダミー診断。監査キーが空のままのため、必須化後は `Diagnostic.set_audit_id` 等でフィクスチャを更新する必要がある。 | Day3 以降でゴールデン再生成。レビュー時に `REMLC_FIXED_TIMESTAMP` を考慮。 |

## 3. 補足メモ
- `main.ml:665-694` の Core IR / Codegen 例外、`main.ml:744-748` の型推論エラー、`main.ml:803-804` のパース失敗は `attach_audit` を経由しており、`cli.audit_id`・`cli.change_set` が付与される。
- `tooling/ci/collect-iterator-audit-metrics.py` は 14 件の audit メタデータキーを必須としている。High 優先度の経路から出力される診断は pass rate を 0.0 に固定する要因となるため、Phase 2-5 内での修正を優先する。*** End Patch*** End Patch

## 4. Legacy / シリアライズ整備 進捗（2025-11-02 更新）
- **監査キー補完**: Builder/Legacy 双方で `ensure_audit_id` / `ensure_change_set` を導入し、空値の場合は `phase2.5.audit.v1` テンプレート（CLI: `audit_id = "cli/" ^ build_id ^ "#" ^ sequence`、Legacy: `audit_id = "legacy-import/" ^ build_id`）を生成してから `Audit_envelope.has_required_keys` を通過させる。`missing` フィールドは必須キーが揃った段階で自動的に除去される（compiler/ocaml/src/diagnostic.ml:304-370）。
- **Audit_envelope 拡張**: `Audit_envelope.has_required_keys` を CLI 監査キー込みで再定義し、`missing_required_keys` を公開して検証・エラーメッセージ両方に利用できるようにした（compiler/ocaml/src/audit_envelope.ml:120-189）。
- **シリアライズ検証**: `Diagnostic_serialization.of_diagnostic` で必須キーと `timestamp` をチェックし、欠落時は `[diagnostic_serialization] …` を stderr に出力して `Invalid_argument` を送出する運用へ移行した（compiler/ocaml/src/diagnostic_serialization.ml:75-88）。
- **テスト/ログ**: `dune runtest`（compiler/ocaml）を再実行し、更新された診断ゴールデン（Typeclass/FFI/Effects）を整合させた。`tooling/ci/collect-iterator-audit-metrics.py` は不足フィールドを stderr に出力するようになり、`--require-success` 実行時のトラブルシューティングが容易になった。

## 5. `phase2.5.audit.v1` テンプレート実装後の検証（2025-11-06 更新）
- **CLI/テスト経路の統一**: `compiler/ocaml/src/main.ml` と `test_cli_diagnostics.ml` / `test_ffi_contract.ml` / `test_effect_residual.ml` を更新し、CLI 実行・ユニットテストいずれの経路でも `audit_id = "cli/<build_id>#<sequence>"` とテンプレート化された change-set を出力するようになった。  
- **ゴールデン更新**: Typeclass / FFI / Effects 系ゴールデン（診断 JSON・監査 JSONL）を再生成し、`bridge.audit_pass_rate`・`effect.handler_stack`・`typeclass.*` など必須メタデータが埋まっていることを確認。  
- **CI メトリクス**: `python3 tooling/ci/collect-iterator-audit-metrics.py --require-success` をローカルで実行し、`iterator.stage.audit_pass_rate`・`typeclass.dictionary_pass_rate`・`ffi_bridge.audit_pass_rate` がすべて 1.0 となることを確認（従来の `auto-*` / `legacy-*` プレースホルダによる欠落は解消済み）。  
- **残タスク**: LSP／Legacy 経路へのテンプレート適用手順と、`timestamp` 生成の最終的な責務分担（`Ptime` への移行可否）を別途整理し、監査チームとの合意を待つ。

## 6. Week31 Day4-5 テスト／ドキュメント反映ログ（2025-10-27）
- `scripts/validate-diagnostic-json.sh` を既定ディレクトリ（`compiler/ocaml/tests/golden/diagnostics`, `compiler/ocaml/tests/golden/audit`）で実行し、スキーマ違反がないことを確認。
- `python3 tooling/ci/collect-iterator-audit-metrics.py --require-success --source compiler/ocaml/tests/golden/diagnostics/effects/invalid-attribute.json.golden --source compiler/ocaml/tests/golden/diagnostics/effects/invalid-attribute-unknown-tag.json.golden --source compiler/ocaml/tests/golden/diagnostics/effects/stage-resolution.json.golden --source compiler/ocaml/tests/golden/typeclass_iterator_stage_mismatch.json.golden --source compiler/ocaml/tests/golden/typeclass_dictionary_resolved.json.golden --audit-source compiler/ocaml/tests/golden/audit/cli-ffi-bridge-linux.jsonl.golden --audit-source compiler/ocaml/tests/golden/audit/cli-ffi-bridge-macos.jsonl.golden --audit-source compiler/ocaml/tests/golden/audit/cli-ffi-bridge-windows.jsonl.golden --audit-source compiler/ocaml/tests/golden/audit/effects-residual.jsonl.golden --audit-source compiler/ocaml/tests/golden/audit/effects-stage.json.golden --audit-source compiler/ocaml/tests/golden/audit/ffi-bridge.jsonl.golden` を完走。`diagnostic.audit_presence_rate` / `typeclass.metadata_pass_rate` / `ffi_bridge.audit_pass_rate` がいずれも `1.0` に到達した。
- 上記に伴い、以下のゴールデンを `phase2.5.audit.v1` テンプレートへ整備:
  `compiler/ocaml/tests/golden/diagnostics/effects/invalid-attribute.json.golden`,
  `compiler/ocaml/tests/golden/diagnostics/effects/invalid-attribute-unknown-tag.json.golden`,
  `compiler/ocaml/tests/golden/diagnostics/effects/stage-resolution.json.golden`,
  `compiler/ocaml/tests/golden/typeclass_iterator_stage_mismatch.json.golden`,
  `compiler/ocaml/tests/golden/typeclass_dictionary_resolved.json.golden`（監査キー重複出力の調整を含む）。
- Spec 3.6 に DIAG-002 完了脚注を追加し、`phase2.5.audit.v1` 必須化の合意を記録。`reports/diagnostic-format-regression.md` チェックリストにも `audit` / `timestamp` の確認項目を追記済み。

# 2-5 レビュー記録 — DIAG-001 Week31 Day1-2 現状棚卸し（2025-11-07 更新）

DIAG-001 ステップ 1「現状棚卸しと仕様突合」の調査メモ。Severity 列挙の定義差異と周辺実装の挙動を整理し、後続ステップの改修範囲を明確化する。

## 1. 列挙定義と仕様参照の比較
| 区分 | 参照先 | 列挙内容 / 状態 | 観測メモ |
| ---- | ------ | ---------------- | -------- |
| 仕様 (Chapter 3) | `docs/spec/3-6-core-diagnostics-audit.md:24-43` | `Severity = Error | Warning | Info | Hint` を正式仕様として定義。 | CLI/LSP で情報診断とヒントを区別することを前提にしている。 |
| 仕様 (Chapter 2) | `docs/spec/2-5-error.md:12-55` | `Severity = Error | Warning | Note` のまま据え置き。 | Chapter 3 と不一致。Phase 2-5 でいずれかを統一する必要あり。 |
| 実装 — モデル層 | `compiler/ocaml/src/diagnostic.ml:39-46` | `type severity = Error | Warning | Note`。`severity_label` も 3 値前提。 | `Hint` 相当のバリアントなし。 |
| 実装 — V2 変換 | `compiler/ocaml/src/diagnostic.ml:803-821` | `module V2` で `Severity = Error | Warning | Info | Hint` を定義し、`Note -> Info` へ丸め込み。 | 新バリアントはここでのみ登場。`Hint` 未使用。 |
| JSON スキーマ | `tooling/json-schema/diagnostic-v2.schema.json:14-37` | LSP 準拠で `severity enum = [1,2,3,4]` を要求。 | スキーマ上は `Hint` 値（4）を許容するが、実装側に対応経路がない。 |

## 2. シリアライズと出力経路の挙動
- `compiler/ocaml/src/diagnostic_serialization.ml:249-269` では `severity_to_string` が `note` を出力し、`severity_level_of_severity` が 1/2/3 のみを返却。CLI JSON（`compiler/ocaml/src/cli/json_formatter.ml:90-145`）および LSP トランスポート（`tooling/lsp/lsp_transport.ml:48-116`）はいずれもこの 3 値を前提にしている。
- `compiler/ocaml/src/cli/color.ml:86-102` は `Note` 用の配色を定義しており、`Info`/`Hint` を考慮していない。
- `tooling/ci/collect-iterator-audit-metrics.py:1004-1025` は診断 JSON の集計時に `note -> info` へ正規化し、`hint` も集計カテゴリとして確保しているが現在は未使用。
- `compiler/ocaml/tests/golden/diagnostics/effects/stage-resolution.json.golden` は `severity: "info"` を保持するが、日本語ラベルや古いフィールド構成が混在しており、`Diagnostic_serialization` 由来の最新形式とは乖離している（改修後に再生成予定）。

## 3. ギャップとフォローアップ
- `Hint` バリアントが仕様に存在する一方で実装経路が未実装のため、Phase 2-5 ステップ 2 での列挙拡張時に CLI/LSP/メトリクスすべてを 4 値対応へ更新する必要がある。
- Chapter 2（`docs/spec/2-5-error.md`）が旧 3 値のままのため、仕様の改訂または脚注での移行方針整理が必要。Chapter 3 の脚注と整合する説明を追加する。
- `reports/diagnostic-format-regression.md` チェックリストには Severity 4 値化のレビューポイントが未記載。DIAG-001 完了時に更新し、情報診断／ヒント診断のゴールデン差分を追跡できるようにする。
- `tooling/json-schema/diagnostic-v2.schema.json` と `scripts/validate-diagnostic-json.sh` は `severity=4` を許容しているが、既存フィクスチャに Hint ケースが存在しない。改修後に AJV フィクスチャを追加する。
- メトリクス集計（`diagnostic.info_hint_ratio` 予定値）を Phase 2-5 で追加する際は、`collect-iterator-audit-metrics.py` の出力拡張と連動させ、旧 `note` データの移行を計画する。

## 4. CLI/LSP/監査パイプライン整合確認（2025-11-09 更新）
- LSP: `tooling/lsp/tests/client_compat/tests/client_compat.test.ts:95` に Info/Hint 専用ケースを追加し、`diagnostic-v2-info-hint.json` で `severity = [3, 4]` を確認。`npm run ci --prefix tooling/lsp/tests/client_compat` を実行し、新フィクスチャが AJV 検証を通過することを確認した。  
- CLI: `compiler/ocaml/tests/golden/diagnostics/severity/info-hint.json.golden` を `scripts/validate-diagnostic-json.sh` で検証し、文字列 Severity が維持されていることと `audit` / `timestamp` が欠落しないことを再確認。  
- 監査メトリクス: `tooling/ci/collect-iterator-audit-metrics.py:993-1036` に `info_fraction` / `hint_fraction` / `info_hint_ratio` を導入し、`python3 tooling/ci/collect-iterator-audit-metrics.py --require-success --source compiler/ocaml/tests/golden/diagnostics/severity/info-hint.json.golden` で Info/Hint の出現比率が `diagnostics.info_hint_ratio` として JSON 出力へ含まれることを確認。  
- ドキュメント: `reports/diagnostic-format-regression.md` へ Info/Hint 用チェックを追加し、Severity 拡張の確認手順をレビュー運用に組み込んだ。

## 5. ドキュメントとメトリクス更新（Week32 Day3, 2025-11-10 更新）
- 仕様反映: `docs/spec/3-6-core-diagnostics-audit.md` に DIAG-001 脚注を追加し、`severity` フィールドが 4 値へ統一された経緯と `Note` 廃止方針を明文化。`Severity` 説明に CLI/LSP/監査での区別運用を追記した。  
- 指標定義: `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の指標表へ `diagnostic.info_hint_ratio` を追加し、CI 集計で情報診断とヒント診断の比率を監視できるようにした。`diagnostic.hint_surface_area` は Phase 2-7 で集計実装予定として暫定登録。  
- 集計スクリプト連携: `collect-iterator-audit-metrics.py` のサマリ出力に追従した説明を同文書へ追記し、`info_fraction` / `hint_fraction` / `info_hint_ratio` が `diagnostics.summary` へ記録されることを明示。  
- 残課題: `diagnostic.hint_surface_area` の算出はスパン計測ロジックを追加した後に `tooling/ci/collect-iterator-audit-metrics.py` へ組み込む。Phase 2-7 で CLI テキスト出力刷新と合わせて優先度を再評価する。

# 2-5 レビュー記録 — EFFECT-001 Day1 タグ棚卸し

Phase 2-5 Week31 Day1。`EFFECT-001` のステップ 1（タグ語彙と既存実装の棚卸し）を実施し、仕様と実装のギャップを整理した。

## 1. Phase 2-5 で扱うタグ語彙
| タグ | 区分 | 主な仕様出典 | 想定 API / Capability 例 |
| ---- | ---- | ------------ | ------------------------ |
| `mut` | Σ_core | docs/spec/1-3-effects-safety.md §A | `var` 再代入、`Vec.push`, `Cell.set` |
| `io` | Σ_core | docs/spec/1-3-effects-safety.md §A | `Core.IO.print`, `Core.File.read` |
| `ffi` | Σ_core | docs/spec/1-3-effects-safety.md §A, docs/spec/3-8-core-runtime-capability.md §10 | `extern "C"` 呼び出し、Capability Bridge |
| `panic` | Σ_core | docs/spec/1-3-effects-safety.md §A | `panic`, `assert`, `Result.expect` |
| `unsafe` | Σ_core | docs/spec/1-3-effects-safety.md §A, docs/spec/3-6-core-diagnostics-audit.md §4.2 | `unsafe { … }`, `addr_of`, 生ポインタ操作 |
| `syscall` | Σ_system | docs/spec/1-3-effects-safety.md §A, docs/spec/3-8-core-runtime-capability.md §8 | `Core.System.raw_syscall`, ランタイム Capability `system.call` |
| `process` | Σ_system | docs/spec/1-3-effects-safety.md §A | `Core.Process.spawn_process`, `Capability.process` |
| `thread` | Σ_system | docs/spec/1-3-effects-safety.md §A | `Core.Process.create_thread`, `Capability.thread` |
| `memory` | Σ_system | docs/spec/1-3-effects-safety.md §A, docs/spec/3-4-core-collection.md §5 | `Core.Memory.mmap`, `Core.Memory.mprotect` |
| `signal` | Σ_system | docs/spec/1-3-effects-safety.md §A | `Core.Signal.register_signal_handler` |
| `hardware` | Σ_system | docs/spec/1-3-effects-safety.md §A | `Core.Hardware.rdtsc`, `Capability.hardware` |
| `realtime` | Σ_system | docs/spec/1-3-effects-safety.md §A | `Core.RealTime.set_scheduler_priority` |
| `audit` | Σ_system | docs/spec/1-3-effects-safety.md §A, docs/spec/3-6-core-diagnostics-audit.md §3 | `Diagnostics.audit_ctx.log`, 監査 Capability |
| `security` | Σ_system | docs/spec/1-3-effects-safety.md §A | `Capability.enforce_security_policy` |
| `mem` | Σ_stdlib | docs/spec/1-3-effects-safety.md §A.1, docs/spec/3-0-core-library-overview.md §2 | `Core.Collection.Vec.reserve`, `@no_alloc` 連携 |
| `debug` | Σ_stdlib | docs/spec/1-3-effects-safety.md §A.1 | `Core.Debug.inspect`, `expect_eq` |
| `trace` | Σ_stdlib | docs/spec/1-3-effects-safety.md §A.1, docs/spec/3-6-core-diagnostics-audit.md §5 | `Core.Diagnostics.emit_trace`, 監査ログ拡張 |
| `unicode` | Σ_stdlib | docs/spec/1-3-effects-safety.md §A.1, docs/spec/3-3-core-text-unicode.md §4 | `Core.Text.normalize`, Unicode テーブル参照 |
| `time` | Σ_stdlib | docs/spec/1-3-effects-safety.md §A.1 | `Core.Time.now`, 高精度タイマ |

> 備考: Phase 2-5 では `Σ_core` と `Σ_system` の主要タグを Typer で検出し、`Σ_stdlib` のタグは監査メタデータ補完と脚注整備を優先する。Capability Registry 側の命名はすべて小文字化して突合する必要がある。

## 2. Effect_analysis 実装観察（compiler/ocaml/src/type_inference.ml:37-190）
| 対象 | 現状実装 | 検出漏れ・論点 | 備考 |
| ---- | -------- | -------------- | ---- |
| `TCall` (関数呼出) | `callee_name = "panic"` の場合のみ `add_tag "panic"`。引数は再帰解析。 | `ffi` / `io` / `syscall` / Capability 付き API を識別する経路が存在しない。`Ffi_contract`・`Effect_profile.normalize_effect_name` 未連携。 | `expr.texpr_span` をタグに付与できるため、判別ロジック追加でスパンは再利用可。 |
| `TAssign` / `TAssignStmt` | 左右を再帰的に解析するのみ。 | `mut` タグが付与されない。`docs/spec/1-3-effects-safety.md §E` の再代入制約と乖離。 | `lhs.texpr_span` が利用できるが範囲が Dummy の場合は fallback 必要。 |
| `TVarDecl` / `TLetDecl` | 初期化式を解析するがタグ付与なし。 | `var` 宣言自体が `mut`（再代入許容）であることをタグに反映していない。 | `collect_decl` では宣言種別を判定できるため、`mut` 追加を検討。 |
| `TUnsafe` / `TUnsafe` ブロック | 内部式のみ解析し、自身でタグ付与しない。 | `unsafe` タグおよびブロック内の残余効果へのマーキングが欠落。 | ブロック span が取得可能。`unsafe` ブロック内で検出した他タグに対する扱いも要設計。 |
| `TCall` (外部呼出検出) | `callee_name` を文字列一致でしか評価しない。 | `extern` / Capability Bridge 呼出を `ffi` / `syscall` 等へ分類できない。 | `Ffi_bridge` スナップショット (`record_ffi_bridge_snapshot`) からタグ推論する案を検討。 |
| `Effect_analysis.add_tag` | 小文字化して重複排除。 | Dummy span (`start=0/end=0`) の扱いは `merge_usage_into_profile` 側で補うのみ。 | 追加タグの span を確保できれば `residual_leaks` へ直接反映可能。 |
| `collect_block` / `collect_stmt` | 逐次的に再帰解析。 | 宣言外の `unsafe` / `io` などを検出する入口は `collect_expr` のまま。 | AST から Statement 種別を判定でき、タグ付けの挿入ポイントは明確。 |

## 3. Stage 判定・Capability 連携メモ
- `Type_inference_effect.resolve_function_profile`（compiler/ocaml/src/type_inference_effect.ml:35-115）は `effect_node.effect_capabilities` の先頭要素しか解決せず、残りの Capability 名を破棄している。Phase 2-5 では配列全体を保持し、`resolved_capabilities` 的な構造を導入する余地がある。
- `stage_for_capability` は Capability 名を小文字化して照合するが、複数 Capability の Stage を合成する仕組みがなく、デフォルト Stage (`Stable`) を返すケースが多い。CI で取り込んだ Stage Trace (`runtime_stage.stage_trace`) との突合タイミングも Typer 側で一回のみ。
- `stage_trace_with_typer` は `cli_option` / `env_var` 由来のステップを先頭に保持しつつ `typer` ステップを挿入するが、Capability が複数ある場合でも `capability` フィールドには先頭名しか格納されない。
- `Effect_analysis.merge_usage_into_profile` の `residual_leaks` は `fallback_span` に関数宣言 span を渡しており、タグ追加時にスパンを確保できれば診断へ反映可能。`normalize_effect_name` で小文字化されるため、タグ一覧も小文字で統一する方針が必要。

## 4. 後続タスクへのインパクト
- タグ検出のギャップを埋めるため、`collect_expr`・`collect_decl` への分岐追加と、Capability 判別のための `Ffi_contract` / 標準ライブラリ API テーブルが必要。ホワイトリスト案は次ステップで `docs/plans/bootstrap-roadmap/2-5-review-log.md` に追記する。
- Stage 判定については `resolved_capability` を単一値で保持しているため、EFFECT-003 で予定している複数 Capability 出力に備えて型拡張が必要。`AuditEnvelope.metadata["effects.required"]` への反映計画とも連動させる。
- スパン情報は `expr.texpr_span` と `decl.tdecl_span` で取得できるため、タグ追加時に Diagnostic へ確実に渡す実装方針を後続工程でまとめる。
