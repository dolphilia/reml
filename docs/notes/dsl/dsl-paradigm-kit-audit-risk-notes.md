# DSL パラダイムキット 監査/リスクメモ

## TODO: フェーズE課題メモ
- [ ] GC 性能: `Arena`/`RefCount` から `MarkAndSweep` へ拡張する際の停止時間・断片化・監査イベント量の増加を評価し、`dsl.gc.root` の発火頻度が過剰にならないサンプリング方針を決める。（参照: `docs/spec/3-16-core-dsl-paradigm-kits.md`, `docs/notes/dsl/dsl-paradigm-support-research.md`）
- [ ] ブリッジ安全性: `MailboxBridge` と `RuntimeBridge` の責務分界を再確認し、`bridge.*` と `dsl.actor.mailbox` の両方に記録すべきキーを整理する。（参照: `docs/spec/3-6-core-diagnostics-audit.md`, `docs/spec/3-8-core-runtime-capability.md`, `docs/spec/3-9-core-async-ffi-unsafe.md`）
- [ ] VM 拡張: `VMCore` の命令セット拡張時に `dsl.vm.execute` の粒度・命令トレースの負荷を見積もり、監査イベントのバッチ化方針を追記する。（参照: `docs/spec/3-16-core-dsl-paradigm-kits.md`, `docs/notes/runtime/runtime-bridges-roadmap.md`）

## 監査ログ運用ルール（暫定）
- ログ粒度: `AuditLevel::Summary` でも `event.kind` と `dsl.id` は必須とし、`dsl.object.dispatch` の高頻度イベントは `audit.level` と `audit.rate_limit` で間引く。`AuditLevel::Full` 以上で `dsl.dispatch.cache` と `dsl.vm.instruction` を必須化する。
- 個人情報: `Diagnostic` から抽出する文字列は [3-6 §4](../spec/3-6-core-diagnostics-audit.md#4-プライバシー保護とセキュリティ) の `redact_pii` を必ず通す。`dsl.*` イベントにユーザー入力を直接含めない。
- パフォーマンス影響: 監査シンクが遅延する場合は `AuditStatus` を `degraded` に切り替え、`dsl.gc.root` と `dsl.vm.execute` はサンプリングを優先する。パフォーマンス計測は `ExecutionMetricsScope`（`docs/spec/3-8-core-runtime-capability.md`）と同一スコープで集約する。

## 参照
- `docs/spec/3-16-core-dsl-paradigm-kits.md`
- `docs/spec/3-6-core-diagnostics-audit.md`
- `docs/spec/3-8-core-runtime-capability.md`
- `docs/spec/3-9-core-async-ffi-unsafe.md`
- `docs/notes/dsl/dsl-paradigm-support-research.md`
- `docs/notes/runtime/runtime-bridges-roadmap.md`
