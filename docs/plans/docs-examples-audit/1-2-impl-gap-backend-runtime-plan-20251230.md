# 1.2 実装ギャップ後続対応計画（Backend / Runtime / 2025-12-30）

`docs/plans/docs-examples-audit/1-2-impl-gap-plan-20251230-1.md` で追加した構文（effect/async/await/unsafe/extern 等）が Backend / Runtime に与える影響を整理し、必要な後続対応を計画する。

## 目的
- Frontend MIR 追加要素と Backend の取り込み仕様を同期する。
- 未対応構文が Backend でパニック/失敗しないよう診断か暫定ロワリングを定義する。
- Runtime 側で追加対応が必要かを明確化し、必要なら別計画へ接続する。

## 対象範囲
- Frontend MIR: `compiler/frontend/src/semantics/mir.rs`
- Backend MIR 取り込み: `compiler/backend/llvm/src/integration.rs`
- Backend コード生成: `compiler/backend/llvm/src/codegen.rs`
- Runtime: `compiler/runtime/`（影響確認のみ）

## 背景
- Frontend MIR には `EffectBlock` / `Async` / `Await` / `Unsafe` が追加されている。
- Backend 側の `MirExprKindJson` はこれらの `kind` を受理していないため、MIR JSON のデシリアライズが失敗する。
- Frontend MIR は `is_async` / `is_unsafe` / `varargs` / `externs` を出力するが、Backend 側で読み取っていない。

## ギャップ一覧（Backend / Runtime 影響）
1) **MIR JSON 取り込みの未対応**
- Frontend: `MirExprKind` に `effect_block` / `async` / `await` / `unsafe` を追加済み。
- Backend: `MirExprKindJson` に同名バリアントが無く、未知タグで失敗する。

2) **関数フラグ / extern 定義の無視**
- Frontend: `MirFunction` に `is_async` / `is_unsafe` / `varargs`、`MirModule` に `externs` を出力。
- Backend: `MirFunctionJson` / `MirModuleSpec` がこれらを取り込まないため、Backend 側で診断が出ない。

3) **未対応構文のコード生成方針が未定義**
- `Async` / `Await` / `EffectBlock` / `Unsafe` の MIR がコード生成段階で解釈されない。
- Runtime 側で対応 API が明示されていないため、Backend の挙動を決める必要がある。

## 実装修正計画

### フェーズ 1: MIR JSON 取り込みの同期
完了（2025-12-30）
- `MirExprKindJson` に `effect_block` / `async` / `await` / `unsafe` を追加して受理。
- `convert_expr_kind` は暫定で `MirExprKind::Unknown` にフォールバック。
- `MirFunctionJson` に `is_async` / `is_unsafe` / `varargs` を追加。
- `MirModuleSpec` に `externs` を追加（`MirExternJson` 定義）。
- 最小 JSON での読み込みテストを追加（Backend 側）。

1) `MirExprKindJson` の追加
- `effect_block` / `async` / `await` / `unsafe` を追加し、Frontend MIR と同じフィールド構成で読み取る。
- 未対応のまま破綻しないよう、取り込み時に暫定診断を生成できる構造を追加する。
- 作業ステップ:
  - `compiler/frontend/src/semantics/mir.rs` の `MirExprKind` 定義を確認し、各バリアントの JSON 形式（`kind` 名・フィールド）を整理する。
  - `compiler/backend/llvm/src/integration.rs` の `MirExprKindJson` に `EffectBlock` / `Async` / `Await` / `Unsafe` を追加する。
  - `EffectBlock` / `Unsafe` は `body: usize`、`Async` は `body: usize` + `is_move: bool`、`Await` は `expr: usize` を Frontend と同名で受理する。
  - `convert_expr_kind` に対応する変換分岐を追加し、暫定で `MirExprKind::Unknown` へフォールバックするか、専用 `MirExprKind` を追加するか判断する。
  - 取り込み失敗時の `serde` エラーが消えることを確認するため、最小 MIR JSON（`kind: "async"` など）で読み込みテストを用意する。

2) `MirFunctionJson` / `MirModuleSpec` の拡張
- `is_async` / `is_unsafe` / `varargs` を任意フィールドとして読み取る。
- `externs` を `MirModuleSpec` に追加し、一覧を取り込めるようにする。
- 作業ステップ:
  - Frontend の `MirFunction` / `MirExtern` を確認し、JSON に出力されるフィールド名を整理する。
  - `MirFunctionJson` に `#[serde(default)] is_async: bool` / `is_unsafe: bool` / `varargs: bool` を追加する。
  - `MirModuleSpec` に `externs: Vec<MirExternJson>` を追加し、構造体を定義する（`name` / `abi` / `symbol` / `span` を `#[serde(default)]` で受理）。
  - `MirFunctionJson::into_mir` でフラグを `MirFunction` に反映するか、Backend 内部で診断に使う専用構造へ保持するかを決める。
  - `MirExternJson` の取り込み経路を作成し、`BackendDiffSnapshot` にログ出力できるか確認する。
  - `externs` が空の場合に既存のスナップショット差分が増えないよう `#[serde(default)]` を徹底する。

### フェーズ 2: Backend 側の暫定ロワリング方針
1) `EffectBlock` / `Unsafe`
- 既存の `Block` へ単純委譲する暫定方針（`body` をそのまま評価）を追加する。
- 効果システム未統合のため、診断 `backend.todo.effect_block` / `backend.todo.unsafe_block` を記録する。
- 作業ステップ:
  - `compiler/backend/llvm/src/codegen.rs` の `emit_expr` / `infer_expr_llvm_type` / `describe_expr` で `MirExprKind` の分岐を確認する。
  - `MirExprKind` に新規バリアントを追加する場合は `EffectBlock { body }` / `Unsafe { body }` を導入する。
  - ロワリングは `emit_expr(body, ...)` に委譲し、戻り値の整合が保てることを確認する。
  - `BackendDiffSnapshot` の `diagnostics` に `backend.todo.effect_block` / `backend.todo.unsafe_block` を追加するフックを用意する。
  - 既存の `MirExprKind::Unknown` を使う方針なら、診断発行位置を `convert_expr_kind` か `collect_todo_diagnostics` に追加する。

2) `Async` / `Await`
- 暫定対応として `Async` / `Await` を `Unknown` へ落とし、診断 `backend.todo.async` / `backend.todo.await` を生成する。
- 併せて `is_async = true` の関数に対して `backend.todo.async_function` を記録する。
- 作業ステップ:
  - `MirExprKindJson` で `Async` / `Await` を受理した後、`convert_expr_kind` で暫定 `Unknown` へ落とすか、専用バリアントを追加する。
  - 専用バリアントを追加する場合は `emit_expr` で `Unknown` 相当の処理へ委譲しつつ診断を積む。
  - `MirFunctionJson` の `is_async` を取り込み、`MirFunction` もしくは `BackendDiffSnapshot` に反映する。
  - `collect_todo_diagnostics` に `is_async` 関数のチェックを追加し、`backend.todo.async_function` を出す。
  - `await` が評価位置に現れた場合の診断文言に `expr_id` を含めて追跡可能にする。

3) `externs` の扱い
- `MirExtern` を Backend 側で受理し、LLVM 側に宣言のみ生成するか、診断 `backend.todo.extern_decl` として未対応扱いにするかを決定する。
- `extern` 呼び出しが `ffi_calls` 経路で既に処理される場合は統合ルールを整理する。
- 作業ステップ:
  - `MirExtern` の利用箇所を調査し、Frontend 側でどの段階の情報が `ffi_calls` に渡るかを確認する。
  - Backend で `externs` を保持する構造体を追加する（保持のみ/診断のみのどちらかを明記）。
  - LLVM へ宣言のみ出す場合は `codegen.rs` に宣言登録処理を追加し、既存の `ffi_calls` と重複しないよう名寄せ規則を定める。
  - 未対応扱いにする場合は `collect_todo_diagnostics` で `backend.todo.extern_decl` を生成し、`extern` 名・ABI を含める。
  - 仕様側の `extern "C"` サンプルが Backend 未対応であることを監査ログに追記する方針を決める。

### フェーズ 3: Runtime 影響確認
- `Async` / `Await` の実装フェーズで必要となる Runtime API（Future 実体、await ブリッジ）を洗い出す。
- 現時点の暫定対応が「Backend 側で未対応診断を返す」方針であれば、Runtime のコード変更は不要と明記する。
- Runtime 側が必要になる場合は、`docs/plans/docs-examples-audit/` に別計画を作成する。
- 作業ステップ:
  - `compiler/runtime/src/runtime/async_bridge.rs` と `compiler/runtime/src/prelude/async.rs` を確認し、現在提供している型/API を一覧化する。
  - `await` に相当する橋渡しが compiler/runtime/ffi 層に存在するかを確認する（`compiler/runtime/ffi/src/registry.rs` の `BridgeIntent::Await` など）。
  - 仕様（`docs/spec/3-9-core-async-ffi-unsafe.md`）と照らし、必要な Runtime ハンドラ/Capability を整理する。
  - Backend 側で未対応診断を返す方針の場合、Runtime への追加作業が無いことを計画書内に明記する。
  - Runtime 側の追加が必要と判断した場合は、別途 `1-2-impl-gap-backend-runtime-plan-YYYYMMDD.md` を追加する。
- 確認結果:
  - `compiler/runtime/src/prelude/async.rs` は `Future<T>` / `AsyncStream<T>` の opaque 型のみで、実行器や `await` の具体 API は未定義。
  - `compiler/runtime/src/runtime/async_bridge.rs` は `ActorSystem` の最小実装があり、`Core.Async` 本体のスケジューラ/タスク実行は未実装。
  - `compiler/runtime/ffi/src/registry.rs` に `BridgeIntent::Await` があるが、`await` を直接処理するブリッジ実装は現状見当たらない。
  - 仕様（`docs/spec/3-9-core-async-ffi-unsafe.md`）は `Future` / `SchedulerHandle` / `block_on` などを定義するため、Runtime の本格対応は別フェーズで必要。
  - 現段階は Backend 側で `backend.todo.async` / `backend.todo.await` を返す方針のため、Runtime への追加変更は不要と判断する。

### フェーズ 4: 検証
- Backend MIR 取り込みのスモークテスト（JSON 変換で失敗しないこと）を追加する。
- `effect_block` / `async` / `await` / `unsafe` を含む最小 MIR を用意し、診断が期待通りに出ることを確認する。
- `externs` を含む MIR で、Backend が落ちずに診断または宣言生成できることを確認する。
- 作業ステップ:
  - 既存の Backend テスト（`compiler/backend/llvm/src/integration.rs` 内の `tests`）を確認し、MIR JSON を渡す最小ケースを追加する。
  - `effect_block` / `async` / `await` / `unsafe` を含む MIR JSON を fixtures として作成し、`MirModuleSpec::from_file` が失敗しないことを確認する。
  - `BackendDiffSnapshot::diagnostics` に追加した `backend.todo.*` が期待通りに出力されることをアサートする。
  - `externs` を含む MIR JSON を追加し、宣言生成/診断のどちらを選んだかで期待値を更新する。
  - `cargo test --manifest-path compiler/backend/llvm/Cargo.toml` を実行し、追加テストが通ることを確認する。

## 受け入れ基準
- Backend が `effect_block` / `async` / `await` / `unsafe` を含む MIR を読み込んでも失敗しない。
- 未対応構文は明示的な診断で報告される。
- `externs` / `is_async` / `is_unsafe` / `varargs` の取り込み方針が明文化されている。

## 進捗管理
- 本計画書作成日: 2025-12-30
- 進捗欄（運用用）:
  - [x] フェーズ 1 完了
  - [x] フェーズ 2 完了
  - [x] フェーズ 3 完了
  - [x] フェーズ 4 完了

## 関連リンク
- `docs/plans/docs-examples-audit/1-2-impl-gap-plan-20251230-1.md`
- `compiler/frontend/src/semantics/mir.rs`
- `compiler/backend/llvm/src/integration.rs`
- `compiler/backend/llvm/src/codegen.rs`
