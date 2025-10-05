# runtime/native ワークスペース

Phase 1 の最小ランタイムおよび Phase 2 以降の Capability 拡張を実装する領域です。詳細タスクは [`docs/plans/bootstrap-roadmap/1-5-runtime-integration.md`](../../docs/plans/bootstrap-roadmap/1-5-runtime-integration.md) と後続フェーズの計画書を参照してください。

## TODO
- [ ] `src/`, `tests/` などのサブディレクトリ構成を確定
- [ ] 最小ランタイム API (`mem_alloc`, `panic`, `inc_ref`, `dec_ref` など) の C/LLVM スタブを追加
- [ ] 監査・メトリクス計測に関する補助スクリプトの配置
- [ ] Windows/MSVC 対応や追加 Capability を Phase 2 計画と同期
