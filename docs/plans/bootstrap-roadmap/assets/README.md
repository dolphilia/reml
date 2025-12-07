# Phase4 資産付録

`docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` で使用する `category` と `spec.chapter` の組み合わせを以下に整理する。分類ルールは Phase4 M1 の棚卸し作業で確定したもので、Chapter 1〜3 の仕様書に記載された `.reml` サンプルを例示しつつリンクを併記する。

## category と spec.chapter の対応表

| category | spec.chapter | 代表 anchor | 代表資産例 | 補足 |
| --- | --- | --- | --- | --- |
| Prelude | `chapter1.syntax`, `chapter3.prelude` | `docs/spec/1-1-syntax.md§B`, `docs/spec/3-0-core-library-overview.md§3.1` | `docs/spec/1-1-syntax/examples/use_nested.reml`, `examples/language-impl-comparison/reml/prelude_guard_template.reml` | モジュール/束縛/プレリュード API の基本挙動。Chapter 1 の構文例と Chapter 3.1 の `Option`/`Result` サンプルを統合して追跡する。 |
| IO | `chapter3.io` | `docs/spec/3-5-core-io-path.md§7` | `examples/core_io/file_copy.reml`, `examples/core_path/security_check.reml` | Core.IO / Core.Path の Reader/Writer, SecurityPolicy を扱う。`reports/spec-audit/ch3/core_io_summary-20251201.md` の監査指標と紐付ける。 |
| Capability | `chapter1.effects`, `chapter3.runtime` | `docs/spec/1-3-effects-safety.md§I`, `docs/spec/3-0-core-library-overview.md§3.8` | `docs/spec/1-1-syntax/examples/effect_handler.reml`, `examples/core_diagnostics/pipeline_branch.reml` | 効果タグ・Capability Stage・handler 契約の整合を検証する資産。`effects.contract.*` 診断を優先的にカバーする。 |
| Runtime | `chapter3.diagnostics`, `chapter2.streaming` | `docs/spec/3-0-core-library-overview.md§3.6`, `docs/spec/2-7-core-parse-streaming.md` | `examples/core_diagnostics/pipeline_branch.reml`, `docs/spec/1-1-syntax/examples/use_nested_rustcap.reml` | CLI/Streaming/Audit 連携を検証する資産。Stage mismatch や `StreamFlowState` 連携のログを `reports/spec-audit/ch1/ch3` と同期させる。 |
| Plugin | `chapter3.runtime`, `chapter3.config` | `docs/spec/3-0-core-library-overview.md§3.7`, `docs/spec/3-8-core-runtime-capability.md` | `examples/core_config/dsl/audit_bridge.reml`, `examples/core_config/dsl/telemetry_bridge.reml` | `@dsl_export`/Manifest 同期と Capability Registry 連携を扱う。`manifest dump` の diff を expected として管理する。 |
| CLI | `chapter1.syntax`, `chapter3.diagnostics` | `docs/spec/1-0-language-core-overview.md§4.1`, `docs/spec/3-6-core-diagnostics-audit.md` | `examples/cli/trace_sample.reml`, `examples/cli/type_error.reml` | `tooling/examples/run_examples.sh --suite cli` で再生されるケースや dual-write ログを対象に、CLI 設定・診断キー (`parser.runconfig.*` など) を固定化する。 |

> **運用メモ**: 上表は Phase4 M1 の出口条件に合わせた初期値であり、新しいカテゴリが必要になった場合は `docs/spec/0-2-glossary.md` を同時に更新し、`phase4-scenario-matrix.csv` へ投入する前に本表へ追記する。
