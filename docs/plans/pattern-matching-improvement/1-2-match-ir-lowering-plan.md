# 1.2 Match/Pattern IR ローアリング計画

## 目的と背景
- 現状は TypedExpr/TypedPattern で Active/Or/Slice/Range/Regex/Binding を正規化しているが、MIR/LLVM などバックエンドへ伝搬する IR が存在せず、Partial Active の `None` フォールスルーや Range/Slice/Or の分岐生成が未実装。
- Phase 4 の網羅性/診断マトリクス（`docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv`）を完了させるには、コード生成側で分岐を組み立て、expected を再取得する必要がある。
- 本計画では IR 仕様の決定 → MIR 生成 → バックエンド変換 → 回帰資産更新までの手順をまとめ、Rust/OCaml 実装間の差分を吸収しやすくする。

## スコープ
- **含む**: MIR への Match/Pattern ノード追加、Active partial の miss パス設計、Range/Slice/Or の分岐マッピング、Phase4 マトリクス/expected の再取得手順。
- **含まない**: LSP 連携 UI、最適化パス（ジャンプ圧縮・ヒューリスティクス）、OCaml バックエンド実装の詳細。

## 現状のギャップ
- フロントエンド出力: `match_lowerings` をデバッグ用に生成するのみで、バックエンド入力となる IR が未定義。
- バックエンド: MIR JSON ローダー（`compiler/rust/backend/llvm/src/integration.rs`）は関数シグネチャのみを扱い、Match/Pattern ノードが存在しない。
- 期待資産: Partial Active miss・Range/Slice/Or 分岐を含む expected/diagnostic の再取得が未実施（run_id の記録もなし）。

## 具体的な作業ステップ
1. **IR 仕様策定 (MIR 拡張案を決定)**  
   - 非終端: `MirExpr::Match { target, arms }`, `MirMatchArm { pattern: MirPattern, guard, alias, body }`。  
   - パターン: `Wildcard | Var | Literal | Constructor | Tuple | Record | Binding(@/as) | Or(Vec) | Slice(Vec<Elem|Rest>) | Range { start, end, inclusive } | Regex { pattern } | Active { name, kind: Partial/Total, arg }`。  
   - Partial Active: `Active { kind=Partial }` は `miss_target` を保持し、`None` なら次アームへジャンプする構造を MIR で表現する。
   - Guard/Alias: 順不同受理だが MIR では `guard -> alias -> body` の評価順を固定する。

2. **フロントエンド MIR 生成の実装**  
   - `TypedExprKind::Match` から上記 MIR を構築するパスを追加。`match_lowerings` 相当の情報を MIR ノードに格納する。  
   - Partial Active の miss パスは `on_miss: Label` を付与し、Range/Slice/Or はノード種別で判別可能にする。  
   - JSON エクスポート: デバッグ用に MIR を `reml_frontend` から出力できるサブコマンド（または `--debug-mir`）を追加。

3. **バックエンド変換 (MIR → LLVM) の追加**  
   - `integration.rs` で MIR JSON に `Match`/`Pattern` を読み込み、`CodegenContext` でジャンプ命令を組み立てる。  
   - 部分 Active: `Some` → 現在アーム本体、`None` → 次アームへ直接ジャンプ。  
   - Range: 比較演算とブール合成で分岐生成（`..=` を含む）。  
   - Slice: 先頭/末尾固定要素と Rest の長さチェックを分岐化。  
   - Or: 各バリアントを順に評価し、成功で抜けるフォールスルー構造を生成。

4. **回帰資産の再取得とマトリクス更新**  
   - 対象: `examples/spec_core/chapter1/match_expr/*.reml`（Range/Slice/Or/Active miss 含む）および `active_patterns/*.reml`。  
   - コマンド記録: Phase4 マトリクスの該当行（CH1-ACT-00x / CH1-MATCH-01x）に run_id・再取得コマンドを追記し、diagnostic.message がレジストリ文面に一致することを確認。  
   - IR ログ: `reports/spec-audit/ch4/logs/` に MIR/LLVM 簡易差分を保存し、ギャップがあればコメントで理由を残す。

5. **クロスチェックとドキュメント同期**  
   - 仕様: 必要に応じて `docs/spec/1-5-formal-grammar-bnf.md` の Match/Pattern セクションに「Partial Active miss は `None` で次アームへ」等の注記を追記。  
   - ガイド: `docs/guides/core-parse-streaming.md` 付録で Match 分岐の実行順とストリーミング評価の関係を整理。  
   - OCaml 実装: MIR 仕様が確定したら差分メモを `docs/plans/rust-migration/` へ残し、移植方針を共有。

## 優先順位とマイルストーン
- **M1: IR 仕様決定** — 上記 MIR ノード案をレビューし、受理。  
- **M2: フロントエンド MIR 生成** — `TypedExprKind::Match` から MIR 生成が通り、JSON エクスポートができる状態。  
- **M3: バックエンド分岐生成** — Partial Active miss と Range/Slice/Or/Regex/Binding を含むサンプルで LLVM 風出力が生成される。  
- **M4: 回帰資産更新** — expected/diagnostic 再取得・マトリクス run_id 記録完了。  
- **M5: クロス実装チェック** — OCaml/回帰レポートとの差分メモ完了。

## 退出条件
- Match/Pattern を含む MIR が生成され、バックエンドでジャンプ分岐が構築される。Partial Active の miss パスがランタイム挙動として確認できる。  
- Phase4 マトリクスの該当行に最新 run_id・再取得コマンドが記録され、expected が更新済み。  
- IR 仕様とドキュメント（必要箇所）が同期され、将来の OCaml 実装に転記できる状態。 
