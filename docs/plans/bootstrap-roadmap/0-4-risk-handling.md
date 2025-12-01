# 0.4 リスク管理とフォローアップ

本章はブートストラップ計画におけるリスク登録、フォローアップ、移行期間中の意思決定手順を定義する。[0-1-project-purpose.md](../../spec/0-1-project-purpose.md) の判断フレームワークと `docs/notes/llvm-spec-status-survey.md` の未決課題を参照し、リスクが可視化された状態でフェーズを進める。

## 0.4.1 リスク登録カテゴリ
| カテゴリ | 説明 | 例 | 対応フェーズ |
|----------|------|----|-------------|
| 技術的負債 | 実装または仕様の不足 | 型クラス辞書の性能懸念 | Phase 2–4 |
| スケジュール | 工期遅延やレビュー待ち | 型チェッカレビュー待機が 2 週超過 | 全フェーズ |
| 互換性 | 旧ツールや OCaml 実装との差分 | 診断フォーマット不一致 | Phase 3–4 |
| セキュリティ/安全性 | Stage/Capability ミスマッチ、所有権違反 | FFI リーク検出 | 全フェーズ |
| エコシステム | 外部ユーザー・プラグインへの影響 | DSL プラグインのバージョン不整合 | Phase 4 |

## 0.4.2 登録フォーマット
```
- 登録日: YYYY-MM-DD
- タイトル: <短い説明>
- カテゴリ: <上記カテゴリのいずれか>
- 詳細: <現象・影響範囲・関連仕様章>
- 対応案: <一次対応/恒久対応>
- 期限: <YYYY-MM-DD>
- 状態: Open / Mitigating / Resolved
- 関連フェーズ: Phase X
- 参照: <コミット、Issue、ノートなど>
```
登録されたリスクはフェーズ進行会議で週次レビューされ、状態変更時には計画書および関連仕様書へ影響を反映する。

## 0.4.3 エスカレーション基準
- **性能指標が 10% 超過**: `parse_throughput` または `memory_peak_ratio` が目標値から 10% 以上逸脱した場合、Phase 進行を停止し、対策タスクを作成。
- **Stage ミスマッチ発生**: `stage_mismatch_count` > 0 の場合、ミスマッチが解消されるまで新機能導入を凍結。
- **診断差分未解決**: `diagnostic_regressions` が 7 日以上解決されない場合、[3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) を見直し、必要なら仕様変更を先行実施。

## 0.4.4 フォローアッププロセス
1. リスク登録後 24 時間以内に担当者を割当。
2. 担当者は対策案を `0-3-audit-and-metrics.md` の週次レポートに記録。
3. 解決後は状態を `Resolved` に更新し、対応内容をフェーズ文書に脚注として追記。
4. 次フェーズへ影響する場合は、該当フェーズの冒頭に「継承リスク」として要約を記載。

## 0.4.5 移行期の特別措置
- OCaml 実装との互換性確保のため、致命的な差分が発生した場合は OCaml 実装を一時的に正とし、Reml 実装を修正する。
- 重大な脆弱性が見つかった場合は、Phase に関わらず緊急パッチを適用し、`docs/notes/` に詳細な対応メモを残す。

---

本章は計画期間中に常に更新される生きたリスク台帳として扱う。未解決のリスクが存在する限り、次フェーズへ進む前に状態をレビューし、必要なタスクを各フェーズ文書（`1-x`〜`4-x` 系列）へ反映させる。

## 0.4.6 現在のリスク登録
<a id="core-io-watcher-risk"></a>
- 登録日: 2025-12-01
- タイトル: 大量ファイル監視による Watcher バッファ飽和と監査欠落
- カテゴリ: セキュリティ/安全性
- 詳細: `Core.IO.Watcher` は `notify` バックエンド（inotify/FSEvents/ReadDirectoryChangesW）を `effect {io.async}` 経由で監査しているが、数千ファイルを同時監視する Phase3 Self-host シナリオで `watch.queue_size` が枯渇し、`core.io.watch.*` 診断から `metadata.io.async_queue` と Capability 情報が欠落するケースが観測された。`watcher.audit.pass_rate` 指標を導入したものの、ベースライン未確立のまま Phase3 W50 へ進むと `reports/spec-audit/ch3/watch_event-metrics.json` と CLI ゴールデンに差分が生じるリスクがある。
- 対応案: `python3 tooling/ci/collect-iterator-audit-metrics.py --section core_io --scenario watcher_audit --require-success` を Nightly CI へ組み込み、`watch.queue_size` が 80% を超えた場合は自動で `watch.delay_ns` を記録する。必要に応じて `compiler/rust/runtime/src/io/watcher.rs` にバックプレッシャ制御 (`WatcherLimits`) を追加し、`reports/spec-audit/ch3/core_io_summary-20251201.md` に週次ログを追記する。`watcher.audit.pass_rate < 0.95` を検出した際は Watcher API を `beta` Stage に格下げし、Phase4 までに恒久対応を計画する。
- 期限: 2026-01-31
- 状態: Open
- 関連フェーズ: Phase 3 (3-5)
- 参照: `docs/plans/bootstrap-roadmap/3-5-core-io-path-plan.md` §5.1, `docs/spec/3-5-core-io-path.md` §5, `reports/spec-audit/ch3/core_io_summary-20251201.md`, `docs/plans/bootstrap-roadmap/assets/core-io-effects-matrix.md`
- 担当: Phase3 Core.IO Runtime チーム（Watcher）

<a id="core-io-permission-risk"></a>
- 登録日: 2025-12-01
- タイトル: Capability Stage 誤判定によるファイル権限不足エラー多発
- カテゴリ: セキュリティ/安全性
- 詳細: `File::open/create/remove` 実装は `fs.permissions.*` Capability を `CapabilityRegistry::verify_capability_stage` で検証するが、Stage テーブルの更新遅延により POSIX/macOS CI で `effect.stage.required = "stable"` に達していないケースが発生し `core.io.file.permission_denied` が連続発火した。`core_io.file_ops_pass_rate` は 1.0 を維持しているものの、`io.error_rate` 指標が 0.04 まで上昇した場合に Root Cause を追跡する仕組みが不足している。
- 対応案: `docs/plans/bootstrap-roadmap/assets/core-io-capability-map.md` を週次で再生成し、`reports/spec-audit/ch3/file_ops-metrics.json` + `reports/spec-audit/ch3/core_io_summary-20251201.md` で Stage ミスマッチ数と `io.error_rate` をクロスチェックする。必要に応じて `compiler/rust/runtime/src/io/file.rs` の Capability 測定を二重化（`verify_capability_stage` + `IoContext.stage_override`）し、閾値（`io.error_rate > 0.02`）を超えた場合は `FsAdapter` レイヤに一時的なリトライポリシーを導入する。
- 期限: 2026-02-15
- 状態: Open
- 関連フェーズ: Phase 3 (3-5)
- 参照: `docs/plans/bootstrap-roadmap/3-5-core-io-path-plan.md` §3.1, `docs/plans/bootstrap-roadmap/assets/core-io-capability-map.md`, `reports/spec-audit/ch3/core_io_summary-20251201.md`, `docs/spec/3-8-core-runtime-capability.md`
- 担当: Phase3 Core.IO Runtime チーム（File/Capability）

<a id="core-path-symlink-risk"></a>
- 登録日: 2025-12-01
- タイトル: シンボリックリンク攻撃・サンドボックス逸脱の検知遅延
- カテゴリ: セキュリティ/安全性
- 詳細: `Path::sandbox_path` と `security::validate_path` は `metadata.security.reason` を出力するが、Windows UNC や macOS のボリュームマウントを跨ぐケースで `core.path.security.*` 診断が `warning` 止まりになり、`path.security.incident_count` の閾値を超えても自動でブロックできない。Self-host CI の設定ディレクトリに対して攻撃ベクトルが残るため、Phase3 での CLI リリースには監査ログの即応性が求められる。
- 対応案: `scripts/validate-diagnostic-json.sh --pattern core.path.security` を PR ごとに実行し、`python3 tooling/ci/collect-iterator-audit-metrics.py --section core_io --scenario path_security --require-success` の結果を `reports/spec-audit/ch3/path_security-metrics.json` と `reports/spec-audit/ch3/core_io_summary-20251201.md` に転記する。`path.security.incident_count >= 3` を検出した場合は `docs/plans/bootstrap-roadmap/3-5-core-io-path-remediation.md` §4.2 の `symlink_escape_guard` タスクを再開し、`SecurityPolicy` の既定を `deny-relative` に切り替える提案を Phase3 会議で行う。
- 期限: 2026-01-15
- 状態: Open
- 関連フェーズ: Phase 3 (3-5)
- 参照: `docs/plans/bootstrap-roadmap/3-5-core-io-path-plan.md` §4.2, `docs/spec/3-5-core-io-path.md` §4, `docs/spec/3-6-core-diagnostics-audit.md`, `reports/spec-audit/ch3/core_io_summary-20251201.md`
- 担当: Phase3 Core.IO Runtime チーム（Path/Security）

- 登録日: 2026-03-31
- タイトル: Unicode XID 識別子実装未完了（SYNTAX-001 / LEXER-001）
- カテゴリ: 技術的負債
- 詳細: 仕様は `XID_Start`/`XID_Continue` に基づく Unicode 識別子を要求しているが、Phase 2-5 時点の実装は ASCII プロファイルのみを許可していた。`lexer.identifier_profile_unicode` 指標は当時 0.0 で、多言語 DSL サンプルや CLI/LSP の補完機能に制約が残っていたため、Phase 2-7 では 1.0 を維持する運用監視が必要となる。
- 対応案: Phase 2-7 `lexer-unicode` タスクで XID テーブル生成・正規化チェック・`--lex-profile=unicode` 既定化を完了し、CI で `REML_ENABLE_UNICODE_TESTS=1` を有効化する。進捗は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` の Unicode セクションと `docs/plans/bootstrap-roadmap/2-5-proposals/SYNTAX-001-proposal.md` Step5/6 を参照する。2026-12-02 に GitHub Actions 3 プラットフォームで環境変数を既定化し、`lexer.identifier_profile_unicode = 1.0` を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` §0.3.5 へ記録済み。
- 期限: 2026-08-31
- 状態: Mitigating
- 関連フェーズ: Phase 2 (2-7)
- 参照: `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md`, `docs/plans/bootstrap-roadmap/2-5-proposals/SYNTAX-001-proposal.md`, `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md`
- 担当: Phase 2-7 Parser チーム（LEXER-001 / SYNTAX-001）
- 登録日: 2026-04-20
- タイトル: 効果構文 Stage 昇格遅延（EFFECT-POC-Stage）
- カテゴリ: 技術的負債
- 詳細: 効果構文 PoC は `Σ_before`/`Σ_after` 記録と KPI (`syntax.effect_construct_acceptance`, `effects.syntax_poison_rate`) を Step4/Step5 で仕様化したが、OCaml 実装は `effect.syntax.constructs` の算出と残余効果の控除を未実装のまま Phase 2-7 へ移管している。Stage 昇格が遅延すると Chapter 1 の脚注撤去と Phase 3 self-host 移行の前提条件に影響する。
- 対応案: Phase 2-7 `EFFECT-003` / `Type_inference_effect` タスクで `TEffectPerform`/`THandle` の残余効果計算と `collect-iterator-audit-metrics.py` 指標を実装し、CI で `syntax.effect_construct_acceptance = 1.0` / `effects.syntax_poison_rate = 0.0` を達成した上で脚注を撤去する。進捗は `docs/notes/effect-system-tracking.md` と `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` (H-O1〜H-O5) で追跡する。
- 期限: 2026-09-30
- 状態: Resolved (2026-12-18)
- 解決メモ: Phase 2-7 Sprint C で Core IR・Runtime まで効果行が伝播し、`type_row_mode` の既定値を `"ty-integrated"` へ切り替え済み。Linux/Windows/macOS の CI で `collect-iterator-audit-metrics.py --require-success --section effects` を恒常運用し、`effect_row_guard_regressions = 0` を確認した。
- 関連フェーズ: Phase 2 (2-7)
- 参照: `docs/plans/bootstrap-roadmap/2-5-proposals/EFFECT-002-proposal.md`, `docs/plans/bootstrap-roadmap/2-5-review-log.md`, `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md`, `docs/notes/effect-system-tracking.md`
- 担当: Phase 2-7 Effects チーム（SYNTAX-003 / EFFECT-002）
- 登録日: 2026-04-24
- タイトル: 効果行統合遅延による型・監査不整合（TYPE-002-ROW-INTEGRATION）
- カテゴリ: 技術的負債
- 詳細: `TYPE-002` Step4 までに `effect_row` 統合ドラフトと移行ガード (`type_row_mode = "metadata-only"`) を整備したが、`ty` へ効果行を統合する実装は Phase 2-7 へ移管している。行統合が Phase 3 までに完了しない場合、`@handles`/Stage 契約の検証が実行時メタデータ依存のままとなり、Self-host CI の合格判定や監査 KPI（`diagnostics.effect_row_stage_consistency`）が 1.0 を維持できないリスクがある。
- 対応案: Phase 2-7 Sprint A/B/C で `effect_row` dual-write → `generalize`/`instantiate`/`Type_unification` 対応 → Core IR/監査伝播を実装し、`type_row_mode` を `dual-write` → `ty-integrated` へ段階的に移行する。`collect-iterator-audit-metrics.py` の `diagnostics.effect_row_stage_consistency` / `type_effect_row_equivalence` / `effect_row_guard_regressions` が各 1.0 / 1.0 / 0.0 を満たした時点で仕様本文を更新し、暫定脚注を撤去する。
- 期限: 2026-10-31
- 状態: Open
- 関連フェーズ: Phase 2 (2-7)
- 参照: `docs/plans/bootstrap-roadmap/2-5-proposals/TYPE-002-proposal.md`, `docs/plans/bootstrap-roadmap/2-5-to-2-7-type-002-handover.md`, `docs/plans/bootstrap-roadmap/2-5-review-log.md`, `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md`, `docs/notes/effect-system-tracking.md`, `compiler/ocaml/docs/effect-system-design-note.md`, `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md#0-3-7b-効果行統合メトリクス運用2026-12-18-更新`
- 担当: Phase 2-7 Type チーム（TYPE-002）

<a id="diagnostic-domain-metrics"></a>
### 診断ドメインメトリクス監視

- 登録日: 2026-12-21
- タイトル: 診断ドメイン KPI の閾値逸脱
- カテゴリ: 技術的負債
- 詳細: Phase 2-7 で導入した `diagnostics.domain_coverage`・`diagnostics.plugin_bundle_ratio`・`diagnostics.effect_stage_consistency` は Phase 2-8 の仕様監査で必須となる。いずれかが閾値（0.95 または 1.0）を下回ると、Plugin/LSP/Capability の差分抽出が不完全となり、`docs/spec/3-6-core-diagnostics-audit.md` と CLI/LSP ゴールデンの同期が崩れるリスクがある。
- 対応案: `collect-iterator-audit-metrics.py --section diagnostics --require-success` を CI で常時動作させ、逸脱が検出された場合は `reports/audit/dashboard/diagnostics.md` の比率を確認した上で互換モードテストを停止し、`docs/notes/dsl-plugin-roadmap.md` と連携してバンドル署名の再発行と Stage 診断の再測定を行う。
- 期限: 2027-03-31
- 状態: Monitoring
- 関連フェーズ: Phase 2 (2-7, 2-8)
- 参照: `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` §5、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md#0-3-7c-診断ドメイン可視化メトリクス運用2026-12-21-更新`、`reports/audit/dashboard/diagnostics.md`, `reports/audit/phase2-7/diagnostics-domain-20261221.json`
- 担当: Phase 2-8 Diagnostics/Plugin チーム
- <a id="diagnostic-lsp-fixture-drift"></a>
- 登録日: 2029-07-05
- タイトル: LSP フィクスチャのスキーマドリフト
- カテゴリ: 互換性
- 詳細: `tooling/lsp/tests/client_compat/fixtures/diagnostic-v2-*.json` が Phase 2-4 時点の `schema_version = "2.0.0-draft"`・`severity = 1` など旧表現のまま凍結されており、Rust Frontend が出力する `schema_version = "3.0.0-alpha"`（`compiler/rust/frontend/src/bin/reml_frontend.rs:150-229` + `compiler/rust/frontend/src/diagnostic/json.rs:5-190`）や `severity = "error"` との整合を検証できない。`jq '.[].schema_version' tooling/lsp/tests/client_compat/fixtures/diagnostic-v2-sample.json` → `"2.0.0-draft"`、`jq '.[0] | has("span_trace")' tooling/lsp/tests/client_compat/fixtures/diagnostic-v2-stream.json` → `false` の通り、`span_trace`／`effect.stage.*` が欠落し LSP CI で Stage/Trace 回帰を検知できないリスクが顕在化している。
- 進捗: Run ID `20290705-cli-output` で `reml_frontend --output lsp` を実装し、CLI から LSP 互換メッセージを生成できるようになった。引き続きフィクスチャを Rust 版出力で再生成し、`schema_version = "3.0.0-alpha"` と `structured_hints` の反映を完了させる必要がある。
- 対応案: フィクスチャと `tooling/lsp/tests/client_compat` のスキーマを `schema_version = "3.0.0-alpha"` へ更新し、`severity` を文字列 Enum に揃えた上で `span_trace`・`effect.stage.*`・`structured_hints` を Rust 出力からダンプするリジェネレーターを追加する。更新後は `npm run ci --prefix tooling/lsp/tests/client_compat` と `scripts/validate-diagnostic-json.sh --suite lsp --effect-tag diagnostic` をゲートに設定し、`reports/diagnostic-format-regression.md` の CLI/LSP 差分節（§1.1）へ Run ID を追記する。
- 期限: 2029-08-15
- 状態: Open
- 関連フェーズ: Phase 3 (3-6)
- 参照: `docs/plans/bootstrap-roadmap/3-6-core-diagnostics-audit-plan.md#1-3`, `reports/diagnostic-format-regression.md#cli-output-note`, `tooling/lsp/tests/client_compat/fixtures/diagnostic-v2-sample.json`, `tooling/lsp/tests/client_compat/fixtures/diagnostic-v2-stream.json`
- 担当: Phase 3 Diagnostics/LSP チーム
- 登録日: 2027-03-30
- タイトル: Unicode Data Drift（R-041）
- カテゴリ: 互換性
- 詳細: Core.Text/Unicode のサンプル（`examples/core-text/text_unicode.reml`）と `docs/spec/3-3-core-text-unicode.md` のコード例が更新されないまま Unicode データを入れ替えると、AI 連携や Streaming decode で CLI/LSP と異なる正規化結果が生成される。`expected/text_unicode.*.golden` や `reports/spec-audit/ch1/core_text_examples-YYYYMMDD.md` が古い場合、`text.grapheme.cache_hit` KPI と `Unicode::VERSION` が乖離し、`InvalidUtf8` や幅計算の差分が検知できなくなる。
- 対応案: Unicode バージョン更新時は必ず `cargo run --manifest-path compiler/rust/runtime/Cargo.toml --bin text_stream_decode -- --input tests/data/unicode/streaming/sample_input.txt --output examples/core-text/expected/text_unicode.stream_decode.golden` を再実行し、`examples/core-text/expected/text_unicode.{tokens,grapheme_stats}.golden` を同じコミットで更新する。CI では `tooling/ci/collect-iterator-audit-metrics.py --section text --scenario grapheme_stats --source examples/core-text/expected/text_unicode.grapheme_stats.golden --require-success` を追加し、逸脱時は `docs/notes/text-unicode-known-issues.md` (TUI-004) に記録する。
- 期限: 2027-06-30
- 状態: Monitoring
- 関連フェーズ: Phase 3 (3-3)
- 参照: `examples/core-text/README.md`, `docs/guides/core-parse-streaming.md` §11, `docs/guides/ai-integration.md` §6, `reports/spec-audit/ch1/core_text_examples-20270330.md`
- 担当: Phase 3 Core.Text チーム
- 登録日: 2025-10-10
- タイトル: Debian sysroot アーカイブのハッシュ未確定
- カテゴリ: 互換性
- 詳細: macOS → Linux x86_64 クロスコンパイルで使用する `tooling/toolchains/cache/debian-bookworm-x86_64.tar.zst` の SHA-256 を算出し、`tooling/toolchains/versions.toml` および `tooling/toolchains/checksums.txt` に `49b9ee8917f7235b6f20aaff3f983d616c53f29354ad180782ed024186df5452` を登録済み。
- 対応案: 登録済みの値を四半期レビューで検証し、sysroot 更新時は再計算して報告する。
- 期限: 2025-10-10
- 状態: Resolved
- 関連フェーズ: Phase 1
- 参照: `tooling/toolchains/versions.toml`, `tooling/toolchains/checksums.txt`, `docs/plans/bootstrap-roadmap/1-5-runtime-integration.md` §10
- 登録日: 2025-10-12
- タイトル: Homebrew 版 LLVM の頻繁な更新による再現性低下
- カテゴリ: 互換性
- 詳細: Phase 1-8 で使用予定の `brew install llvm@15` が頻繁に更新され、CI とローカル環境で異なるビルド番号が導入される懸念がある。`llvm-config` の出力差異が出た場合、IR 検証に失敗する可能性がある。
- 対応案: `brew extract` によるフォーミュラ固定、もしくは GitHub Actions 内でのバイナリアーカイブ展開を採用する。決定後は `docs/notes/llvm-spec-status-survey.md` に手順を記録し、`bootstrap-macos.yml` に反映する。
- 期限: 2025-10-26
- 状態: Open
- 関連フェーズ: Phase 1 (1-8)
- 参照: `docs/plans/bootstrap-roadmap/1-8-macos-prebuild-support.md` §3, `docs/notes/llvm-spec-status-survey.md`
- 登録日: 2025-10-12
- タイトル: GitHub Actions macOS ランナーの起動待ち時間長期化
- カテゴリ: スケジュール
- 詳細: macOS ランナーのジョブ待機時間が 15 分を超えるケースが増加すると、Phase 1-8 の CI フィードバックが遅延し、`ci_build_time_macos` 指標の収集に影響が出る。Linux CI よりもスループットが低下する懸念がある。
- 対応案: 待機時間が 10 分を超えた状態が 2 週間継続した場合、セルフホストランナー導入またはジョブスケジュールの週次バッチ化を検討する。判断結果を `docs/plans/bootstrap-roadmap/1-8-macos-prebuild-support.md` §7 に追記する。
- 期限: 2025-11-02
- 状態: Open
- 関連フェーズ: Phase 1 (1-8)
- 参照: `docs/plans/bootstrap-roadmap/1-8-macos-prebuild-support.md` §2, `0-3-audit-and-metrics.md`
- 登録日: 2025-10-12
- タイトル: Linux CI フォーマットジョブで ocamlformat が未導入
- カテゴリ: 技術的負債
- 詳細: `Bootstrap Linux CI` の Lint ステージにおいて `opam exec -- dune build @fmt` が `ocamlformat` 実行ファイル不在で失敗している。Phase 1-8 の macOS CI 着手条件として Linux CI の健全性を維持する必要があり、Lint チェックの継続失敗は PR マージのボトルネックとなる。
- 対応案: `ocamlformat` バージョン固定（`dune-project` での `using fmt` 宣言と `opam install` による dev-dependency 化）を実施し、暫定的には Lint ステージで `opam install ocamlformat.0.26.2 --yes` を追加する。修正完了後は GitHub Actions ログと `compiler/ocaml/README.md` の進捗欄に復旧記録を残す。
- 期限: 2025-10-14
- 状態: Mitigating
- 関連フェーズ: Phase 1 (1-8)
- 参照: `.github/workflows/bootstrap-linux.yml`, `docs/plans/bootstrap-roadmap/1-8-macos-prebuild-support.md` §0, `docs/plans/bootstrap-roadmap/1-7-to-1-8-handover.md`

## 0.4.7 Core.IO リーク検出フォローアップ
- `compiler/rust/runtime/src/io/scope.rs` のリークトラッカー (`leak_tracker_snapshot`, `reset_leak_tracker`) と `reports/spec-audit/ch3/io_leak-detection.md` を参照し、`cargo test --manifest-path compiler/rust/runtime/Cargo.toml leak_detection::scoped_resources_cleanup_matches_expected_snapshot` を週次運用に組み込む。
- `tests/data/core_io/leak_detection/scoped_cleanup.json` の期待値（`open_files = 0`, `temp_dirs = 0`）が逸脱した場合は `valgrind`/`miri` チェックの導入計画を本節へ追記し、`docs/plans/bootstrap-roadmap/3-5-core-io-path-plan.md#33` の TODO と同期する。
