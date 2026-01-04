# 3.0 標準ライブラリ仕様 概要

## 概要
標準ライブラリ章は `Core.*` モジュール群の契約と設計判断を集約し、言語コア仕様で定義した概念を実際の API とユーティリティとして提示します。プレリュードから環境連携までの各モジュールが相互運用できるよう、効果・診断・Capability ポリシーを横断して調整しています。

## セクションガイド
- [3.1 プレリュードと反復制御](3-1-core-prelude-iteration.md): 失敗制御ヘルパと `Iter<T>` の生成・変換 API、パイプライン運用および診断連携をまとめます。`Option.ok_or` による遅延エラー生成など `Option`/`Result` の橋渡しが追加されています。テンプレート DSL における `ensure`/`ensure_not_null` の実例は `examples/language-impl-samples/reml/prelude_guard_template.reml` で `core.prelude.ensure_failed` メタデータ付きの診断とともに検証しています。【F:../examples/language-impl-samples/reml/prelude_guard_template.reml†L9-L165】
- [3.2 コレクション](3-2-core-collections.md): 永続／可変コレクションの API と性能指針、Iter/Collector 連携、監査ワークフローへの接続を定義します。`Map.from_pairs` により初期マップ構築を安全に行うユーティリティも導入しています。
- [3.3 テキストと Unicode サポート](3-3-core-text-unicode.md): 文字列層構造、正規化・境界判定・検索 API を通じて Unicode モデルと IO/診断連携をカバーし、`Core.Diagnostics` が `display_width` を通じてハイライト幅を揃える統合手順を提示します。
- [3.4 数値演算と時間管理](3-4-core-numeric-time.md): 数値プリミティブ、統計ユーティリティ、時間/期間型とタイムゾーン処理、監査メトリクス連携を整理します。
- [3.5 入出力とパス操作](3-5-core-io-path.md): Reader/Writer 抽象、ファイル・ストリーム操作、Path セキュリティヘルパ、同期/非同期ブリッジを定義します。[examples/practical/core_io/file_copy/canonical.reml](../../examples/practical/core_io/file_copy/canonical.reml) と [examples/practical/core_path/security_check/relative_denied.reml](../../examples/practical/core_path/security_check/relative_denied.reml) を参照しながら `IoContext` と `SecurityPolicy` の監査メタデータを追跡できるように更新しました（`tooling/examples/run_examples.sh --suite core_io|core_path` で実行）。
- [3.6 診断と監査](3-6-core-diagnostics-audit.md): `Diagnostic` 構造と監査ログ、プライバシー制御、CLI/LSP 統合やメトリクス連携のベストプラクティスに加えて、`AuditEvent` の標準タクソノミーと必須メタデータを定義します。`StructuredHint`/`FixIt`/`TraceFrame`/`audit_metadata` など Phase 3 で追加されたフィールドは CLI/LSP/AI が共有する `schema_version = "3.0.0-alpha"` の JSON に揃えられており、`examples/core_diagnostics/` で CLI ゴールデン（`*.expected.diagnostic.json` / `*.expected.audit.jsonl`）を維持し、`tooling/examples/run_examples.sh --suite core_diagnostics --update-golden` で再生成できるように整備しました。
- [3.7 設定とデータ管理](3-7-core-config-data.md): `reml.toml` マニフェスト、Config/Data スキーマ API、互換モード `ConfigCompatibility` と監査連携、マイグレーション安全性に加えて、`DslExportSignature.requires_capabilities` と Stage 範囲の同期手順を定義します。
- [3.8 ランタイムと Capability レジストリ](3-8-core-runtime-capability.md): Capability Registry の構造、セキュリティモデル、各 Capability の概要と DSL プロファイル生成フローに加え、Runtime Bridge 契約と Stage/監査ポリシー、外部マニフェストを Reml 形式へ正規化する `transform_capability_manifest_to_reml` ユーティリティを解説します。`Core.Native` / 埋め込み API の Stage 監査も本章で扱い、`native.intrinsic` / `native.embed` のキー体系を統一します。`reml_capability list --format json` の出力（`reports/spec-audit/ch3/capability_list-20251205.json`）を `scripts/capability/generate_md.py` で再利用することで、本章のテーブルと README のスナップショットを常に最新 Stage/Provider 情報に揃えられるようになっています。
- [3.9 非同期・FFI・アンセーフ](3-9-core-async-ffi-unsafe.md): 非同期実行モデル、FFI サンドボックス、`Core.Unsafe` 指針と Capability 連携、`ExecutionPlan` 静的検証とセキュリティ/性能最適化を扱います。`SupervisorSpec` / `RestartStrategy` による標準 Supervisor パターンと診断・監査の連携手順もここで定義します。
- [3.10 環境機能とプラットフォーム連携](3-10-core-env.md): 環境変数アクセス、プラットフォーム情報取得、`REML_CONFIG_*` による互換フラグ供給、`@cfg` 連携ガイドラインと `Core.System.Env` への統合方針を提供します。
- [3.11 テスト基盤](3-11-core-test.md): ゴールデン/スナップショット/ファジングの標準 API と診断・監査の連携を定義します。
- [3.12 CLI 基盤](3-12-core-cli.md): 宣言的 CLI 仕様、ヘルプ生成、解析エラーの診断ルールをまとめます。
- [3.13 プリティプリンタ](3-13-core-text-pretty.md): `Doc` コンビネータとレイアウト規則を定義し、フォーマッタ実装の基盤とします。
- [3.14 LSP ツールキット](3-14-core-lsp.md): LSP 型と JSON-RPC ヘルパを標準化し、診断ブリッジの経路を整理します。
- [3.15 ドキュメント生成](3-15-core-doc.md): ドキュメントコメント抽出・レンダリング・Doctest の最小仕様を定義します。
- [3.16 DSL パラダイムキット](3-16-core-dsl-paradigm-kits.md): `Core.Dsl.Object`/`Core.Dsl.Gc`/`Core.Dsl.Actor`/`Core.Dsl.Vm` の最小 API を整備し、DSL 実装の意味論基盤を標準化します。
- [3.17 ネットワーク基盤](3-17-core-net.md): URL 解析 → HTTP リクエスト送信の最小シナリオを軸に、HTTP クライアント/TCP/UDP/URL の最小 API と `effect {net}` を定義し、Capability/監査ログと整合する基盤を提示します。TLS/HTTP2 は Phase 5 拡張項目として整理します。
- [3.18 システム統合](3-18-core-system.md): `Core.System` を標準ライブラリの OS 連携窓口として定義し、`Process`/`Signal`/`Env`/`Daemon` の安全 API と Capability ブリッジの境界を整理します。

標準ライブラリの再エクスポートは `use Core.Parse.{Lex, Op.{Infix, Prefix}}` のような多段ネスト `use` を前提に整理されており、Phase 2-5 `SYNTAX-002` 計画 S5（2025-11-12 更新）でテストと測定指標が揃った。詳細は [`../plans/bootstrap-roadmap/2-5-proposals/SYNTAX-002-proposal.md`](../plans/bootstrap-roadmap/2-5-proposals/SYNTAX-002-proposal.md) と [0-3-audit-and-metrics.md](../plans/bootstrap-roadmap/0-3-audit-and-metrics.md) の `parser.use_nested_support` を参照して最新状態を確認すること。Capability Stage の検証や `effects.contract.stage_mismatch` のサンプルは `examples/core_diagnostics/pipeline_branch.reml` と `reports/spec-audit/ch3/capability_stage-mismatch-20251206.json` に収録されており、Core.Runtime が記録する `capability.*` / `effect.stage.*` メタデータの流れを Chapter 3.6/3.8 から一貫して追跡できる。
