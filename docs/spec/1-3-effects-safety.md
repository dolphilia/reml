# 1.3 効果と安全性（Effects & Safety）— Reml (Readable & Expressive Meta Language) 言語コア仕様

> 目的：**書きやすさ・読みやすさ・高品質エラー**を保ったまま、**実用性能**と**静的安全**を両立。
> 方針：MVPでは **HM 型推論 + 値制限 + 属性ベースの効果契約** を採用し、複雑な型レベル効果（行多相など）は**任意の拡張段**に留める。**純粋関数がデフォルト**、副作用は明示。
> 評価順序と短絡に関する基礎は [1.1 構文 C.9](1-1-syntax.md#c9-評価順序と短絡規則) を参照し、ここではその順序上で発生する効果の分類と制御を扱う。

---

## A. 効果の分類（コア + システム拡張）

Reml は関数や式の "外界への作用" を明示化するため、効果タグ集合 `Σ` を定義する。基本となるコア集合 `Σ_core` は以下のとおりで、言語仕様全体で常に解析・表示・抑制が行われる。

| 効果       | 意味                             | 例                            | 既定           |
| -------- | ------------------------------ | ---------------------------- | ------------ |
| `mut`    | 局所的な可変状態（`var` の再代入、可変コンテナの更新） | `y := y + 1`, `vec.push(x)`  | 許可           |
| `io`     | I/O・時刻・乱数など観測可能な外部作用           | `print`, `readFile`, `now()` | 許可           |
| `ffi`    | FFI 呼び出し（言語外の未検査境界）            | `extern "C" puts`            | `unsafe` 内のみ |
| `panic`  | 非全称（中断）・アサート失敗                 | `panic("…")`, `assert(x>0)`  | 許可（制限可）      |
| `unsafe` | メモリ安全や型安全の前提を破りうる操作            | 原始ポインタ操作、未定義レイアウトへのキャスト      | `unsafe` 内のみ |

システムプログラミング機能を取り込むに当たり、`Σ_core` を拡張する追加タグ `Σ_system` を正式採用する。これらは Capability Registry と連携し、危険度の高い操作を粒度細かく追跡する。

| 効果           | 意味                                             | 主なAPI例                                        | 既定 |
| ------------- | ------------------------------------------------ | ----------------------------------------------- | ---- |
| `syscall`     | OSシステムコールを直接呼び出す                   | `Core.System.raw_syscall`                       | `unsafe` 内のみ |
| `process`     | プロセス生成・制御・情報取得                     | `Core.System.Process.spawn`                     | 許可（契約で制限可） |
| `thread`      | OSスレッドの生成・同期・アフィニティ制御         | `Core.System.Process.create_thread`             | 許可（契約で制限可） |
| `memory`      | アドレス空間操作（mmap/munmap、共有メモリ等）     | `Core.Memory.mmap`, `Core.Memory.mprotect`      | `unsafe` 内のみ |
| `signal`      | OSシグナルの登録・送信・待機                     | `Core.System.Signal.send`                       | 許可（契約で制限可） |
| `hardware`    | ハードウェア固有命令・性能カウンタ               | `Core.Hardware.rdtsc`, `Core.Hardware.prefetch` | `unsafe` 内のみ |
| `native`      | ABI/メモリ境界を跨ぐネイティブ操作（intrinsic/埋め込み） | `@intrinsic`, `Core.Native.*`, `Core.Embed.*` | 許可（監査必須） |
| `realtime`    | リアルタイムスケジューラや高精度タイマ制御       | `Core.RealTime.set_scheduler_priority`          | 許可（契約で制限可） |
| `audit`       | 監査ログへの記録・参照義務                       | `Diagnostics.audit_ctx.log`, `audited_syscall`  | 許可（`@no_audit` 等で抑制可） |
| `security`    | セキュリティポリシー適用・検証操作               | `Capability.enforce_security_policy`            | 許可（契約で制限可） |

### A.1 標準ライブラリによる補助タグ {#stdlib-effect-tags}

Chapter 3 の標準ライブラリは `Σ_core` / `Σ_system` を細分化する補助タグを追加で利用する。これらはコア言語の型検査では任意ラベルとして扱われるが、`@no_alloc` や Capability Registry の検証、監査ポリシーで重要な指標となる。ライブラリが参照する主要タグは次の通り。

| タグ | 目的 | 主な参照箇所 |
| --- | --- | --- |
| `mem` | 一般的なヒープ確保やバッファ再配置。`@no_alloc` 属性と連携し、`memory`（mmap 等）とは区別する。 | 3.1, 3.2, 3.3, 3.4, 3.5 |
| `debug` | デバッグ専用 API（`expect`, ブレークポイント等）で発生し、リリースビルドでは無効化される。 | 3.1, 3.5, 3.8 |
| `trace` | 実行トレースや詳細ログ出力を伴う操作。 | 3.6, 3.9 |
| `unicode` | Unicode テーブル参照や正規化など高コストの文字処理。 | 3.3, 3.4 |
| `time` | 高精度タイマや時間計測。`io` の亜種として扱い、`@no_timer` 等のポリシーで制御する。 | 3.4 |
| `runtime` | Capability Registry やランタイム状態の照会。 | 3.8, 3.10 |
| `config` | 設定スキーマやマニフェストを変更・検証する操作。 | 3.7 |
| `migration` | データ移行計画やスキーマ変更の適用。 | 3.7 |
| `regex` | 正規表現エンジンの実行（遅延 DFA の内部状態保持を含む）。 | 3.3 |
| `privacy` | 個人情報のマスキングや匿名化ポリシーの適用。 | 3.6 |
| `cell` | `Core.Collections.Cell` など内部可変構造の操作。 | 3.2 |
| `rc` | 参照カウント（`Ref`, `Arc` 相当）の増減。 | 3.2 |
| `jit` | JIT コンパイルや動的コード生成。 | 3.9 |
| `test` | テストハーネス固有の副作用（シミュレーター・検証ログ）。 | 3.9 |

タグは `EffectTag` として単一化され、`allows_effects`／`effect_scope` の検査では `Σ` の一部として扱う。標準ライブラリ外で新しいタグを導入する場合は、本節に追記し関連仕様との整合を確認する。

> **実装メモ（Phase 3-7）**: `effect {migration}` は Rust 実装では `reml_runtime::config::migration`（`compiler/runtime/src/config/migration.rs`）に定義された `MigrationPlan` API を利用する時のみ発生させる。`Cargo.toml` では `--features experimental-migration` を明示し、ベータ段階でのみオプトインできるようにしている。【P:docs/plans/bootstrap-roadmap/3-7-core-config-data-plan.md#5.1】

`Σ = Σ_core ∪ Σ_system` が言語仕様で追跡する効果全体であり、コンパイラは任意の式・関数について潜在効果集合 `effects(expr) ⊆ Σ` を算出する。
> **実装状況（Phase 2-7）**: 効果集合は `TArrow` に統合済みであり、既定の `RunConfig.extensions["effects"].type_row_mode` は `"ty-integrated"`。互換目的で効果行を診断メタデータとして扱いたい場合は `"metadata-only"` を明示し、移行期の検証では `"dual-write"` を使用する。

> 参考：`Parser` などの**ライブラリ内“擬似効果”**（バックトラック、`cut`、`trace`）は**言語効果ではない**。外界を変えず、`Parser<T>` の**戻り値に閉じ込める**のが原則。

---

## B. デフォルトの純粋性と値制限

* **純粋（pure）デフォルト**：関数は**効果を持たない**と仮定される。
* **効果検出**：本体が `Σ` 任意のタグを生成すると、関数はその効果を**潜在効果**として保持する。`mut` や `io` に加え、システム系タグ（例：`syscall`, `memory`, `audit`）も同一の仕組みで集計される。
* **値制限（1.2 で予告）**：`let` 束縛の一般化は**効果のない確定値**に限る。効果を含む右辺は**単相**。

  ```reml
  let id = |x| x                 // 一般化: ∀a. a -> a
  let line = readLine()          // io 効果 → 単相
  ```

> **実装メモ（Phase 2-5）**: `Value_restriction.evaluate` は `Effect_analysis.collect_expr` が返すタグと Capability/Stage 解決（`Type_inference_effect.resolve_function_profile`）を統合し、`RunConfig.extensions["effects"].value_restriction_mode` を参照して一般化可否を決定する。Strict モードで効果が検出された場合、診断 `effects.contract.value_restriction` は `effect.stage.required` / `effect.stage.actual` / `value_restriction.mode` / `value_restriction.evidence[]` を `Diagnostic.extensions` と `AuditEnvelope.metadata` に同時出力する。【P:docs/plans/bootstrap-roadmap/2-5-proposals/TYPE-001-proposal.md†L52-L154】【R:docs/plans/bootstrap-roadmap/2-5-review-log.md†L22-L38】

---

## C. 効果の宣言と抑制（属性）

型システムに効果を織り込みすぎないため、\*\*属性（アトリビュート）\*\*で「効果契約」を表明・検査する。

```reml
@pure         // Σ の全効果を禁止（純粋関数）
@no_panic     // panic を禁止（→ コンパイル時チェック）
@no_alloc     // 文字列/ベクタ等のヒープ確保を禁止（MIR検査）
@async_free   // io.async を禁止（イベントループ安全性）
@no_blocking  // io.blocking を禁止（非同期環境での誤用検知）
@no_timer     // io.timer を禁止（決定性重視セクション）
@must_await   // Future/Task の未使用を警告
@must_use     // 戻り値の未使用を禁止（Result 等に推奨）
@inline       // 最適化ヒント
fn effect_contract_sample(xs: [i64]) -> i64 =
  fold(xs, 0, |acc, x| acc + x)
```

* **主な属性の役割**：
  * `@async_free` / `@no_blocking` / `@no_timer` は `io` サブ効果の安全境界を宣言し、静的検査で動作保証を得る。
  * `@must_await` は `@must_use` の非同期版として、戻り値放置によるロジック欠落を防ぐ。
  * `@pure` と `@no_panic` は引き続き上位互換であり、他属性と併用可能。
* **違反時は型エラー同等のわかりやすい診断**を出す。
* 例：

```reml
@pure
fn sum(xs: [i64]) -> i64 = {
  print("x")
  fold(xs, 0, |acc, x| acc + x)
}
// error: @pure 関数で io 効果が検出されました … at print
```

> 補足: Capability Registry で Stage 契約が有効な環境では、`@pure` 違反を検出した際に `effects.contract.stage_mismatch` も併発する場合があります。たとえば `Console.log` のように `StageRequirement::AtLeast(Beta)` を要求する効果を、`RunConfig` が `at_least:stable` のまま実行すると `effects.purity.violated` に加えて Stage 不一致診断が報告されます（[3-6-core-diagnostics-audit.md](3-6-core-diagnostics-audit.md) §1、[3-8-core-runtime-capability.md](3-8-core-runtime-capability.md) §10 を参照）。Phase 4 では `diagnostic_keys` に両コードを登録し、監査ログでも `effect.stage.required/actual` を追跡してください。

### C.1 条件付きコンパイルと効果境界 {#cfg-attribute}

* `@cfg` で無効化されたブロック/宣言は **効果解析の対象外** となり、残った分岐のみで潜在効果集合を計算する。
* 同一シンボルに複数の `@cfg` 付き定義を与える場合、アクティブになり得る定義の効果は**和集合**として扱われる。結果が `@pure` 等の契約を破る場合、`effects.cfg.contract_violation` 診断で停止する。
* `@cfg` によって無効化されたブロックでのみ `unsafe` や `ffi` を使用する構成は許可されない。コンパイラは **全条件が偽になる可能性** を考慮し、常に少なくとも 1 つは有効になることを証明できない `@cfg` の組合せには `effects.cfg.unreachable` 警告を出す。
* `@cfg` で分岐させる際は、**効果タグが整合するように設計**すること。特定プラットフォームのみ追加効果を許容する場合は、関数全体の効果集合にそのタグが含まれる前提で API を設計するか、プラットフォーム別モジュールへ分割する。
* `RunConfig.extensions["target"]` を用いたカスタムキーを `@cfg` に供給する場合は、ビルドスクリプトや CLI が同じキー名を提供しないと `target.config.unknown_key` が発生する。
* `@cfg(feature = "...")` 系の条件で参照された機能集合と、`ConfigCompatibility.feature_guard`（3-7 §1.5.5）・`RunConfigTarget.features`（3-10 §4）に登録された集合は常に一致していなければならない。構文解析フェーズは `@cfg` で使用された機能名を収集し、ターゲット解決フェーズで `feature_guard` と突き合わせる。差異がある場合、コンパイラは `Diagnostic.code = "config.feature.mismatch"` を発行し、`Diagnostic.extensions["config"].feature_guard` に `FeatureGuardDigest` を格納して未同期機能と `@cfg` 条件を明示する。この診断は Stage::Stable 以上では `Severity::Error` を推奨し、0-1 §1.2 の安全性原則に従って本番環境での挙動差を遮断する。


### C.2 監査・セキュリティ効果の扱い

* `audit` 効果は **監査ログ出力が必須**であることを示す。`audited_syscall` 等のヘルパは内部で `audit` を発生させ、`@pure` や `@no_audit`（導入検討中）と矛盾しない構造に分解する必要がある。
* `security` 効果は **ポリシー検査・適用**を行う API に付与される。`Capability.enforce_security_policy` は `security` と `audit` の両方を伴うケースが多く、組み合わせの診断メッセージを [3-6-core-diagnostics-audit.md](3-6-core-diagnostics-audit.md) で規定する。
* 監査・セキュリティ関連 API は Capability Registry に接続されるため、`@cfg` で無効化せずとも他プラットフォームでダミー実装を提供し、効果集合の整合性を維持する。

### C.3 アクティブパターンの効果扱い（ドラフト）

* **評価順序**：`match` はスクラティニーを評価した後、各アームでアクティブパターンを呼び出し、その結果に `when` ガードと `as` エイリアスを適用する。呼び出し時に発生した効果はそのアームの効果集合に加算される。
* **`@pure` 整合**：`@pure` 関数やブロック内でアクティブパターンを利用する場合、定義側が効果を持たないことが前提。`io` などを含む場合は `pattern.active.effect_violation` を発行し、`Diagnostic.extensions["effects"]` に発生効果を記録する。
* **効果伝搬**：部分パターン（`Option<T>` 戻り値）であっても `body` が持つ効果は常に伝搬する。完全パターンは常に成功するため、効果を伴う場合は後続アームの到達性に影響しうる点に注意。
* **Stage / Capability**：呼び出し先の効果タグが Capability Registry の Stage 要件を持つ場合、通常の関数呼び出しと同様に検査され、満たされないと `effects.contract.stage_mismatch` が併発する。
* **診断の棲み分け**：戻り値契約違反は型側の `pattern.active.return_contract_invalid`、効果違反は本節の `pattern.active.effect_violation` で報告し、必要に応じて `Diagnostic.secondaries` に補助診断を付ける。


---

## D. 例外・エラー処理と全称性

* **例外は言語機能として持たない**。ランタイム停止は `panic`（開発時向け）だけ。
* **失敗は型で扱う**：`Option<T>`, `Result<T,E>` を標準化。
* **伝播糖衣 `?`**（MIR で早期 return に降格）：

  ```reml
  fn readConfig(path: String) -> Result<Config, Error> = {
    let s  = readFile(path)?       // Result の Err を自動伝播
    parseConfig(s)?
  }
  ```
* **`@no_panic`** を付けた関数内では `panic`/`assert` 使用をコンパイル時に禁止。

---

## E. 可変性（mut）とデータの不変性

* **値は原則イミュータブル**。配列・レコード・文字列はデフォルト不変。
* **再代入**は `var` 束縛に限定（`mut` 効果）。

  ```reml
fn sum(xs: [i64]) -> i64 = {
  var acc = 0
  for x in xs { acc := acc + x }   // `:=` は再代入
  acc
}
```
* **可変コンテナ**は標準ライブラリで提供（例：`Vec<T>`, `Cell<T>`, `Map<K,V>`）。
  これらの更新操作は `mut` 効果。
* **性能指針**：実装は参照カウント（RC）＋**コピーオンライト**を併用し、関数型スタイルでも実用性能を確保（仕様上の約束事）。

> 解析器（Parser）を書く文脈では、**不変データ + 明示的な畳み込み**が既定の流儀。

---

## F. FFI と unsafe

* **FFI 宣言**：

  ```reml
  extern "C" fn puts(ptr: Ptr<u8>) -> i32;
  ```

  * FFI は **`ffi` 効果**を持つ。呼び出しは **`unsafe` ブロック**内でのみ許可。
* **`unsafe { … }` ブロック**：

  * 原始ポインタ `Ptr<T>` やレイアウト未定義のキャスト等、**未定義動作を起こしうる操作**を囲う。
  * コンパイラは `unsafe` 境界を**明示化**し、内部の効果を外へ**押し上げ**る（呼び出し側が `unsafe` でなくても `ffi` や `syscall` 効果が残る）。
  * FFI 先でシステムコールをラップする場合は、ラッパ API に `syscall`／`memory` 等のタグを付与しておくことで、静的契約と Capability Registry の設定を同期させる。
* **安全設計の原則**：`unsafe` を**小さく閉じ込め**、安全なラッパ API を公開。`pub` API は極力 Safe に。

### F.1 `effect {native}` の意味と境界

* `effect {native}` は **ABI/メモリ境界を跨ぐネイティブ操作**を示す監査対象の効果であり、`@intrinsic` や埋め込み API (`Core.Embed.*`) を利用する関数/モジュールに必須となる。
* `unsafe` は局所的な危険区画であり、`effect {native}` を **置き換えない**。`unsafe` を使っても `native` 効果は残留し、監査ログと Capability 検証の対象になる。
* Inline ASM / LLVM IR 直書き (`inline_asm` / `llvm_ir!`) は `effect {native}` と `unsafe` の **両方**を要求し、`@unstable("inline_asm")` / `@unstable("llvm_ir")` を伴う。関数シグネチャの `!{native}` が欠落している場合は型検査で拒否し、`native.inline_asm.missing_effect` / `native.llvm_ir.missing_effect` を報告する。
* Inline ASM は `@cfg(target_arch/target_os/target_family)` を必須とし、LLVM IR 直書きも `@cfg(target_...)` によるターゲット限定を必須とする。`@cfg` が欠落している場合は `native.inline_asm.missing_cfg` / `native.llvm_ir.missing_cfg` を報告する。
* `@intrinsic` を付与する場合は、関数の効果注釈に `native` を含める（例: `fn sqrt_f64(x: f64) -> f64 !{native}`）。`native` が欠落した場合は型検査で拒否し、`native.intrinsic.missing_effect` を報告する。
* ネイティブ依存の関数は **`@cfg` でターゲット条件を限定することを推奨**する。複数ターゲットを想定する場合は `@cfg` 付き定義とポリフィルを併記し、効果集合の差分が `@pure` 等の契約を破らないように設計する。
* 埋め込み API と Runtime Bridge の運用手順は [docs/guides/runtime/runtime-bridges.md](../guides/runtime/runtime-bridges.md) を参照し、Capability 監査キーの整合性を確認する。

```reml
@cfg(target_arch = "aarch64")
@intrinsic("llvm.ctpop.i64")
fn popcount(x: i64) -> i64 !{native} = x

@cfg(not(target_arch = "aarch64"))
fn popcount(x: i64) -> i64 = Core.Native.ctpop_fallback(x)
```

---

## G. リソース安全（スコープ終端保証）

* **`defer expr`**：ブロック脱出時に `expr` を必ず実行。
  例：ファイルやロックの確実な解放。

  ```reml
  fn write(path: String, bytes: [u8]) -> Result<(), Error> = {
    let f = File.open(path, "wb")?
    f.writeAll(bytes)?
    defer f.close()
    Ok(())
  }
  ```
* RC の**破棄順序は未規定**だが、`defer` で**局所的な確実性**を担保。

---

## H. 逐次性・並行性（将来拡張の足場）

* MVP では **明示的な並行構文は未搭載**。
* 将来の `async/await`・スレッド・チャネル導入時に備え、`Send`/`Sync` 相当の**マーカートレイト**は予約（デフォルトは `Send/Sync` 可能な純粋値）。
* 導入時は `async` を **`io` のサブ分類**として扱い、属性で抑制可能にする。

### H.1 `io` 効果の細分化ドラフト

> 目的：`async` 導入時に **既存の純粋性検査と互換性を保ちつつ**、ブロッキング I/O やタイマー操作を静的に区別できるようにする。

| サブフラグ            | 意味                                                   | 親効果 |
| ----------------- | ------------------------------------------------------ | ------ |
| `io.async`        | ノンブロッキング I/O／イベントループ協調を要求する操作               | `io`   |
| `io.blocking`     | スレッド阻塞を伴う呼び出し（同期ファイル I/O、長時間待機など）         | `io`   |
| `io.timer`        | タイマー／スケジューラ登録、ディレイ、周期起動など時間イベントの操作   | `io`   |
| `io.network.raw`  | RAW ソケットや特権ネットワーク機能（`cap_net_raw` 等を要求する操作） | `io`   |
| `io.filesystem.raw` | デバイスファイル・マウント・低レベルファイルシステム制御             | `io`   |

* **包含関係**：関数が `io.async` を持つ場合、集計上は `io` も保持する（`io.async ⊆ io`）。逆方向は成立しない。
* **推論規則**：
  * `async fn`（導入予定）はシグネチャ解釈時に暗黙で `io.async` を付与。
  * ブロッキング API を `async` 関数内で呼ぶ場合は `await blocking { ... }` のような隔離シンタックスを経由させ、`io.blocking` を局所に閉じ込める方針。
  * タイマー操作は `io.timer` を生成し、`@no_timer` で抑制できるようにする。
  * ネットワーク・ファイルシステムの特権操作は `io.network.raw`／`io.filesystem.raw` を追加付与し、Capability Registry の許可が無い場合は診断を出す。

### H.2 属性による静的契約案

| 属性名            | 効果
| ---------------- | ------------------------------------------------------------------ |
| `@async_free`    | `io.async` を禁止。主に同期専用 API やブロッキングセクションで利用。
| `@no_blocking`   | `io.blocking` を禁止。イベントループ上でブロッキング I/O を誤用した場合に即エラー。
| `@no_timer`      | `io.timer` を禁止。リアルタイム制約のある関数や determinism 重視セクション向け。
| `@must_await`    | `Future`／`Task` 戻り値の未使用を警告（`@must_use` の async 版）。

* `@pure` は従来通り `io` を含む全効果を禁止するため、`async fn` に付けた場合は「`@pure` 関数で `io.async` が検出されました」の診断を出す。
* 属性違反時のエラー文言テンプレートは `2-5-error.md` の既存方針（期待集合・ `SpanTrace`）に合わせて明示する。

### H.3 既存仕様への影響

* **互換性**：従来コードは `io` 効果のみで記録されるため、サブフラグを導入しても既存属性（`@pure`, `@no_panic` 等）の挙動は変わらない。
* **診断表示**：効果一覧表示やドキュメント生成時は `io` をトップレベルに、サブフラグを括弧内で併記（例：`io (async, timer)`）。
* **将来の API**：`RunConfig.extensions["runtime"].async`（計画中）でスケジューラ設定を行う際、`io.async`/`io.timer` の有無を前提にした検査フローを組む。
* **PoC 課題**：効果推論の一般化抑制（値制限）が `io.async` を含む場合にどう影響するかをテストで確認する。

---

## I. 効果宣言とハンドラ（実験段階）

> 本節は `-Zalgebraic-effects` フラグが有効な場合に適用される。安定化時に文言・数値の確定を行う。
> 他章（1-1, 2-5, 2-6 など）で引用するステージ遷移と Capability 整合は本節を基準としており、更新時はここを先に改定する。

### I.1 効果宣言と Capability 連携

* `effect <Name> : <tag>` は 1.3 節 A のタグ集合 `Σ` に属する `tag` を基底効果として宣言し、`EffectDecl { tag, operations, stage }` を生成する。
* 宣言時に `stage ∈ {Experimental, Beta, Stable}` を付与し、Capability Registry の `register` と整合させる（3.8 §1）。`stage = Experimental` の効果は `@requires_capability(stage="experimental")` を持つ API のみが利用可。
* 各 `operation` にはシグネチャ `Args -> Ret` を付与し、暗黙に `effect` 本体のタグを潜在効果として報告する。
* Stage 検査は要求 Capability の全件を対象とし、診断および監査ログに `required_capabilities` / `actual_capabilities` の配列を出力して証跡を残す。[^effect003-phase25-capability-array]


[^effect003-phase25-capability-array]:
    Phase 2-5 EFFECT-003 複数 Capability 解析計画 Step4（2025-12-06 完了）で `Diagnostic.extensions["effects"]` と `AuditEnvelope.metadata` を配列対応へ拡張した。計画書: `docs/plans/bootstrap-roadmap/2-5-proposals/EFFECT-003-proposal.md`、レビュー記録: `docs/plans/bootstrap-roadmap/2-5-review-log.md`「EFFECT-003 Week33 Day2」参照。

[^effects-sigma-poc-phase25]:
    Phase 2-5 `EFFECT-002 Step4`（2026-04-18 完了）で `extensions.effects.sigma.*` と `audit.metadata["effect.syntax.constructs.*"]` の出力形式、および CI 指標 `syntax.effect_construct_acceptance` / `effects.syntax_poison_rate` の算出基準を PoC として確定した。引き継ぎ条件は `docs/notes/effects/effect-system-tracking.md` と `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に記録されている。

### I.2 効果発生と潜在効果集合

* `perform Effect.operation(args)`（または `do` シュガー）を含む式は、既存の潜在効果集合 `effects(expr)` に `Effect.tag` を追加する。
* 効果宣言で列挙された操作は **純粋な戻り値**を期待する。`operation` 内で追加の効果が発生した場合はその効果タグも集合に含める。
* ハンドラ適用前の効果集合を `Σ_before`、ハンドラが捕捉対象として列挙したタグ集合を `Σ_handler` とすると、ハンドラ適用後の潜在効果集合は次のように計算する。

  ```
  Σ_after = (Σ_before - Σ_handler) ∪ Σ_residual
  ```

 ここで `Σ_residual` はハンドラ本体（`return` 節を含む）が発生させた効果タグ集合。`Σ_after = ∅` の場合、ハンドラ式は純粋値として扱える。
  `Σ_before` と `Σ_after` の記録・検証は Phase 2-5 時点では PoC として運用されており、診断出力 `extensions.effects.sigma.*` および CI 指標 `syntax.effect_construct_acceptance` / `effects.syntax_poison_rate` の算出基準は [`EFFECT-002` Step4](../plans/bootstrap-roadmap/2-5-proposals/EFFECT-002-proposal.md#4-診断・ci-計測整備week33-day1-2) と `docs/notes/effects/effect-system-tracking.md` に従う。[^effects-sigma-poc-phase25]

### I.3 `@handles` と契約検査

* `@handles(Console, …)` は関数・ハンドラに付与でき、捕捉可能な効果タグ集合 `Σ_handles` を宣言する。
* 関数本体の潜在効果集合を `Σ_body`、ハンドラ適用後の残余効果を `Σ_after` とすると、`Σ_after ⊆ allows_effects` が成立しなければ診断 `effects.contract.mismatch` を発生させる。
* `@pure` は従来通り `Σ_body = ∅` を要求するが、ハンドラで完全捕捉される場合は `Σ_after = ∅` と判定できれば許容する。

### I.4 Stage と Capability の整合

* `stage = Experimental` の効果を捕捉・発生させる、または `@reentrant` を付与するハンドラには `@requires_capability(stage="experimental")` を併用しなければならない。Capability Registry は登録時に stage を検証し、未承認の場合は `CapabilityError::SecurityViolation` を返す。
* `stage` を `Beta`/`Stable` に昇格させる際は、残余効果計算と `@dsl_export` / マニフェスト（3.7 節）の期待集合が一致することを整合チェックで確認する。

### I.5 効果行の整列とハンドラ合成順序 {#effect-line-ordering}

> 0-1-project-purpose.md §2.3 の「段階的な抽象化拡張」を満たすため、効果の列挙とハンドラ構成を読者が一目で追えるよう規約を定義する。

* **効果行の整列基準**：`fn ... -> R with ...`、`handler ... for ...`、およびドキュメントコメント内の効果一覧は、次の優先順位で並べる。
  1. ビルトイン効果タグ（`mut` → `io` → `panic` → `unsafe` → `ffi` → `runtime`）。
  2. ビルトインタグに属するサブタグ（例：`io (async, timer)`）は括弧内をアルファベット順に並べる。
  3. ユーザー定義効果は基底タグ名を辞書順に整列し、同一タグに型パラメータがある場合は型名を再帰的に辞書順で比較する。
  4. Capability に結び付いたタグ（3.8 節）を列挙する際は、要求される `stage` が高いもの（`Experimental` → `Beta` → `Stable` の順）を先に書き、ポリシー差分を明示する。
  順序は実行時の意味に影響しないが、仕様・ドキュメント・診断出力で同一順序を維持することで差分レビューと自動生成物の整合性を確保する。

* **`perform` の評価順序**：`perform Effect.operation(arg1, arg2, …)` は [1-1-syntax.md §C.9](1-1-syntax.md#c9-評価順序と短絡規則) に従い、
  1. `Effect` と `operation` の解決を静的に行う。
  2. 引数を左から順に評価し、途中で例外が発生した場合は以降の引数評価・効果発火を行わない。
  3. 潜在効果集合 `Σ_before` に `Effect.tag` を追加したのち、スタック上のハンドラ探索に移る。

* **ハンドラ探索と合成順序**：
  - `handle h1 do handle h2 do expr` のようにネストした場合、内側から外側へ順に探索する。`h2` が対象タグを捕捉すれば `resume` 先は `h2` より外側のフレームに転送され、`h1` は残余効果にのみ作用する。
  - `resume` を複数回呼び出した場合、各 `resume` 呼び出しは呼び出し順に同じ continuation を再開し、継続から戻った効果を再び `Σ_residual` へ合成する。`resume` 前後で `stage` や Capability 要件が変化した場合は [3-8-core-runtime-capability.md §1.2](3-8-core-runtime-capability.md#capability-stage-contract) の検査を再評価する。

* **ハンドラの入れ替え規則**：
  - 動的順序が意味に影響するため、ハンドラの再配置は `Σ_after` が変わらないことを条件にのみ認める。`State` → `Except` → `Choose` を `Choose` → `State` → `Except` に並び替えると `State` の操作が複数回実行され得るため、変更理由をコメントで明示し、テストで挙動を検証する。
  - `@handles` 属性を持つハンドラを別段に移動する場合は、捕捉順序の差異を `effects.contract.reordered` 診断として通知する（詳細は 3-6-core-diagnostics-audit.md §2.4 を参照）。

* **診断とドキュメント生成**：効果行の整列基準に従っていない場合、ドキュメント生成と LSP 診断は整列済みのリストを提示し、開発者に差分を提示する。これにより、DSL エクスポートや Capability Registry（3.8 節）と同じ序列で比較できる。

---

## J. 効果と型推論の接続（実装規約）

* **効果は“型には織り込まない”**（MVP）。

  * ただしコンパイラ内部では各関数に**潜在効果集合 `{mut, io, …}`**を持たせ、

    * 値制限の判定
    * `@pure`/`@no_panic` 等の**契約検査**
    * ドキュメント・警告の出力
      に用いる。
* **双方向型付け**（1.2）：注釈がある箇所では**効果も検査**を厳密化（例：`@pure` 関数内で `print` を発見→即時エラー）。
* **将来の拡張**：必要になれば**行多相ベースの効果型**を**オプトイン**で提供（Koka 風）。MVP のコードは**そのまま**動く方針。

### I.1 DSLエクスポートの効果境界 {#dsl-export-effects}

`@dsl_export` 付き宣言では、型検査で得られた潜在効果集合 `effects(decl)` と属性の `allows_effects` パラメータを比較し、次の契約を強制する。

- `effects(decl) ⊆ allows_effects`。`allows_effects` が省略された場合は空集合とみなすため、単一でも効果が検出されるとエラー `E1303` を報告する。
- `allows_effects` に `io` や `ffi` など危険度の高いタグを含める場合は `@dsl_export` と併用で `@requires_capability` 属性（3.8 §1）を要求し、Capability Registry が相互参照できるようにする。
- `conductor` から生成される `ConductorSpec` は内部で使用する DSL エクスポートの `allows_effects` を合成し、`@dsl_export(allows_effects=...)` に明示された集合と一致するかチェックする。一致しない場合は `E1304`（効果合成の不一致）を報告する。

また、`@dsl_export(requires=[...])` で列挙された DSL カテゴリは `ConductorSpec` のチャネル定義と照合される。存在しないカテゴリや、`@dsl_export` が純粋値として宣言された DSL を副作用付きチャネルへ接続した場合は `diagnostic("dsl.conductor.incompatible")` を出す。

`reml.toml` の `dsl.<name>.expect_effects` フィールド（3.7 §2.1）と比較することで、ビルド CLI はターゲット環境固有のポリシー（例: CI では `io` を禁止）を適用できる。型検査はこれらの差異を `Core.Diagnostics` に報告し、CLI 側で失敗しないよう JSON 形式で差分を出力する。

---

## J. 具体例

### J.1 パーサは純粋

```reml
@pure
fn intLit() -> Parser<i64> =
  digits().map(parseInt)             // 文字列→数値, 外界に作用しない
```

### J.2 効果を持つ境界を薄くする

```reml
@pure
fn parseFile(path: String) -> Result<AST, Error> = {
  // error: io 効果の `readFile` は @pure で禁止
  let s = readFile(path)?
  parseModule(s)
}

fn parseFile(path: String) -> Result<AST, Error> = {
  // OK: 効果は境界関数に出す
  let s = readFile(path)?            // io
  parseModule(s)                     // pure
}
```

### J.3 unsafe と FFI

```reml
extern "C" fn qsort(ptr: Ptr<u8>, len: usize, elem: usize, cmp: Ptr<void>) -> void;

fn sortBytes(xs: Vec<u8>) -> Vec<u8> = {
  unsafe {
    qsort(xs.ptr(), xs.len(), 1, cmp_ptr)  // ffi + unsafe
  }
  xs
}
```

### J.4 `@no_panic` と `?`

```reml
@no_panic
fn strictlyPositive(n: i64) -> Result<i64, Error> =
  if n <= 0 then Err(Error::Invalid) else Ok(n)

fn total(xs: [i64]) -> Result<i64, Error> = {
  let ys = (xs |> map(strictlyPositive) |> sequence)?
  sumOk(ys)
}
```
## K. 拡張効果タグの扱い

Reml コアで追跡する効果は `mut`・`io`・`ffi`・`panic`・`unsafe` の 5 種類に限定します。プロジェクト固有のガバナンス要件（設定変更の監査やクラウド API の統制など）が必要な場合は、標準ライブラリやプラグインが追加のタグ／属性を提供する想定です。コアコンパイラはそれらを知らなくても動作し、拡張側では `@requires(...)` などの属性を通じて独自検査を実装できます。

---

## L. 仕様チェックリスト（実装者向け）

* [ ] 各式ノードに**効果ビット集合**を付与・合成（AST→TAST→MIR）。
* [ ] `let` 一般化は **効果なし**かつ**純度がわかる式**に限定。
* [ ] `@pure/@no_panic/@no_alloc` 検査は**コンパイル時強制**。
* [ ] `unsafe` 境界を CFG に刻み、`ffi`/原始操作は **境界内のみ許可**。
* [ ] `defer` はスコープ退出（正常/早期 `return`/`?`/`panic`）の**いずれでも発火**。
* [ ] エラーメッセージは**効果名 + 位置 + 修正提案**を必ず含む。
* [ ] `Parser` 標準APIは**外界作用を持たない**ことを CI で回帰検査。

<a id="unsafe-ptr-spec"></a>
## M. unsafe ポインタ仕様

> 目的：`unsafe` 境界内での原始ポインタ操作を体系化し、FFI・GC・高性能バッファ処理を安全ラッパと共存させる。

### M.1 原始ポインタ型の分類

Reml は `Core.Unsafe.Ptr` モジュールで `Ptr<T>` / `MutPtr<T>` / `NonNullPtr<T>` / `Ptr<void>` / `FnPtr` を提供する（詳細は [Core.Unsafe.Ptr API 草案](../guides/ffi/core-unsafe-ptr-api-draft.md)）。
それぞれに `unsafe` 効果が付随し、`MutPtr<T>` と `FnPtr` は `ffi` 効果とも組み合わせて扱う。
`NonNullPtr<T>` は NULL 不許可を静的に表現し、`Span<T>` など境界チェック付きビューの基礎となる。

### M.2 生成と取得

`addr_of` / `addr_of_mut` は評価順序を固定したまま参照のアドレスを取得し、`Buffer.asPtr` など安全ラッパからのダウングレードもここに集約する。
外部ポインタは `require_non_null` を通じて `Option<NonNullPtr<T>>` に昇格させ、NULL を検出すれば `NullError` として `Result` に反映する。
FFI 経由で取得した `Ptr<void>` は型情報を欠くため、以降のキャストは必ず `unsafe` ブロック内で行う（[guides/reml-ffi-handbook.md](../guides/ffi/reml-ffi-handbook.md) 参照）。

### M.3 読み書きと境界検査

`read`/`write`/`copy_to` などの操作は整列や領域サイズを満たさないと未定義動作になる。
境界保証が必要な場合は `Span<T>` や `Slice<T>` を経由し、ここから `Ptr<T>` へ降格する位置をコードレビューで明示する。
`copy_nonoverlapping` と `copy_to` の区別により、`memcpy`/`memmove` を効率的に選択できる。

### M.4 アドレス計算とキャスト

`add`/`offset`/`byte_offset` は同一アロケーション内に留まる前提でのみ定義される。
整数キャスト（`to_int`/`from_int`）や型変更（`cast`/`cast_mut`）は `unsafe` の明示と共に、整列要件を仕様書 ([LLVM連携ノート](../guides/compiler/llvm-integration-notes.md) の ABI 節) に従わせる。
ポインタ比較は `==`/`!=` のみに限定し、順序比較は未規定とする。

### M.5 所有権とリソース管理

RC で管理する値を指すポインタは `inc_ref`/`dec_ref` を `unsafe` ブロック内で対にし、`defer` による解放を推奨する。
スレッド境界では `Send`/`Sync` 相当のマーカートレイトを付与しない限り `Ptr<T>` の共有を禁止し、必要な場合は拡張で定義される効果契約（例: `@requires(runtime, unsafe)`）を併記して境界を明示する。
所有権の移譲や回収は `Result` で伝播し、必要なら監査拡張が提供するロギング API と連携させる。

### M.6 適用シナリオ別ガイド

- **FFI**: `extern "C"` 呼び出し時に `Ptr<u8>` や `FnPtr` を利用し、`ffi` 効果タグと必要に応じて監査拡張の記録 API を組み合わせる。
- **GPU/IO**: `Ptr<void>` をデバイスハンドルとして扱う場合は、拡張が提供する `runtime`/`gpu` 系の効果タグを用いて境界を明示し、`defer` でリソース解放を保証する。
- **GC ルート**: `NonNullPtr<Object>` を `runtime::register_root` に渡し、`write_barrier` と連携して世代間更新を安全に処理する（[3-8-core-runtime-capability.md](3-8-core-runtime-capability.md) 参照）。


## N. まとめ

* **純粋デフォルト**・**効果は属性で宣言/検査**という軽量設計で、MVP段階でも**読みやすさと実用**を両立。
* **例外なし、Result/Option（`?`）**で失敗を型に昇格。
* **不変データ＋局所的 mut**、**RC+defer**でリソース安全を担保。
* **unsafe/FFI は小さく閉じ込める**ルールを強制。
* 将来は**行多相の効果型**へ拡張可能だが、**現仕様のコードがそのまま有効**であることを保証する。

---

## 関連仕様

* [1.1 構文](1-1-syntax.md) - unsafeブロックとdefer文の構文定義
* [1.2 型と推論](1-2-types-Inference.md) - 値制限と効果システムの連携
* [1.4 文字モデル](1-4-test-unicode-model.md) - 文字列の安全性保証
* [2.1 パーサ型](2-1-parser-type.md) - パーサの純粋性と効果分離
* [LLVM連携ノート](../guides/compiler/llvm-integration-notes.md) - FFI・unsafe・メモリ管理の実装方針
