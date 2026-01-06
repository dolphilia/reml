# 第16章: 解析・DSL・診断

## 1. 概要 (Introduction)

本章では、Reml ランタイムの中核機能の中でも、特に「言語内言語（DSL）」の構築と運用を支えるモジュール群に焦点を当てます。

標準ライブラリのプリミティブ（第15章）が「データ」を扱うための道具だとすれば、本章で扱う `parse`、`dsl`、`diagnostics`、`config` といったモジュールは、**「ロジックとルール」** を扱うための道具と言えます。ランタイムには、独自の DSL を安全に組み込むためのパーサーコンビネータや、仮想マシン（VM）、GC、アクターモデルといった実行基盤が含まれており、これらは設定ファイル（`reml.toml`）の解析にも利用されています。

具体的な対象範囲は以下の通りです。

- **Parse** (`compiler/runtime/src/parse`): メモ化機能（Packrat Parsing）とエラー回復を備えたパーサーコンビネータ。
- **DSL Kit** (`compiler/runtime/src/dsl`): オブジェクトシステム、GC、アクター、軽量 VM の最小実装。
- **Diagnostics** (`compiler/runtime/src/diagnostics`): 監査ログやメトリクス収集の基盤。
- **Config / Data** (`compiler/runtime/src/config`, `data`): マニフェスト解析と、変更差分（ChangeSet）の管理。

### このモジュール群の目的

なぜランタイムにこれほどリッチな機能セットが必要なのでしょうか？ 最大の理由は **「監査可能性（Auditability）と柔軟性の両立」** です。

Reml では、アプリケーションの設定やドメイン固有ロジックを DSL として記述することを推奨しています。ランタイムが標準で DSL 実行キットを提供することで、開発者は以下の恩恵を享受できます。

1. **一貫したエラー処理**: DSL のパースエラーや実行時エラーが、Reml 標準の診断形式（`GuardDiagnostic`）に統一される。
2. **自動化された監査**: DSL 上でのオブジェクト操作や GC イベントが、自動的に監査ログとして記録される。
3. **安全な設定変更**: 設定ファイルの変更が差分（`ChangeSet`）として管理され、どのパラメータがいつ変更されたか追跡可能になる。

### 入力と出力

- **入力**: DSL ソースコード文字列、`reml.toml` ファイル、メトリクス測定値など。
- **出力**: `ParseResult`（CST/AST）、`GuardDiagnostic`（診断情報）、監査イベント、`ConfigChange`（設定差分）。

この章の地図として、`parse/mod.rs` を参照すると良いでしょう。ここにはパーサー API のエントリポイントが集約されています。

- `compiler/runtime/src/parse/mod.rs:1-30`

## 2. データ構造 (Key Data Structures)

これらのモジュールは相互に連携していますが、まずはそれぞれの中心となるデータ構造を理解しましょう。

### 2.1 Parse: 位置追跡とコンビネータ

DSL の解析は `Parser` トレイトを中心に回りますが、その土台となるのが `Input` です。

- **`Input`**: UTF-8 文字列のスライスをラップし、現在の行（Line）や列（Column）といった位置情報を追跡します。単なるポインタ移動ではなく、文字境界を正しく扱うことで、マルチバイト文字を含むソースコードでも正確なエラー位置を報告できます。
  - `compiler/runtime/src/parse/combinator.rs:120-243`
- **`Parser`**: パーサー本体を表現する構造体です。内部に関数（クロージャ）を持ち、入力を消費して結果（`Reply`）を返します。Reml のパーサーはコンビネータスタイルを採用しており、小さなパーサーを組み合わせて複雑な文法を定義します。
  - `compiler/runtime/src/parse/combinator.rs:1387-1518`

### 2.2 DSL Kit: 言語エンジンの部品

`dsl` モジュールは、小さな言語処理系を作るための「レゴブロック」のような存在です。

- **`DispatchTable` / `MethodCache`**: 動的なメソッドディスパッチ（呼び出し）を実現します。メソッドルックアップの結果をキャッシュすることで、繰り返し実行時のパフォーマンスを確保しています。
  - `compiler/runtime/src/dsl/object.rs:12-102`
- **`GcHeap` / `GcRef`**: 単純なマーク＆スイープ方式のガベージコレクタ（GC）です。DSL 内で生成されたオブジェクトのライフサイクルを管理します。
  - `compiler/runtime/src/dsl/gc.rs:16-99`
- **`VmState` / `CallFrame`**: スタックベースの軽量 VM の状態を保持します。命令ポインタ（IP）やオペランドスタック、コールスタックを含み、DSL のバイトコードを実行します。
  - `compiler/runtime/src/dsl/vm.rs:10-55`

### 2.3 Diagnostics: 監査への架け橋

診断機能は、エラーメッセージをユーザーに届けるだけでなく、システムの健全性を監査システムへ伝える役割も担います。

- **`MetricPoint`**: CPU 使用率やメモリ割り当て回数などの計測値を保持します。
  - `compiler/runtime/src/diagnostics/metric_point.rs:23-178`
- **`GuardDiagnostic`**: ランタイム特有の診断型です。フロントエンドの診断と異なり、**「監査メタデータ」**（`audit_metadata`）を保持できる点が特徴です。これにより、エラー発生時にも「どのユーザーコンテキストで」「どの DSL が」エラーを起こしたかを詳細に記録できます。
  - `compiler/runtime/src/parse/combinator.rs:1225-1311`

### 2.4 Config / Data: 変更の履歴書

設定値の管理には、単なるファイル読み込み以上の機能が備わっています。

- **`Manifest` / `ConfigRoot`**: `reml.toml` の構造を型定したものです。プロジェクト設定や互換性プロファイルなどを管理します。
  - `compiler/runtime/src/config/manifest.rs:40-124`
- **`ChangeSet` / `SchemaDiff`**: 設定データに対する「変更差分」を表現します。例えば「キー A の値を 10 から 20 に変更」という操作自体をデータとして扱い、これを監査ログに記録することで、構成変更の追跡性を高めています。
  - `compiler/runtime/src/data/change_set.rs:4-141`
  - `compiler/runtime/src/config/collection_diff.rs:14-249`

## 3. アルゴリズムと実装 (Core Logic)

ここでは、これらのデータ構造が実際にどのように動作しているか、主要なアルゴリズムを解説します。

### 3.1 Packrat Parsing と左再帰ガード

Reml ランタイムのパーサーは、**Packrat Parsing** と呼ばれる手法を採用しています。これは解析（Parse）の結果をメモ化することで、バックトラック（手戻り）が頻発する文法でも線形時間に近いパフォーマンスを実現する手法です。

`Parser::parse` メソッドは、以下のフローで動作します。

1. 現在の入力位置をキーとして、メモ化テーブル（`ParseState` の `memo`）を検索する。
2. ヒットすれば、保存された結果を即座に返す。
3. ヒットしなければ、実際の解析ロジックを実行し、結果をメモ化テーブルに保存してから返す。
4. この際、**左再帰（Left Recursion）** を検知すると、無限ループを防ぐためのガードロジックが働きます。

- `compiler/runtime/src/parse/combinator.rs:1492-1517`

### 3.2 DSL 実行時の監査イベント発火

DSL の実行エンジンは、単にコードを動かすだけでなく、動作の「証拠」を残す義務があります。

例えば、オブジェクトのメソッド呼び出しを行う `Object::call` は、内部で `dsl.object.dispatch` という監査イベントを発行します。

```rust
// 概念的なフロー
fn call(&self, method: &str, args: &[Value]) -> Result<Value> {
    // 1. 監査イベントの発行
    audit::emit("dsl.object.dispatch", ...);
    
    // 2. メソッドキャッシュの確認
    if let Some(func) = self.cache.get(method) {
        return func.exec(args);
    }
    
    // 3. 動的ディスパッチとキャッシュ更新
    // ...
}
```

同様に、GC の割り当て（`Gc::alloc`）や VM の命令実行ステップ（`VmCore::step`）も、それぞれ個別の監査イベントを発火します。これにより、DSL 内で不正な挙動やリソースの乱用があった場合でも、後から詳細に調査することが可能です。

- `compiler/runtime/src/dsl/object.rs:133-172`
- `compiler/runtime/src/dsl/gc.rs:123-192`

### 3.3 エラー回復（Error Recovery）戦略

パーサーが構文エラーに遭遇したとき、そこで処理を停止するのではなく、**「可能な限り読み進めて、すべてのエラーを報告する」** ことが現代的なコンパイラには求められます。

`run_with_recovery_config` 関数は、このエラー回復モードを制御します。特別な設定（`RunConfig.extensions["recover"]`）が有効な場合、パーサーは「同期トークン（セミコロンや閉じ括弧など）」が見つかるまで入力をスキップし、そこから解析を再開します。これにより、一度のコンパイルで複数の文法エラーをユーザーに提示できます。

- `compiler/runtime/src/parse/combinator.rs:4365-4397`

### 3.4 演算子順位の動的構築

数式パーサーなどで必要となる「演算子の優先順位」を扱うために、`OpBuilder` が用意されています。これは `start_expression` から始まり、中置演算子（`infix`）や前置演算子（`prefix`）を登録した後、`build()` を呼ぶことで、優先順位が解決された `OpTable` を生成します。

この仕組みにより、DSL の設計者は複雑な再帰下降パーサーを手書きすることなく、宣言的に式の文法を定義できます。

- `compiler/runtime/src/parse/op_builder.rs:86-103`

### 3.5 設定差分のマージと監査

`Config` モジュールにおける `merge_maps_with_audit` は、設定データの更新処理の核心です。

1. 新旧のマップ（設定値の集合）を比較し、`ChangeSet`（差分）を計算する。
2. 単にメモリ上の値を更新するだけでなく、その `ChangeSet` を監査ログに記録する。
3. `reml.toml` などの永続化層へ変更を反映する。

このフローにより、「誰が設定を変更したか」だけでなく「具体的にどの値がどう変わったか」が完全に透明化されます。

- `compiler/runtime/src/config/mod.rs:70-84`

## 4. エラー処理 (Error Handling)

この層のエラー処理は、**「構造化」** と **「コンテキスト付与」** が鍵となります。

- **ParseError から GuardDiagnostic へ**: パースエラーは、期待されたトークン情報や、エラー発生位置周辺のソースコード（Span）を含んでいます。`to_guard_diagnostic` メソッドは、これらを `extensions` フィールドに格納し、JSON 形式などで機械可読な詳細情報を出力できるように変換します。
  - `compiler/runtime/src/parse/combinator.rs:1286-1311`
- **DSL エラーの正規化**: `DslError`、`DispatchError`、`VmError` などの各種エラーは、共通の `IntoDiagnostic` トレイトを通じて、統一されたエラーコード体系（例: `NoteId::DslDispatch`）にマッピングされます。これにより、どのレイヤでエラーが起きても、利用者は統一されたフォーマットでエラーを受け取ることができます。
  - `compiler/runtime/src/dsl/mod.rs:102-118`

## 5. 発展的トピック (Advanced Topics)

### 5.1 Parser プロファイリング

パフォーマンスチューニングのために、`ParserProfile` という仕組みが組み込まれています。これは Packrat Parsing のヒット率（メモ化がどれくらい効いているか）や、エラー回復が発動した回数を計測します。開発者は `RunConfig` を通じてこのプロファイル情報を出力させることができ、DSL の文法定義におけるボトルネック（例えば、バックトラックが多すぎるルール）を特定するのに役立ちます。

- `compiler/runtime/src/parse/combinator.rs:399-499`

### 5.2 監査イベントの拡張性

現在の `AuditPayload` は、`event.kind` などの基本的な共通フィールドを持っていますが、ペイロード部分は柔軟な Key-Value 構造になっています。これにより、将来的に新しい種類の DSL（例えば、SQL のようなクエリ言語）を追加した場合でも、コアランタイムの変更なしに、その DSL 特有の監査情報を監査ログに埋め込むことが可能です。

- `compiler/runtime/src/dsl/mod.rs:56-99`

## 6. 章末まとめ (Checkpoint)

本章では、Reml ランタイムにおける解析と実行、そして診断の仕組みについて学びました。

- **Parse**: メモ化（Packrat）とエラー回復を備えた強力なパーサーコンビネータにより、信頼性の高い DSL 解析を実現しています。
- **DSL Kit**: Object、GC、VM といった部品群は、独自の言語機能を組み込むための土台を提供すると同時に、自動的な監査ログ出力を保証します。
- **Diagnostics**: エラーレポートとシステム監査を結びつけ、問題発生時のコンテキストを詳細に記録します。
- **Config**: 設定ファイルの変更を `ChangeSet` として管理することで、環境構成の透明性を確保しています。

これらの機能により、Reml は単なるプログラミング言語であるだけでなく、**「監査可能で安全なドメイン固有言語のためのプラットフォーム」** としても機能します。

次章「第17章: FFI とネイティブ連携」では、ランタイムの外側の世界、すなわちネイティブコードとの安全なインターフェースについて解説します。

## 7. 仕様との対応 (Spec Sync)

本章の実装は以下の仕様に基づいています。

- Parse / DSL: `docs/spec/3-16-core-dsl-paradigm-kits.md`
- Diagnostics / Audit: `docs/spec/3-6-core-diagnostics-audit.md`
- Config / Data: `docs/spec/3-7-core-config-data.md`
