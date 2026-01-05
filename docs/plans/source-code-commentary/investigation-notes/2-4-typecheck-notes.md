# 第2部 第7章: 型チェックと型推論 調査メモ

## 参照した資料
- `compiler/frontend/src/typeck/mod.rs:1-31`（型推論モジュールの位置づけと再エクスポート）
- `compiler/frontend/src/typeck/driver.rs:1-240`（TypecheckDriver の入口と主要フロー）
- `compiler/frontend/src/typeck/driver.rs:716-980`（TypecheckReport/TypecheckViolation/ViolationKind 定義）
- `compiler/frontend/src/typeck/types.rs:1-220`（Type/TypeVariable/BuiltinType/TypeVarGen）
- `compiler/frontend/src/typeck/constraint.rs:1-220`（Constraint/Substitution/ConstraintSolver/unify）
- `compiler/frontend/src/typeck/constraint/iterator.rs:1-177`（Iterator 制約と Stage プロファイル）
- `compiler/frontend/src/typeck/env.rs:1-220`（TypecheckConfig/StageContext の定義）
- `compiler/frontend/src/typeck/env.rs:640-980`（TypeRowMode/DualWrite/TypeEnv 周辺）
- `compiler/frontend/src/typeck/scheme.rs:1-48`（Scheme と instantiate）
- `compiler/frontend/src/typeck/metrics.rs:1-73`（TypecheckMetrics）
- `compiler/frontend/src/typeck/telemetry.rs:1-220`（TraitResolutionTelemetry/ConstraintGraph）
- `compiler/frontend/src/typeck/capability.rs:1-210`（CapabilityDescriptor/RuntimeCapability）
- `compiler/frontend/src/bin/reml_frontend.rs:640-707`（TypecheckDriver 呼び出しと成果物生成）
- `compiler/frontend/src/semantics/typed.rs:12-173`（Typed AST の主要構造）
- `docs/spec/1-2-types-Inference.md`（型と推論の仕様）

## 調査メモ

### 型推論モジュールの位置づけ
- `typeck` はフロントエンドの型推論スタックのルートで、W3 時点では骨組みのみを提供する前提が明記されている。(`compiler/frontend/src/typeck/mod.rs:1-4`)
- CLI ではパース後に `TypecheckDriver::infer_module` を呼び、`TypecheckReport` と `TypeckArtifacts` を生成する。(`compiler/frontend/src/bin/reml_frontend.rs:656-707`)

### 型表現と型変数の取り扱い
- 型は `Type` enum（変数/組み込み/関数/型適用/スライス/参照）として表現され、`TypeVariable` は ID で識別する。(`compiler/frontend/src/typeck/types.rs:6-106`)
- `Type::contains_variable` で occurs check のための探索が行える。(`compiler/frontend/src/typeck/types.rs:109-156`)
- `BuiltinType` は Int/UInt/Float/Bool/Char/Str/Bytes/Unit/Unknown をカバー。(`compiler/frontend/src/typeck/types.rs:208-220`)
- `TypeVarGen` が新しい型変数を採番し、`fresh_type` で `Type::Var` を生成する。(`compiler/frontend/src/typeck/types.rs:243-260`)

### 制約と単一化
- `Constraint` は Equal/HasCapability/ImplBound の 3 系統。(`compiler/frontend/src/typeck/constraint.rs:10-27`)
- `Substitution` は `TypeVariable -> Type` の写像で、型への適用 `apply` を持つ。(`compiler/frontend/src/typeck/constraint.rs:50-108`)
- `ConstraintSolver::unify` は構造的単一化を行い、関数型/型適用/参照/スライスを再帰的に突き合わせる。(`compiler/frontend/src/typeck/constraint.rs:117-205`)
- `bind_variable` で occurs check を行い循環型を拒否する。(`compiler/frontend/src/typeck/constraint.rs:243-259`)
- `solve` は現時点では substitution を返すだけのスタブ。(`compiler/frontend/src/typeck/constraint.rs:261-268`)

### 型スキームと環境
- `Scheme` は量化変数・制約付き型を保持し、`instantiate` で新しい型変数に差し替える。(`compiler/frontend/src/typeck/scheme.rs:9-48`)
- `TypeEnv` は `Binding`/`TypeDeclBinding`/`TypeConstructorBinding` を束縛し、`enter_scope` でスコープをネストできる。(`compiler/frontend/src/typeck/env.rs:833-980`)
- `TypecheckConfig` は effect/stage 文脈や type_row_mode 等をまとめ、OnceCell でグローバルに保持する。(`compiler/frontend/src/typeck/env.rs:27-135`)
- `TypeRowMode` は MetadataOnly/DualWrite/Integrated を切り替えられる。(`compiler/frontend/src/typeck/env.rs:691-714`)
- `DualWriteGuards` が `reports/dual-write/front-end` 以下への出力を補助する。(`compiler/frontend/src/typeck/env.rs:740-815`)

### TypecheckReport と診断情報
- `TypecheckReport` は metrics/functions/violations/typed_module/mir/constraints/used_impls を束ねる。(`compiler/frontend/src/typeck/driver.rs:716-725`)
- `TypecheckViolation` は code/message/span/notes と recovery 情報や stage mismatch 情報を保持する。(`compiler/frontend/src/typeck/driver.rs:763-786`)
- `TypecheckViolationKind` は type/effects/parser/runtime に跨る多様な違反種別を列挙する。(`compiler/frontend/src/typeck/driver.rs:803-852`)

### 型付き AST と MIR への接続
- `TypedModule` は型付き AST のトップで、TypedFunction/TypedExpr などを保持する。(`compiler/frontend/src/semantics/typed.rs:12-173`)
- `TypecheckReport` が `typed::TypedModule` と `mir::MirModule` を同時に保持するため、型推論は後続の意味解析/パイプラインと密接に接続される。(`compiler/frontend/src/typeck/driver.rs:716-725`)

### Capability/Stage と Iterator 制約
- `CapabilityDescriptor::resolve` が effect 名から capability ID と stage を決める。(`compiler/frontend/src/typeck/capability.rs:85-152`)
- `RuntimeCapability` は CLI/設定からの capability 入力を受け付け、`id@stage` 形式を解釈する。(`compiler/frontend/src/typeck/capability.rs:155-199`)
- `IteratorDictInfo` と `IteratorStageProfile` が `Iterator` の種別に応じた stage/capability を保持する。(`compiler/frontend/src/typeck/constraint/iterator.rs:9-115`)

### テレメトリとメトリクス
- `TypecheckMetrics` は型推論フェーズの統計（typed expr, constraints 等）を保持する。(`compiler/frontend/src/typeck/metrics.rs:4-73`)
- `TraitResolutionTelemetry` は制約グラフと解決結果を JSON にするためのスナップショットを構築する。(`compiler/frontend/src/typeck/telemetry.rs:15-76`)

### 仕様との照合メモ
- 実装は `docs/spec/1-2-types-Inference.md` の HM/制約ベース推論の方向性を志向しているが、`ConstraintSolver::solve` は未実装で段階的移行中である。

### 未確認事項 / TODO
- `TypecheckViolation` の各種コードがどの診断テンプレート（`diagnostic/messages`）と対応しているかを追跡する。
- `TypeRowMode::DualWrite` の出力先と JSON スキーマの整合性を確認する。
- `TypecheckDriver` が生成する `mir::MirModule` の詳細と Typed AST の対応関係を追跡する。
