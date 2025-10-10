# 0.4 リスク管理とフォローアップ

本章はブートストラップ計画におけるリスク登録、フォローアップ、移行期間中の意思決定手順を定義する。[0-1-project-purpose.md](../../spec/0-1-project-purpose.md) の判断フレームワークと `docs/notes/llvm-spec-status-survey.md` の未決課題を参照し、リスクが可視化された状態でフェーズを進める。

## 0.4.1 リスク登録カテゴリ
| カテゴリ | 説明 | 例 | 対応フェーズ |
|----------|------|----|-------------|
| 技術的負債 | 実装または仕様の不足 | 型クラス辞書の性能懸念 | Phase 2–4 |
| スケジュール | 工期遅延やレビュー待ち | 型チェッカレビュー待機が 2 週超過 | 全フェーズ |
| 互換性 | 旧ツールや OCaml 実装との差分 | 診断フォーマット不一致 | Phase 3–4 |
| セキュリティ/安全性 | Stage/Capability ミスマッチ、所有権違反 | FFI リーク検出 | 全フェーズ |
| エコシステム | 外部ユーザー・プラグインへの影響 | DSL プラグインのバージョン不整合 | Phase 4 |

## 0.4.2 登録フォーマット
```
- 登録日: YYYY-MM-DD
- タイトル: <短い説明>
- カテゴリ: <上記カテゴリのいずれか>
- 詳細: <現象・影響範囲・関連仕様章>
- 対応案: <一次対応/恒久対応>
- 期限: <YYYY-MM-DD>
- 状態: Open / Mitigating / Resolved
- 関連フェーズ: Phase X
- 参照: <コミット、Issue、ノートなど>
```
登録されたリスクはフェーズ進行会議で週次レビューされ、状態変更時には計画書および関連仕様書へ影響を反映する。

## 0.4.3 エスカレーション基準
- **性能指標が 10% 超過**: `parse_throughput` または `memory_peak_ratio` が目標値から 10% 以上逸脱した場合、Phase 進行を停止し、対策タスクを作成。
- **Stage ミスマッチ発生**: `stage_mismatch_count` > 0 の場合、ミスマッチが解消されるまで新機能導入を凍結。
- **診断差分未解決**: `diagnostic_regressions` が 7 日以上解決されない場合、[3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) を見直し、必要なら仕様変更を先行実施。

## 0.4.4 フォローアッププロセス
1. リスク登録後 24 時間以内に担当者を割当。
2. 担当者は対策案を `0-3-audit-and-metrics.md` の週次レポートに記録。
3. 解決後は状態を `Resolved` に更新し、対応内容をフェーズ文書に脚注として追記。
4. 次フェーズへ影響する場合は、該当フェーズの冒頭に「継承リスク」として要約を記載。

## 0.4.5 移行期の特別措置
- OCaml 実装との互換性確保のため、致命的な差分が発生した場合は OCaml 実装を一時的に正とし、Reml 実装を修正する。
- 重大な脆弱性が見つかった場合は、Phase に関わらず緊急パッチを適用し、`docs/notes/` に詳細な対応メモを残す。

---

本章は計画期間中に常に更新される生きたリスク台帳として扱う。未解決のリスクが存在する限り、次フェーズへ進む前に状態をレビューし、必要なタスクを各フェーズ文書（`1-x`〜`4-x` 系列）へ反映させる。

## 0.4.6 現在のリスク登録
- 登録日: 2025-10-10
- タイトル: Debian sysroot アーカイブのハッシュ未確定
- カテゴリ: 互換性
- 詳細: macOS → Linux x86_64 クロスコンパイルで使用する `tooling/toolchains/cache/debian-bookworm-x86_64.tar.zst` の SHA-256 を算出し、`tooling/toolchains/versions.toml` および `tooling/toolchains/checksums.txt` に `49b9ee8917f7235b6f20aaff3f983d616c53f29354ad180782ed024186df5452` を登録済み。
- 対応案: 登録済みの値を四半期レビューで検証し、sysroot 更新時は再計算して報告する。
- 期限: 2025-10-10
- 状態: Resolved
- 関連フェーズ: Phase 1
- 参照: `tooling/toolchains/versions.toml`, `tooling/toolchains/checksums.txt`, `docs/plans/bootstrap-roadmap/1-5-runtime-integration.md` §10
- 登録日: 2025-10-12
- タイトル: Homebrew 版 LLVM の頻繁な更新による再現性低下
- カテゴリ: 互換性
- 詳細: Phase 1-8 で使用予定の `brew install llvm@15` が頻繁に更新され、CI とローカル環境で異なるビルド番号が導入される懸念がある。`llvm-config` の出力差異が出た場合、IR 検証に失敗する可能性がある。
- 対応案: `brew extract` によるフォーミュラ固定、もしくは GitHub Actions 内でのバイナリアーカイブ展開を採用する。決定後は `docs/notes/llvm-spec-status-survey.md` に手順を記録し、`bootstrap-macos.yml` に反映する。
- 期限: 2025-10-26
- 状態: Open
- 関連フェーズ: Phase 1 (1-8)
- 参照: `docs/plans/bootstrap-roadmap/1-8-macos-prebuild-support.md` §3, `docs/notes/llvm-spec-status-survey.md`
- 登録日: 2025-10-12
- タイトル: GitHub Actions macOS ランナーの起動待ち時間長期化
- カテゴリ: スケジュール
- 詳細: macOS ランナーのジョブ待機時間が 15 分を超えるケースが増加すると、Phase 1-8 の CI フィードバックが遅延し、`ci_build_time_macos` 指標の収集に影響が出る。Linux CI よりもスループットが低下する懸念がある。
- 対応案: 待機時間が 10 分を超えた状態が 2 週間継続した場合、セルフホストランナー導入またはジョブスケジュールの週次バッチ化を検討する。判断結果を `docs/plans/bootstrap-roadmap/1-8-macos-prebuild-support.md` §7 に追記する。
- 期限: 2025-11-02
- 状態: Open
- 関連フェーズ: Phase 1 (1-8)
- 参照: `docs/plans/bootstrap-roadmap/1-8-macos-prebuild-support.md` §2, `0-3-audit-and-metrics.md`
