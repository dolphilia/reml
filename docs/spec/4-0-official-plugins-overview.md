# 4.0 公式プラグイン仕様 概要

## 概要
公式プラグイン章は Reml が提供する標準 Capability プラグイン群の設計指針をまとめ、システム統合や監査要件を満たしながらプラットフォーム機能を公開する方法を示します。各 Capability が Runtime Registry・診断・セキュリティポリシーと整合するように、API 構造と運用ベストプラクティスを整理しています。現在は標準ライブラリ拡張の再整理に伴い、章全体をドラフトとして再検討中です。

## ドラフト運用と移行メモ
標準ライブラリ拡張の調査・計画（`docs/notes/stdlib-expansion-research.md` / `docs/plans/bootstrap-roadmap/4-1-stdlib-improvement-implementation-plan.md` / `docs/plans/stdlib-improvement/`）により、公式プラグインの一部は標準ライブラリへ移行する方針が示されています。現時点で把握できている変更・改訂案は以下のとおりです。

- **4.2 Process**: `Core.System.Process` への昇格を検討。`Core.Env` と統合し `Core.System` 配下へ集約する方針（`Core.System.Env` / `Core.System.Daemon` など）。
- **4.4 Signal**: `Core.System.Signal`（または `Core.System.Process.Signal`）として標準ライブラリへ移行する案を検討。低レベルのハンドラ登録は Capability 側へ残す。
- **4.1 System**: 生の syscall は安全性・移植性の観点から Capability 側に留める。標準ライブラリ側は安全なラッパ API を提供する方針。
- **4.3 Memory / 4.5 Hardware / 4.6 RealTime**: 現時点では標準ライブラリ移行が確定していないため、プラグイン側に残す。必要があれば安全なサブセットを標準ライブラリへ切り出す。
- **4.7 Core.Parse.Plugin**: DSL 拡張の契約として維持しつつ、`Core.Dsl` 系モジュール（[3.16](3-16-core-dsl-paradigm-kits.md)）との接続方針を再整理する。

## セクションガイド
- [4.1 システムコール & プラットフォームバインディング](4-1-system-plugin.md): `SyscallCapability` API、プラットフォーム別ラッパ、監査付きシステムコール実装とセキュリティ統合を解説します。
- [4.2 プロセスとスレッド制御](4-2-process-plugin.md): プロセス生成・待機・制限、スレッド API、監査イベントと審査フローを定義します（標準ライブラリ移行を検討中）。
- [4.3 仮想メモリと共有領域](4-3-memory-plugin.md): メモリ要求モデル、共有メモリ抽象、エラー診断と高レベルユーティリティ、セキュリティポリシーを示します。
- [4.4 プロセス間シグナル](4-4-signal-plugin.md): シグナル API と型、安全なハンドリング、運用例と将来拡張をまとめます（標準ライブラリ移行を検討中）。
- [4.5 ハードウェア情報取得](4-5-hardware-plugin.md): ハードウェア Capability の API 構造、監査付きプロービング、利用例と拡張の見通しを提供します。
- [4.6 スケジューリングと高精度タイマー](4-6-realtime-plugin.md): リアルタイム Capability の API、エラー・監査設計、高精度タイマー活用の指針と拡張アイデアを整理します。
- [4.7 Core.Parse.Plugin と DSL 拡張契約](4-7-core-parse-plugin.md): DSL プラグインが `Core.Parse` に Capability を注入する際の API 契約、署名検証、監査・Stage 連携を定義します。
