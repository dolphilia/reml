# 型推論ロードマップ — 値制限復元メモ

Phase 2-5 TYPE-001 計画で共有する調査結果とチェックリストをまとめ、後続ステップの議論を容易にする。更新時は関連する仕様書・計画書へのリンクを明示し、棚卸しの根拠を追跡できるようにする。

## TYPE-001 値制限復元チェックリスト（2025-10-31 Step0）

- **確定的な値カテゴリの再確認**  
  - `docs/spec/1-2-types-Inference.md:129-150` で定義される確定的な値（ラムダ、コンストラクタ、数値・文字列リテラル等）を列挙し、`var`/`let` の右辺がどのカテゴリに属するか判定できるよう分類表を作成する。  
  - 今後 `Expr_utils.is_value`（仮称）を実装する際は、このリストをソース・オブ・トゥルースとして参照し、仕様更新時に差分をここへ反映する。

- **効果タグと値制限の関係**  
  - `docs/spec/1-3-effects-safety.md:11-83` に列挙される `mut` / `io` / `ffi` / `unsafe` / `panic`（および Stage 依存タグ）を一般化判定のブロックリストとして扱う。  
  - EFFECT-001 で強化する予定の `Effect_analysis.collect_expr` からタグを受け取り、`is_generalizable` 判定で「効果集合が空または安全タグのみ」を条件化する。

- **監査・RunConfig 連携の前提**  
  - CLI/RunConfig から値制限モード（`strict` / `legacy`）を切り替える計画を `Type_inference.make_config` へ橋渡しする必要がある。`docs/plans/bootstrap-roadmap/2-5-proposals/TYPE-001-proposal.md` Step2 で API モデルを決定し、このノートに要件を記録する。  
  - メトリクス `type_inference.value_restriction_violation` と診断コード（仮称 `effects.contract.value_restriction`）は収集対象とし、`0-3-audit-and-metrics.md` への登録内容と同期する。

- **再現ログと差分管理**  
  - `dune exec remlc -- tmp/value_restriction_var.reml --emit-tast` で `var poly = |x| x;` を多相的に利用できる現行挙動を確認済み。詳細は `docs/plans/bootstrap-roadmap/2-5-review-log.md` の「TYPE-001 Day1 値制限棚卸し」および `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` 脚注 `[^type001-step0-review]` を参照。  
  - 再現スニペットを更新した場合は、このノートとレビュー記録の双方に日付付きで追記する。

## Step4 仕様反映メモ（2025-11-08）

- **仕様書の追補**  
  - `docs/spec/1-2-types-Inference.md` §C.3 に `Value_restriction.evaluate` と `RunConfig.extensions["effects"].value_restriction_mode` の整合要件を記述し、Strict/Legacy モードの切替条件（Legacy は移行用・既定は Strict）を明示した。【S:docs/spec/1-2-types-Inference.md†L142-L166】  
  - `docs/spec/1-3-effects-safety.md` §B に値制限と効果タグ判定の接続、および `effects.contract.value_restriction` 診断で `effect.stage.*`／`value_restriction.*` を共有する監査要件を追記した。【S:docs/spec/1-3-effects-safety.md†L70-L104】
- **RunConfig / CLI 連携**  
  - `docs/spec/2-1-parser-type.md` §D および `docs/spec/2-6-execution-strategy.md` §B-2 に `extensions["effects"]` ネームスペースと CLI スイッチ（`--value-restriction={strict|legacy}`／`--legacy-value-restriction`）を記録し、Parser→Typer 間でモードを同期する橋渡し要件を明文化した。【S:docs/spec/2-1-parser-type.md†L118-L170】【S:docs/spec/2-6-execution-strategy.md†L38-L134】
- **フォローアップの割当**  
  - Phase 2-7 `execution-config` へ RunConfig CLI の統合テスト（Strict/Legacy 切替、将来的な Legacy 廃止通知）を移譲する。  
  - Phase 2-7 `effect-metrics` へ `type_inference.value_restriction_violation` 指標の監視と Legacy 経路検出アラートの設計を依頼する。

## TODO / 次ステップ

- [ ] Step1 で設計する `is_generalizable`（仮称）が参照する式分類テーブルをこのノートに掲載し、レビュー時に引用できるようにする。  
- [ ] `Effect_analysis.collect_expr` が返すタグ一覧と Stage 情報のマッピングを整理し、複数 Capability（`Type_inference_effect.resolve_function_profile`）との整合要件を追記する。  
- [ ] Phase 2-7 `execution-config` で実施する CLI 統合テスト計画（`--value-restriction` 系スイッチの廃止スケジュール含む）をまとめる。  
- [ ] Phase 2-7 `effect-metrics` へ引き継ぐ監視ジョブ（違反検出・Legacy 経路検知）と通知チャネルを整理し、CI 設定の更新手順を書き起こす。
