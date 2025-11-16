use std::{
    io,
    net::{TcpListener, TcpStream, ToSocketAddrs},
};

use serde_json::{Map, Value};

use crate::capability::AdapterCapability;

const NETWORK_EFFECT_SCOPE: &[&str] = &["effect {io.async}", "effect {security}"];

/// ネットワーク操作の Capability。
pub const NETWORK_CAPABILITY: AdapterCapability = AdapterCapability::new(
    "adapter.net",
    "beta",
    NETWORK_EFFECT_SCOPE,
    "adapter.net",
);

/// TCP ソケット接続。
pub fn connect<A: ToSocketAddrs>(addr: A) -> io::Result<TcpStream> {
    TcpStream::connect(addr)
}

/// TCP ソケットで待ち受ける。
pub fn listen<A: ToSocketAddrs>(addr: A) -> io::Result<TcpListener> {
    TcpListener::bind(addr)
}

/// 監査メタデータ。
pub fn audit_metadata(operation: &str, status: &str) -> Map<String, Value> {
    NETWORK_CAPABILITY.audit_metadata(operation, status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        thread,
    };

    #[test]
    fn audit_metadata_contains_network_keys() {
        let metadata = audit_metadata("connect", "success");
        assert_eq!(metadata["capability.id"], "adapter.net");
        assert_eq!(metadata["adapter.net.operation"], "connect");
    }

    #[test]
    fn tcp_connect_roundtrip() {
        let listener = listen("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            stream.write_all(b"ok").expect("write");
        });
        let mut stream = connect(addr).expect("connect");
        let mut buf = [0u8; 2];
        stream.read_exact(&mut buf).expect("read");
        assert_eq!(&buf, b"ok");
        handle.join().expect("thread");
    }
}
