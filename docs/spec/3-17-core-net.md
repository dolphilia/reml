# 3.17 Core Net

> 目的：HTTP/TCP/UDP/URL の最小 API を標準化し、`effect {net}` と Capability/監査ログの整合を確保する。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | ドラフト（Phase 4 対象） |
| 効果タグ | `effect {net}`, `effect {io}`, `effect {io.async}` |
| 依存モジュール | `Core.Prelude`, `Core.Collections`, `Core.Text`, `Core.IO`, `Core.Async`, `Core.Diagnostics`, `Core.Runtime` |
| 相互参照 | [3-0 Core Library Overview](3-0-core-library-overview.md), [3-5 Core IO & Path](3-5-core-io-path.md), [3-6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md), [3-8 Core Runtime & Capability](3-8-core-runtime-capability.md), [3-9 Core Async / FFI / Unsafe](3-9-core-async-ffi-unsafe.md) |

## 1. 位置付け

- **最小 API から開始**: HTTP クライアント、TCP/UDP、URL 構造の最小セットを優先する。
- **安全性優先**: 失敗は `Result` で返し、`NetError`/`HttpError`/`UrlError` を明示する。
- **効果/Capability 統合**: `effect {net}` を新設し、`Core.Runtime` の Capability Registry と監査ログに連携する。
- **段階的導入**: HTTP サーバーや TLS は Phase 5 以降で拡張する。

## 2. 効果タグと Capability の対応

`effect {net}` は DNS 解決、接続、送受信、ネットワーク構成に依存する URL の検証を含む。`Core.Net` の API は原則として `effect {net}` を要求し、実行経路で `effect {io}` / `effect {io.async}` を併記する。

- `effect {net}` を伴う操作は `AuditEnvelope.metadata` に `net.*` キーを必須で記録する。
- `Core.Net.Url.parse` / `build` は `@pure` とし、URL 解析のみでは `effect {net}` を要求しない。
- Capability 初期 Stage は `Experimental` とし、`@requires_capability(stage="experimental")` を添付する。

対象 Capability:
- `net.http.client`
- `net.tcp.connect`
- `net.tcp.listen`
- `net.udp.bind`
- `net.udp.send`

## 3. Core.Net.Url

### 3.1 主要型と不変条件

```reml
pub type Url = {
  scheme: Str,
  authority: UrlAuthority,
  path: Str,
  query: Option<Str>,
  fragment: Option<Str>,
}

pub type UrlAuthority = {
  user_info: Option<Str>,
  host: Str,
  port: Option<Int>,
}

pub type UrlParts = {
  scheme: Str,
  user_info: Option<Str>,
  host: Str,
  port: Option<Int>,
  path: Str,
  query: Option<Str>,
  fragment: Option<Str>,
}
```

- `scheme` は ASCII 小文字で保持する（`"HTTPS"` 等は `"https"` に正規化）。
- `host` は `IPv4`/`IPv6` リテラルまたは DNS 名。IDN は `punycode` へ正規化する。
- `port` は `1..=65535` に限定する。未指定時は `None` とし、スキーム既定値は呼び出し側で補完する。
- `path` は `/` で始まる絶対パス。空の場合は `"/"` に正規化する。
- `query`/`fragment` は先頭の `?` / `#` を含めない。

### 3.2 API

```reml
module Core.Net.Url

fn parse(text: Str) -> Result<Url, UrlError> // `@pure`
fn build(parts: UrlParts) -> Result<Url, UrlError> // `@pure`
```

### 3.3 UrlError

```reml
pub type UrlError = {
  kind: UrlErrorKind,
  message: Str,
  diagnostic_key: Str,
}

pub enum UrlErrorKind =
  | InvalidScheme
  | InvalidHost
  | InvalidPort
  | InvalidPath
  | InvalidQuery
  | InvalidFragment
  | MissingAuthority
  | UnsupportedScheme
  | InvalidEncoding
```

| `UrlErrorKind` | 既定の診断キー | 補足 |
| --- | --- | --- |
| `InvalidScheme` | `net.url.invalid_scheme` | スキームが空、または許可外の文字を含む。 |
| `InvalidHost` | `net.url.invalid_host` | 空ホスト、無効な IPv4/IPv6、IDN 正規化失敗。 |
| `InvalidPort` | `net.url.invalid_port` | 数値範囲外または空ポート。 |
| `InvalidPath` | `net.url.invalid_path` | `/` で始まらない、または許容外の文字列。 |
| `InvalidQuery` | `net.url.invalid_query` | `?` を含む、またはデコード不能。 |
| `InvalidFragment` | `net.url.invalid_fragment` | `#` を含む、またはデコード不能。 |
| `MissingAuthority` | `net.url.missing_authority` | `scheme` を持つのに `host` が空。 |
| `UnsupportedScheme` | `net.url.unsupported_scheme` | `http`/`https`/`tcp`/`udp` 以外。 |
| `InvalidEncoding` | `net.url.invalid_encoding` | UTF-8 で解釈できない。 |

## 4. Core.Net.Http

### 4.1 主要型

```reml
module Core.Net.Http

pub type Url

pub enum HttpMethod =
  | Get
  | Post
  | Put
  | Patch
  | Delete
  | Head
  | Options

pub type Request = {
  method: HttpMethod,
  url: Url,
  headers: Map<Str, Str>,
  body: Bytes,
}

pub type Response = {
  status: Int,
  headers: Map<Str, Str>,
  body: Bytes,
}

pub type ClientConfig = {
  timeout_ms: Option<Int>,
  max_redirects: Int,
  default_headers: Map<Str, Str>,
}

pub type Client = { config: ClientConfig }
```

- `Request.body` は空バイト列を許容する。`GET`/`HEAD` は空ボディを推奨する。
- `Response.status` は HTTP ステータスコード（100-599）の範囲で保持する。
- `Request.url` には `Core.Net.Url` の `Url` 型を使用する。

### 4.2 API

```reml
fn client(config: ClientConfig) -> Client // `@pure`
fn request(client: Client, request: Request) -> Result<Response, HttpError> // `effect {net}`
```

### 4.3 HttpError

```reml
pub type NetError

pub type HttpError = {
  kind: HttpErrorKind,
  message: Str,
  diagnostic_key: Str,
  status: Option<Int>,
  cause: Option<NetError>,
}

pub enum HttpErrorKind =
  | Timeout
  | ConnectionFailed
  | InvalidResponse
  | RedirectLoop
  | BodyTooLarge
  | TlsUnavailable
  | ProtocolViolation
  | InvalidUrl
```

| `HttpErrorKind` | 既定の診断キー | 補足 |
| --- | --- | --- |
| `Timeout` | `net.http.timeout` | 接続・読み取りタイムアウト。 |
| `ConnectionFailed` | `net.http.connection_failed` | `NetError` を `cause` に格納。 |
| `InvalidResponse` | `net.http.invalid_response` | ステータス範囲外または必須ヘッダ欠落。 |
| `RedirectLoop` | `net.http.redirect_loop` | `max_redirects` を超過。 |
| `BodyTooLarge` | `net.http.body_too_large` | サイズ制限超過。 |
| `TlsUnavailable` | `net.http.tls_unavailable` | TLS が無効または未サポート。 |
| `ProtocolViolation` | `net.http.protocol_violation` | HTTP 仕様違反、フレーミング不整合。 |
| `InvalidUrl` | `net.http.invalid_url` | URL 解析失敗を再掲。 |

## 5. Core.Net.Tcp

### 5.1 主要型

```reml
module Core.Net.Tcp

pub type TcpStream
pub type TcpListener

pub type SocketAddr = {
  host: Str,
  port: Int,
}
```

`TcpStream` は `Core.IO.Reader` / `Core.IO.Writer` を実装し、IO 系 API で再利用できる。

### 5.2 API

```reml
fn connect(url: Url) -> Result<TcpStream, NetError> // `effect {net}`
fn listen(url: Url) -> Result<TcpListener, NetError> // `effect {net}`
fn accept(listener: TcpListener) -> Result<(TcpStream, SocketAddr), NetError> // `effect {net}`
fn close(stream: TcpStream) -> Result<(), NetError> // `effect {net}`
```

## 6. Core.Net.Udp

### 6.1 主要型

```reml
module Core.Net.Udp

pub type SocketAddr

pub type UdpSocket

pub type Datagram = {
  bytes: Bytes,
  peer: SocketAddr,
}
```

### 6.2 API

```reml
fn bind(url: Url) -> Result<UdpSocket, NetError> // `effect {net}`
fn send_to(socket: UdpSocket, bytes: Bytes, peer: SocketAddr) -> Result<Int, NetError> // `effect {net}`
fn recv_from(socket: UdpSocket) -> Result<Datagram, NetError> // `effect {net}`
fn close(socket: UdpSocket) -> Result<(), NetError> // `effect {net}`
```

## 7. NetError

```reml
pub type NetError = {
  kind: NetErrorKind,
  message: Str,
  diagnostic_key: Str,
  retryable: Bool,
  url: Option<Url>,
}

pub enum NetErrorKind =
  | DnsFailure
  | ConnectionRefused
  | ConnectionReset
  | Timeout
  | NetworkUnreachable
  | InvalidAddress
  | PermissionDenied
  | ProtocolViolation
  | Unsupported
```

| `NetErrorKind` | 既定の診断キー | 補足 |
| --- | --- | --- |
| `DnsFailure` | `net.dns.failure` | 解決失敗、または DNS 応答が不正。 |
| `ConnectionRefused` | `net.tcp.connect_refused` | TCP 接続拒否。 |
| `ConnectionReset` | `net.tcp.connection_reset` | 送受信中の切断。 |
| `Timeout` | `net.tcp.timeout` | TCP/UDP の送受信タイムアウト。 |
| `NetworkUnreachable` | `net.network_unreachable` | ルーティング不可、ネットワーク到達不能。 |
| `InvalidAddress` | `net.address.invalid` | URL/SocketAddr の不正。 |
| `PermissionDenied` | `net.permission_denied` | Capability や OS 権限不足。 |
| `ProtocolViolation` | `net.protocol_violation` | TCP/UDP のヘッダ不整合。 |
| `Unsupported` | `net.unsupported` | プラットフォーム非対応。 |

## 8. 使用例

### 8.1 URL 解析と HTTP GET

```reml
use Core;
use Core.Net.Http;
use Core.Net.Url;
use Core.Text;

fn parse_http_url(text: Str) -> Result<Url, Http.HttpError> {
  match Url.parse(text) with
  | Ok(url) -> Ok(url)
  | Err(err) -> Err({
      kind: Http.HttpErrorKind::InvalidUrl,
      message: err.message,
      diagnostic_key: "net.http.invalid_url",
      status: None,
      cause: None,
    })
}

fn get_text(url_text: Str) -> Result<Http.Response, Http.HttpError>  // effect {net}
{
  let url = parse_http_url(url_text)?;
  let headers = Map.empty_map();
  let client = Http.client({
    timeout_ms: Some(1500),
    max_redirects: 2,
    default_headers: headers,
  });

  let request = {
    method: Http.HttpMethod::Get,
    url: url,
    headers: headers,
    body: Text.as_bytes(""),
  };

  Http.request(client, request)
}
```

### 8.2 UDP 送信

```reml
use Core;
use Core.Net;
use Core.Net.Tcp;
use Core.Net.Udp;
use Core.Net.Url;
use Core.Text;

fn parse_udp_url(text: Str) -> Result<Url, Net.NetError> {
  match Url.parse(text) with
  | Ok(url) -> Ok(url)
  | Err(err) -> Err({
      kind: Net.NetErrorKind::InvalidAddress,
      message: err.message,
      diagnostic_key: "net.address.invalid",
      retryable: false,
      url: None,
    })
}

fn send_ping() -> Result<Int, Net.NetError>  // effect {net}
{
  let bind_url = parse_udp_url("udp://0.0.0.0:0/")?;
  let socket = Udp.bind(bind_url)?;
  let payload = Text.as_bytes("ping");
  let peer: Tcp.SocketAddr = { host: "127.0.0.1", port: 9000 };
  Udp.send_to(socket, payload, peer)
}
```

## 9. 監査ログと診断キー

### 9.1 監査イベント

`event.kind` には `net.*` を設定し、`AuditEnvelope.metadata` / `Diagnostic.audit_metadata` の双方に同一キーで記録する。

| `event.kind` | 必須メタデータ | 補足 |
| --- | --- | --- |
| `net.http.request` | `net.url`, `net.method`, `net.request_bytes`, `net.elapsed_ms` | 送信開始時に記録する。 |
| `net.http.response` | `net.url`, `net.status`, `net.response_bytes`, `net.elapsed_ms` | 受信完了時に記録する。 |
| `net.tcp.connect` | `net.url`, `net.elapsed_ms` | 接続成功時に記録する。 |
| `net.tcp.listen` | `net.url`, `net.listen_port` | リスナー確立時に記録する。 |
| `net.udp.bind` | `net.url`, `net.listen_port` | バインド成功時に記録する。 |
| `net.udp.send` | `net.peer`, `net.request_bytes`, `net.elapsed_ms` | 送信完了時に記録する。 |

### 9.2 診断キー

| 診断キー | 既定 Severity | 発生条件 | 推奨対応 |
| --- | --- | --- | --- |
| `net.http.timeout` | Error | HTTP タイムアウト | `timeout_ms` と再試行戦略を確認。 |
| `net.http.connection_failed` | Error | HTTP 接続失敗 | `NetError` の `kind` と `retryable` を確認。 |
| `net.tcp.connect_refused` | Error | TCP 接続拒否 | 宛先ポート・Firewall を点検。 |
| `net.tcp.timeout` | Error | TCP/UDP タイムアウト | `RunConfig` のタイムアウト設定を再検討。 |
| `net.dns.failure` | Error | DNS 失敗 | ネームサーバーとホスト名を検証。 |
| `net.url.invalid_scheme` | Error | URL 解析失敗 | 許可スキームへ修正。 |

`net.*` 診断は `Diagnostic.domain = Some(DiagnosticDomain::Net)` を既定とし、`AuditEnvelope.metadata` に `net.url`, `net.method`, `net.status`, `net.request_bytes`, `net.response_bytes`, `net.elapsed_ms` の該当項目を必須で残す。
必須メタデータが欠落した場合は `net.audit.missing_metadata` を `Warning` として記録する。
