# 1.3 効果と安全性（Effects & Safety）— Kestrel 言語コア仕様

> 目的：**書きやすさ・読みやすさ・高品質エラー**を保ったまま、**実用性能**と**静的安全**を両立。
> 方針：MVPでは **HM 型推論 + 値制限 + 属性ベースの効果契約** を採用し、複雑な型レベル効果（行多相など）は**任意の拡張段**に留める。**純粋関数がデフォルト**、副作用は明示。

---

## A. 効果の分類（MVP）

Kestrel は関数や式の“外界への作用”を次の**5種の効果フラグ**に分類し、**検出・表示**し、必要に応じて**静的に禁止**できる。

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

  ```kestrel
  let id = |x| x                 // 一般化: ∀a. a -> a
  let line = readLine()          // io 効果 → 単相
  ```

---

## C. 効果の宣言と抑制（属性）

型システムに効果を織り込みすぎないため、\*\*属性（アトリビュート）\*\*で「効果契約」を表明・検査する。

```kestrel
@pure        // mut/io/ffi/panic/unsafe を禁止
@no_panic    // panic を禁止（→ コンパイル時チェック）
@no_alloc    // 文字列/ベクタ等のヒープ確保を禁止（MIR検査）
@must_use    // 戻り値の未使用を禁止（Result 等に推奨）
@inline      // 最適化ヒント
```

* **違反時は型エラー同等のわかりやすい診断**を出す。
* 例：

  ```kestrel
  @pure
  fn sum(xs: [i64]) -> i64 = { print("x"); fold(xs, 0, (+)) }
  // error: @pure 関数で io 効果が検出されました … at print
  ```

---

## D. 例外・エラー処理と全称性

* **例外は言語機能として持たない**。ランタイム停止は `panic`（開発時向け）だけ。
* **失敗は型で扱う**：`Option<T>`, `Result<T,E>` を標準化。
* **伝播糖衣 `?`**（MIR で早期 return に降格）：

  ```kestrel
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

  ```kestrel
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

  ```kestrel
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

  ```kestrel
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

```kestrel
@pure
fn intLit() -> Parser<i64> =
  digits().map(parseInt)             // 文字列→数値, 外界に作用しない
```

### J.2 効果を持つ境界を薄くする

```kestrel
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

```kestrel
extern "C" fn qsort(ptr: Ptr<u8>, len: usize, elem: usize, cmp: Ptr<void>) -> void

fn sortBytes(xs: Vec<u8>) -> Vec<u8> = {
  unsafe {
    qsort(xs.ptr(), xs.len(), 1, cmp_ptr)  // ffi + unsafe
  }
  xs
}
```

### J.4 `@no_panic` と `?`

```kestrel
@no_panic
fn strictlyPositive(n: i64) -> Result<i64, Error> = {
  if n <= 0 { return Err(Error::Invalid) }
  Ok(n)
}

fn total(xs: [i64]) -> Result<i64, Error> =
  xs |> map(strictlyPositive) |> sequence ? |> sumOk
```

---

## K. 仕様チェックリスト（実装者向け）

* [ ] 各式ノードに**効果ビット集合**を付与・合成（AST→TAST→MIR）。
* [ ] `let` 一般化は **効果なし**かつ**純度がわかる式**に限定。
* [ ] `@pure/@no_panic/@no_alloc` 検査は**コンパイル時強制**。
* [ ] `unsafe` 境界を CFG に刻み、`ffi`/原始操作は **境界内のみ許可**。
* [ ] `defer` はスコープ退出（正常/早期 `return`/`?`/`panic`）の**いずれでも発火**。
* [ ] エラーメッセージは**効果名 + 位置 + 修正提案**を必ず含む。
* [ ] `Parser` 標準APIは**外界作用を持たない**ことを CI で回帰検査。

---

### まとめ

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
