# Phase4: Core.Net 実装計画（HTTP/TCP/UDP/URL）

## 背景と決定事項
- `docs/notes/stdlib-expansion-research.md` の **P0** として `Core.Net` を最優先拡張に指定した。
- `docs/spec/0-1-project-purpose.md` の「安全性」「実用性能」「段階的導入」を満たすため、最小APIから段階的に拡張する。
- `Core.Async` と `Core.Io` との整合が必須であり、`effect {net}` を `io` / `io.async` に包含させる設計を採る。

## 目的
1. Reml 標準ライブラリに `Core.Net` の最小 API を導入し、HTTP/TCP/UDP/URL を扱える基盤を提供する。
2. Rust 実装（`compiler/rust/runtime`）にネットワーク実装を追加し、効果・監査・Capability と統合する。
3. Phase 4 の実用シナリオにネットワークユースケースを追加し、実行ログと診断を整備する。

## スコープ
- **含む**: `Core.Net` 仕様策定、Rust 実装追加、監査/Capability 連携、サンプル/回帰シナリオ。
- **含まない**: HTTP/2, WebSocket, gRPC, 本格的なTLS終端の実装（Phase 5 以降で検討）。

## 成果物
- `docs/spec/3-17-core-net.md`（新規）と関連概要の更新。
- `compiler/rust/runtime/src/net/` 以下の実装と `runtime/lib.rs` への公開。
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
1. `docs/spec/3-17-core-net.md` を新設し、HTTP/TCP/UDP/URL の最小 API を定義する。
2. `docs/spec/3-0-core-library-overview.md` と `docs/spec/README.md` に `Core.Net` を追記する。
3. `docs/spec/3-6-core-diagnostics-audit.md` に `net.*` 系診断/監査キーを追加する。
4. `docs/spec/3-8-core-runtime-capability.md` に `net.http.client` / `net.tcp.connect` などの Capability を登録する。

### フェーズB: Rust Runtime の最小実装
1. `compiler/rust/runtime/src/net/` を追加し、`http.rs` / `tcp.rs` / `udp.rs` / `url.rs` を配置する。
2. HTTP クライアントは `reqwest` 等を候補とし、`Client.request` を同期/非同期のどちらで提供するか決定する。
3. TCP/UDP は `std::net` の同期版を最小構成として実装し、`Core.Async` と接続するための拡張口を用意する。
4. `runtime/lib.rs` で `Core.Net` API を公開し、`net` 効果と監査イベントを紐付ける。

### フェーズC: 非同期統合と Effect 設計
1. `Core.Async` と `Core.Net` の接続点を整理し、`io.async` と `net` の効果整合を明文化する。
2. `Core.Net.Http` のストリーミングボディと `Core.Io.Reader/Writer` の互換ルールを追加する。
3. `run_config` にネットワークのタイムアウト/リトライポリシーを追加し、監査メタデータに残す。

### フェーズD: サーバー/高レベル API
1. `Core.Net.Http.Server` の最小 API を追加し、ルーティングとリクエストハンドラを定義する。
2. `Core.Net.Tcp` の `TcpListener` から `Stream` への変換を定義し、`Core.Async` で非同期受理する。
3. TLS や HTTP/2 は Phase 5 以降の拡張項目として別途計画を作成する。

### フェーズE: サンプル・回帰・監査ログ
1. `examples/practical/core_net/` に HTTP クライアントと TCP/UDP サンプルを追加する。
2. `expected/` に CLI 出力を追加し、`phase4-scenario-matrix.csv` にシナリオを登録する。
3. `reports/spec-audit/ch4/logs/` に実行ログを保存し、診断/監査キーを検証する。

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
- `docs/notes/stdlib-expansion-research.md`
- `docs/spec/0-1-project-purpose.md`
- `docs/spec/3-5-core-io-path.md`
- `docs/spec/3-6-core-diagnostics-audit.md`
- `docs/spec/3-8-core-runtime-capability.md`
- `docs/spec/3-9-core-async-ffi-unsafe.md`
