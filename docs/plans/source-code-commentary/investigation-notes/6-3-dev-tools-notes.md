# 調査メモ: 第21章 開発支援ツール

## 対象モジュール

- `compiler/xtask/README.md`
- `compiler/xtask/src/main.rs`
- `tooling/README.md`
- `tooling/ci/README.md`
- `tooling/ci/ci-validate-audit.sh`
- `tooling/runtime/README.md`
- `tooling/review/README.md`
- `tooling/scripts/README.md`
- `tooling/json-schema/validate-diagnostic-json.sh`
- `docs/guides/tooling/audit-metrics.md`

## 入口と全体像

- 開発支援ツールの中心は `compiler/xtask` クレートで、`cargo xtask` 経由の補助タスクを定義する。現状のサブコマンドは `prelude-audit` のみ。
  - `compiler/xtask/README.md:1-12`
  - `compiler/xtask/src/main.rs:16-163`
- `tooling/` は CI/監査/レビュー/配布などの周辺ツール群の集約ディレクトリで、`tooling/README.md` に全体像が整理されている。
  - `tooling/README.md:1-21`
- 監査・メトリクス運用は `docs/guides/tooling/audit-metrics.md` に集約され、診断 JSON 検証・監査メトリクス集計・監査インデックス生成を入力としている。
  - `docs/guides/tooling/audit-metrics.md:1-66`

## 主要データとパラメータ

- `prelude-audit` はインベントリ TOML を読み取り、`--wbs` / `--section` / `--module` / `--strict` 等で絞り込みを行う。
  - インベントリ既定: `docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml`
  - `compiler/xtask/src/main.rs:45-163`
- `SectionFilter` は `Option` / `Result` / `Iter` / `Collector` のモジュール名と一致判定を行う。
  - `compiler/xtask/src/main.rs:200-245`
- 監査スキーマ検証は `tooling/runtime/audit-schema.json` を参照し、`tooling/ci/ci-validate-audit.sh` が JSON/JSONL を Ajv で検証する。
  - `tooling/ci/ci-validate-audit.sh:1-149`

## 監査・メトリクス運用フロー

- 診断 JSON の検証は `tooling/json-schema/validate-diagnostic-json.sh` が担い、スキーマの既定パスや対象ディレクトリを定義している。
  - `tooling/json-schema/validate-diagnostic-json.sh:1-108`
- 監査メトリクスの運用方針・出力先・KPI は `docs/guides/tooling/audit-metrics.md` で規定される。
  - `docs/guides/tooling/audit-metrics.md:11-59`
- `tooling/ci/README.md` は CI 補助スクリプトの配置方針と `record-metrics.sh` の利用例を示す。
  - `tooling/ci/README.md:1-24`
- 監査ログの差分・可視化は `tooling/review/` 配下の Python ツール群を使う。
  - `tooling/review/README.md:1-12`

## 補助ツールの配置

- `tooling/runtime/` は監査スキーマと Capability 一覧生成の補助データを保持する。
  - `tooling/runtime/README.md:1-11`
- `tooling/scripts/` は単発メンテナンス用スクリプトの置き場。
  - `tooling/scripts/README.md:1-11`

## TODO / 不明点

- `docs/guides/tooling/audit-metrics.md` が参照する `scripts/validate-diagnostic-json.sh` は存在せず、実体は `tooling/json-schema/validate-diagnostic-json.sh` になっている。
- `docs/guides/tooling/audit-metrics.md` に記載されている `tooling/ci/collect-iterator-audit-metrics.py` と `tooling/ci/sync-iterator-audit.sh` がリポジトリ内で見つからない（`find` では未検出）。
- `tooling/ci/ci-validate-audit.sh` の usage 表記が `scripts/ci-validate-audit.sh` を名乗っており、配置パスと差異がある。
  - `tooling/ci/ci-validate-audit.sh:13-22`
