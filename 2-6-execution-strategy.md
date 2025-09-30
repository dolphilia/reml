# 2.6 実行戦略（Execution Strategy）

> 目的：**最小コアで高い実用性能**（線形時間・ゼロコピー・良質な診断）を実現し、**左再帰・ストリーム**・**インクリメンタル**も無理なく扱える実行系を定義する。
> 前提：2.1 の `State/Reply{consumed, committed}`、2.2 の合成規則、2.3/1.4 の Unicode/入力モデルと整合。

---

## A. ランナーとモード

### A-1. ランナー API（外部インターフェイス）

```reml
fn run<T>(p: Parser<T>, src: String, cfg: RunConfig = {}) -> ParseResult<T>
fn run_partial<T>(p: Parser<T>, src: String, cfg: RunConfig = {}) -> ParseResultWithRest<T>
```

* `ParseResult` は 2.1 節と同様、成功時の値と診断をまとめて返す。
* `ParseResultWithRest` は未消費入力を併せて返し、REPL やインクリメンタル更新に備える。
* ストリーミング処理や継続再開は拡張モジュール `Core.Parse.Streaming` に委ねる（§F 参照）。

### A-2. 実行モードと拡張の棲み分け

コア仕様の `RunConfig` はバッチ解析を前提とし、Packrat や左再帰の切替、追跡の有無など最小限の選択肢のみを持ちます。ストリーミング処理やハイブリッド実行、差分再利用といった高度な戦略は拡張モジュールで定義され、コアからは opt-in で利用します。

---

## B. コアエンジン（ステップ実行・安全弁）

### B-1. トランポリン & 末尾最適化

* すべてのコンビネータは **ループ化**／**トランポリン**で実装し、**スタック深度は O(1)**。
* 再帰下降は `call(rule_id)` → `jump`（継続渡し）で表現。

### B-2. RunConfig のコアスイッチ

`RunConfig` はバッチ解析に必要な最小限のスイッチだけを提供し、燃料制御や追加の安全弁は拡張モジュール側で定義する。

```reml
type RunConfig = {
  require_eof: Bool = false,
  packrat: Bool = false,
  left_recursion: "off" | "on" | "auto" = "auto",
  trace: Bool = false,
  merge_warnings: Bool = true,
  legacy_result: Bool = false,
  extensions: RunConfigExtensions = {}
}

type RunConfigExtensions = Map<Str, Any>
```

* `require_eof` で余剰入力を拒否するかどうかを切り替える。
* `packrat` と `left_recursion` はメモ化と seed-growing 左再帰を制御する主要スイッチ。
* `trace` は SpanTrace を収集し、`merge_warnings` は回復警告をまとめてノイズを抑制する。
* `legacy_result` は旧来の戻り値形式を要求するツールチェーンとの互換用スイッチ。
* 追加の燃料制御や GC 連携、ストリーミング用バッファ、LSP 設定などは `extensions` に格納されるモジュール固有設定として扱い、必要なときだけ読み込む（推奨ネームスペースは [2-1](2-1-parser-type.md) を参照）。

#### B-2-1. ターゲット情報拡張 `extensions["target"]`

* `@cfg` やバックエンド切替に必要なターゲット情報は `RunConfig.extensions["target"]` に格納する。コア仕様では `RunConfigTarget` を次のように定義する。

```reml
type RunConfigTarget = {
  os: Str,
  family: Str,
  arch: Str,
  abi: Option<Str>,            // gnu / msvc / musl 等
  vendor: Option<Str>,
  env: Option<Str>,            // 追加ツールチェーン識別子
  profile_id: Option<Str>,
  triple: Option<Str>,
  features: Set<Str>,
  capabilities: Set<Str>,
  stdlib_version: Option<SemVer>,
  runtime_revision: Option<Str>,
  diagnostics: Bool,
  extra: Map<Str, Str>
}

RunConfig.extensions["target"] = RunConfigTarget
```

この定義は 1-2 §C.7 の型検査仕様と同一であり、フィールドの増減は両章で同時に更新する。

* CLI やビルドツールは `RunConfigTarget` を構築して `RunConfig` へ注入し、パーサーはパース段階で `@cfg` に渡す。未設定の場合は `profile_id` を含む任意のキー参照で `target.profile.missing` を報告し、ビルドを停止する。
* `capabilities` セットは `Core.Runtime` の Capability Registry で宣言された識別子と同期する。存在しない Capability を参照した場合は `target.capability.unknown` を生成し、性能 1.1 を損なわずに誤設定を早期検出する。
* `extra` 以下のキーは `@cfg` から参照可能だが、辞書登録時に `RunConfig::register_target_key(name, allowed_values)` で値テーブルを宣言し、誤字を防ぐ。
* 実行時にターゲットを切り替える場合は `RunConfigTarget.features` または `capabilities` を差し替え、`platform_info()`（[3-8](3-8-core-runtime-capability.md)）と同期させる。
* `diagnostics = true` を設定すると `@cfg` 評価ログを `Diagnostic.extensions["cfg"]` に出力し、CI/IDE でターゲット分岐の可視化を行える。

##### B-2-1-a. コンパイラメタデータの生成

```reml
type RunArtifactMetadata = {
  target: RunConfigTarget,
  llvm_triple: Str,
  data_layout: Str,
  runtime_revision: Str,
  stdlib_version: SemVer,
  emitted_capabilities: Set<Str>,
  timestamp: DateTime,
  hash: Str
}
```

* コンパイラはクロスビルド時に `RunArtifactMetadata` を生成し、バイナリや `.remlpkg` に添付する。`runtime_revision` や `stdlib_version` が CLI/レジストリから提供された値と一致しない場合、ビルド中に `target.abi.mismatch` を発行して停止する。
* `llvm_triple` と `data_layout` は LLVM バックエンドへ渡す最終値であり、`RunConfigTarget.triple` が存在する場合は一致していなければならない。不一致時は `target.config.unknown_value`（詳細値付き）を生成する。
* CLI は `RunArtifactMetadata.hash` を利用して標準ライブラリのキャッシュを検証し、性能 1.1 の線形処理を保つ。ハッシュ計算は入力サイズに対して線形成分のみに限定する。

#### B-2-2. プラットフォーム適応設定サンプル

```reml
fn specialise_config(profile: BuildProfile) -> RunConfig = {
  let info = platform_info();
  let mut cfg = RunConfig { extensions = { "target": default_target(profile) } };
  if has_capability(RuntimeCapability::SIMD) {
    cfg.packrat = true;
  }
  if platform_features().contains("io.blocking.strict") {
    cfg.extensions["target"].extra.insert("io.blocking", "strict");
    cfg.merge_warnings = false; // ブロッキング時の警告を逐次報告
  }
  if info.family == TargetFamily::Wasm {
    cfg.left_recursion = "off";
  }
  cfg
}
```

* `platform_info()` と `platform_features()` を併用し、ランタイム最適化（Packrat 有効化/無効化、左再帰サポート切替など）をプラットフォームごとに調整する。
* `default_target(profile)` は `Core.Env.infer_target_from_env()` や CLI パラメータから構築した基準値であり、ここで追加した `extra` キーは `@cfg` による宣言切替と診断に利用できる。
* WASM や一時的な実験ターゲットでは左再帰やブロッキング I/O を制限し、`guides/portability.md` のチェックリストに従って差異を記録する。

* **空成功の繰返し**検出は必須（2.2 に準拠）。

### B-3. 効果ハンドラと継続管理（実験段階）

> `-Zalgebraic-effects` フラグが有効な場合の挙動。安定化時に最終仕様へ更新する。
> ステージ遷移と Capability の要件は [1.3 §I.4](1-3-effects-safety.md#i4-stage-と-capability-の整合) を参照する。

* **ワンショット継続**: ハンドラ適用時は継続フレームをヒープに退避し、`resume` 呼び出し後に即破棄する。フレームは `EffectFrame { handler_id, resume_ptr, stage }` で管理し、`stage` が `Experimental` の場合は Capability Registry を介した opt-in を要求する。
* **マルチショット禁止（既定）**: `resume` を複数回呼び出した場合は `effects.handler.invalid_resume` を生成し、`stage = Experimental` の環境であっても `@reentrant` 属性と Capability 許可がなければコンパイルエラーとする。
* **RunConfig 拡張**: `RunConfig.extensions["effects"].max_handler_depth` を導入し、深いハンドラネストによるスタック肥大を防ぐ。未設定時は `32` を推奨値とし、超過時は `AsyncErrorKind::RuntimeUnavailable` 相当の実行時エラーを発生させる。
* **残余効果検査**: ハンドラ適用後の残余効果 `Σ_after` は 1-3 §I に従って計算され、`Diagnostic.extensions["effects"].residual` に保存する。`Σ_after = ∅` であればハンドラ式は純粋扱いとなり、`@pure` との整合が確認される。


### B-3. 期待集合の早期確定

* `cut_here()` を通過したら **親の期待集合を破棄**し、その地点からの期待を再構築（2.5 と同一）。

---

## C. メモ化（Packrat）と左再帰

### C-1. メモ化キーと値

```reml
type ParserId = u32
type MemoKey = (ParserId, byte_off)
type MemoVal<T> = Reply<T>  // Ok/Err 丸ごと
```

* 値は**スパン・consumed/committed**を含む `Reply` を**丸ごと**キャッシュ。
* **命中時は入力を進めず**、`Reply` をそのまま返す。

### C-2. メモの窓（スライディング）

* 実装は **前方最遠コミット水位**（`commit_watermark`）を基準に古いエントリを段階的に破棄し、メモリ使用量を制御する。
* `commit_watermark` は **最後に `committed=true` で確定した `byte_off` の最大値**。→ `cut` によって**安全に掃除**できる。

### C-3. 左再帰（seed-growing）

* `left_recursion = on|auto` かつ Packrat 有効のとき、**Warth et al.** の **種成長**を実装：

  1. `(A, pos)` に「**評価中**」フラグと \*\*種（失敗）\*\*を入れる。
  2. 右辺を評価し、**より遠く進めた**結果が得られたら **更新**して再試行。
  3. 進捗がなくなるまで繰返し、最終結果を確定。
* `auto` は **`precedence` 使用時は無効**（必要ない）／**直接左再帰を検知**したルールのみ有効化。
* **非終端 A の複数定義**（演算子階層など）には `precedence` を推奨（高速）。

---

## D. 選択的メモ化（Hybrid）

* **ホットルール自動検出**：短時間に同位置で頻出する `ParserId` を **ホット**とみなし、それのみメモ化。
* **閾値**と**上限**は実装側の LRU などポリシーに委ねる。
* PEG 的線形性を緩く保ちつつ、**メモリ消費を数十 MB に抑制**できる。

---

## E. エラー・トレース・計測

### E-1. 最遠エラー集約

* 2.5 の **farthest-first** を実装：`byte_end` → `committed` → `expected ∪`。
* `then` で失敗したら **直前の `rule/label` を `context` に積む**。

### E-2. トレース・プロファイル（オプション）

```reml
type TraceEvent =
  | Enter(ParserId, Input)
  | ExitOk(ParserId, Span)
  | ExitErr(ParserId, ParseError)

fn with_trace<T>(p: Parser<T>, on_event: TraceEvent -> ()) -> Parser<T>
```

* `cfg.trace=true` で **SpanTrace** と **イベントフック**を活性化。
* 最小限のオーバーヘッド（ビルド時に NOP へ落ちる）。

### E-3. カバレッジ

* どの分岐が一度も走っていないかを `ParserId` 単位で可視化（テスト補助）。

---

## F. Regex 実行連携 {#regex-run-policy}

`feature {regex}` を有効化した場合、`RunConfig.extensions["regex"]` には次の構造体を渡す。

```reml
type RegexRunConfig = {
  engine: RegexEngineMode = "auto",
  memo: RegexMemoPolicy = "auto",
  unicode_profile: Option<UnicodeClassProfile> = None,
  max_backtrack_depth: usize = 256,
  timeout: Option<Duration> = None,
}

enum RegexEngineMode = "nfa" | "dfa" | "jit" | "auto"
enum RegexMemoPolicy = "auto" | "force" | "off"
```

* **エンジン選択**: `engine = "auto"` は [3.8 §1.4](3-8-core-runtime-capability.md#regex-capability) の Capability を参照し、`RegexJit` が利用可能で `PatternFlags::Jit` が指定されたときのみ JIT を起動する。Capability が無い場合は `nfa` を選ぶ。
* **メモ化ポリシー**: `memo = "auto"` は `RunConfig.packrat` と連動し、`regex_capture`／`regex_token` が 3 段以上ネストした場合に限り Packrat を強制する。`force` は常時 Packrat、`off` は必ず無効化する。
* **Unicode 整合**: `unicode_profile` を指定しない場合でも `RegexHandle` 側のプロフィールと `RunConfig.extensions["target"].features` を突き合わせ、差異があれば `regex.unicode.mismatch` を発行する（0-1 §3.1 の国際化要件）。
* **安全弁**: `max_backtrack_depth` を超えた場合は `regex.backtrack.limit` を診断し、実行を停止する。`timeout` を設定すると `Duration` 経過後に `regex.timeout` を返す。いずれも `DiagnosticDomain::Regex` に分類される。
* **性能配慮**: `memo="auto"` と `timeout` の組み合わせで、0-1 §1.1 に掲げる線形時間目標を維持する。JIT が無効な環境でも NFA 実装で 50MB 入力を O(n) で処理できることを確認する。

---

## G. ストリーミング＆インクリメンタル

コア仕様ではストリーミングおよび差分適用の詳細を定義しません。これらの機能は `Core.Parse.Streaming` 拡張に委ねられ、`Parser` の意味論と互換な `run_stream`/`resume` API、継続メタデータ、バックプレッシャ制御を個別に定義します。詳細は [Core.Parse.Streaming 拡張ガイド](guides/core-parse-streaming.md) を参照してください。

ここでは以下の契約のみを前提とします。

* ストリーミングランナーは `run`/`run_partial` と同じ `ParseResult`/`Diagnostic` 形式を再利用する。
* インクリメンタル再パースは `commit_watermark` と `ParserId` 依存グラフを利用して影響範囲を絞り込む。
* Feeder や DemandHint などの詳細型は拡張側で定義され、コアからは不透明。

---

## H. 新ターゲット戦略（ドラフト）

### H.1 WASM / WASI

```reml
fn wasm_run<T>(p: Parser<T>, bytes: Bytes, cfg: RunConfig) -> Result<T, Diagnostic> = {
  let mut wasm_cfg = cfg;
  wasm_cfg.left_recursion = "off";
  wasm_cfg.packrat = false; // メモリ制約に合わせる
  wasm_cfg.extensions["target"].extra.insert("wasi", "preview2");
  run(p, bytes, wasm_cfg)
}
```

* `target_family = "wasm"` の場合、Packrat/左再帰を既定で無効化し、`guides/runtime-bridges.md` の WASI サンドボックス指針に従って I/O を限定する。
* エラー診断は `Diagnostic.extensions["cfg"].evaluated` に `wasi` プロファイルを記録し、ホストとの差異を監査可能にする。

### H.2 ARM64 / 組み込み

```reml
fn specialise_for_arm64(cfg: RunConfig) -> RunConfig = {
  let mut cfg = cfg;
  cfg.extensions["target"].extra.insert("cache_policy", "conservative");
  cfg.merge_warnings = false; // フラッシュ遅延を即時通知
  cfg
}
```

* ARM64 ではキャッシュ戦略やメモリ消費を抑えるため `RunConfig.extensions["target"].extra` に制約を記録し、`@cfg` 経由でヒープ確保・GC の挙動を切り替える。

### H.3 クラウドネイティブ / コンテナ

```reml
fn container_profile(profile: &str) -> RunConfig = match profile {
  | "serverless" -> RunConfig { packrat = false, merge_warnings = true, ..default }
  | "latency"   -> RunConfig { packrat = true, left_recursion = "auto", ..default }
  | _            -> default,
}
```

* コンテナ上での実行を想定し、プロファイルごとに Packrat/左再帰や診断の集約ポリシーを切り替える。`guides/portability.md` のフェーズ指針に沿って追加ターゲットを段階的に導入する。

---

## I. 並列性・再入性

* パーサ値は **不変**であり、`State` は実行ごとに分離される。共有を行う場合は拡張側でスレッド安全性を保証する。
* 同一入力内での並列実行は推奨しないが、モジュール単位の分割統治は上位レイヤで並列化できる。
* GC やランタイム統合に関する詳細なコールバックは `guides/runtime-bridges.md` に委ね、コア仕様では純粋性と境界の明示のみを要求する。


## J. パフォーマンス方針（実装規約）

1. **ホット経路を手でループ化**

   * `many`, `takeWhile`, `string`, `symbol` は **分配関数呼び出しを避け**、ASCII 最適化。
2. **Arena（解放一括）**

   * 一時ノードは **ランナー専用アリーナ**へ確保→終了時に一括解放。
3. **ゼロコピー**

   * `Str` は親 `String` を参照共有（SSO/RC）。
4. **境界キャッシュ**

   * コードポイント/グラフェム境界は **遅延構築**かつ **ビュー共有**。
5. **メモ掃除**

   * `cut` と `commit_watermark` を利用して **前方を積極的に破棄**。
6. **オペランド後 Cut**

   * `precedence` は **演算子読取時に cut** を自動挿入（右項欠落の診断改善＋バックトラック削減）。

---

## K. 互換と移行

* **`precedence` を使う限り左再帰は不要**（デフォルト `left_recursion=auto` がそれを尊重）。
* 既存 PEG ルールを移植する場合は `packrat=true` を推奨、メモ窓でメモリをコントロール。
* 大規模入力・REPL・LSP 連携が必要な場合は `Core.Parse.Streaming` 拡張を利用する。

---

## L. 仕様チェックポイント

- `run` / `run_partial` が `ParseResult` / `ParseResultWithRest` を返し、診断・未消費入力を一貫して扱うことを確認する。
- `RunConfig` のコアスイッチ（require_eof / packrat / left_recursion / trace / merge_warnings）を実装し、既定値を明記する。
- Packrat メモ化でキー `(ParserId, byte_off)` と `commit_watermark` に基づく掃除、実装依存の窓上限を備える。
- 左再帰は seed-growing で解決し、`left_recursion=auto` と `precedence` の協調動作を検証する。
- 最遠エラー統合と `cut` による期待リセットが期待どおりに動作する。
- `trace` と `merge_warnings` の挙動をテストし、診断ノイズを制御する。
- インクリメンタル処理やストリーミングを提供する場合は `Core.Parse.Streaming` 拡張の契約に従うことを文書化する。

---

### まとめ

* 既定は **前進解析 + cut/label による制御可能なバックトラック**で、Packrat と左再帰サポートをスイッチ可能にする。
* `RunConfig` は最小限のスイッチに留め、燃料制御・ストリーミング・GC 連携などは拡張モジュールで opt-in する。
* 診断品質（最遠エラー、SpanTrace、警告集約）とゼロコピー入力を中核に据え、DSL から大規模入力まで一貫した挙動を提供する。
---

## M. Conductor 統合ポイント

- Conductor 構文（[1-1 B.8](1-1-syntax.md)）で宣言された `ExecutionPlan` は本章のランナー設定と同一概念を共有し、`strategy`/`backpressure`/`error`/`scheduling` を `RunConfig.extensions` にエンコードして Core.Async へ伝達する。
- `ExecutionPlan.strategy` が `adaptive_parallel` の場合、ランナーは依存 DAG を解析し、Packrat/左再帰の設定を自動調整する。
- `ExecutionPlan.backpressure` は `run` 実行時にチャネル深度監視を有効化し、メトリクス名 `dsl.in_flight`（[3-6 Core Diagnostics](3-6-core-diagnostics-audit.md)）へ数値を転送する。
- DSLごとの成功/失敗は `RunConfig` の `extensions` を通じて `record_dsl_success` / `record_dsl_failure` に引き渡し、監査ログと性能指標を同期させる。
