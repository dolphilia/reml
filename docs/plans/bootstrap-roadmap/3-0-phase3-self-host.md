# 3.0 Phase 3 — Core Library 完成

Phase 3 では、Reml 標準ライブラリ Chapter 3 の正式仕様を Reml 実装へ揃えます。Prelude から Runtime Capability までの各モジュールを仕様と照合し、効果タグ・監査・Capability 契約が一貫して動作する状態を構築します。

## 3.0.0 Rust 実装への再統合
- Phase 2 以降は `docs/plans/rust-migration/` にある計画体系で Rust 版 Reml コンパイラの移植を進めてきましたが、Phase 3 では `docs/plans/bootstrap-roadmap/2-8-spec-integrity-audit.md` の監査完了を合図に元の Bootstrap Roadmap と接続し、Rust 実装を標準仕様に完全整合させる工程へシフトします。
- この段階では OCaml 実装はコード参照用として残し、Dual-write や検証に持ち込むことは避け、Rust 実装を唯一のアクティブな実装として扱います。
- 2-8 で確定した差分リストと監査結果をベースに、3-x 以降の標準ライブラリ計画を Rust の `compiler/rust/` 実装と同期させ、監査済み仕様とのギャップを残さない状態で Phase 3 に進みます。

## 3.0.1 目的
- `Core.Prelude`/`Core.Collections`/`Core.Text`/`Core.Numeric`/`Core.IO`/`Core.Diagnostics`/`Core.Config`/`Core.Runtime` の API を Reml で実装し、仕様書と相互参照が成立した状態で提供する。
- 効果タグと Capability Stage の境界を検証し、Chapter 3 全体の診断・監査連携が [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) と一致するように統合する。
- 標準ライブラリのコード例・サンプル・メトリクスを最新化し、Phase 4 の移行とエコシステム展開に備えたベースラインを整備する。

## 3.0.2 スコープ境界
- **含む**: Core Prelude/Collections/Text/Numeric/IO/Diagnostics/Config/Runtime Capability の実装・テスト・ドキュメント更新、効果タグ・Capability 検証、監査／メトリクスの記録。
- **含まない**: 非同期ランタイム (`3-9`)、プラグイン／DSL 拡張 (`4-x`)、エコシステム仕様 (`5-x`) の本格対応（Phase 4 以降に委譲）。
- **前提条件**: Phase 2 で確定した型クラス・効果システム・診断仕様が利用可能であり、`0-3-audit-and-metrics.md` と `0-4-risk-handling.md` に基準値・リスク管理手順が登録済みであること。
- **実装対象**: Rust 実装（`compiler/rust/` 配下）を第一実装とし、`docs/plans/rust-migration/` に記録された成果を反映して 2-8 監査で認められたカバレッジを維持する。OCaml 実装は差分比較や歴史資料として残すが、Phase 3 の開発フローでは積極的に利用しない。

## 3.0.2a 作業ディレクトリ
- `compiler/ocaml/src` : 標準ライブラリ各モジュールの実装（レガシー参照用）
- `compiler/ocaml/tests` : API ゴールデンテスト・性能ベンチマーク（比較用ゴールデン）
- `examples/` : API 使用例の整理（`examples/algebraic-effects/`, `examples/language-impl-comparison/` 等）
- `docs/spec/3-x` : 仕様本文の更新とリンク整備
- `docs/guides/` : 運用ガイドの同期 (`docs/guides/runtime-bridges.md` など)
- `docs/notes/` : 設計判断・メトリクスの記録 (`docs/notes/core-library-outline.md` ほか)
- `compiler/rust/` : Rust 版標準ライブラリの実装とテストベンチ
- `compiler/rust/tests` : Rust 実装向けの API テスト・監査ゴールデン
- `docs/plans/rust-migration/` : Rust 移植計画と Phase 2-8 の監査結果を参照しながら Bootstrap Roadmap へ戻すハンドオーバー資料

この段階では `docs/plans/bootstrap-roadmap/2-8-spec-integrity-audit.md` で生成された差分リスト・監査スナップショットを定常的に参照し、Phase 3 の各マイルストーンで 2-8 に記録されたリスクと TODO に対する完了コメント・フォローアップを残します。

## 3.0.3 成果物とマイルストーン
| マイルストーン | 内容 | 検証方法 | 期限目安 |
|----------------|------|----------|----------|
| M1: Prelude & Iteration | `Option`/`Result`/`Iter` と Collector を実装し効果タグを整合 | API テスト、効果タグ静的検証、サンプル実行 | Phase 3 開始後 8 週 |
| M2: Collections | 永続・可変コレクションと差分 API を実装 | 構造共有プロパティテスト、`CollectError` シナリオ CI | 開始後 16 週 |
| M3: Text & Unicode | 文字列三層モデル・Unicode 正規化・Builder を実装 | UAX コンフォーマンス、Decode/Encode ストリーミングテスト | 開始後 20 週 |
| M4: Numeric / IO & Path | 統計・時間 API と IO 抽象／Path セキュリティを実装 | ベンチマーク ±15% 以内、IO/Path 統合テスト | 開始後 26 週 |
| M5: Diagnostics & Config | Diagnostic/Audit と Manifest/Schema を統合 | 診断スナップショット、Config Lint、監査ログ比較 | 開始後 30 週 |
| M6: Runtime Capability | Capability Registry と Stage 検証を完成 | Capability テストマトリクス、Manifest 契約検証 | 開始後 34 週 |

### 3.0.3a M1 Prelude & Iteration 進行管理
- `docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md` の **Step 2**（Option/Result 系 API 実装, 35-36 週）で定義した WBS `2.1a〜2.3b` を Phase 3 `M1` のガイドラインとして採用する。`prelude_api_inventory.toml` の更新と `cargo xtask prelude-audit --wbs 2.1b --strict`（`.cargo/config.toml` で alias 化）の導入を M1 着手条件、`panic_forbidden.rs` + `scripts/validate-diagnostic-json.sh` の追加を完了条件とする。
- M1 の成果は `compiler/rust/runtime`（Prelude モジュール）, `compiler/rust/frontend/tests/core_prelude_option_result.{rs,snap}`, `tooling/ci/collect-iterator-audit-metrics.py` の拡張、`docs/spec/3-0-core-library-overview.md`/`docs/notes/core-library-outline.md`/`docs-migrations.log` の脚注更新を含む。これらは Step 2 のチェックリストにより週次レビューされ、`0-3-audit-and-metrics.md` の KPI (`core_prelude.missing_api`, `core_prelude.panic_path`) を更新することで完了を確定する。
- OCaml 実装は比較対象としてのみ参照し、`compiler/ocaml/tests/test_type_inference.ml` で収集した診断 JSON を `reports/spec-audit/ch0/links.md` へリンクして差分説明を残す。Rust 実装の snapshot と `scripts/validate-diagnostic-json.sh` の結果が一致しない場合は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に追記し、M1 の出口ゲートを閉じる。
- `WBS 2.2a` で定義した `ensure`/`ensure_not_null` + `core.prelude.ensure_failed` の登録は `Phase 3 M1` の品質ゲートとして扱い、`docs/spec/3-6-core-diagnostics-audit.md` の該当節と `tooling/ci/collect-iterator-audit-metrics.py --section prelude-guard` のレポートが揃っていない限り `core_prelude.guard.failures` KPI を `0` にできない運用とする。`reports/spec-audit/ch0/links.md` には `scripts/validate-diagnostic-json.sh` で取得したサンプル診断を貼り付け、`core_prelude` セクションの存在をレビューで確認する。
- `WBS 2.2b` の成果物として `examples/language-impl-comparison/reml/prelude_guard_template.reml` を公開し、DSL 上での `ensure`/`ensure_not_null` 利用例と `core.prelude.ensure_failed` メタデータの付与手順を仕様脚注（`docs/spec/3-0-core-library-overview.md`）に紐付ける。`docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md` の該当 WBS 行は同サンプルのパスと検証ログ（`render_preview` ガードの Result 伝播）を参照する形で更新し、M1 の進行レビューで再利用可能な状態に保つ。

### 3.0.3b Iter コア構造（WBS 3.1a）進行管理
- `docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md` の「WBS 3.1a 実装指針」で規定した F0〜F3 を M1 の第 2 サイクルとして採用する。F0（仕様精査）は `docs/spec/3-1-core-prelude-iteration.md` と `compiler/ocaml/src/constraint_solver.ml` を照合し、`IterStep` が保持する `effect`/`stage`/`capability` 情報と `IteratorDict` の JSON 形式を `docs/notes/core-library-outline.md` に記す。
- F1 では `compiler/rust/runtime/src/prelude/iter/mod.rs` に `Iter`/`IterState`/`IterSeed`/`IterSource`/`IterStep` を追加し、`docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` に `module = "Iter"` を登録する。完了時に `docs-migrations.log` と本節へ日付付きログ（例: `2025W36 F1 done`) を追加する。
- F2 は `compiler/rust/frontend/src/typeck/constraint/iterator.rs` を新設して `IteratorDictInfo` を生成し、`collect-iterator-audit-metrics.py` の `iterator.dict` 列へ書き出す。`cargo xtask prelude-audit --section iter --strict` の結果を `reports/spec-audit/ch0/links.md` に貼り付け、`0-3-audit-and-metrics.md` へ `iterator.stage.audit_pass_rate` KPI を登録する。
- F3 は `compiler/rust/frontend/tests/core_iter_pipeline.rs` の 6 シナリオ snapshot と `scripts/validate-diagnostic-json.sh`/`collect-iterator-audit --section iter` を組み合わせた検証を行い、`iterator.effect.debug = 0` と `core_iter_pipeline` 実行時間（Phase 2 ベンチ比 ±10%）を確認する。逸脱は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に記録する。
- KPI/出口条件: `cargo xtask prelude-audit --section iter --strict` 差分 0、`collect-iterator-audit-metrics.py` で `iterator.stage.audit_pass_rate = 1.0`、`docs/notes/core-library-outline.md` に Iter 実装メモと診断ログが揃っていること。未達時は M1 マイルストーンを Go/No-Go とし、`Phase 3` 全体レビュー前に是正する。

## 3.0.4 主要タスク

1. **Core Prelude & Iteration** (`3-1`)
   - `Option`/`Result`/`Never` と `Iter` 本体・アダプタ・終端操作を Reml で実装。
   - 効果タグと `@must_use` 属性を静的解析し、Chapter 1 の構文・効果仕様と一致させる。
   - `Collector` 契約を定義し、`Core.Collections`／`Core.Text` から再利用できるよう拡張。
2. **Core Collections** (`3-2`)
   - 永続構造（List/Map/Set）と可変構造（Vec/Cell/Ref/Table）を実装し、構造共有・順序保持・効果タグを検証。
   - `Iter` との相互運用（`collect_*`, `Map.from_iter`）と監査差分 (`ChangeSet`) を整備。
3. **Core Text & Unicode** (`3-3`)
   - 文字列三層モデル（Bytes/Str/String）と `GraphemeSeq`/`TextBuilder` を実装。
   - Unicode 正規化・ケース変換・幅変換 API を `UnicodeError` と診断変換で統合。
   - IO/Diagnostics と連携したストリーミング decode・監査ログ API (`log_grapheme_stats`) を検証。
4. **Core Numeric & Time** (`3-4`)
   - 数値トレイト／統計ヘルパ／Histogram／回帰 API を実装し、`Iter` ベースでテスト。
   - `Timestamp`/`Duration`/`Timezone` とフォーマット／パースを整備し、`Core.IO` と統合。
   - `MetricPoint` と監査メトリクス送出を整備し、`AuditEnvelope` メタデータを共通化。
5. **Core IO & Path** (`3-5`)
   - `Reader`/`Writer` 抽象、ファイル API、バッファリング、IO エラー体系を実装。
   - Path 抽象・セキュリティヘルパ・ファイル監視 (オプション) を整備し、クロスプラットフォーム差異を `TargetCapability` で吸収。
6. **Core Diagnostics & Audit** (`3-6`)
   - `Diagnostic` 構造・`AuditEnvelope`・`TraitResolutionTelemetry` 等を実装。
   - CLI/LSP 出力フォーマット、ステージ別フィルタ・抑制ポリシー、監査ログ記録を統合。
7. **Core Config & Data** (`3-7`)
   - `Manifest`/`Schema`/`ConfigCompatibility` API を実装し、DSL エクスポート情報・Capability Stage を同期。
   - Config Diff・ChangeSet を Diagnostics/Audit に連携し、CLI (`reml config lint/diff`) フローを整備。
8. **Core Runtime & Capability** (`3-8`)
   - `CapabilityRegistry`・`CapabilityHandle`・`verify_capability_stage`・`verify_conductor_contract` を実装。
   - Stage/Capability 情報を Diagnostics/Config/Runtime 各層で共有し、監査イベント (`CapabilityMismatch`) を記録。
9. **横断タスク**
   - API ドキュメント・サンプル・ガイド（`docs/guides/runtime-bridges.md`, `docs/notes/dsl-plugin-roadmap.md` 等）を更新。
   - `0-3-audit-and-metrics.md` へベンチマーク・監査指標を継続記録し、差分理由を明示。

## 3.0.4a Rust CLI 安定化
- Phase 3 は `reml_frontend` を唯一の CLI とし、`schema_version = "3.0.0-alpha"` を含む診断・監査 JSON（`reports/spec-audit/ch1/use_nested-YYYYMMDD-typeck.json` 等）をベースラインに採用する。Stage/Audit 由来の `stage_trace` と `used_impls` は `typeck/typeck-debug.rust.json` に保管され、Chapter 3 の Capability 監査を直接支援する。
- AST を生成できなかった入力は `typeck.aborted.ast_unavailable` 診断で停止し、`docs/spec/0-3-code-style-guide.md` / `docs/spec/1-1-syntax/examples/README.md` / `docs/spec/3-6-core-diagnostics-audit.md` に記載された CLI 手順と整合する必要がある。Phase 3 の CI/ガイド更新では同じコマンドを利用し、`reports/spec-audit/ch1/typeck-fallback-removal-20251122.md` を参照して検証する。

## 3.0.5 測定と検証
- **API 完全性**: 仕様書に列挙された公開 API が Reml 実装に存在し、効果タグ・属性が一致することを静的チェックと API テストで確認。
- **効果タグ／Capability 整合**: `effect` タグと `CapabilityStage` の組み合わせを検証し、違反時は Diagnostics で再現できることを確認。
- **性能ベンチマーク**: Prelude/Collections/Numeric/IO の代表関数で Rust 実装の Phase 2 ベースライン（`docs/plans/rust-migration/3-2-benchmark-baseline.md` で定義）に対し ±15% 以内を目標に測定し、結果を `0-3-audit-and-metrics.md` に記録。必要に応じて OCaml 実装のデータを参考資料として付録に掲載する。
- **ドキュメント同調**: 仕様書・ガイド・サンプルが更新され、リンク切れ・用語揺れがないことをレビュー。
- **監査／診断スナップショット**: `Diagnostic` と `AuditEnvelope` の出力をゴールデンテスト化し、CI で差分を検出。

## 3.0.6 リスクとフォローアップ
- **効果タグの逸脱**: 実装と仕様で効果タグが不一致の場合、クロスレビューとツール支援を追加し、`0-4-risk-handling.md` に改善タスクを登録。
- **Unicode/IO の性能劣化**: UAX コンフォーマンスを優先した結果として性能が不足する場合、キャッシュ・バッファリング戦略の改善や Phase 4 の最適化項目として記録。
- **Config/Capability のルール変更**: Manifest と Capability の整合が難航した場合、Phase 4 の移行計画と連携し、`docs/notes/dsl-plugin-roadmap.md` に暫定運用を明記。
- **テストボリューム増加**: Chapter 3 全体の CI 実行時間が長くなる恐れがあるため、テスト分割・キャッシュ・nightly ジョブを検討。
- **Phase 2 実装との差分**: 型クラス方式や効果システムが Phase 2 結果と異なる場合、差分を `docs/notes/llvm-spec-status-survey.md` にまとめ、Phase 4 の移行判断に備える。

## 3.0.7 2-8 監査からのフォローアップ
- `docs/plans/bootstrap-roadmap/2-8-spec-integrity-audit.md` で記録された差分リスト・未解決リスクを Phase 3 各マイルストーンの「監査結果」欄で参照・完了報告し、必要があれば 2-9 監査補足セクションを新設して Rust 実装固有の検証項目（ランタイム Capability、監査ログ整合、ABI）を明文化します。
- 監査用スナップショット（`reports/spec-audit/`）および `docs/notes/spec-integrity-audit-checklist.md` の TODO を `docs/plans/rust-migration/` の関連ドキュメントと同期させ、Rust 実装の進捗が 2-8 の前提（CI リンクチェック、ガイド参照整合、診断/効果タグの一致）に照らして評価できるようにします。
- 2-8 監査の成果は `docs/plans/rust-migration/overview.md` や `docs/plans/rust-migration/3-0-ci-and-dual-write-strategy.md` にも脚注として残し、Phase 3 以降の標準ライブラリ作業と CI/監査運用の間をつなぎます。

---

Phase 3 完了時点で Reml 標準ライブラリの基盤が整い、Phase 4 ではマルチターゲット互換性検証とエコシステム移行に集中できる状態になる。
