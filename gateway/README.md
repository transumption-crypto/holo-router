Proxy that peeks the socket for TLS Client Hello message without consuming it,
transforms `holohost.net` into `holohost.internal` in SNI hostname, resolves
new hostname against Registry and splices socket to the resolved IP address.

[Rustls][] is used for parsing TLS, [Tokio][] is used for handling TCP.

[Rustls]: https://github.com/ctz/rustls
[Tokio]: https://tokio.rs

