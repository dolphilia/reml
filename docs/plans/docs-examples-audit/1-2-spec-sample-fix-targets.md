# 1.2 docs/spec サンプル修正 対象リスト（ドラフト）

`docs/plans/docs-examples-audit/1-1-spec-code-block-inventory.md` の `ng` を対象に、修正方針の当たりを付けた一覧。

## 集計
- 対象ドキュメント数: 35
- NG サンプル数: 383

## 方針の凡例
| category | 方針 |
| --- | --- |
| syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 |
| example | 仕様例自体の誤りが疑われるため、仕様記述と併せて例を修正。 |
| staging | 実験段階。@unstable/@stage/Capability 要件を明示し、rustcap 併記を検討。 |
| fallback | rustcap サンプル追加。元サンプルは仕様用に保持。 |
| unknown | 診断詳細を確認後に方針決定。 |

## 修正状況の凡例
- `todo`: 未着手
- `doing`: 対応中
- `done`: 修正完了（在庫表更新まで完了）
- `hold`: 保留（追加調査/仕様整理待ち）

## 章別対象
### docs/spec/1-1-syntax.md
| 節 | コード名 | .reml パス | category | 方針 | diag_code | 修正状況 |
| --- | --- | --- | --- | --- | --- | --- |
| B.1.1 DSLエントリーポイント宣言 | sec_b_1_1 | examples/docs-examples/spec/1-1-syntax/sec_b_1_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| B.4 宣言の種類 | sec_b_4-c | examples/docs-examples/spec/1-1-syntax/sec_b_4-c.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| B.4 宣言の種類 | sec_b_4-e | examples/docs-examples/spec/1-1-syntax/sec_b_4-e.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| B.4 宣言の種類 | sec_b_4-f | examples/docs-examples/spec/1-1-syntax/sec_b_4-f.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| B.5 効果宣言とハンドラ構文（実験段階） | sec_b_5-c | examples/docs-examples/spec/1-1-syntax/sec_b_5-c.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| B.6 属性（Attributes） | sec_b_6 | examples/docs-examples/spec/1-1-syntax/sec_b_6.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 不安定機能属性 `@unstable` | sec_section-b | examples/docs-examples/spec/1-1-syntax/sec_section-b.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| B.8.3.2 `with_embedded` の合成契約（草案） | sec_b_8_3_2 | examples/docs-examples/spec/1-1-syntax/sec_b_8_3_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| B.8.5 Capability 検証契約と `with_capabilities` | sec_b_8_5 | examples/docs-examples/spec/1-1-syntax/sec_b_8_5.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| C.2 関数適用・引数 | sec_c_2 | examples/docs-examples/spec/1-1-syntax/sec_c_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| C.4 制御構文 | sec_c_4-a | examples/docs-examples/spec/1-1-syntax/sec_c_4-a.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| C.4 制御構文 | sec_c_4-b | examples/docs-examples/spec/1-1-syntax/sec_c_4-b.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| C.4 制御構文 | sec_c_4-c | examples/docs-examples/spec/1-1-syntax/sec_c_4-c.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| C.4 制御構文 | sec_c_4-d | examples/docs-examples/spec/1-1-syntax/sec_c_4-d.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| C.4 制御構文 | sec_c_4-e | examples/docs-examples/spec/1-1-syntax/sec_c_4-e.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| C.6 ブロックと束縛 | sec_c_6 | examples/docs-examples/spec/1-1-syntax/sec_c_6.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| C.7 `unsafe` ブロック | sec_c_7 | examples/docs-examples/spec/1-1-syntax/sec_c_7.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| E.2 代数的データ型（ADT） | sec_e_2 | examples/docs-examples/spec/1-1-syntax/sec_e_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| G. 例（仕様の運用感） | sec_g | examples/docs-examples/spec/1-1-syntax/sec_g.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |

### docs/spec/1-2-types-Inference.md
| 節 | コード名 | .reml パス | category | 方針 | diag_code | 修正状況 |
| --- | --- | --- | --- | --- | --- | --- |
| B.3 トレイト制約の表記 | sec_b_3 | examples/docs-examples/spec/1-2-types-Inference/sec_b_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| F. 代表的な型（標準 API・コンビネータ想定） | sec_f | examples/docs-examples/spec/1-2-types-Inference/sec_f.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| H.2 制約の持ち上げ | sec_h_2-a | examples/docs-examples/spec/1-2-types-Inference/sec_h_2-a.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |

### docs/spec/1-3-effects-safety.md
| 節 | コード名 | .reml パス | category | 方針 | diag_code | 修正状況 |
| --- | --- | --- | --- | --- | --- | --- |
| C. 効果の宣言と抑制（属性） | sec_c-a | examples/docs-examples/spec/1-3-effects-safety/sec_c-a.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| C. 効果の宣言と抑制（属性） | sec_c-b | examples/docs-examples/spec/1-3-effects-safety/sec_c-b.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| E. 可変性（mut）とデータの不変性 | sec_e | examples/docs-examples/spec/1-3-effects-safety/sec_e.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| F. FFI と unsafe | sec_f | examples/docs-examples/spec/1-3-effects-safety/sec_f.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| G. リソース安全（スコープ終端保証） | sec_g | examples/docs-examples/spec/1-3-effects-safety/sec_g.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| J.3 unsafe と FFI | sec_j_3 | examples/docs-examples/spec/1-3-effects-safety/sec_j_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| J.4 `@no_panic` と `?` | sec_j_4 | examples/docs-examples/spec/1-3-effects-safety/sec_j_4.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |

### docs/spec/1-4-test-unicode-model.md
| 節 | コード名 | .reml パス | category | 方針 | diag_code | 修正状況 |
| --- | --- | --- | --- | --- | --- | --- |
| I. 標準 API（抜粋シグネチャ） | sec_i | examples/docs-examples/spec/1-4-test-unicode-model/sec_i.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |

### docs/spec/2-1-parser-type.md
| 節 | コード名 | .reml パス | category | 方針 | diag_code | 修正状況 |
| --- | --- | --- | --- | --- | --- | --- |
| A. 主要型 | sec_a | examples/docs-examples/spec/2-1-parser-type/sec_a.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| C. スパンとトレース | sec_c | examples/docs-examples/spec/2-1-parser-type/sec_c.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| D. 実行設定 `RunConfig` とメモ | sec_d | examples/docs-examples/spec/2-1-parser-type/sec_d.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| D-1. `RunConfig` ユーティリティ | sec_d_1 | examples/docs-examples/spec/2-1-parser-type/sec_d_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 利用例（CLI/LSP 共通設定） | sec_clilsp | examples/docs-examples/spec/2-1-parser-type/sec_clilsp.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| G. ランナー API（外部からの呼び出し） | sec_g | examples/docs-examples/spec/2-1-parser-type/sec_g.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |

### docs/spec/2-2-core-combinator.md
| 節 | コード名 | .reml パス | category | 方針 | diag_code | 修正状況 |
| --- | --- | --- | --- | --- | --- | --- |
| A-1. 基本 | sec_a_1 | examples/docs-examples/spec/2-2-core-combinator/sec_a_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| A-2. 直列・選択 | sec_a_2 | examples/docs-examples/spec/2-2-core-combinator/sec_a_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| A-3. 変換・コミット・回復 | sec_a_3 | examples/docs-examples/spec/2-2-core-combinator/sec_a_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| A-3-a. 回復糖衣（最小セット） | sec_a_3_a | examples/docs-examples/spec/2-2-core-combinator/sec_a_3_a.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| A-3-b. 回復ヘルパ（同期点／パニック／欠落補挿） | sec_a_3_b | examples/docs-examples/spec/2-2-core-combinator/sec_a_3_b.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| A-4. 繰り返し・任意 | sec_a_4 | examples/docs-examples/spec/2-2-core-combinator/sec_a_4.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| A-5. 括り・前後関係 | sec_a_5 | examples/docs-examples/spec/2-2-core-combinator/sec_a_5.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| A-6. 先読み・否定 | sec_a_6 | examples/docs-examples/spec/2-2-core-combinator/sec_a_6.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| A-7. チェーン（演算子の左/右結合） | sec_a_7 | examples/docs-examples/spec/2-2-core-combinator/sec_a_7.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| A-8. スパン・位置 | sec_a_8 | examples/docs-examples/spec/2-2-core-combinator/sec_a_8.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| B. 前後空白（字句インターフェイス） | sec_b | examples/docs-examples/spec/2-2-core-combinator/sec_b.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| B-1. 空白プロファイルの共有 | sec_b_1 | examples/docs-examples/spec/2-2-core-combinator/sec_b_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| B-2. autoWhitespace / Layout（Phase 9 ドラフト） | sec_b_2 | examples/docs-examples/spec/2-2-core-combinator/sec_b_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| C-1. CST ノードと Trivia | sec_c_1-a | examples/docs-examples/spec/2-2-core-combinator/sec_c_1-a.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| C-2. ロスレスパース API | sec_c_2 | examples/docs-examples/spec/2-2-core-combinator/sec_c_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| C-5. 埋め込み DSL インターフェイス（Phase 4 草案） | sec_c_5 | examples/docs-examples/spec/2-2-core-combinator/sec_c_5.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| B-2-a. layout_token（Layout 連携） | sec_b_2_a | examples/docs-examples/spec/2-2-core-combinator/sec_b_2_a.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| B-3. 観測/プロファイル（Phase 10 実験フラグ） | sec_b_3 | examples/docs-examples/spec/2-2-core-combinator/sec_b_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| C. 便利だが派生（derived）に落とすもの | sec_c | examples/docs-examples/spec/2-2-core-combinator/sec_c.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| C-1. 優先度ビルダー（Phase 8 ドラフト） | sec_c_1-b | examples/docs-examples/spec/2-2-core-combinator/sec_c_1-b.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| D. 消費／コミットの要点（実務上の指針） | sec_d-a | examples/docs-examples/spec/2-2-core-combinator/sec_d-a.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| D. 消費／コミットの要点（実務上の指針） | sec_d-b | examples/docs-examples/spec/2-2-core-combinator/sec_d-b.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| D. 消費／コミットの要点（実務上の指針） | sec_d-c | examples/docs-examples/spec/2-2-core-combinator/sec_d-c.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| E. 例：四則演算（べき乗右結合、カッコ、単項 -） | sec_e | examples/docs-examples/spec/2-2-core-combinator/sec_e.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| G-1. ParserMeta の最小構造 | sec_g_1 | examples/docs-examples/spec/2-2-core-combinator/sec_g_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| J. Capability 要求パターン | sec_j | examples/docs-examples/spec/2-2-core-combinator/sec_j.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |

### docs/spec/2-3-lexer.md
| 節 | コード名 | .reml パス | category | 方針 | diag_code | 修正状況 |
| --- | --- | --- | --- | --- | --- | --- |
| A. 設計の核（プリミティブ 6） | sec_a | examples/docs-examples/spec/2-3-lexer/sec_a.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| B. 空白・改行・コメント（スキップ系） | sec_b-a | examples/docs-examples/spec/2-3-lexer/sec_b-a.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| C. トークン化の基本ユーティリティ | sec_c | examples/docs-examples/spec/2-3-lexer/sec_c.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| D-1. プロファイル | sec_d_1 | examples/docs-examples/spec/2-3-lexer/sec_d_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| D-2. キーワードと境界 | sec_d_2 | examples/docs-examples/spec/2-3-lexer/sec_d_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| E. 数値リテラル（区切り `_` / 基数 / 範囲チェック） | sec_e | examples/docs-examples/spec/2-3-lexer/sec_e.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| E-1. 数値エラーの診断変換 | sec_e_1 | examples/docs-examples/spec/2-3-lexer/sec_e_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| F. 文字列・文字リテラル（エスケープ/生/複数行） | sec_f | examples/docs-examples/spec/2-3-lexer/sec_f.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| G. 汎用“取り込み”ユーティリティ | sec_g | examples/docs-examples/spec/2-3-lexer/sec_g.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| G-1. 設定ファイル互換プロファイル | sec_g_1 | examples/docs-examples/spec/2-3-lexer/sec_g_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| H. 行頭・行末・インデント（任意） | sec_h | examples/docs-examples/spec/2-3-lexer/sec_h.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| H-2. LayoutProfile（Phase 9 ドラフト） | sec_h_2 | examples/docs-examples/spec/2-3-lexer/sec_h_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| I. セキュリティ・正規化（安全モード） | sec_i | examples/docs-examples/spec/2-3-lexer/sec_i.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| J. エラー品質のための流儀 | sec_j | examples/docs-examples/spec/2-3-lexer/sec_j.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| L-1. Reml 識別子/キーワード | sec_l_1 | examples/docs-examples/spec/2-3-lexer/sec_l_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| L-4. 既定ランナー統合 | sec_l_4 | examples/docs-examples/spec/2-3-lexer/sec_l_4.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| O. 文字モデル統合（1.4 連携） | sec_o | examples/docs-examples/spec/2-3-lexer/sec_o.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |

### docs/spec/2-4-op-builder.md
| 節 | コード名 | .reml パス | category | 方針 | diag_code | 修正状況 |
| --- | --- | --- | --- | --- | --- | --- |
| A-1. ビルダーの入口 | sec_a_1 | examples/docs-examples/spec/2-4-op-builder/sec_a_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| A-2. レベル宣言（fixity） | sec_a_2 | examples/docs-examples/spec/2-4-op-builder/sec_a_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| A-3. 完成 | sec_a_3 | examples/docs-examples/spec/2-4-op-builder/sec_a_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| B. 使い方（API と DSL） | sec_b | examples/docs-examples/spec/2-4-op-builder/sec_b.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| E. 拡張：演算子パーサの“型” | sec_e | examples/docs-examples/spec/2-4-op-builder/sec_e.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| I-1. 右結合の `?:` 三項 | sec_i_1 | examples/docs-examples/spec/2-4-op-builder/sec_i_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| I-2. パイプ演算子（最弱） | sec_i_2 | examples/docs-examples/spec/2-4-op-builder/sec_i_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |

### docs/spec/2-5-error.md
| 節 | コード名 | .reml パス | category | 方針 | diag_code | 修正状況 |
| --- | --- | --- | --- | --- | --- | --- |
| A. 型（データモデル） | sec_a | examples/docs-examples/spec/2-5-error/sec_a.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| B-1. 単一パーサの失敗を作る | sec_b_1 | examples/docs-examples/spec/2-5-error/sec_b_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| B-12. `Async.timeout` 由来の診断を統一する | sec_b_12 | examples/docs-examples/spec/2-5-error/sec_b_12.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| C. 表示（pretty）と多言語 | sec_c | examples/docs-examples/spec/2-5-error/sec_c.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| C-0. ParseError 診断プリセットの利用 | sec_c_0 | examples/docs-examples/spec/2-5-error/sec_c_0.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| E. `recover`（回復）の仕様 | sec_e | examples/docs-examples/spec/2-5-error/sec_e.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| F. API（作る・混ぜる・見せる） | sec_f | examples/docs-examples/spec/2-5-error/sec_f.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| F-3. サンプル | sec_f_3-a | examples/docs-examples/spec/2-5-error/sec_f_3-a.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| F-3. サンプル | sec_f_3-b | examples/docs-examples/spec/2-5-error/sec_f_3-b.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |

### docs/spec/2-6-execution-strategy.md
| 節 | コード名 | .reml パス | category | 方針 | diag_code | 修正状況 |
| --- | --- | --- | --- | --- | --- | --- |
| A-1. ランナー API（外部インターフェイス） | sec_a_1 | examples/docs-examples/spec/2-6-execution-strategy/sec_a_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| B-2. RunConfig のコアスイッチ | sec_b_2 | examples/docs-examples/spec/2-6-execution-strategy/sec_b_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| B-2-1. ターゲット情報拡張 `extensions["target"]` | sec_b_2_1 | examples/docs-examples/spec/2-6-execution-strategy/sec_b_2_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| B-2-2. プラットフォーム適応設定サンプル | sec_b_2_2 | examples/docs-examples/spec/2-6-execution-strategy/sec_b_2_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| E-2. トレース・プロファイル（オプション） | sec_e_2 | examples/docs-examples/spec/2-6-execution-strategy/sec_e_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| F. Regex 実行連携 | sec_f | examples/docs-examples/spec/2-6-execution-strategy/sec_f.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| H.1 WASM / WASI | sec_h_1 | examples/docs-examples/spec/2-6-execution-strategy/sec_h_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| H.2 ARM64 / 組み込み | sec_h_2 | examples/docs-examples/spec/2-6-execution-strategy/sec_h_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| H.3 クラウドネイティブ / コンテナ | sec_h_3 | examples/docs-examples/spec/2-6-execution-strategy/sec_h_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |

### docs/spec/2-7-core-parse-streaming.md
| 節 | コード名 | .reml パス | category | 方針 | diag_code | 修正状況 |
| --- | --- | --- | --- | --- | --- | --- |
| A-1. 署名と戻り値 | sec_a_1-a | examples/docs-examples/spec/2-7-core-parse-streaming/sec_a_1-a.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| A-1. 署名と戻り値 | sec_a_1-b | examples/docs-examples/spec/2-7-core-parse-streaming/sec_a_1-b.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| A-2. StreamingConfig | sec_a_2 | examples/docs-examples/spec/2-7-core-parse-streaming/sec_a_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| B-1. 入出力契約 | sec_b_1 | examples/docs-examples/spec/2-7-core-parse-streaming/sec_b_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| B-2. StreamError | sec_b_2 | examples/docs-examples/spec/2-7-core-parse-streaming/sec_b_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| C-1. Continuation 型 | sec_c_1 | examples/docs-examples/spec/2-7-core-parse-streaming/sec_c_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| D. フロー制御とバックプレッシャ | sec_d | examples/docs-examples/spec/2-7-core-parse-streaming/sec_d.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| E. StreamDriver ヘルパ | sec_e | examples/docs-examples/spec/2-7-core-parse-streaming/sec_e.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| G-1. StreamMeta | sec_g_1 | examples/docs-examples/spec/2-7-core-parse-streaming/sec_g_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |

### docs/spec/3-1-core-prelude-iteration.md
フェーズ 3 の正準例復元まで対応済み（2025-12-27）。
| 節 | コード名 | .reml パス | category | 方針 | diag_code | 修正状況 |
| --- | --- | --- | --- | --- | --- | --- |
| 2.1 型定義 | sec_2_1 | examples/docs-examples/spec/3-1-core-prelude-iteration/sec_2_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 3.2 生成関数 | sec_3_2 | examples/docs-examples/spec/3-1-core-prelude-iteration/sec_3_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 3.4 終端操作 | sec_3_4 | examples/docs-examples/spec/3-1-core-prelude-iteration/sec_3_4.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 3.5 パイプライン使用例 | sec_3_5 | examples/docs-examples/spec/3-1-core-prelude-iteration/sec_3_5.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 4.1 専用コレクタ | sec_4_1 | examples/docs-examples/spec/3-1-core-prelude-iteration/sec_4_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 4.2 カスタムコレクタの実装例 | sec_4_2 | examples/docs-examples/spec/3-1-core-prelude-iteration/sec_4_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 6.2 非同期処理との将来統合 | sec_6_2 | examples/docs-examples/spec/3-1-core-prelude-iteration/sec_6_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |

### docs/spec/3-10-core-env.md
| 節 | コード名 | .reml パス | category | 方針 | diag_code | 修正状況 |
| --- | --- | --- | --- | --- | --- | --- |
| 1. 環境変数アクセス | sec_1-a | examples/docs-examples/spec/3-10-core-env/sec_1-a.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 1. 環境変数アクセス | sec_1-b | examples/docs-examples/spec/3-10-core-env/sec_1-b.reml | syntax | Rust Frontend 対応済みのため enum 記法へ復元。 | - | done |
| 2. 一時ディレクトリとパス補助 | sec_2 | examples/docs-examples/spec/3-10-core-env/sec_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 3. プラットフォーム情報の取得 | sec_3 | examples/docs-examples/spec/3-10-core-env/sec_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 4. `@cfg` 連携ガイドライン | sec_4 | examples/docs-examples/spec/3-10-core-env/sec_4.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | todo |

### docs/spec/3-11-core-test.md
| 節 | コード名 | .reml パス | category | 方針 | diag_code | 修正状況 |
| --- | --- | --- | --- | --- | --- | --- |
| 2. 型と API | sec_2 | examples/docs-examples/spec/3-11-core-test/sec_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 2.1 テストブロック | sec_2_1 | examples/docs-examples/spec/3-11-core-test/sec_2_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 4. テーブル駆動テスト | sec_4 | examples/docs-examples/spec/3-11-core-test/sec_4.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 5. ファジングと再現性 | sec_5 | examples/docs-examples/spec/3-11-core-test/sec_5.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 7.1 最小構文 | sec_7_1 | examples/docs-examples/spec/3-11-core-test/sec_7_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 7.2 型とシグネチャ（Core.Parse / Core.Test との整合） | sec_7_2 | examples/docs-examples/spec/3-11-core-test/sec_7_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 7.3 Matcher 仕様（最小セット） | sec_7_3 | examples/docs-examples/spec/3-11-core-test/sec_7_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |

### docs/spec/3-12-core-cli.md
| 節 | コード名 | .reml パス | category | 方針 | diag_code | 修正状況 |
| --- | --- | --- | --- | --- | --- | --- |
| 2. 型と API | sec_2 | examples/docs-examples/spec/3-12-core-cli/sec_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 5. 例 | sec_5 | examples/docs-examples/spec/3-12-core-cli/sec_5.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |

### docs/spec/3-13-core-text-pretty.md
| 節 | コード名 | .reml パス | category | 方針 | diag_code | 修正状況 |
| --- | --- | --- | --- | --- | --- | --- |
| 2. 型と API | sec_2 | examples/docs-examples/spec/3-13-core-text-pretty/sec_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 4. 例 | sec_4-b | examples/docs-examples/spec/3-13-core-text-pretty/sec_4-b.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |

### docs/spec/3-14-core-lsp.md
| 節 | コード名 | .reml パス | category | 方針 | diag_code | 修正状況 |
| --- | --- | --- | --- | --- | --- | --- |
| 1. 基本型 | sec_1 | examples/docs-examples/spec/3-14-core-lsp/sec_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 2. JSON-RPC ヘルパ | sec_2 | examples/docs-examples/spec/3-14-core-lsp/sec_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 4. Core.Lsp.Derive | sec_4 | examples/docs-examples/spec/3-14-core-lsp/sec_4.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 5. LspDerive 出力仕様 | sec_5 | examples/docs-examples/spec/3-14-core-lsp/sec_5.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |

### docs/spec/3-15-core-doc.md
| 節 | コード名 | .reml パス | category | 方針 | diag_code | 修正状況 |
| --- | --- | --- | --- | --- | --- | --- |
| 2. 型と API | sec_2 | examples/docs-examples/spec/3-15-core-doc/sec_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 4. 例 | sec_4 | examples/docs-examples/spec/3-15-core-doc/sec_4.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |

### docs/spec/3-16-core-dsl-paradigm-kits.md
| 節 | コード名 | .reml パス | category | 方針 | diag_code | 修正状況 |
| --- | --- | --- | --- | --- | --- | --- |
| 3.1 主要型 | sec_3_1 | examples/docs-examples/spec/3-16-core-dsl-paradigm-kits/sec_3_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 3.2 最小 API | sec_3_2 | examples/docs-examples/spec/3-16-core-dsl-paradigm-kits/sec_3_2.reml | syntax | 修飾付き関数宣言（`Object.call`）へ復元済み。 | - | done |
| 4.1 主要型 | sec_4_1 | examples/docs-examples/spec/3-16-core-dsl-paradigm-kits/sec_4_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 4.2 最小 API | sec_4_2 | examples/docs-examples/spec/3-16-core-dsl-paradigm-kits/sec_4_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 4.3 例 | sec_4_3 | examples/docs-examples/spec/3-16-core-dsl-paradigm-kits/sec_4_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 5.1 主要型 | sec_5_1 | examples/docs-examples/spec/3-16-core-dsl-paradigm-kits/sec_5_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 5.2 最小 API | sec_5_2 | examples/docs-examples/spec/3-16-core-dsl-paradigm-kits/sec_5_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 6.1 主要型 | sec_6_1 | examples/docs-examples/spec/3-16-core-dsl-paradigm-kits/sec_6_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 6.2 最小 API | sec_6_2 | examples/docs-examples/spec/3-16-core-dsl-paradigm-kits/sec_6_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 6.3 例 | sec_6_3 | examples/docs-examples/spec/3-16-core-dsl-paradigm-kits/sec_6_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |

### docs/spec/3-17-core-net.md
| 節 | コード名 | .reml パス | category | 方針 | diag_code | 修正状況 |
| --- | --- | --- | --- | --- | --- | --- |
| 3.1 主要型と不変条件 | sec_3_1 | examples/docs-examples/spec/3-17-core-net/sec_3_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 3.2 API | sec_3_2 | examples/docs-examples/spec/3-17-core-net/sec_3_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 3.3 UrlError | sec_3_3 | examples/docs-examples/spec/3-17-core-net/sec_3_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 4.1 主要型 | sec_4_1 | examples/docs-examples/spec/3-17-core-net/sec_4_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 4.2 API | sec_4_2 | examples/docs-examples/spec/3-17-core-net/sec_4_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 4.3 HttpError | sec_4_3 | examples/docs-examples/spec/3-17-core-net/sec_4_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 5.1 主要型 | sec_5_1 | examples/docs-examples/spec/3-17-core-net/sec_5_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 5.2 API | sec_5_2 | examples/docs-examples/spec/3-17-core-net/sec_5_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 6.1 主要型 | sec_6_1 | examples/docs-examples/spec/3-17-core-net/sec_6_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 6.2 API | sec_6_2 | examples/docs-examples/spec/3-17-core-net/sec_6_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 7. NetError | sec_7 | examples/docs-examples/spec/3-17-core-net/sec_7.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |

### docs/spec/3-2-core-collections.md
| 節 | コード名 | .reml パス | category | 方針 | diag_code | 修正状況 |
| --- | --- | --- | --- | --- | --- | --- |
| 2.1 `List<T>` | sec_2_1 | examples/docs-examples/spec/3-2-core-collections/sec_2_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 2.2 `Map<K, V>` と `Set<T>` | sec_2_2 | examples/docs-examples/spec/3-2-core-collections/sec_2_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 3.1 `Vec<T>` | sec_3_1 | examples/docs-examples/spec/3-2-core-collections/sec_3_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 3.2 `Cell<T>` / `Ref<T>` | sec_3_2 | examples/docs-examples/spec/3-2-core-collections/sec_3_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 3.3 `Table<K, V>` | sec_3_3 | examples/docs-examples/spec/3-2-core-collections/sec_3_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 7. 使用例（Iter パイプライン） | sec_7 | examples/docs-examples/spec/3-2-core-collections/sec_7.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 8. コレクション間変換ヘルパ | sec_8 | examples/docs-examples/spec/3-2-core-collections/sec_8.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |

### docs/spec/3-3-core-text-unicode.md
| 節 | コード名 | .reml パス | category | 方針 | diag_code | 修正状況 |
| --- | --- | --- | --- | --- | --- | --- |
| 2. 文字列型の層構造 | sec_2 | examples/docs-examples/spec/3-3-core-text-unicode/sec_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 3.1 正規化 API | sec_3_1 | examples/docs-examples/spec/3-3-core-text-unicode/sec_3_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 3.2 ケース・幅調整 | sec_3_2 | examples/docs-examples/spec/3-3-core-text-unicode/sec_3_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 3.3 診断連携と ParseError | sec_3_3 | examples/docs-examples/spec/3-3-core-text-unicode/sec_3_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 4.1 Grapheme / Word / Sentence 境界 | sec_4_1 | examples/docs-examples/spec/3-3-core-text-unicode/sec_4_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 4.2 部分一致と検索 | sec_4_2 | examples/docs-examples/spec/3-3-core-text-unicode/sec_4_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 6. テキスト構築とビルダー | sec_6 | examples/docs-examples/spec/3-3-core-text-unicode/sec_6.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 7.3 セキュリティ考慮事項 | sec_7_3 | examples/docs-examples/spec/3-3-core-text-unicode/sec_7_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 8.1 セグメント構造と抽象構文 | sec_8_1 | examples/docs-examples/spec/3-3-core-text-unicode/sec_8_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 8.2 フィルター登録と Capability 連携 | sec_8_2 | examples/docs-examples/spec/3-3-core-text-unicode/sec_8_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 8.3 コンパイルとレンダリング API | sec_8_3 | examples/docs-examples/spec/3-3-core-text-unicode/sec_8_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 8.4 エラーモデル | sec_8_4 | examples/docs-examples/spec/3-3-core-text-unicode/sec_8_4.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 9. 使用例（Lex 連携と Grapheme 操作） | sec_9 | examples/docs-examples/spec/3-3-core-text-unicode/sec_9.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 9.1 エンコーディング変換ヘルパ | sec_9_1 | examples/docs-examples/spec/3-3-core-text-unicode/sec_9_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 10.1 API と型 | sec_10_1 | examples/docs-examples/spec/3-3-core-text-unicode/sec_10_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |

### docs/spec/3-4-core-numeric-time.md
| 節 | コード名 | .reml パス | category | 方針 | diag_code | 修正状況 |
| --- | --- | --- | --- | --- | --- | --- |
| 1. 数値プリミティブとユーティリティ | sec_1 | examples/docs-examples/spec/3-4-core-numeric-time/sec_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 2. 統計・データ品質サポート | sec_2 | examples/docs-examples/spec/3-4-core-numeric-time/sec_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 3. 時間・期間型 | sec_3-a | examples/docs-examples/spec/3-4-core-numeric-time/sec_3-a.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 3. 時間・期間型 | sec_3-b | examples/docs-examples/spec/3-4-core-numeric-time/sec_3-b.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 3.1 時刻フォーマット | sec_3_1 | examples/docs-examples/spec/3-4-core-numeric-time/sec_3_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 3.2 タイムゾーンサポート | sec_3_2 | examples/docs-examples/spec/3-4-core-numeric-time/sec_3_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 4. メトリクスと監査連携 | sec_4 | examples/docs-examples/spec/3-4-core-numeric-time/sec_4.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 5. 使用例（統計 + メトリクス） | sec_5 | examples/docs-examples/spec/3-4-core-numeric-time/sec_5.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 6.1 数値精度の制御 | sec_6_1 | examples/docs-examples/spec/3-4-core-numeric-time/sec_6_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 6.2 金融計算向け最適化 | sec_6_2 | examples/docs-examples/spec/3-4-core-numeric-time/sec_6_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |

### docs/spec/3-5-core-io-path.md
| 節 | コード名 | .reml パス | category | 方針 | diag_code | 修正状況 | 備考 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 2. Reader / Writer 抽象 | sec_2-a | examples/docs-examples/spec/3-5-core-io-path/sec_2-a.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done | - |
| 2. Reader / Writer 抽象 | sec_2-b | examples/docs-examples/spec/3-5-core-io-path/sec_2-b.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done | Rust Frontend で `operation` 識別子を再許可（実装対応完了）。 |
| 3. ファイルとストリーム | sec_3 | examples/docs-examples/spec/3-5-core-io-path/sec_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done | - |
| 3.1 ストリーミングとバッファ | sec_3_1 | examples/docs-examples/spec/3-5-core-io-path/sec_3_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done | - |
| 4. Path 抽象 | sec_4 | examples/docs-examples/spec/3-5-core-io-path/sec_4.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done | Rust Frontend で `pattern` 識別子を再許可（実装対応完了）。 |
| 4.2 セキュリティヘルパ | sec_4_2 | examples/docs-examples/spec/3-5-core-io-path/sec_4_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done | - |
| 4.3 文字列ユーティリティ（クロスプラットフォーム） | sec_4_3 | examples/docs-examples/spec/3-5-core-io-path/sec_4_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done | - |
| 4.4 ファイル監視（オプション） | sec_4_4-a | examples/docs-examples/spec/3-5-core-io-path/sec_4_4-a.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done | - |
| 4.4 ファイル監視（オプション） | sec_4_4-b | examples/docs-examples/spec/3-5-core-io-path/sec_4_4-b.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done | - |
| 5. リソース解放と `defer` | sec_5-a | examples/docs-examples/spec/3-5-core-io-path/sec_5-a.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done | - |
| 5. リソース解放と `defer` | sec_5-b | examples/docs-examples/spec/3-5-core-io-path/sec_5-b.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done | - |
| 6. 監査ログ連携 | sec_6 | examples/docs-examples/spec/3-5-core-io-path/sec_6.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done | - |
| 7. 使用例（設定ファイル読み込み） | sec_7 | examples/docs-examples/spec/3-5-core-io-path/sec_7.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done | - |
| 8.1 同期・非同期ブリッジ | sec_8_1 | examples/docs-examples/spec/3-5-core-io-path/sec_8_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done | - |
| 8.2 リソースプールと最適化 | sec_8_2 | examples/docs-examples/spec/3-5-core-io-path/sec_8_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done | Rust Frontend で `operation` 識別子を再許可（実装対応完了）。 |
| 9. Resource Limit ユーティリティ (`Core.Resource`) | sec_9 | examples/docs-examples/spec/3-5-core-io-path/sec_9.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done | - |

### docs/spec/3-6-core-diagnostics-audit.md
| 節 | コード名 | .reml パス | category | 方針 | diag_code | 修正状況 |
| --- | --- | --- | --- | --- | --- | --- |
| 1. `Diagnostic` 構造体 | sec_1 | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 1.1 `AuditEnvelope` | sec_1_1 | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_1_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 1.1.1 監査イベント `AuditEvent` | sec_1_1_1 | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_1_1_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 1.3 効果診断拡張 `effects` | sec_1_3 | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_1_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 1.4 型クラス診断拡張 `typeclass` | sec_1_4 | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_1_4.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 2. 診断生成ヘルパ | sec_2 | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 2.1 `Result`/`Option` との連携 | sec_2_1 | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_2_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 2.2 Core.Parse 連携（`Parse.fail` / `Parse.recover`） | sec_2_2 | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_2_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 2.3 エラーコードカタログ | sec_2_3 | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_2_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 2.4.1 Core.Parse プリセット `parse_error_defaults` | sec_2_4_1 | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_2_4_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 2.4.6 Stage 差分プリセット `EffectDiagnostic` | sec_2_4_6 | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_2_4_6.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 2.5.1 Supervisor 診断拡張 | sec_2_5_1 | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_2_5_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 2.6 Core Prelude ガード診断 | sec_2_6 | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_2_6.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 3. 監査ログ出力 | sec_3-a | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_3-a.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 3. 監査ログ出力 | sec_3-b | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_3-b.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 3.1 `AuditError` | sec_3_1 | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_3_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 3.2 監査ポリシー管理 | sec_3_2 | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_3_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 3.3 監査コンテキストとシステム呼び出し連携 | sec_3_3-a | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_3_3-a.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 3.3 監査コンテキストとシステム呼び出し連携 | sec_3_3-b | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_3_3-b.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 4.1 個人情報の除去 | sec_4_1 | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_4_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 4.2 セキュリティ監査 | sec_4_2 | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_4_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 5. 差分・監査テンプレート | sec_5 | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_5.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 6.1 メトリクスプリセット | sec_6_1 | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_6_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 6.1.2 Conductor 診断拡張 `conductor` | sec_6_1_2 | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_6_1_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 6.1.3 Config 診断拡張 `config` | sec_6_1_3 | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_6_1_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 6.2 トレース統合 | sec_6_2 | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_6_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 6.3 ターゲット診断メトリクス | sec_6_3 | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_6_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 6.4 テンプレート診断ドメイン | sec_6_4 | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_6_4.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 8.1 拡張データ `bridge` | sec_8_1 | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_8_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 9.1 `pipeline_*` サンプルと CLI/監査ログ | sec_9_1 | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_9_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 10.1 CLI ツール統合 | sec_10_1 | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_10_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 10.2 LSP サーバー統合 | sec_10_2 | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_10_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 11. CLI コマンドプロトコル | sec_11 | examples/docs-examples/spec/3-6-core-diagnostics-audit/sec_11.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |

### docs/spec/3-7-core-config-data.md
| 節 | コード名 | .reml パス | category | 方針 | diag_code | 修正状況 |
| --- | --- | --- | --- | --- | --- | --- |
| 1.2 API | sec_1_2 | examples/docs-examples/spec/3-7-core-config-data/sec_1_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 1.5 互換モード（`ConfigCompatibility`） | sec_1_5 | examples/docs-examples/spec/3-7-core-config-data/sec_1_5.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 1.5.3 診断生成と効果タグ | sec_1_5_3 | examples/docs-examples/spec/3-7-core-config-data/sec_1_5_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 1.5.4 Config 診断拡張の適用 | sec_1_5_4 | examples/docs-examples/spec/3-7-core-config-data/sec_1_5_4.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 2. Config スキーマ API（再整理） | sec_2 | examples/docs-examples/spec/3-7-core-config-data/sec_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 2.1 スキーマ差分 | sec_2_1 | examples/docs-examples/spec/3-7-core-config-data/sec_2_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 3. Config 実行 API | sec_3 | examples/docs-examples/spec/3-7-core-config-data/sec_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 3.1 マイグレーション安全性 | sec_3_1 | examples/docs-examples/spec/3-7-core-config-data/sec_3_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 4.1 Nest.Data スキーマ構築 | sec_4_1-a | examples/docs-examples/spec/3-7-core-config-data/sec_4_1-a.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 4.1 Nest.Data スキーマ構築 | sec_4_1-b | examples/docs-examples/spec/3-7-core-config-data/sec_4_1-b.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 4.2 制約とプロファイル検証 | sec_4_2 | examples/docs-examples/spec/3-7-core-config-data/sec_4_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 4.3 プロファイル別評価とメトリクス | sec_4_3 | examples/docs-examples/spec/3-7-core-config-data/sec_4_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 4.7 データ品質検証 API | sec_4_7 | examples/docs-examples/spec/3-7-core-config-data/sec_4_7.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 4.8 統計との連携 | sec_4_8 | examples/docs-examples/spec/3-7-core-config-data/sec_4_8.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 5. CLI / ツール連携 | sec_5 | examples/docs-examples/spec/3-7-core-config-data/sec_5.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 6. 使用例（差分レビュー） | sec_6 | examples/docs-examples/spec/3-7-core-config-data/sec_6.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 7.1 スキーマバージョニング | sec_7_1 | examples/docs-examples/spec/3-7-core-config-data/sec_7_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 7.2 動的スキーマ生成 | sec_7_2 | examples/docs-examples/spec/3-7-core-config-data/sec_7_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 7.3 スキーマ演算 | sec_7_3 | examples/docs-examples/spec/3-7-core-config-data/sec_7_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |

### docs/spec/3-8-core-runtime-capability.md
| 節 | コード名 | .reml パス | category | 方針 | diag_code | 修正状況 |
| --- | --- | --- | --- | --- | --- | --- |
| 1. Capability Registry の基本構造 | sec_1 | examples/docs-examples/spec/3-8-core-runtime-capability/sec_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 1.1 CapabilityHandle のバリアント | sec_1_1 | examples/docs-examples/spec/3-8-core-runtime-capability/sec_1_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 1.2 セキュリティモデル | sec_1_2 | examples/docs-examples/spec/3-8-core-runtime-capability/sec_1_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 効果ステージとハンドラ契約 | sec_section-a | examples/docs-examples/spec/3-8-core-runtime-capability/sec_section-a.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 効果ステージとハンドラ契約 | sec_section-b | examples/docs-examples/spec/3-8-core-runtime-capability/sec_section-b.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 効果ステージとハンドラ契約 | sec_section-c | examples/docs-examples/spec/3-8-core-runtime-capability/sec_section-c.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 1.3 プラットフォーム情報と能力 | sec_1_3 | examples/docs-examples/spec/3-8-core-runtime-capability/sec_1_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 1.3.1 `@dsl_export` との整合 | sec_1_3_1 | examples/docs-examples/spec/3-8-core-runtime-capability/sec_1_3_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 1.4 非同期・Actor Capability | sec_1_4 | examples/docs-examples/spec/3-8-core-runtime-capability/sec_1_4.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 1.6 CapabilityError | sec_1_6 | examples/docs-examples/spec/3-8-core-runtime-capability/sec_1_6.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 2.1 SyscallCapability | sec_2_1 | examples/docs-examples/spec/3-8-core-runtime-capability/sec_2_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 2.2 ProcessCapability | sec_2_2 | examples/docs-examples/spec/3-8-core-runtime-capability/sec_2_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 2.3 MemoryCapability | sec_2_3 | examples/docs-examples/spec/3-8-core-runtime-capability/sec_2_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 2.4 SignalCapability | sec_2_4 | examples/docs-examples/spec/3-8-core-runtime-capability/sec_2_4.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 2.5 HardwareCapability | sec_2_5 | examples/docs-examples/spec/3-8-core-runtime-capability/sec_2_5.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 2.6 RealTimeCapability | sec_2_6 | examples/docs-examples/spec/3-8-core-runtime-capability/sec_2_6.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 2.7 SecurityCapability | sec_2_7 | examples/docs-examples/spec/3-8-core-runtime-capability/sec_2_7.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 3. GC Capability インターフェイス | sec_3 | examples/docs-examples/spec/3-8-core-runtime-capability/sec_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 3.1 メモリ管理の高度制御 | sec_3_1 | examples/docs-examples/spec/3-8-core-runtime-capability/sec_3_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 4. Metrics & Audit Capability | sec_4 | examples/docs-examples/spec/3-8-core-runtime-capability/sec_4.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 5.1 IoCapability | sec_5_1 | examples/docs-examples/spec/3-8-core-runtime-capability/sec_5_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 6. プラグイン Capability | sec_6 | examples/docs-examples/spec/3-8-core-runtime-capability/sec_6.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 6.1 プラグインサンドボックス | sec_6_1 | examples/docs-examples/spec/3-8-core-runtime-capability/sec_6_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 7. DSL Capability Utility | sec_7-a | examples/docs-examples/spec/3-8-core-runtime-capability/sec_7-a.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 7. DSL Capability Utility | sec_7-b | examples/docs-examples/spec/3-8-core-runtime-capability/sec_7-b.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 7.3 テンプレート Capability プリセット | sec_7_3 | examples/docs-examples/spec/3-8-core-runtime-capability/sec_7_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 7.4 Capability マニフェスト変換ユーティリティ | sec_7_4 | examples/docs-examples/spec/3-8-core-runtime-capability/sec_7_4.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 8. 使用例（GC + Metrics 登録） | sec_8 | examples/docs-examples/spec/3-8-core-runtime-capability/sec_8.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 9.1 リアルタイムメトリクス | sec_9_1 | examples/docs-examples/spec/3-8-core-runtime-capability/sec_9_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 9.2 パフォーマンスプロファイリング | sec_9_2 | examples/docs-examples/spec/3-8-core-runtime-capability/sec_9_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 9.3 ランタイムデバッグ | sec_9_3 | examples/docs-examples/spec/3-8-core-runtime-capability/sec_9_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 10.1 RuntimeBridgeRegistry とメタデータ | sec_10_1-a | examples/docs-examples/spec/3-8-core-runtime-capability/sec_10_1-a.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 10.1 RuntimeBridgeRegistry とメタデータ | sec_10_1-b | examples/docs-examples/spec/3-8-core-runtime-capability/sec_10_1-b.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 10.1 RuntimeBridgeRegistry とメタデータ | sec_10_1-c | examples/docs-examples/spec/3-8-core-runtime-capability/sec_10_1-c.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 10.3 Reload 契約 | sec_10_3-a | examples/docs-examples/spec/3-8-core-runtime-capability/sec_10_3-a.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 10.3 Reload 契約 | sec_10_3-b | examples/docs-examples/spec/3-8-core-runtime-capability/sec_10_3-b.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 10.5 ストリーミング Signal ハンドラ | sec_10_5 | examples/docs-examples/spec/3-8-core-runtime-capability/sec_10_5.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |

### docs/spec/3-9-core-async-ffi-unsafe.md
| 節 | コード名 | .reml パス | category | 方針 | diag_code | 修正状況 |
| --- | --- | --- | --- | --- | --- | --- |
| 1. Core.Async の枠組み | sec_1 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 1.2 高度な非同期パターン | sec_1_2 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_1_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 1.3 ストリームとアシンクイテレータ | sec_1_3 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_1_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 1.4 DSLオーケストレーション支援 API | sec_1_4 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_1_4.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 1.4.1 Codec 契約 | sec_1_4_1 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_1_4_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 1.4.3 ExecutionPlan の整合性 | sec_1_4_3 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_1_4_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 1.4.5 チャネルメトリクス API | sec_1_4_5 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_1_4_5.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 1.5 プラットフォーム適応スケジューラ | sec_1_5 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_1_5.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 1.8 AsyncError | sec_1_8 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_1_8.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 1.9 アクターモデルと分散メッセージング | sec_1_9 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_1_9.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 1.9.2 分散トランスポート | sec_1_9_2 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_1_9_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 1.9.3 DSL からの利用例 | sec_1_9_3 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_1_9_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 1.9.5 Supervisor パターンと再起動戦略 | sec_1_9_5 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_1_9_5.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 2. Core.Ffi の枠組み | sec_2 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 2.0 バインディング生成と Capability 連携 | sec_2_0-a | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_2_0-a.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 2.0 バインディング生成と Capability 連携 | sec_2_0-b | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_2_0-b.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 2.3 効果ハンドラによる FFI サンドボックス（実験段階） | sec_2_3 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_2_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 2.4 タイプセーフな FFI ラッパー | sec_2_4 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_2_4.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 2.4.1 Core.Ffi.Dsl | sec_2_4_1 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_2_4_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 2.4.1.1 DSL 利用例 | sec_2_4_1_1 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_2_4_1_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 2.5 呼出規約とプラットフォーム適応 | sec_2_5 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_2_5.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 2.6 メモリ管理と所有権境界 | sec_2_6 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_2_6.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 3.1 型定義 | sec_3_1 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_3_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 3.2 生成・変換 API | sec_3_2 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_3_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 3.3 読み書き・コピー API | sec_3_3 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_3_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 3.4 アドレス計算と Span ユーティリティ | sec_3_4 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_3_4.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 3.5 診断・監査補助 | sec_3_5 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_3_5.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 3.6.1 FFI コール境界 | sec_3_6_1 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_3_6_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 3.6.2 バッファ操作 | sec_3_6_2 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_3_6_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 3.6.3 GC ルート登録 | sec_3_6_3 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_3_6_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 4. Core.Unsafe の指針 | sec_4 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_4.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 4.1 安全性検証メカニズム | sec_4_1 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_4_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 4.2 監査された unsafe 操作 | sec_4_2 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_4_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 5.1 非同期 Capability | sec_5_1 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_5_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 5.2 FFI Capability | sec_5_2 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_5_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 5.3 Unsafe Capability | sec_5_3 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_5_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 6. 使用例（調査メモ） | sec_6 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_6.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 7.1 非同期セキュリティ | sec_7_1 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_7_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 7.2 FFI セキュリティ | sec_7_2 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_7_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 7.3 Unsafe セキュリティ | sec_7_3 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_7_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 8.1 非同期最適化 | sec_8_1 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_8_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 8.2 FFI 最適化 | sec_8_2 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_8_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 9.1 非同期デバッグ | sec_9_1 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_9_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 9.2 FFI テスト | sec_9_2 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_9_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 9.3 Unsafe テスト | sec_9_3 | examples/docs-examples/spec/3-9-core-async-ffi-unsafe/sec_9_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |

### docs/spec/5-1-system-plugin.md
| 節 | コード名 | .reml パス | category | 方針 | diag_code | 修正状況 |
| --- | --- | --- | --- | --- | --- | --- |
| 1. SyscallCapability API | sec_1 | examples/docs-examples/spec/5-1-system-plugin/sec_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 2. プラットフォーム別ラッパ構造 | sec_2 | examples/docs-examples/spec/5-1-system-plugin/sec_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 3. SyscallDescriptor と監査連携 | sec_3 | examples/docs-examples/spec/5-1-system-plugin/sec_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 3.1 `audited_syscall` の実装指針 | sec_3_1 | examples/docs-examples/spec/5-1-system-plugin/sec_3_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 4. システムエラーと変換 | sec_4 | examples/docs-examples/spec/5-1-system-plugin/sec_4.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |
| 5. セキュリティポリシーとの統合 | sec_5 | examples/docs-examples/spec/5-1-system-plugin/sec_5.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | done |

### docs/spec/5-2-process-plugin.md
| 節 | コード名 | .reml パス | category | 方針 | diag_code | 修正状況 |
| --- | --- | --- | --- | --- | --- | --- |
| 1. ProcessCapability API | sec_1 | examples/docs-examples/spec/5-2-process-plugin/sec_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | todo |
| 2. プロセス生成と監査 | sec_2 | examples/docs-examples/spec/5-2-process-plugin/sec_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | todo |
| 2.1 `spawn_process` の動作例 | sec_2_1 | examples/docs-examples/spec/5-2-process-plugin/sec_2_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | todo |
| 3. プロセス終了待ちと時間制限 | sec_3 | examples/docs-examples/spec/5-2-process-plugin/sec_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | todo |
| 4. スレッド API | sec_4 | examples/docs-examples/spec/5-2-process-plugin/sec_4.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | todo |
| 5. エラー構造 | sec_5 | examples/docs-examples/spec/5-2-process-plugin/sec_5.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | todo |

### docs/spec/5-3-memory-plugin.md
| 節 | コード名 | .reml パス | category | 方針 | diag_code | 修正状況 |
| --- | --- | --- | --- | --- | --- | --- |
| 1. MemoryCapability API | sec_1 | examples/docs-examples/spec/5-3-memory-plugin/sec_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | todo |
| 2. 型定義 | sec_2 | examples/docs-examples/spec/5-3-memory-plugin/sec_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | todo |
| 2.1 共有メモリ | sec_2_1 | examples/docs-examples/spec/5-3-memory-plugin/sec_2_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | todo |
| 3. リクエスト構造 | sec_3 | examples/docs-examples/spec/5-3-memory-plugin/sec_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | todo |
| 4. MemoryError と診断 | sec_4 | examples/docs-examples/spec/5-3-memory-plugin/sec_4.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | todo |
| 5. 監査テンプレート | sec_5 | examples/docs-examples/spec/5-3-memory-plugin/sec_5.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | todo |
| 6. 高レベルユーティリティ | sec_6 | examples/docs-examples/spec/5-3-memory-plugin/sec_6.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | todo |

### docs/spec/5-4-signal-plugin.md
| 節 | コード名 | .reml パス | category | 方針 | diag_code | 修正状況 |
| --- | --- | --- | --- | --- | --- | --- |
| 1. SignalCapability API | sec_1 | examples/docs-examples/spec/5-4-signal-plugin/sec_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | todo |
| 2. 型定義 | sec_2 | examples/docs-examples/spec/5-4-signal-plugin/sec_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | todo |
| 3. 監査とセキュリティ | sec_3 | examples/docs-examples/spec/5-4-signal-plugin/sec_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | todo |
| 4.1 グレースフルシャットダウン | sec_4_1 | examples/docs-examples/spec/5-4-signal-plugin/sec_4_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | todo |
| 4.2 シグナル待ち | sec_4_2 | examples/docs-examples/spec/5-4-signal-plugin/sec_4_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | todo |

### docs/spec/5-5-hardware-plugin.md
| 節 | コード名 | .reml パス | category | 方針 | diag_code | 修正状況 |
| --- | --- | --- | --- | --- | --- | --- |
| 1. HardwareCapability API | sec_1 | examples/docs-examples/spec/5-5-hardware-plugin/sec_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | todo |
| 2. 型定義 | sec_2 | examples/docs-examples/spec/5-5-hardware-plugin/sec_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | todo |
| 3. エラーと監査 | sec_3 | examples/docs-examples/spec/5-5-hardware-plugin/sec_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | todo |

### docs/spec/5-6-realtime-plugin.md
| 節 | コード名 | .reml パス | category | 方針 | diag_code | 修正状況 |
| --- | --- | --- | --- | --- | --- | --- |
| 1. RealTimeCapability API | sec_1 | examples/docs-examples/spec/5-6-realtime-plugin/sec_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | todo |
| 2. 型定義 | sec_2 | examples/docs-examples/spec/5-6-realtime-plugin/sec_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | todo |
| 3. エラーと監査 | sec_3 | examples/docs-examples/spec/5-6-realtime-plugin/sec_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | todo |

### docs/spec/5-7-core-parse-plugin.md
| 節 | コード名 | .reml パス | category | 方針 | diag_code | 修正状況 |
| --- | --- | --- | --- | --- | --- | --- |
| 1.1 ID・バージョン・互換性 | sec_1_1 | examples/docs-examples/spec/5-7-core-parse-plugin/sec_1_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | todo |
| 1.2 Capability 宣言 | sec_1_2 | examples/docs-examples/spec/5-7-core-parse-plugin/sec_1_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | todo |
| 1.3 依存とバンドル構造 | sec_1_3 | examples/docs-examples/spec/5-7-core-parse-plugin/sec_1_3.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | todo |
| 2.1 `ParserPlugin` 構造 | sec_2_1 | examples/docs-examples/spec/5-7-core-parse-plugin/sec_2_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | todo |
| 2.2 ランタイム API | sec_2_2 | examples/docs-examples/spec/5-7-core-parse-plugin/sec_2_2.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | todo |
| 3.1 署名・ステージ検証の連携 | sec_3_1 | examples/docs-examples/spec/5-7-core-parse-plugin/sec_3_1.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | todo |
| 4. エラーモデルと診断 | sec_4-a | examples/docs-examples/spec/5-7-core-parse-plugin/sec_4-a.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | todo |
| 4. エラーモデルと診断 | sec_4-b | examples/docs-examples/spec/5-7-core-parse-plugin/sec_4-b.reml | syntax | Rust Frontend 未対応の可能性。仕様優先で簡略サンプル/宣言順調整/関数ラップを検討。必要なら rustcap を併記。 | - | todo |
