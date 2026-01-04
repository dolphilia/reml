# DSLファースト導入ガイド

Reml の DSL ファースト戦略をプロジェクトへ導入するための実践ガイド。`5-1-dsl-first-development.md` の仕様書を踏まえ、現場での立ち上げ手順とチェックリストを整理する。段階的学習ポリシーを守るため、代数的効果やハンドラなどの高度機能は `-Z` 実験フラグを併用し、DSL ごとに `stage = Experimental | Beta | Stable` を宣言することを推奨する。

## 1. 背景と準備

- Reml の価値観（実用性能・安全性・可観測性）をプロジェクト関係者と共有する。
- Core.Parse / Core.Async / Core.Diagnostics の対応バージョンを確認し、依存を固定する。
- Capability Registry の初期設定（FFI、Async、Diagnostics）を `runtime.cap.toml` 等で準備する。
- DSL マニフェストに `stage` を記録し、Experimental の DSL はサンドボックス環境での PoC 専用とする。Beta 以上へ昇格させる際は `docs/guides/ecosystem/manifest-authoring.md` の互換チェックリストを完了する。

## 2. 導入ステップ

1. **コンセプト選定** — DSL に切り出すドメイン境界を定義し、既存仕様のどこまで流用するか決める。
2. **最小DSL実装** — Core.Parse で `rule` を実装し、サンプル入力に対するパース確認を行う。
3. **Conductor 統合** — `conductor` ブロックを作成し、DSL間依存・リソース上限・監視設定を宣言する。
4. **FFI/Capability 検証** — 外部システム連携が必要な場合は `auto_bind` と `FfiCapability` を使って安全性を確認する。
5. **観測性セットアップ** — Core.Diagnostics の DSL メトリクスを登録し、トレース/監査が期待通りに収集されるか検証する。

## 3. 課題と軽減策

| 課題 | 対策 |
| --- | --- |
| 学習コスト | テンプレートプラグインを利用し、最小DSLから段階的に拡張する。 |
| 初期コスト | DSLをフェーズ分割し、優先度の高い機能から導入する。 |
| デバッグ難度 | `label`/`recover` を積極的に付与し、Conductor の `monitoring` セクションでトレースを収集する。 |
| エコシステム断片化 | ガイド・テンプレートを共通リポジトリから配布し、Capability 設定を共有する。 |

## 4. チェックリスト

- [ ] Conductor 定義に `depends_on` とリソース制限が明示されている。
- [ ] `ExecutionPlan` とバックプレッシャー設定が Core.Async の実装と一致している。
- [ ] DSLごとのメトリクス (`dsl.latency`, `dsl.throughput` など) が監視基盤へ送信されている。
- [ ] 失敗時の監査ログが `AuditEnvelope` に `dsl_id` と原因を含めて出力される。

## 5. 参考資料

- [5-1 Reml実用プロジェクト開発：DSLファーストアプローチ](../5-1-dsl-first-development.md)
- [1-1 構文仕様（Conductor節）](../../spec/1-1-syntax.md)
- [3-9 Core Async / FFI / Unsafe](../../spec/3-9-core-async-ffi-unsafe.md)
- [3-6 Core Diagnostics & Audit](../../spec/3-6-core-diagnostics-audit.md)
