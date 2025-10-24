# TYPE-001 値制限の再導入計画

## 1. 背景と症状
- 仕様では「一般化は確定的な値のみ」と定義されており、副作用を持つ束縛は単相に制限する（docs/spec/1-2-types-Inference.md:136）。  
- 現行 OCaml 実装では `let` / `var` いずれも効果に関係なく `generalize` を適用しており（compiler/ocaml/src/type_inference.ml:2172-2235, compiler/ocaml/src/type_inference.ml:2236-2283）、`var` 再代入や `ffi` 呼び出しを含む束縛も多相化される。  
- 効果解析が `panic` しか検出していないため（TYPE-001 と連動する EFFECT-001）、残余効果に基づく制限が機能せず、`@pure` 契約や Stage 要件の検証が破綻する可能性がある。

## 2. Before / After
### Before
- `infer_decl` が束縛種別に関わらず `generalize` を呼び出し、`scheme.constraints` が空であれば辞書解決なしで環境へ登録する。
- 効果情報は `typed_fn_decl.tfn_effect_profile` にのみ保持され、束縛の型スキームには反映されない。
- `0-3-audit-and-metrics.md` の値制限関連メトリクスは未計測。

### After
- 束縛右辺が「確定的な値」かを判定する `is_generalizable`（純粋式 + 効果集合が空/安全タグのみ）を導入し、`let` では条件付き一般化、`var` では常に単相化する。
- `Effect_analysis.collect_from_fn_body` の結果を束縛評価へ渡し、`mut` / `io` / `ffi` / `unsafe` / `panic` のタグを持つ場合は単相に固定する。
- 一般化可否を `0-3-audit-and-metrics.md` の診断指標へ記録し、値制限違反が排除されたことを CI で確認する。

#### 擬似コード案
```ocaml
let is_generalizable ~effects expr_ty =
  Effect_tags.is_pure effects
    && Expr_utils.is_value expr_ty
```
`Effect_tags.is_pure` は EFFECT-001 の修正で導入するタグ集合判定を再利用する想定。

## 3. 影響範囲と検証
- **テスト**: 既存の型推論テストへ値制限ケースを追加し、`mut` / `ffi` / `unsafe` を含む束縛が単相に推論されることを確認。  
- **メトリクス**: `0-3-audit-and-metrics.md` に `type_inference.value_restriction_violation` を新設し、CI で 0 件を保証。  
- **互換性**: 多相化に依存していたサンプル（存在する場合）は `let` への変更や効果抑制で復元する。
- **監査ログ**: `collect-iterator-audit-metrics.py` に値制限違反検知イベントの集計を追加し、診断とメトリクスが同時に更新されるようにする。

## 4. フォローアップ
- EFFECT-001 で追加する効果タグ検出ロジックと同時レビューとし、タグ不足による誤判定を避ける。  
- Phase 2-7 `execution-config` タスクへ「値制限メトリクス収集」の連携を追加し、`RunConfig` 差分や CLI 表示と同期する。  
- Phase 3 で予定されている Reml 実装移植時に、同じ値制限ロジックを導入するため `docs/notes/core-parser-migration.md`（予定）にも計画の要点を共有する。
- `docs/notes/type-inference-roadmap.md` に値制限再導入の段階計画と既知の互換性リスクを記録し、PoC から正式導入までのレビュー履歴を残す。
- **タイミング**: EFFECT-001 のタグ拡張完了直後に Phase 2-5 中盤で実装へ着手し、Phase 2-5 終盤までに値制限違反ゼロを確認する。

## 残課題
- 値制限判定に利用する「純粋式」判定の粒度（例: `const fn` 呼び出しを許容するか）について、Phase 2-1 型クラス戦略チームと調整が必要。  
- 効果タグ解析の段階的適用（`-Zalgebraic-effects` 未使用時でも強制するか）を決定したい。
