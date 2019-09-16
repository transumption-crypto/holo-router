# Holo Router

![Diagram](./diagram.svg)

## Components

- Agent: HTTP `POST /v1` client
- Gateway: transparent domain substitution proxy with dispatch by SNI
- Registry: DNS and HTTP `POST /v1` server backed by [Scylla][]

[Scylla]: https://www.scylladb.com

## Rationale

Current Kong/Zato centralized services have a number of issues:

- As a result of being built on top of Nginx, Kong can't act as SNI
  peek-and-splice proxy. It can only reverse proxy, which incurs a lot of
  overhead (running Lua, parsing HTTP, copying packets to and back from
  userspace) and allows Holo to see all traffic in cleartext.

- Kong is built on top of Nginx that is not memory-safe. Most high-severity
  vulnerabilities are caused by lack of memory safety. See [Nginx CVEs][].

- `holo-init` Zato services are vulnerable to abuse:
  + `/holo-init-cloudflare-dns-create` is vulnerable to denial of service
  + `/holo-init-proxy-route-create` does not validate ownership of public key and 
    can be used to launch social engineering attacks by claiming addresses such as 
    http://www.hydrа.holohost.net
  + `/holo-init-proxy-service-create` does not validate target IP address and
    as a result will act as open proxy for anyone in the internet

- Hosts don't check that it's proxy that makes the request, which allows for
  selective denial of service, and circumvention of authorization flows.

- Kong is not designed to be horizontally scalable. Mapping of public keys to
  ZeroTier addresses and actual proxying is done by the same entity, which is
  going to be extremely challenging to deploy worldwide.

  This alone blocks us from having any kind of reliability, geographically
  distributed proxies, or pushing more traffic than ~100 Mbps.

- Currently HTTPS is not being handled at all.

- Kong and Zato do not have Nix derivations and NixOS services and should we
  want to deploy these on top of NixOS in the future, it would be very
  challenging to package them because of many assumptions that they make.

- `holo-init` Zato services presume having unrestricted access to Cloudflare
  API, while proposed alternative works with a single `CNAME` wildcard record.

- `holo-init` Zato services do not allow to change ZeroTier identity while
  keeping the same Holochain key. If ZeroTier key is lost, Holochain key
  becomes unusable for Holo hosting purposes.

- `holo-init` is designed to be run once during initialization only, which means that 
  public key to ZeroTier address mapping becomes part of precious durable state that
  we can't afford to lose.

Holo Router, on the other hand:

- allows for end-to-end encryption all the way from the client back to HoloPort

- built with operations security in mind: single HTTP endpoint that requires
  signature made with matching Holochain key, only proxies matching internal
  ZeroTier resources, doesn't require Cloudflare API access

- is flexible: allows user to change their ZeroTier address while keeping the
  same Holochain identity

- is memory safe and written in Rust

- leverages system-level DNS caching, reducing load on DNS resolver

- designed from the ground up for horizontal scaling, hundreds of proxy servers,
  distributed DNS

- records don't have to be durable: Holo Router Agent will periodically notify
  Registry about current ZeroTier IP address

[Nginx CVEs]: https://www.cvedetails.com/vulnerability-list/vendor_id-10048/product_id-17956/Nginx-Nginx.html

### How TLS is being handled?

HoloPorts can leverage their residential egress to register a [Let's Encrypt][]
account, which is the only endpoint rate limited per IP address. They can still
receive the challenge to another IP address without any limitations.

See: https://letsencrypt.org/docs/rate-limits/

See [Gateway docs](gateway/README.md). Gateway doesn't need to decrypt user
traffic thanks to [SNI][] which is sent in cleartext, which contains enough
information to dispatch request to specific HoloPort.

Gateway will eventually implement [ESNI in Split
Mode](https://tools.ietf.org/html/draft-ietf-tls-esni-04#section-5.4).

[Let's Encrypt]: https://letsencrypt.org
[SNI]: https://en.wikipedia.org/wiki/Server_Name_Indication

### Why not Cloudflare?

[Magic Transit][] doesn't work for us because GRE tunneling can't punch through
NAT without explicit setup on every router. We also don't know the price.
Additionally, we won't be able to use Magic Transit on L7, only L3, while
keeping end-to-end encryption.

[Argo Tunnel][] is prohibitively expensive at $0.1/GB, and [accounts are
limited to 1000 tunnels per account][argo-tunnel-max]. It's also not obvious if
it supports passing through end-to-end encrypted TLS traffic.

[Magic Transit]: https://www.cloudflare.com/magic-transit/
[Argo Tunnel]: https://www.cloudflare.com/products/argo-tunnel/
[argo-tunnel-max]: https://developers.cloudflare.com/argo-tunnel/faq/#what-is-the-maximum-number-of-tunnels-that-can-be-run-per-account
