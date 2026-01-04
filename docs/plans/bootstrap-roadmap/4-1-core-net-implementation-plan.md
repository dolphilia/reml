# Phase4: Core.Net 実装計画（HTTP/TCP/UDP/URL）

## 背景と決定事項
- `docs/notes/stdlib/stdlib-expansion-research.md` の **P0** として `Core.Net` を最優先拡張に指定した。
- `docs/spec/0-1-project-purpose.md` の「安全性」「実用性能」「段階的導入」を満たすため、最小APIから段階的に拡張する。
- `Core.Async` と `Core.Io` との整合が必須であり、`effect {net}` を `io` / `io.async` に包含させる設計を採る。

## 目的
1. Reml 標準ライブラリに `Core.Net` の最小 API を導入し、HTTP/TCP/UDP/URL を扱える基盤を提供する。
2. Rust 実装（`compiler/runtime`）にネットワーク実装を追加し、効果・監査・Capability と統合する。
3. Phase 4 の実用シナリオにネットワークユースケースを追加し、実行ログと診断を整備する。

## スコープ
- **含む**: `Core.Net` 仕様策定、Rust 実装追加、監査/Capability 連携、サンプル/回帰シナリオ。
- **含まない**: HTTP/2, WebSocket, gRPC, 本格的なTLS終端の実装（Phase 5 以降で検討）。

## 成果物
- `docs/spec/3-17-core-net.md`（新規）と関連概要の更新。
- `compiler/runtime/src/net/` 以下の実装と `runtime/lib.rs` への公開。
- `examples/practical/core_net/` と `expected/` の最小サンプル、Phase 4 シナリオ登録。
- `docs/spec/3-6-core-diagnostics-audit.md` / `3-8-core-runtime-capability.md` の追記。

## 依存関係
- `docs/spec/3-5-core-io-path.md`（IO基盤）
- `docs/spec/3-9-core-async-ffi-unsafe.md`（非同期実行基盤）
- `docs/spec/3-6-core-diagnostics-audit.md`（診断/監査）
- `docs/spec/3-8-core-runtime-capability.md`（Capability）

## 設計方針
- **最小 API から開始**: まずは HTTP クライアント + TCP/UDP + URL を最小構成で提供する。
- **安全性優先**: 明示的な `Result`/`Option` を使い、未捕捉例外で落ちない設計にする。
- **効果/Capabilityの統合**: `effect {net}` を定義し、Capability Registry へ `net.*` を追加する。
- **監査ログの一貫性**: `net.http.request` / `net.tcp.connect` 等の監査イベントを残す。
- **段階的導入**: HTTP サーバーとTLSは Phase 4 後半以降で段階的に導入する。

## API 仕様（ドラフト）

```reml
module Core.Net.Http

struct Request {
  method: HttpMethod
  url: Core.Net.Url
  headers: Core.Collections.Map<Text, Text>
  body: Core.Io.Bytes
}

struct Response {
  status: Int
  headers: Core.Collections.Map<Text, Text>
  body: Core.Io.Bytes
}

val Client.request : Client -> Request -> effect {net} Result<Response, NetError>
```

```reml
module Core.Net.Tcp

val connect : Core.Net.Url -> effect {net} Result<TcpStream, NetError>
val listen : Core.Net.Url -> effect {net} Result<TcpListener, NetError>
```

```reml
module Core.Net.Url

val parse : Text -> Result<Url, UrlError>
val build : UrlParts -> Result<Url, UrlError>
```

## 作業ステップ

### フェーズA: 仕様設計とドキュメント整備
1. `docs/spec/3-17-core-net.md` を新設し、`Core.Net.Http` / `Core.Net.Tcp` / `Core.Net.Udp` / `Core.Net.Url` の最小 API を定義する。
   - `Request`/`Response` の最小フィールド（`method`, `url`, `headers`, `body`）と `HttpMethod` 列挙、`Url`/`UrlParts` の不変条件を明文化する。
   - エラー型を `NetError`/`HttpError`/`UrlError` に分離し、`Result` での失敗時に返す診断キーを併記する。
   - `effect {net}` の意味範囲（DNS/接続/送受信/URL解析）を定義し、`io`/`io.async` との包含関係を記載する。
2. `docs/spec/3-0-core-library-overview.md` と `docs/spec/README.md` に `Core.Net` を追記し、章内リンクとセクション要約を追加する。
   - 主要モジュール、最小 API の利用シナリオ、Phase 5 で拡張予定の項目（TLS/HTTP2）を簡潔に記載する。
3. `docs/spec/3-6-core-diagnostics-audit.md` に `net.*` 系診断/監査キーを追加する。
   - `net.http.request` / `net.http.response` / `net.tcp.connect` / `net.tcp.listen` / `net.udp.bind` / `net.udp.send` の監査イベントと必須メタデータ（`url`, `method`, `status`, `bytes`, `elapsed_ms` など）を定義する。
   - 失敗時の診断キー（例: `net.http.timeout`, `net.tcp.connect_refused`）と再試行・タイムアウト時の扱いを明記する。
4. `docs/spec/3-8-core-runtime-capability.md` に `net.http.client` / `net.tcp.connect` / `net.tcp.listen` / `net.udp.bind` / `net.udp.send` などの Capability を登録する。
   - Capability の Stage 初期値と監査証跡（`effect.stage.required` / `capability.granted`）を揃える。
5. `docs/spec/0-2-glossary.md` に必要な新語（`UrlParts`, `NetError`, `TcpStream` 等）を追記し、用語揺れを防ぐ。

### フェーズB: Rust Runtime の最小実装
1. `compiler/runtime/src/net/` を追加し、`mod.rs` と `http.rs` / `tcp.rs` / `udp.rs` / `url.rs` を配置する。
   - モジュール公開順と `pub(crate)` 範囲を決め、`runtime/lib.rs` から `Core.Net` を再輸出する。
2. URL 解析は `url` クレートなどを候補に、`Url`/`UrlParts` への変換と `UrlError` マッピングを実装する。
   - `parse` と `build` の逆写像を確認し、失敗時の `UrlError` 種別と診断キーを固定する。
3. HTTP クライアントは `reqwest`（同期/非同期）や `ureq` などの候補比較を行い、最小構成の依存を決定する。
   - 初期は同期 API を優先し、`Client.request` のブロッキング/非ブロッキング方針と `Core.Async` 連携の分岐条件を記録する。
4. TCP/UDP は `std::net` の同期版で最小実装を用意し、タイムアウト/バッファ制御を `RunConfig` から注入できるようにする。
   - `TcpStream`/`TcpListener`/`UdpSocket` の薄いラッパを用意し、`Core.Io` の `Reader/Writer` と接続するための変換関数を追加する。
5. 監査イベントの発火位置を実装内に明記し、`net.*` キーを `AuditEnvelope` へ書き込む経路を通す。

### フェーズC: 非同期統合と Effect 設計
1. `Core.Async` と `Core.Net` の接続点を整理し、`io.async` と `net` の効果整合を明文化する。
   - `effect {net}` の実行経路が `Async` 経由か同期経由かを判定する規則を定義する（例: `RunConfig.net.mode = sync|async`）。
2. `Core.Net.Http` のストリーミングボディと `Core.Io.Reader/Writer` の互換ルールを追加する。
   - 受信ボディの最大サイズ制限、メモリ上限超過時のエラー、ストリーム途中終了時の診断キーを規定する。
3. `RunConfig` にネットワークのタイムアウト/リトライポリシーを追加し、監査メタデータに残す。
   - `net.timeout.connect` / `net.timeout.read` / `net.retry.max_attempts` を設け、`AuditEnvelope.metadata` に記録する。
4. `Core.Net` の API 仕様に `Async` 版のシグネチャを追記するか、`Core.Async` 側のアダプタで吸収するかを決定し、仕様書に方針を明記する。

### フェーズD: サーバー/高レベル API
1. `Core.Net.Http.Server` の最小 API を追加し、ルーティングとリクエストハンドラを定義する。
   - `Server.start : ServerConfig -> (Request -> effect {net} Result<Response, NetError>) -> effect {net} Result<ServerHandle, NetError>` のような形で最小署名を固定する。
2. `Core.Net.Tcp` の `TcpListener` から `Stream` への変換を定義し、`Core.Async` で非同期受理する。
   - `accept` の戻り値（`(TcpStream, SocketAddr)`）とキャンセル規約、バックプレッシャーの扱いを明記する。
3. サーバー実装は `runtime` 側で最小の依存に留め、Phase 4 ではローカル/開発用の用途に限定する方針を明文化する。
4. TLS や HTTP/2 は Phase 5 以降の拡張項目として別途計画を作成し、`docs/plans/bootstrap-roadmap/4-2-*` との接続点を整理する。

### フェーズE: サンプル・回帰・監査ログ
1. `examples/practical/core_net/` に HTTP クライアント / URL 解析 / TCP エコー / UDP 送受信の最小サンプルを追加する。
   - 成功系と失敗系（タイムアウト、接続拒否、URL 解析失敗）をセットで用意する。
2. `expected/` に stdout / diagnostics / audit のゴールデンを追加し、`docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` にシナリオを登録する。
   - `diagnostic_keys` / `audit_events` / `stage_requirement` を埋め、`resolution=pending` で初期登録する。
3. `reports/spec-audit/ch5/logs/` に実行ログを保存し、診断/監査キーが `docs/spec/3-6-core-diagnostics-audit.md` と一致することを確認する。
4. 追加したサンプルが `docs/spec/3-17-core-net.md` のコード例と一致することを点検し、相互参照リンクを補完する。

## タイムライン（目安）

| 週 | タスク |
| --- | --- |
| 79 週 | フェーズA: 仕様設計とドキュメント整備 |
| 80 週 | フェーズB: Rust Runtime 最小実装 |
| 81 週 | フェーズC: 非同期統合と Effect 設計 |
| 82 週 | フェーズD: HTTP サーバー最小実装 |
| 83 週 | フェーズE: サンプル/回帰/監査ログ |

## リスクと緩和策

| リスク | 影響 | 緩和策 |
| --- | --- | --- |
| ネットワークI/Oの安全性が揺らぐ | 実行時エラー増加 | `Result` 徹底、タイムアウト/リトライを `RunConfig` に明示 |
| TLS 実装が肥大化 | スケジュール遅延 | Phase 5 以降に分離し、Phase 4 では非TLS/ローカル通信を優先 |
| 監査ログの過多 | ログ管理の複雑化 | `net.*` の監査キーを最小集合で定義して段階的に追加 |

## 進捗状況
- ドラフト作成時点では未着手。各フェーズ完了時に日付を追記する。

## 参照
- `docs/notes/stdlib/stdlib-expansion-research.md`
- `docs/spec/0-1-project-purpose.md`
- `docs/spec/3-5-core-io-path.md`
- `docs/spec/3-6-core-diagnostics-audit.md`
- `docs/spec/3-8-core-runtime-capability.md`
- `docs/spec/3-9-core-async-ffi-unsafe.md`
