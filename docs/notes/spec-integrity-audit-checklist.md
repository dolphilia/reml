# spec-integrity-audit-checklist 草案

> TODO: Phase 2-8 `spec-integrity-audit` 着手時に正式版へ昇格させる。現時点では Phase 2-5 ERR-001 の共有タスクで把握した監視項目のみを記録する。

## 期待集合（ERR-001）
- [ ] `parser.expected_summary_presence` が 1.0 を維持していることを `tooling/ci/collect-iterator-audit-metrics.py --require-success` で確認する。欠落した場合は `docs/plans/bootstrap-roadmap/2-5-review-log.md` 2025-11-16〜17 の手順を参照して検証をやり直す。
- [ ] `parser.expected_tokens_per_error` が 0.0 を下回らないことをチェックし、閾値を超える場合は `docs/spec/2-5-error.md` §B-7 の縮約ルールに従って上限設定を検討する。
- [ ] ストリーミング経路 (`docs/guides/core-parse-streaming.md` §3/§7) が `Diagnostic.expected` を CLI/LSP と同じ `ExpectationSummary` で公開しているか確認する。`StreamEvent::Error` で `ExpectedSummary` が欠落している場合は Phase 2-5 ERR-001 S5 の共有事項に沿って修正する。

## ドキュメント整合
- [ ] `docs/spec/2-5-error.md` と `docs/spec/3-6-core-diagnostics-audit.md` の脚注 `[^err001-phase25]` / `[^err001-phase25-core]` をレビューし、将来の仕様改訂で状態が変わった場合は脚注内容とリンクを更新する。
