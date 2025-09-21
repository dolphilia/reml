# 1.3 効果と安全性（Effects & Safety）— Reml (Readable & Expressive Meta Language) 言語コア仕様

> 目的：**書きやすさ・読みやすさ・高品質エラー**を保ったまま、**実用性能**と**静的安全**を両立。
> 方針：MVPでは **HM 型推論 + 値制限 + 属性ベースの効果契約** を採用し、複雑な型レベル効果（行多相など）は**任意の拡張段**に留める。**純粋関数がデフォルト**、副作用は明示。

---

## A. 効果の分類（MVP）

Reml は関数や式の"外界への作用"を次の**5種の効果フラグ**に分類し、**検出・表示**し、必要に応じて**静的に禁止**できる。

| 効果       | 意味                             | 例                            | 既定           |
| -------- | ------------------------------ | ---------------------------- | ------------ |
| `mut`    | 局所的な可変状態（`var` の再代入、可変コンテナの更新） | `y := y + 1`, `vec.push(x)`  | 許可           |
| `io`     | I/O・時刻・乱数など観測可能な外部作用           | `print`, `readFile`, `now()` | 許可           |
| `ffi`    | FFI 呼び出し（言語外の未検査境界）            | `extern "C" puts`            | `unsafe` 内のみ |
| `panic`  | 非全称（中断）・アサート失敗                 | `panic("…")`, `assert(x>0)`  | 許可（制限可）      |
| `unsafe` | メモリ安全や型安全の前提を破りうる操作            | 原始ポインタ操作、未定義レイアウトへのキャスト      | `unsafe` 内のみ |

> 参考：`Parser` などの**ライブラリ内“擬似効果”**（バックトラック、`cut`、`trace`）は**言語効果ではない**。外界を変えず、`Parser<T>` の**戻り値に閉じ込める**のが原則。

---

## B. デフォルトの純粋性と値制限

* **純粋（pure）デフォルト**：関数は**効果を持たない**と仮定される。
* **効果検出**：本体に `mut/io/ffi/panic/unsafe` を含むと、関数は該当効果を**潜在効果**として持つ。
* **値制限（1.2 で予告）**：`let` 束縛の一般化は**効果のない確定値**に限る。効果を含む右辺は**単相**。

  ```reml
  let id = |x| x                 // 一般化: ∀a. a -> a
  let line = readLine()          // io 効果 → 単相
  ```

---

## C. 効果の宣言と抑制（属性）

型システムに効果を織り込みすぎないため、\*\*属性（アトリビュート）\*\*で「効果契約」を表明・検査する。

```reml
@pure        // mut/io/ffi/panic/unsafe を禁止
@no_panic    // panic を禁止（→ コンパイル時チェック）
@no_alloc    // 文字列/ベクタ等のヒープ確保を禁止（MIR検査）
@must_use    // 戻り値の未使用を禁止（Result 等に推奨）
@inline      // 最適化ヒント
```

* **違反時は型エラー同等のわかりやすい診断**を出す。
* 例：

  ```reml
  @pure
  fn sum(xs: [i64]) -> i64 = { print("x"); fold(xs, 0, (+)) }
  // error: @pure 関数で io 効果が検出されました … at print
  ```

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
  var acc = 0
  for x in xs { acc := acc + x }   // `:=` は再代入
  ```
* **可変コンテナ**は標準ライブラリで提供（例：`Vec<T>`, `Cell<T>`, `Map<K,V>`）。
  これらの更新操作は `mut` 効果。
* **性能指針**：実装は参照カウント（RC）＋**コピーオンライト**を併用し、関数型スタイルでも実用性能を確保（仕様上の約束事）。

> 解析器（Parser）を書く文脈では、**不変データ + 明示的な畳み込み**が既定の流儀。

---

## F. FFI と unsafe

* **FFI 宣言**：

  ```reml
  extern "C" fn puts(ptr: Ptr<u8>) -> i32
  ```

  * FFI は **`ffi` 効果**を持つ。呼び出しは **`unsafe` ブロック**内でのみ許可。
* **`unsafe { … }` ブロック**：

  * 原始ポインタ `Ptr<T>` やレイアウト未定義のキャスト等、**未定義動作を起こしうる操作**を囲う。
  * コンパイラは `unsafe` 境界を**明示化**し、内部の効果を外へ**押し上げ**る（呼び出し側が `unsafe` でなくても `ffi` 効果が残る）。
* **安全設計の原則**：`unsafe` を**小さく閉じ込め**、安全なラッパ API を公開。`pub` API は極力 Safe に。

---

## G. リソース安全（スコープ終端保証）

* **`defer expr`**：ブロック脱出時に `expr` を必ず実行。
  例：ファイルやロックの確実な解放。

  ```reml
  fn write(path: String, bytes: [u8]) -> Result<(), Error> = {
    let f = File.open(path, "wb")?; defer f.close()
    f.writeAll(bytes)?; Ok(())
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

| サブフラグ        | 意味                                           | 親効果 |
| ---------------- | ---------------------------------------------- | ------ |
| `io.async`       | ノンブロッキング I/O／イベントループ協調を要求する操作             | `io`   |
| `io.blocking`    | スレッド阻塞を伴う呼び出し（同期ファイル I/O、長時間待機など）       | `io`   |
| `io.timer`       | タイマー／スケジューラ登録、ディレイ、周期起動など時間イベントの操作 | `io`   |

* **包含関係**：関数が `io.async` を持つ場合、集計上は `io` も保持する（`io.async ⊆ io`）。逆方向は成立しない。
* **推論規則**：
  * `async fn`（導入予定）はシグネチャ解釈時に暗黙で `io.async` を付与。
  * ブロッキング API を `async` 関数内で呼ぶ場合は `await blocking { ... }` のような隔離シンタックスを経由させ、`io.blocking` を局所に閉じ込める方針。
  * タイマー操作は `io.timer` を生成し、`@no_timer`（後述）で抑制できるようにする。

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
* **将来の API**：`RunConfig.async`（計画中）でスケジューラ設定を行う際、`io.async`/`io.timer` の有無を前提にした検査フローを組む。
* **PoC 課題**：効果推論の一般化抑制（値制限）が `io.async` を含む場合にどう影響するかをテストで確認する。

---

## I. 効果と型推論の接続（実装規約）

* **効果は“型には織り込まない”**（MVP）。

  * ただしコンパイラ内部では各関数に\*\*潜在効果集合 `{mut, io, …}`\*\*を持たせ、

    * 値制限の判定
    * `@pure`/`@no_panic` 等の**契約検査**
    * ドキュメント・警告
      に用いる。
* **双方向型付け**（1.2）：注釈がある箇所では**効果も検査**を厳密化（例：`@pure` 関数内で `print` を発見→即時エラー）。
* **将来の拡張**：必要になれば**行多相ベースの効果型**を**オプトイン**で提供（Koka 風）。MVP のコードは**そのまま**動く方針。

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
extern "C" fn qsort(ptr: Ptr<u8>, len: usize, elem: usize, cmp: Ptr<void>) -> void

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
fn strictlyPositive(n: i64) -> Result<i64, Error> = {
  if n <= 0 { return Err(Error::Invalid) }
  Ok(n)
}

fn total(xs: [i64]) -> Result<i64, Error> =
  xs |> map(strictlyPositive) |> sequence ? |> sumOk
```
## K. 効果分類と運用ガイドライン

> 横断シナリオ（設定適用／監査／ホットリロード／分散実行）で必要となる**拡張効果タグ**を公式化し、`@requires(effect = …)` と `effect { … }` 表記が解釈すべき前提と義務を定義する。

### K.1 公式効果タグと意味論

Reml の効果システムは基礎 5 効果（A 節）に加えて、次の **領域別タグ** を持つ。タグは部分順序付き集合 `EffectTag` に属し、**静的検査**と **監査ログ** の両方で利用する。

| タグ | 区分 | 定義 | 静的要件 | 監査要件 |
| --- | --- | --- | --- | --- |
| `config` | 設定適用 | スキーマ宣言／差分適用 API (`Core.Config`) を通じて永続設定を変更する操作 | `Schema<T>` / `SchemaDiff` を引数に持ち、失敗時は `Result` で伝播 | `audit_id` と `change_set` を `AuditRecord` に残す |
| `audit` | 監査記録 | 変更・検証結果を永続監査ストアへ記録する | `AuditSink`（2-7）を経由して**コミット点**を明示し、エラーでも必ず `finalize()` | すべての `Diagnostic` に `domain`・`audit_id` を付与（2-5） |
| `runtime` | ランタイム制御 | 実行時のホットリロード・コード差し替え・FFI ハンドル操作 | `unsafe`/`ffi` が混在する場合は `@requires(effect={runtime, unsafe})` を併記 | `AuditRecord.kind = RuntimeChange` を記録し、巻き戻し手順を添付 |
| `db` | データストア | トランザクション駆動の永続データ変更（SQL/NoSQL） | `Transaction` / `Connection` 型を介して境界を固定。コミット前に `Result` が保証されること | `audit` と組み合わせ、コミットログに `change_set` を書き出す |
| `network` | 分散操作 | クラウド API・RPC・リモートサービスへの変更要求 | タイムアウト・リトライ戦略を `RunConfig`（2-6）で明示。暗号化/署名は型レベルで検査 | 主要イベントごとに `AuditRecord.domain = Network` を付与 |
| `gpu` | アクセラレータ | GPU / TPU などデバイスへの命令転送・メモリ確保 | `DeviceHandle` と `defer` 解放の組合せを強制。`unsafe` ブロックでの境界を厳格に | リソース利用量を `audit.metric` として集約 |

> `audit` は**副作用そのもの**というより、他効果を伴う操作に「監査責務」を課す**ガバナンス効果**である。`audit` の付く関数は失敗・成功の双方で `audit.log`（もしくは等価 API）を**ちょうど 1 回以上**呼ぶ必要がある。

### K.2 効果セットの組合せ規則

1. **タグの閉包**：`effect {config, audit}` のような宣言は、実行時に `config` が満たすべき監査義務を `audit` が補強する。`audit` を併用しない `config` 関数はコンパイラが警告（`W3101`）を発行する。
2. **`audit` の必須フィールド**：`audit` を含む関数から生成される `Diagnostic` / `AuditRecord` には、`{ domain, audit_id, change_set }` が必須。欠落時は型検査段で `DiagnosticFieldsMissing` エラーを生成する。
3. **コミット順序**：`db` と `audit` を同時に持つ場合、`audit.finalize()` はトランザクションコミット後かつ同一 `Span` で報告される必要がある。`@requires(effect = {db, audit})` はこの順序制約をチェックする。
4. **ホットリロード**：`runtime` と `network` を併用する関数は、`RunConfig` に `with_syntax_highlight` や `with_structured_log` が設定されている場合のみホットリロード対象とみなされる（2-6, Phase2 要件）。

### K.3 FFI / ランタイム連携指針

1. **クラウド API (`network`)**：署名・リトライを `RunConfig` に宣言し、すべてのリクエスト結果を `audit.log("network.call", …)` で永続化する。`timeout` が発生した場合は必ず `recover` で巻き戻し戦略を提示する。
2. **GPU/アクセラレータ (`gpu`)**：`runtime` + `unsafe` を組み合わせ、`DeviceHandle` 取得と `defer handle.close()` を対にする。性能メトリクスは `AuditRecord.metrics` へ集約する。
3. **組み込み I/O (`runtime`)**：メモリマップトレジスタ操作は `Resource<P, K>` 型で表現し、`@requires(effect={runtime, audit})` を付けて書き込み経路を監査する。

チェックリスト

- **クラウド API**：リトライ・認証・`audit_id` の発番を `RunConfig` と `AuditRecord` の双方に記録したか。
- **GPU**：割当/解放が `defer` で対になっているか。`unsafe` ブロックの境界が最小か。
- **組み込み**：レジスタマップの検証・割込みマスク・フェイルセーフが型レベルで確認できるか。

### K.4 ホットリロード / 差分適用

* `runtime` を含む関数のみホットリロード対象とし、適用履歴は `audit` へ蓄積する（`AuditRecord.kind = Reload`）。
* `config` 効果を伴う差分適用は `SchemaDiff` を評価し、`audit.change_set` に差分を JSON 形式で格納する。
* 失敗時は `recover` コンビネータで旧世代へ復旧し、`Diagnostic.severity_hint = Rollback` を出力する。

#### サンプル

```reml
@requires(effect = {runtime, audit})
fn reloadParser(parser: Parser<AppConfig>, diff: SchemaDiff<Old, New>)
  -> Result<Parser<AppConfig>, ReloadError>
  effect {runtime, audit} =
    recover({
      let updated = applyDiff(parser, diff)?;
      audit.log("parser.reload", {
        domain = "Core.Config",
        audit_id = diff.audit_id(),
        change_set = diff.asChangeSet()
      });
      Ok(updated)
    }, with: |err| {
      audit.log("parser.reload.fail", {
        domain = "Core.Config",
        audit_id = diff.audit_id(),
        change_set = diff.asChangeSet(),
        diagnostics = err.toDiagnostics()
      });
      Err(err)
    })
```

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

Reml は `Core.Unsafe.Ptr` モジュールで `Ptr<T>` / `MutPtr<T>` / `NonNullPtr<T>` / `Ptr<void>` / `FnPtr` を提供する（詳細は [Core.Unsafe.Ptr API 草案](notes/core-unsafe-ptr-api-draft.md)）。
それぞれに `unsafe` 効果が付随し、`MutPtr<T>` と `FnPtr` は `ffi` 効果とも組み合わせて扱う。
`NonNullPtr<T>` は NULL 不許可を静的に表現し、`Span<T>` など境界チェック付きビューの基礎となる。

### M.2 生成と取得

`addr_of` / `addr_of_mut` は評価順序を固定したまま参照のアドレスを取得し、`Buffer.asPtr` など安全ラッパからのダウングレードもここに集約する。
外部ポインタは `require_non_null` を通じて `Option<NonNullPtr<T>>` に昇格させ、NULL を検出すれば `NullError` として `Result` に反映する。
FFI 経由で取得した `Ptr<void>` は型情報を欠くため、以降のキャストは必ず `unsafe` ブロック内で行う（[guides/reml-ffi-handbook.md](guides/reml-ffi-handbook.md) 参照）。

### M.3 読み書きと境界検査

`read`/`write`/`copy_to` などの操作は整列や領域サイズを満たさないと未定義動作になる。
境界保証が必要な場合は `Span<T>` や `Slice<T>` を経由し、ここから `Ptr<T>` へ降格する位置をコードレビューで明示する。
`copy_nonoverlapping` と `copy_to` の区別により、`memcpy`/`memmove` を効率的に選択できる。

### M.4 アドレス計算とキャスト

`add`/`offset`/`byte_offset` は同一アロケーション内に留まる前提でのみ定義される。
整数キャスト（`to_int`/`from_int`）や型変更（`cast`/`cast_mut`）は `unsafe` の明示と共に、整列要件を仕様書 (`a-jit.md` の ABI 節) に従わせる。
ポインタ比較は `==`/`!=` のみに限定し、順序比較は未規定とする。

### M.5 所有権とリソース管理

RC で管理する値を指すポインタは `inc_ref`/`dec_ref` を `unsafe` ブロック内で対にし、`defer` による解放を推奨する。
スレッド境界では `Send`/`Sync` 相当のマーカートレイトを付与しない限り `Ptr<T>` の共有を禁止し、必要な場合は `@requires(effect={runtime, unsafe})` を併記する。
所有権の移譲や回収は `Result` と `audit.log` に記録し、監査タグ（K 節）と連動させる。

### M.6 適用シナリオ別ガイド

- **FFI**: `extern "C"` 呼び出し時に `Ptr<u8>` や `FnPtr` を利用し、`ffi` 効果タグと `audit` 記録をセットにする。
- **GPU/IO**: `Ptr<void>` をデバイスハンドルとして扱う場合は `effect {runtime, gpu, unsafe}` を宣言し、`defer` でリソース解放を保証。
- **GC ルート**: `NonNullPtr<Object>` を `runtime::register_root` に渡し、`write_barrier` と連携して世代間更新を安全に処理する（[2-6-execution-strategy.md](2-6-execution-strategy.md#L284) 参照）。


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
* [a-jit.md](a-jit.md) - FFI・unsafe・メモリ管理の実装方針
