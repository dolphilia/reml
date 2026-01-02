# P2R-02: ランタイム Bridge バックプレッシャ診断実装計画

この文書は `docs/plans/rust-migration/p2-runtime-gap-report.md` に記された P2R-02 ギャップ（ランタイム Bridge による Stage mismatch/backpressure 検出）の解消に向けて、必要な調査項目・実装ステップ・検証指標を整理したものです。各ステップでは OCaml 実装と Rust 版との比較、対応する仕様ページ、および今後の具体的な作業ブックマークを明示します。

## 目的
- `docs/spec/3-6-core-diagnostics-audit.md` §3 で要求されている `bridge.stage.backpressure` / `effects.contract.stage_mismatch` の診断出力と JSON 監査キーを Rust パーサに提供する。
- `docs/spec/3-8-core-runtime-capability.md` §4 で定義される Runtime Bridge の Stage 違反検出とロールバック診断を Rust ランタイムとフロントエンドの連携パスに再現する。
- `docs/plans/rust-migration/overview.md` および `docs/plans/rust-migration/2-1-runtime-integration.md` で定められた Phase 2 成果物と整合するよう、診断ログ／CLI メトリクス／監査スキーマを OCaml 実装と揃える。

## 現状の欠落と参照コード
| 項目 | OCaml 実装 | Rust 実装 | ギャップ |
| --- | --- | --- | --- |
| Bridge backpressure 検出 | `parser_driver.ml:633-682` で `build_bridge_stage_diagnostic` を起点に `Runtime_bridge_registry.stream_signal` へ伝搬 | `streaming/flow.rs` はフロー管理のみ、`poc_frontend.rs` は recover 調整とトレース整形のみで Bridge 信号がない | Streaming parser から Bridge 判定を引くフックが未実装 |
| Stage mismatch 監査 | `runtime_bridge_registry.ml` が `stream_signal` で `AuditEnvelope` の `bridge.stream.*` を更新 | Rust 側 `effects::EffectAuditContext` が CLI stage の写経のみを保持し、Stream イベントのメタデータがない | runtime/ffi 側で Stage trace を保持するキャッシュが未整備 |
| CLI/監査メトリクス | `main.ml:995-1007` の `Cli.stats` に await/resume/backpressure カウンタを含め、`collect-iterator-audit-metrics.py` が参照 | Rust CLI は `stream_meta.flow.checkpoints_closed` などに限定され、Bridge 事象を出力しない | CLI/dual-write で Bridge 監査データが欠落 |

## 実装ステップ（P2R-02 の 5 サブステップに対応）
1. **Stage mismatch/backpressure トレースの再構成**  
   - `parser_driver.ml` と `runtime_bridge_registry.ml` を精査し、Bridge 判定のタイミング、`stage_trace` に含めるべき情報（`parser_offset`・`stream_sequence`・`stage_capability`）を洗い出す。  
   - `docs/spec/3-6-core-diagnostics-audit.md` §3 および `docs/spec/3-8-core-runtime-capability.md` §2 で定義されている診断タグ・監査キーの一覧を作成し、Rust 側で生成すべき JSON スキーマを確定する。
2. **Streaming フローへのバックプレッシャフック追加**  
   - `reml_frontend::streaming::flow::StreamFlowState` と `StreamingRunner` に Bridge 判定用に `RuntimeBridgeSignal` や `StreamingEffectContext` を注入するインタフェースを設計する。  
   - フックは `StreamFlowState::checkpoint_end` のような既存メソッドの流れに沿って追加し、バックプレッシャを検知する条件（Runtime が要求 Stage を受け入れない／Stage mismatch）で `RuntimeBridgeSignal` を発火させる。
3. **Runtime Bridge Registry の Rust 実装**  
   - `compiler/rust/runtime/ffi/src/registry.rs` に `RuntimeBridgeRegistry` を追加し、Stage/backpressure 事象をキャッシュする `stream_signal` API を公開する（参考: `compiler/ocaml/src/runtime_bridge_registry.ml`）。  
   - signal には期待 Stage、現在の Stage Trace、`intent`（await/resume/backpressure）を含め、CLI ↔ Runtime の境界で `StageTrace` を延長できるようにする。
4. **診断生成器の拡張**  
   - `StageAuditPayload` と `effects::EffectAuditContext` に Bridge 信号情報を受け取るフィールド（例: `bridge_origin`, `bridge_signal`, `bridge_stage_trace`）を追加し、`poc_frontend.rs::build_parser_diagnostics`、`build_type_diagnostics` で `bridge` 拡張を注入する。  
   - `compiler/rust/frontend/src/diagnostic/effects.rs` へ `BridgeBackpressure`/`ContractStageMismatch` の診断テンプレートを追加し、`bridge.stage.backpressure` および `effects.contract.stage_mismatch` を `extensions`/`metadata` として記録する。
5. **メトリクス・監査ロギング・ドキュメント**  
   - CLI 出力（`stream_meta`、`runconfig`）および `collect-iterator-audit-metrics.py --section streaming` に出力される JSON で `await_count`/`resume_count`/`backpressure_count` を追加する。  
   - Dual-write で生成する JSON（`TypecheckMetricsPayload` など）にも Bridge カウンタを含め、OCaml 側と比較可能な `bridges` セクションを設ける。  
   - `docs/plans/rust-migration/unified-porting-principles.md` の「環境差異の明示」原則に従い、Bridge backpressure に関する環境差分の記録と `docs-migrations.log` への `Bridge backpressure diagnostics` 追加を行う。

## 進捗チェックリスト
- [ ] Bridge backpressure 診断タグの一覧と必要 `stage_trace` フィールドを定義（`docs/spec/3-6`/`3-8`との照合）。
- [ ] Streaming flow で `RuntimeBridgeSignal` を捕捉して `StreamFlowState` が保持できるように拡張。  
- [ ] `Runtime_bridge_registry` 相当の Rust 実装を実装し、`stream_signal` が Stage mismatch/backpressure を記録。  
- [ ] Diagnostics/effects で `BridgeBackpressure`/`ContractStageMismatch` を出力し、`stage_trace` に発生元データを追加。  
- [ ] CLI/dual-write の JSON 出力に Bridge カウンタを含め、`collect-iterator-audit-metrics.py --section streaming` が参照できるフォーマットを提供。  
- [ ] `docs/migrations.log` に Bridge backpressure 診断カテゴリを記録し、CI/監査チームが追跡可能に。

## 検証とテスト
1. **単体テスト**: `compiler/rust/frontend/tests/` に Streaming parser で明示的に await/resume/backpressure をトリガーする仕様資産を追加し、生成される `diagnostic.extensions.bridge` と `effects.contract.stage_trace` に期待値が含まれるかを検証。  
2. **dual-write 比較**: `poc_frontend` の DualWrite オプションを使い、OCaml 版と Rust 版で生成される Bridge関連メタが一致するかを JSON diff（`scripts/poc_dualwrite_compare.sh` など）で確認。  
3. **監査ログ**: `collect-iterator-audit-metrics.py --section streaming` が `bridge.backpressure_count` を読み取り、期待した統計値を出力できること。  
4. **ドキュメント整合**: `docs/plans/rust-migration/2-1-runtime-integration.md`、`2-3-p2-backend-integration-roadmap.md` に Bridge backpressure 監査結果とテストシナリオを追記し、`README.md` などで参照される計画概要を更新。

## フォローアップ
- Bridge signal/Runtime Bridge Registry の共有データ構造を `reml_frontend` の `diagnostic` や `streaming` モジュール API について `docs/spec/2-5-error.md` や `docs/guides/compiler/core-parse-streaming.md` に記述し、他チームとのインタフェースを明示する。  
- 実装中に判明した監査スキーマ差分は `docs/notes/stdlib/core-library-outline.md` に記録し、次のリリースノートで取りまとめる。
- 大きな設計変更があった場合は `docs/notes/dsl/dsl-plugin-roadmap.md` に TODO を追加し、長期的な監査フローと Capability 拡張の整合を維持。
