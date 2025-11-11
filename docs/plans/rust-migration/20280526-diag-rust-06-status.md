# DIAG-RUST-06 進捗レポート（2028-05-26）

## 現状整理
- Stage/Audit ペイロード再実装は機能しており、最新ラン `reports/dual-write/front-end/w4-diagnostics/20280526-w4-diag-effects-stagefix/effects/ffi_ownership_mismatch/effects-metrics.rust.json:1-118` では `extensions.effects.stage.*` / `extensions.bridge.stage.*` / `audit_metadata.effect.stage.*` がすべて出力され、`effects-metrics.rust.err.log` も空だった。同ファイルには `stage_trace` や `capability.ids` も含まれ、DIAG-RUST-06 ステップ1（Stage/Audit 欠落キー解消）は達成済み。
- `scripts/poc_dualwrite_compare.sh --force-type-effect-flags --mode diag --run-id 20280526-w4-diag-effects-stagefix ...` を実行し、FFI 3 ケースの最新データを取得（summary: `reports/.../20280526-w4-diag-effects-stagefix/summary.md:1-4`）。`metric` 判定はいずれも `parser.expected_summary_presence < 1.0` で失敗し、OCaml 1 件 vs Rust 64/42 件という旧来の過剰診断が残っていることを再確認。
- Type/Effect 4 ケースも同条件で再測定（`reports/.../20280526-w4-diag-type-effect/summary.md:1-5`）。`type_condition_literal_bool` は Rust 側診断 0 件、`effect_residual_leak` / `effect_stage_cli_override` は Rust 側 5/6 件のまま。計画ステップ2（型チェックで `ConditionLiteralBool` / `ResidualLeak` を出す）とステップ3（診断件数圧縮）が未着手である証拠。
- `python3 tooling/ci/collect-iterator-audit-metrics.py --section effects --source ... --require-success` を `ffi_ownership_mismatch` で実行したところ、`effect_scope.audit_presence`/`effect_stage.audit_presence`/`bridge_stage.audit_presence` は 1.0 を達成した一方、`typeck_debug_match` が `typeck-debug.ocaml.json` 欠落で失敗（詳細: `/tmp/ffi_ownership_metrics.json:1-92`）。`typeck-debug.ocaml.json` は成功ケース（`ffi_async_dispatch` など）にのみ生成されており、エラー発生時にも最低限のダンプを保存する仕組みが必要。
- `scripts/poc_dualwrite_compare.sh` 自体に `option_requires_value` の typo があり（`scripts/poc_dualwrite_compare.sh:405-418`）、今回の検証中に修正済み。以後 `--runtime-capabilities-file` 付きケースでも CLI フラグの展開が正しく動作する。

## 未完了ポイント
1. **Rust 型推論側の欠落**
   - `type_condition_literal_bool` で診断 0 件のまま：`compiler/rust/frontend/src/typeck/driver.rs` に `check_bool_condition` や `ConditionLiteralBool` 相当の分岐が未実装。OCaml の `Type_inference.condition_not_bool` 挙動を移植する必要がある。
   - `effect_residual_leak` / `effect_stage_cli_override` の残余効果診断：`ResidualLeak` 検出や `Type_inference_effect` の `Type_row_mode::DualWrite` 処理が Rust に存在しない。`emit_effect_violation` と `type_row_mode=dual-write` のメトリクス経路を追加する必要がある。
2. **OCaml 側成果物の不足**
   - エラー発生時に `typeck/typeck-debug.ocaml.json` が生成されず、新設した `typeck_debug_match` が常時 NG になる。`remlc` の `--emit-typeck-debug` 実装を確認し、エラーでも最小限の JSON を書き出す（またはスクリプト側で OCaml 成果物をダミー補完する）対策が必要。
3. **診断件数のギャップ**
   - Rust 側は recover/lex 診断を 40～60 件出し続けており、`parser.expected_summary_presence` が常時 <1.0。`FrontendDiagnostic` に `ResidualLeak`/`ConditionBool` を実装するだけでなく、`parser_expectation` 同等のマージ（`docs/plans/rust-migration/1-2-diagnostic-compatibility.md:209-217`）を進める必要あり。
4. **CLI 再測定の自動化**
   - 現状 `--case-filter` がないため、ケース抽出用の一時ファイルを手作業で作成した。再現性のため `scripts/poc_dualwrite_compare.sh` にフィルタ機能追加か、専用ケースファイルをリポジトリに置く運用整理が望ましい。
5. **計画書反映**
   - `docs/plans/rust-migration/1-0-front-end-transition.md:228-229` で Stage/Audit 実装済みである旨は追記済みだが、今回の検証結果（parser metrics/ typeck debug での失敗理由）を `1-2-diagnostic-compatibility.md:209-217` および `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` の該当 TODO に追記する必要がある。

## 推奨作業ステップ
1. **Rust 型推論のエラー実装**
   1.1 `compiler/rust/frontend/src/typeck/driver.rs` に `check_bool_condition` / `emit_effect_violation` / `ResidualLeak` 判定を実装し、`TypeRowMode::DualWrite` で `type_condition_literal_bool` と `effect_residual_leak` が OCaml と同じ `code`/`message` を生成するようにする。  
   1.2 `compiler/rust/frontend/src/bin/poc_frontend.rs` の `TypecheckMetricsPayload` に `type_row_mode` 等を残しているので、新診断が `extensions.effects.residual` を埋めるようにシリアライズする。
2. **OCaml Typeck Debug のフォールバック**
   2.1 `compiler/ocaml/src/main.ml` で `--emit-typeck-debug` 指定時にエラーでも空 JSON を書く（`typeck_debug_writer.ml` 等で `Result` をハンドリング）。  
   2.2 暫定策として `scripts/poc_dualwrite_compare.sh` にフォールバックコピー（Rust 生成物を `typeck-debug.ocaml.json` へ複製し “placeholder” と記録）を入れ、CI メトリクスを unblock する。
3. **診断圧縮とメトリクス改善**
   3.1 `compiler/rust/frontend/src/diagnostic/recover.rs` を改良して `ExpectedTokenCollector` のマージを行い、`parser.expected_summary_presence` を 1.0 に戻す。  
   3.2 `collect-iterator-audit-metrics.py` の `parser.expected_summary_presence: total=0` エラーを解消するため、Rust 側で `expected` を常に出力し、OCaml 側でも空集合を明示する。
4. **再測定と計画更新**
   4.1 上記修正後、`scripts/poc_dualwrite_compare.sh --force-type-effect-flags --mode diag --run-id <新ID> --cases docs/.../w4-diagnostic-cases.txt` をフルで再実行し、`reports/.../summary.md` を `docs/plans/rust-migration/appendix/w4-diagnostic-case-matrix.md` と `p1-front-end-checklists.csv` に転記。  
   4.2 `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md#TODO:-diag-rust-06` に今回の Run ID（20280526-...）と残課題（parser metrics / typeck debug）を追記し、完了条件を更新する。

## 実行ログ
- `scripts/poc_dualwrite_compare.sh --force-type-effect-flags --mode diag --run-id 20280526-w4-diag-effects-stagefix --cases <ffi-only file> --emit-expected-tokens expected_tokens`
- `scripts/poc_dualwrite_compare.sh --force-type-effect-flags --mode diag --run-id 20280526-w4-diag-type-effect --cases <type/effect file> --emit-expected-tokens expected_tokens`
- `python3 tooling/ci/collect-iterator-audit-metrics.py --section effects --source reports/.../ffi_ownership_mismatch/diagnostics.rust.json --require-success`

これらの結果を基に、DIAG-RUST-06 は「Stage/Audit キーは解消済み・型推論/診断整合は未完」という段階であることが明確になった。
