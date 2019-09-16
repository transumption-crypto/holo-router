# Holo Router

![Diagram](./diagram.svg)

## Components

- Agent: HTTP `POST /v1` client
- Proxy: transparent domain substitution proxy with dispatch by SNI
- Registry: DNS and HTTP `POST /v1` server backed by [Scylla][]

[Scylla]: https://www.scylladb.com
