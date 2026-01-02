# Reml Algebraic Effects/Handlers 仕様草案

## 1. 目的と成功基準
- **性能最優先**: 継続捕捉のコストを抑え、10MB 規模の DSL 解析で線形時間を維持するため、ワンショットハンドラを既定とし、マルチショットは明示オプトインの拡張として扱う。
- **安全性**: 既存の効果タグ検査と互換にすることで、`@pure`/`@no_panic` 契約が崩れないよう、ハンドラ適用後の残余効果を静的に証明できる型体系を導入する。
- **段階的習得性**: Reml の「30 分で基本、1 日で DSL」を損なわないよう、エントリポイントはタグベースの理解に留め、行多相など高度な仕組みは上級者向け注釈に隔離する。
- **エコシステム統合**: Capability Registry・LSP・監査ログ機構と繋がるメタデータ（効果宣言の UUID、責務分離レベル）を設計に含める。

## 2. 基本方針
1. **タグ互換の効果行モデル**: 既存タグ（`io`, `panic`, `ffi` など）を効果行のラベルとして扱い、アルジェブラ的効果は `effect` 宣言でタグに属する操作集合として定義する。
2. **静的消費の明示**: ハンドラが効果を消費する際は、型推論段階で「残余効果行」が空であることを証明できれば `@pure` 署名にダウングレードする。証明できない場合は既存と同じくタグを保持する。
3. **ワンショット優先ランタイム**: コアランタイムはワンショット継続を前提とした軽量フレームを提供し、マルチショットは継続のコピーと再実行を行う `@reentrant` ハンドラで明示化。
4. **Capability 統合**: `@dsl_export` や `Capability Registry` は「残余効果行 ⊆ 許可集合」で判定し、効果宣言に `capability realm` 情報を付与する。
5. **段階的導入**: 1.x 系では「実験フラグ + 明示注釈」→ 2.x 系で HM + 行多相統合→ 安定化後に Core.Async/Diagnostics と統合、という三段階で展開。

## 3. 言語仕様案

### 3.1 効果宣言
```reml
effect Console : io {
  operation log : Text -> Unit
  operation ask : Text -> Text
}
```
- `effect <Name> : <tag>` で既存タグへ紐付け。
- 各 `operation` は型を持ち、暗黙に `perform` 呼び出しを生成。
- 効果宣言はモジュールスコープで一意（UUID を生成し Capability Registry に登録）。

### 3.2 効果を持つ式
```reml
fn greet(name: Text) -> Text ! { io } {
  let message = perform Console.ask("name?")
  Console.log("hi" + message)
  "hello " + name
}
```
- `-> T ! { io, panic }` が効果行注釈。省略時は空集合を推定。
- 行多相変数 `!ε` はトップレベル多相同時のみに許容（ランク1）。
- 推論ルール: `perform Console.log` は `io.Console.log` ラベルを生成し、タグ単位で集約。

### 3.3 ハンドラ構文
```reml
handle greet("Reml") with
  handler Console {
    operation log(msg, resume) {
      Diagnostics.trace(msg)
      resume(Void)
    }
    operation ask(prompt, resume) {
      resume("Reml")
    }
  }
```
- `handler <EffectName>` ブロックで該当操作を列挙。未列挙の操作は暗黙に外側へエスケープ。
- `resume(arg)` はワンショットがデフォルト。`resume` を複数回呼びたい場合はハンドラ宣言に `@reentrant` を付与し、型検査で `!multi` 効果タグを追加。
- ハンドラ自身の型: `handler Console : { io.Console.log, io.Console.ask } -> {}` のように「捕捉」→「残余」を明示。

### 3.4 残余効果の推論
- ハンドラ適用後、式の効果行は `(expr_effects - handler_captured) ∪ handler_residual` として計算。
- 型推論では `Σ_before` と `Σ_after` を比較し、差が空なら純粋化可能。
- `@pure` 署名の関数本体で効果が発生した場合、ハンドラで完全捕捉されればエラーにしない一方、捕捉漏れがある場合は従来通りコンパイルエラー。

### 3.5 Capability Registry との連携
- `effect` 宣言時に `capability realm`（例: `io.console`, `diagnostics.audit`）を必須メタフィールドとして定義。
- `@dsl_export` は `allows_effects=[io.console]` のようにタグ単位で許可を記述。ハンドラ適用後に残余が空であれば警告なく公開可能。
- LSP では効果宣言を解析し、ハンドラの捕捉状況を可視化（診断パネルで「未処理効果: panic.abort」などを提示）。

### 3.6 既存機能との整合
- **Core.Async**: `async/await` は `effect Async : io.async { operation await : Future<T> -> T }` で表現し、ランタイムはトランポリン上で継続を格納。
- **エラー処理**: `panic` は `effect Panic : panic` として再定義し、既存 `raise` は `perform Panic.raise` にシンタックスシュガーを提供。
- **Packrat/左再帰**: ハンドラ捕捉は再帰下降パーサに影響を与えるため、継続の保持コストを `Parser` 状態フレームにメモ化しない設計を推奨（ワンショット限定）。

### 3.7 Stage 管理と Capability

- `Stage = Experimental | Beta | Stable` を効果宣言・Capability Registry・診断拡張で共有する。
- `effect Console : io { ... }` のメタデータに `stage` を含め、`@requires_capability(stage="experimental")` で opt-in を強制する。
- `Diagnostic.extensions["effects"].stage` に現在の Stage を記録し、CLI/LSP が Experimental 診断を既定で警告扱いにできるようにする。
- Stage 昇格フロー:
  1. Experimental: `reml capability enable <effect> --stage experimental`、PoC を `-Zalgebraic-effects` で実行。
  2. Beta: `reml capability stage promote <effect> --to beta`。監査ログと `@dsl_export`/マニフェストの `expect_effects_stage` を更新。
  3. Stable: 実運用で問題が確認できたら `--to stable` を実行し、実験フラグ無しのビルドを許可。
- 昇格時は `effects.stage.promote_without_checks` 診断が発生しないことをもって整合が取れたと判断する。

### 3.8 Async/FFI 利用例（更新）

```reml
@handles(Console)
fn collect_logs(iter: Iter<Text>) -> Result<List<Text>, Diagnostic> ! {} =
  handle iter.try_fold(List::empty(), |acc, msg| {
    do Console.log(msg)
    Ok(acc.push(msg))
  }) with
    handler Console {
      operation log(msg, resume) {
        audit.log("bridge.console", msg)
        resume(())
      }
      return value { value }
    }
```

```reml
effect ForeignCall : ffi {
  operation call(name: Text, payload: Bytes) -> Result<Bytes, FfiError>
}

@handles(ForeignCall)
@requires_capability(stage="experimental")
fn with_foreign_stub(req: Request) -> Result<Response, FfiError> ! {} =
  handle do ForeignCall.call("service", encode(req)) with
    handler ForeignCall {
      operation call(name, payload, resume) {
        audit.log("ffi.call", { "name": name, "bytes": payload.len() })
        resume(Ok(stub_response(name, payload)))
      }
      return result { result.and_then(decode_response) }
    }
```

- どちらの例も `Diagnostic.extensions["effects"].residual = {}` を確認できれば純粋化可能。
- Stage 昇格後は `@requires_capability` の引数を `stage="beta"`/`"stable"` へ更新し、Capability Registry とマニフェストの値を同期させる。

## 4. 実装ロードマップ

1. **Experimental (1.x)**
   - 機能フラグ `-Zalgebraic-effects` を導入。
   - 型検査は注釈必須、推論なし。ハンドラはワンショット限定。
   - Runtime: スレッドローカルな `EffectStack` に継続フレームを格納、`resume` で復帰。
2. **Inference Integration (2.0)**
   - 行多相推論（Daan Leijen 形式）を Algorithm W 拡張として実装。効果劣モノポリズムはトップレベルのみ。
   - `@pure` 判定を残余効果で再定義、Capability Registry を新 API に対応。
3. **Ecosystem Stabilization (2.x)**
   - Core.Async/Diagnostics/IO をハンドラ基盤へ移行。
   - LSP で効果ツリー情報を提示、テストユーティリティにモックハンドラを追加。
   - マルチショット `@reentrant` を最適化（継続コピーのリングバッファ化）。

## 5. 性能と安全性の評価指標
- **性能**: 継続捕捉が 200ns 以下、再開が 150ns 以下（Core.Async ベンチマーク基準）。ワンショットではクロージャ割当を避け、スタックフレームを再利用。
- **メモリ**: 継続フレームは最大 3KB を上限とする固定スロットを持ち、巨大クロージャはヒープへスピル。ハンドラネスト 32 層でのピークメモリを現行設計 +15% 以下に抑制。
- **安全性**: `resume` 多重呼出しは型レベルで禁止し、`@reentrant` ハンドラには `!multi` 効果タグを付与して Capability Registry で明示許可が必要。
- **診断**: 未処理効果はコンパイルエラー。ハンドラ内部の例外は `panic` タグに再分類し、SpanTrace にハンドラ境界を記録。

## 6. ドキュメント・ツール整備計画
- `1-3-effects-safety.md`: タグ集合の定義を効果行仕様へアップデートし、`@pure` 判定ロジックを追記。
- `2-6-execution-strategy.md`: 継続捕捉フレームの実装指針、ワンショット/マルチショット分岐を解説。
- `3-6-core-diagnostics-audit.md`: ハンドラを利用した監査ログ注入パターンを追加。
- `docs/guides/`: DSL 開発者向けに「効果ハンドラでモック実装を差し替える」ハウツーを執筆。
- LSP 拡張仕様: 効果可視化、未処理検出、`resume` 誤用警告の診断メッセージフォーマットを策定。

## 7. 未解決課題
- **並列実行**: マルチスレッド環境での継続共有戦略（STM かローカルコピーか）を決定する必要がある。
- **ネイティブ FFI**: C から呼び出される際のハンドラ境界（カレント継続の保存ルール）を定義していない。
- **記憶領域安全性**: `resume` 後に古いスタックフレームへアクセスするケースを LLVM lowering でどう防ぐか検証が必要。
- **学習教材**: 効果ハンドラを段階的に導入するチュートリアルと演習問題の整備が未定。
- **Packrat 最適化**: 継続捕捉と Packrat メモ化を同時に行うときの再入制御アルゴリズムを要調査。

---
この草案は Reml の価値観（性能、安全性、学習容易性、DSL ファースト）を守りつつ、Algebraic Effects/Handlers を段階的に導入するための基盤設計をまとめたものである。今後は各章の仕様書へ反映する際に、ここで挙げた評価指標と未解決課題を進行管理のチェックリストとして活用する。
