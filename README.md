# Holo Router

## Architecture

![Architecture diagram](./diagram.svg)

### Gateway

Gateway dispatches unaltered TCP traffic by TLS SNI that is resolved by
replacing `.holohost.net` suffix with `.internal-holohost.net`.

[dnscrypt-proxy]: https://github.com/DNSCrypt/dnscrypt-proxy
[letsencrypt]: https://letsencrypt.org
[wikipedia-sni]: https://en.wikipedia.org/wiki/Server_Name_Indication
