# 5.0 公式プラグイン仕様 概要

## 概要
公式プラグイン章は Reml が提供する標準 Capability プラグイン群の設計指針をまとめ、システム統合や監査要件を満たしながらプラットフォーム機能を公開する方法を示します。各 Capability が Runtime Registry・診断・セキュリティポリシーと整合するように、API 構造と運用ベストプラクティスを整理しています。現在は標準ライブラリ拡張の再整理に伴い、章全体をドラフトとして再検討中です。

## ドラフト運用と移行メモ
標準ライブラリ拡張の調査・計画（`docs/notes/stdlib/stdlib-expansion-research.md` / `docs/plans/bootstrap-roadmap/4-1-stdlib-improvement-implementation-plan.md` / `docs/plans/stdlib-improvement/`）により、公式プラグインの一部は標準ライブラリへ移行する方針が示されています。現時点で把握できている変更・改訂案は以下のとおりです。

- **5.2 Process**: `Core.System.Process` として標準ライブラリへ移行済み（[3-18](3-18-core-system.md)）。公式プラグインは低レベル Capability として維持する。
- **5.4 Signal**: `Core.System.Signal` として標準ライブラリへ移行済み（[3-18](3-18-core-system.md)）。ハンドラ登録など低レベル操作は Capability 側に残す。
- **5.1 System**: 生の syscall は安全性・移植性の観点から Capability 側に留める。標準ライブラリ側は安全なラッパ API を提供する方針。
- **5.3 Memory / 5.5 Hardware / 5.6 RealTime**: 現時点では標準ライブラリ移行が確定していないため、プラグイン側に残す。必要があれば安全なサブセットを標準ライブラリへ切り出す。
- **5.7 Core.Parse.Plugin**: DSL 拡張の契約として維持しつつ、`Core.Dsl` 系モジュール（[3.16](3-16-core-dsl-paradigm-kits.md)）との接続方針を再整理する。

## セクションガイド
- [5.1 システムコール & プラットフォームバインディング](5-1-system-plugin.md): `SyscallCapability` API、プラットフォーム別ラッパ、監査付きシステムコール実装とセキュリティ統合を解説します。
- [5.2 プロセスとスレッド制御](5-2-process-plugin.md): 低レベル Capability としてのプロセス/スレッド操作を定義します（標準 API は [3-18](3-18-core-system.md) に移行済み）。
- [5.3 仮想メモリと共有領域](5-3-memory-plugin.md): メモリ要求モデル、共有メモリ抽象、エラー診断と高レベルユーティリティ、セキュリティポリシーを示します。
- [5.4 プロセス間シグナル](5-4-signal-plugin.md): 低レベルシグナル操作とハンドラ登録を扱います（標準 API は [3-18](3-18-core-system.md) に移行済み）。
- [5.5 ハードウェア情報取得](5-5-hardware-plugin.md): ハードウェア Capability の API 構造、監査付きプロービング、利用例と拡張の見通しを提供します。
- [5.6 スケジューリングと高精度タイマー](5-6-realtime-plugin.md): リアルタイム Capability の API、エラー・監査設計、高精度タイマー活用の指針と拡張アイデアを整理します。
- [5.7 Core.Parse.Plugin と DSL 拡張契約](5-7-core-parse-plugin.md): DSL プラグインが `Core.Parse` に Capability を注入する際の API 契約、署名検証、監査・Stage 連携を定義します。
