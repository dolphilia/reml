# 3.0 標準ライブラリ仕様 概要

## 概要
標準ライブラリ章は `Core.*` モジュール群の契約と設計判断を集約し、言語コア仕様で定義した概念を実際の API とユーティリティとして提示します。プレリュードから環境連携までの各モジュールが相互運用できるよう、効果・診断・Capability ポリシーを横断して調整しています。

## セクションガイド
- [3.1 プレリュードと反復制御](3-1-core-prelude-iteration.md): 失敗制御ヘルパと `Iter<T>` の生成・変換 API、パイプライン運用および診断連携をまとめます。`Option.ok_or` による遅延エラー生成など `Option`/`Result` の橋渡しが追加されています。
- [3.2 コレクション](3-2-core-collections.md): 永続／可変コレクションの API と性能指針、Iter/Collector 連携、監査ワークフローへの接続を定義します。`Map.from_pairs` により初期マップ構築を安全に行うユーティリティも導入しています。
- [3.3 テキストと Unicode サポート](3-3-core-text-unicode.md): 文字列層構造、正規化・境界判定・検索 API を通じて Unicode モデルと IO/診断連携をカバーします。
- [3.4 数値演算と時間管理](3-4-core-numeric-time.md): 数値プリミティブ、統計ユーティリティ、時間/期間型とタイムゾーン処理、監査メトリクス連携を整理します。
- [3.5 入出力とパス操作](3-5-core-io-path.md): Reader/Writer 抽象、ファイル・ストリーム操作、Path セキュリティヘルパ、同期/非同期ブリッジを定義します。
- [3.6 診断と監査](3-6-core-diagnostics-audit.md): `Diagnostic` 構造と監査ログ、プライバシー制御、CLI/LSP 統合やメトリクス連携のベストプラクティスを示します。
- [3.7 設定とデータ管理](3-7-core-config-data.md): `reml.toml` マニフェスト、Config/Data スキーマ API、マイグレーション安全性と CLI 連携をまとめます。
- [3.8 ランタイムと Capability レジストリ](3-8-core-runtime-capability.md): Capability Registry の構造、セキュリティモデル、各 Capability の概要と DSL プロファイル生成フローを解説します。
- [3.9 非同期・FFI・アンセーフ](3-9-core-async-ffi-unsafe.md): 非同期実行モデル、FFI サンドボックス、`Core.Unsafe` 指針と Capability 連携、セキュリティ/性能最適化を扱います。
- [3.10 環境機能とプラットフォーム連携](3-10-core-env.md): 環境変数アクセス、プラットフォーム情報取得、`@cfg` 連携ガイドラインと将来拡張メモを提供します。
