# 3.1 監査・オブザーバビリティ整合計画

Phase P3 では dual-write CI を通じて Rust 実装の観測データを既存の監査基盤へ統合する。本書は `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の指標群を Rust 版へ拡張し、`reports/audit/` のダッシュボードに同一形式で集約するための要件・実装手順・フォローアップを整理する。

## 3.1.1 目的
- OCaml / Rust で生成される `Diagnostic`・`AuditEnvelope`・ストリーミングメトリクスの差異を可視化し、`collect-iterator-audit-metrics.py` の pass/fail 判定を Rust 実装に対応させる。
- 監査・ログ・トレース・メトリクスを同一リポジトリ内に保存し、`reports/audit/dashboard/*.md` と `reports/iterator-stage-summary*.md` のフォーマットでプラットフォーム別に比較できる状態を維持する。
- P3 完了後に Rust 実装を default へ切り替える際の証跡として、差分トリアージ・閾値逸脱の履歴・再発防止策を継続的に記録する。

## 3.1.2 スコープと前提
- **対象**: `tooling/ci/collect-iterator-audit-metrics.py`、`tooling/ci/create-audit-index.py`、`reports/audit/**`、`reports/iterator-stage-summary*.md`、`reports/diagnostic-format-regression.md`。
- **除外**: ベンチマーク指標（`3-2-benchmark-baseline.md`）、CI フロー自体の整備（`3-0-ci-and-dual-write-strategy.md`）、Rust 実装内のロギング API 設計（別途ランタイム設計で扱う）。
- **前提条件**:
  - P2 で導入した監査指標 (`stage_mismatch_count`, `diagnostic.audit_presence_rate`, ほか) が OCaml CI で安定運用されている。
  - `collect-iterator-audit-metrics.py` が baseline/candidate モードをサポートし、`--platform` および `--section` オプションの組み合わせを Rust 実装でも再利用できる。
  - `reports/audit/index.json` とダッシュボードの更新手順が [reports/audit/dashboard/diagnostics.md](../../../reports/audit/dashboard/diagnostics.md) で共有済み。

## 3.1.3 データソースと保存先
| ドメイン | ソースコマンド | OCaml 保存先 | Rust 保存先 (新規/拡張) |
| --- | --- | --- | --- |
| 診断 JSON | `remlc --frontend {ocaml,rust} --emit diagnostics` | `compiler/ocaml/tests/golden/diagnostics/*.json.golden` | `reports/dual-write/<platform>/diagnostics-rust.json` |
| 監査メトリクス | `collect-iterator-audit-metrics.py --section diagnostics --require-success` | `tooling/ci/iterator-audit-metrics.json` | `tooling/ci/iterator-audit-metrics-rust.json` |
| ストリーミング統計 | `collect-iterator-audit-metrics.py --section streaming` | `reports/audit/streaming-*.json` | `reports/audit/streaming-*-rust.json` |
| FFI/Capability 監査 | `collect-iterator-audit-metrics.py --section effects` | `reports/audit/phase2-7/effects/*.json` | `reports/audit/phase3/effects-rust/*.json`（新設） |
| Windows 環境診断 | `tooling/toolchains/check-windows-bootstrap-env.ps1` | `reports/windows-env-check*.json` | `reports/windows-env-check-rust.json` |

`reports/audit/index.json` には `kind=dual-write` のセクションを追加し、OCaml/Rust/dual-write の 3 系統が一覧できるよう `ci:linux:dual-write@rust` のような識別子を導入する。

## 3.1.4 指標マッピング
`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` で定義された主要指標への対応状況を以下に整理する。Rust 実装では既存スクリプトをそのまま利用できるが、`frontend` ラベルをメタデータに追加して区別する。

| 指標名 | OCaml 側収集方法 | Rust 側追加作業 | 備考 |
| --- | --- | --- | --- |
| `iterator.stage.audit_pass_rate` | `collect-iterator-audit-metrics.py --require-success` | Rust 版 `AuditEnvelope` に Stage キーを実装し、dual-write 比較時に `--baseline/--candidate` で同一 JSON を比較 | P3 で pass_rate = 1.0 を維持する |
| `diagnostic.audit_presence_rate` | `validate-diagnostic-json.sh` で schema 検証 | Rust CLI で `audit` フィールドを JSON/LSP 両方に埋め込み、`reports/diagnostic-format-regression.md` フローへ組み込む | 欠落時は dual-write ジョブが失敗する |
| `typeclass.metadata_pass_rate` | ゴールデン JSON の `extensions.typeclass.*` | Rust AST 生成時に辞書情報を再構築。`1-1-ast-and-ir-alignment.md` の対応表を参照 | 同等の IndexMap 順序を保証 |
| `parser.stream.packrat_hit` | `collect-iterator-audit-metrics.py --section streaming` | Rust パーサで Packrat 統計を導入し、`reports/audit/streaming-*-rust.json` へ出力 | `docs/guides/core-parse-streaming.md` 参照 |
| `effect.capability_array_pass_rate` | `collect-iterator-audit-metrics.py --section effects` | Rust 効果解析で `required_capabilities` 等を維持し、`AuditEnvelope.metadata` へ格納 | `EFFECT-002-proposal.md` で定義した配列順序を踏襲 |
| `collector.effect.mem_reservation_hits` | `collect-iterator-audit-metrics.py --section collector --baseline ocaml`（WBS 3.1b 以降で有効化） | Rust の `core_prelude::collectors` が出力する `Diagnostic.extensions["prelude.collector"]` / `AuditEnvelope.metadata.collector.*` を収集し、`CollectOutcome` に記録された Stage/Effect ラベルを JSON へ転写する | `docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md` WBS 3.1b F1、`core_prelude` の EffectMarker (`collector.effect.{mem_reservation,reserve,finish}`) を参照 |

`collector.effect.*` 系の数値は `core_prelude::collectors` が提供する `CollectError`/`CollectOutcome` を経由して Rust 側 `Diagnostic` と `AuditEnvelope` へ書き出す。`collector.effect.mem_reservation_hits` は `with_capacity`/`reserve`/`finish` が付与する EffectMarker (`collector.effect.mem_reservation` / `collector.effect.reserve` / `collector.effect.finish`) を参照し、OCaml 版の監査 JSON と同じキー（`prelude.collector` 拡張配下）で比較できるようにする。

## 3.1.5 ログ・トレース整備
- **構造化ログ**: Rust 実装の `tracing` crate で `diagnostic.emit`、`effect.audit`、`parser.streaming` のイベントを JSON 出力。`reports/dual-write/logs/<run-id>.jsonl` に保存し、OCaml 実装のログと同じキー (`trace_id`, `stage`, `capability`, `origin`) を持つようにする。
- **トレース計測**: `iterator.stage.audit_pass_rate` の検証で必要な Stage トレースを `reports/iterator-stage-summary-rust.md` に出力。`tooling/ci/sync-iterator-audit.sh` を拡張し、Rust 版でも `verify-log` を生成する。
- **ダッシュボード連携**: `reports/audit/dashboard/*.md` を更新し、各テーブルに `Rust` 列と `Dual` 列を追加。閾値逸脱時のエスカレーション先は `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` に統一する。

## 3.1.6 実装ステップ
1. **スキーマ整合テスト**: Rust 実装の診断 JSON を `scripts/validate-diagnostic-json.sh` に通し、`diagnostic.audit_presence_rate` ≥ 1.0 を確認。失敗したキーは `collect-iterator-audit-metrics.py --dump-missing` で特定する。
2. **監査インデックス拡張**: `tooling/ci/create-audit-index.py` に `--tag dual-write` オプションを追加し、`reports/audit/index.json` へ Rust 項目を登録。`tooling/ci/verify-audit-metadata.py --strict` で整合性を検証。
3. **ダッシュボード拡張**: `reports/audit/dashboard/diagnostics.md` / `streaming.md` に Rust 列を追加し、差分が 0.95 未満となった場合は自動で issue を開く GitHub Actions ジョブ (`audit-dashboard-regression.yml`) を追加検討。
4. **ログローテーション**: `reports/dual-write/logs/` と `reports/audit/phase3/` に保存するファイルは 30 日でクリーンアップするスクリプト (`tooling/ci/prune-observability-artifacts.py`) を整備。
5. **レビュー共有**: 週次の `dual-write` レビューで監査差分を確認し、`reports/audit/dashboard/changelog.md` に更新履歴を追記。重大な欠落が見つかった場合は `docs/notes/observability-gap-analysis.md`（新設予定）に調査ノートを残す。

## 3.1.7 エスカレーションとアラート
- `iterator.stage.audit_pass_rate < 1.0`：dual-write ジョブを失敗扱いにし、`CI_FAILURE_TYPE=audit-gap` を付与。`3-0-ci-and-dual-write-strategy.md` の手順で triage。
- `diagnostic.audit_presence_rate < 1.0`：`collect-iterator-audit-metrics.py` の `--require-success` が失敗。Rust CLI のシリアライズ層へ差し戻し。
- `streaming.metrics` の欠落：`reports/audit/streaming-*-rust.json` が出力されない場合、Rust パーサが `packrat_stats` を実装していない可能性あり。`1-0-front-end-transition.md` のストリーミング実装タスクを再確認。
- `Windows env diff`：`reports/windows-env-check-rust.json` が OCaml 版と異なる場合は `0-2-windows-toolchain-audit.md` の fallback 手順を参照し、Dual 管理に戻す。

## 3.1.8 リスクとフォローアップ
- **メトリクス計算コスト**: dual-write で 2 倍の JSON を処理するため `collect-iterator-audit-metrics.py` の実行時間が増加。`--profile` オプションを導入し、並列化 (`--jobs`) を検討する。
- **スキーマドリフト**: Rust 実装で新しいフィールドが増えると OCaml 側の検証が失敗する可能性がある。追加フィールドは `docs/spec/3-6-core-diagnostics-audit.md` に脚注を入れ、`appendix/glossary-alignment.md` で用語を統一。
- **ログ肥大化**: `reports/dual-write/logs/` が肥大化しやすい。週次で自動削除し、長期保存が必要なログは `reports/audit/history/` へ手動移動。
- **監査ダッシュボードの更新漏れ**: Rust 専用列が更新されないと数字が stale になる。P3 中は週次レビュー時に `diagnostics.md` / `streaming.md` を必ず更新するチェック項目を追加。

---

本計画は dual-write で得られた観測データを統一的に扱うための指針である。ベンチマーク指標の整合については [3-2-benchmark-baseline.md](3-2-benchmark-baseline.md)、CI 実行フローの詳細は [3-0-ci-and-dual-write-strategy.md](3-0-ci-and-dual-write-strategy.md) を参照する。
