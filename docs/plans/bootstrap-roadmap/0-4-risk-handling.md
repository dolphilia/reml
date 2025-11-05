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
- 登録日: 2026-03-31
- タイトル: Unicode XID 識別子実装未完了（SYNTAX-001 / LEXER-001）
- カテゴリ: 技術的負債
- 詳細: 仕様は `XID_Start`/`XID_Continue` に基づく Unicode 識別子を要求しているが、Phase 2-5 時点の実装は ASCII プロファイルのみを許可していた。`lexer.identifier_profile_unicode` 指標は当時 0.0 で、多言語 DSL サンプルや CLI/LSP の補完機能に制約が残っていたため、Phase 2-7 では 1.0 を維持する運用監視が必要となる。
- 対応案: Phase 2-7 `lexer-unicode` タスクで XID テーブル生成・正規化チェック・`--lex-profile=unicode` 既定化を完了し、CI で `REML_ENABLE_UNICODE_TESTS=1` を有効化する。進捗は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` の Unicode セクションと `docs/plans/bootstrap-roadmap/2-5-proposals/SYNTAX-001-proposal.md` Step5/6 を参照する。2026-12-02 に GitHub Actions 3 プラットフォームで環境変数を既定化し、`lexer.identifier_profile_unicode = 1.0` を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` §0.3.5 へ記録済み。
- 期限: 2026-08-31
- 状態: Mitigating
- 関連フェーズ: Phase 2 (2-7)
- 参照: `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md`, `docs/plans/bootstrap-roadmap/2-5-proposals/SYNTAX-001-proposal.md`, `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md`
- 担当: Phase 2-7 Parser チーム（LEXER-001 / SYNTAX-001）
- 登録日: 2026-04-20
- タイトル: 効果構文 Stage 昇格遅延（EFFECT-POC-Stage）
- カテゴリ: 技術的負債
- 詳細: 効果構文 PoC は `Σ_before`/`Σ_after` 記録と KPI (`syntax.effect_construct_acceptance`, `effects.syntax_poison_rate`) を Step4/Step5 で仕様化したが、OCaml 実装は `effect.syntax.constructs` の算出と残余効果の控除を未実装のまま Phase 2-7 へ移管している。Stage 昇格が遅延すると Chapter 1 の脚注撤去と Phase 3 self-host 移行の前提条件に影響する。
- 対応案: Phase 2-7 `EFFECT-003` / `Type_inference_effect` タスクで `TEffectPerform`/`THandle` の残余効果計算と `collect-iterator-audit-metrics.py` 指標を実装し、CI で `syntax.effect_construct_acceptance = 1.0` / `effects.syntax_poison_rate = 0.0` を達成した上で脚注を撤去する。進捗は `docs/notes/effect-system-tracking.md` と `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` (H-O1〜H-O5) で追跡する。
- 期限: 2026-09-30
- 状態: Resolved (2026-12-18)
- 解決メモ: Phase 2-7 Sprint C で Core IR・Runtime まで効果行が伝播し、`type_row_mode` の既定値を `"ty-integrated"` へ切り替え済み。Linux/Windows/macOS の CI で `collect-iterator-audit-metrics.py --require-success --section effects` を恒常運用し、`effect_row_guard_regressions = 0` を確認した。
- 関連フェーズ: Phase 2 (2-7)
- 参照: `docs/plans/bootstrap-roadmap/2-5-proposals/EFFECT-002-proposal.md`, `docs/plans/bootstrap-roadmap/2-5-review-log.md`, `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md`, `docs/notes/effect-system-tracking.md`
- 担当: Phase 2-7 Effects チーム（SYNTAX-003 / EFFECT-002）
- 登録日: 2026-04-24
- タイトル: 効果行統合遅延による型・監査不整合（TYPE-002-ROW-INTEGRATION）
- カテゴリ: 技術的負債
- 詳細: `TYPE-002` Step4 までに `effect_row` 統合ドラフトと移行ガード (`type_row_mode = "metadata-only"`) を整備したが、`ty` へ効果行を統合する実装は Phase 2-7 へ移管している。行統合が Phase 3 までに完了しない場合、`@handles`/Stage 契約の検証が実行時メタデータ依存のままとなり、Self-host CI の合格判定や監査 KPI（`diagnostics.effect_row_stage_consistency`）が 1.0 を維持できないリスクがある。
- 対応案: Phase 2-7 Sprint A/B/C で `effect_row` dual-write → `generalize`/`instantiate`/`Type_unification` 対応 → Core IR/監査伝播を実装し、`type_row_mode` を `dual-write` → `ty-integrated` へ段階的に移行する。`collect-iterator-audit-metrics.py` の `diagnostics.effect_row_stage_consistency` / `type_effect_row_equivalence` / `effect_row_guard_regressions` が各 1.0 / 1.0 / 0.0 を満たした時点で仕様本文を更新し、暫定脚注を撤去する。
- 期限: 2026-10-31
- 状態: Open
- 関連フェーズ: Phase 2 (2-7)
- 参照: `docs/plans/bootstrap-roadmap/2-5-proposals/TYPE-002-proposal.md`, `docs/plans/bootstrap-roadmap/2-5-to-2-7-type-002-handover.md`, `docs/plans/bootstrap-roadmap/2-5-review-log.md`, `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md`, `docs/notes/effect-system-tracking.md`, `compiler/ocaml/docs/effect-system-design-note.md`, `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md#0-3-7b-効果行統合メトリクス運用2026-12-18-更新`
- 担当: Phase 2-7 Type チーム（TYPE-002）

<a id="diagnostic-domain-metrics"></a>
### 診断ドメインメトリクス監視

- 登録日: 2026-12-21
- タイトル: 診断ドメイン KPI の閾値逸脱
- カテゴリ: 技術的負債
- 詳細: Phase 2-7 で導入した `diagnostics.domain_coverage`・`diagnostics.plugin_bundle_ratio`・`diagnostics.effect_stage_consistency` は Phase 2-8 の仕様監査で必須となる。いずれかが閾値（0.95 または 1.0）を下回ると、Plugin/LSP/Capability の差分抽出が不完全となり、`docs/spec/3-6-core-diagnostics-audit.md` と CLI/LSP ゴールデンの同期が崩れるリスクがある。
- 対応案: `collect-iterator-audit-metrics.py --section diagnostics --require-success` を CI で常時動作させ、逸脱が検出された場合は `reports/audit/dashboard/diagnostics.md` の比率を確認した上で互換モードテストを停止し、`docs/notes/dsl-plugin-roadmap.md` と連携してバンドル署名の再発行と Stage 診断の再測定を行う。
- 期限: 2027-03-31
- 状態: Monitoring
- 関連フェーズ: Phase 2 (2-7, 2-8)
- 参照: `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` §5、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md#0-3-7c-診断ドメイン可視化メトリクス運用2026-12-21-更新`、`reports/audit/dashboard/diagnostics.md`, `reports/audit/phase2-7/diagnostics-domain-20261221.json`
- 担当: Phase 2-8 Diagnostics/Plugin チーム
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
- 登録日: 2025-10-12
- タイトル: Linux CI フォーマットジョブで ocamlformat が未導入
- カテゴリ: 技術的負債
- 詳細: `Bootstrap Linux CI` の Lint ステージにおいて `opam exec -- dune build @fmt` が `ocamlformat` 実行ファイル不在で失敗している。Phase 1-8 の macOS CI 着手条件として Linux CI の健全性を維持する必要があり、Lint チェックの継続失敗は PR マージのボトルネックとなる。
- 対応案: `ocamlformat` バージョン固定（`dune-project` での `using fmt` 宣言と `opam install` による dev-dependency 化）を実施し、暫定的には Lint ステージで `opam install ocamlformat.0.26.2 --yes` を追加する。修正完了後は GitHub Actions ログと `compiler/ocaml/README.md` の進捗欄に復旧記録を残す。
- 期限: 2025-10-14
- 状態: Mitigating
- 関連フェーズ: Phase 1 (1-8)
- 参照: `.github/workflows/bootstrap-linux.yml`, `docs/plans/bootstrap-roadmap/1-8-macos-prebuild-support.md` §0, `docs/plans/bootstrap-roadmap/1-7-to-1-8-handover.md`
