# Holo Router

This is a new proposed infrastructure for Holo hosting routing. Resolver
(handles blacklists and tranches) is explicitly not part of scope and is left
unchanged. Similarly, components that get HoloPorts whitelisted on ZeroTier
network are not considered as part of routing infrastructure per se.

In comparison to Kong/Zato infrastructure, this design adds end-to-end
encryption all the way from the client back to HoloPort, protection against
denial-of-service attacks, operational flexibility, scalability, disaster
recovery and security.

## Components

![Components diagram](./components.svg)

### Agent

Agent sends a JSON payload of `instant` (current Unix time in milliseconds,
used to protect against replay attacks), `holochain_public_key` in `HcS`
format, and `zerotier_public_key` in hexadecimal to [Internal
DNS](#internal-dns) `POST /v1/update` endpoint. Payload is signed by Holochain
and ZeroTier keys. Signatures are specified as HTTP headers in Base64 format.

Example request:

```
HTTP POST https://internal-dns.holo.host/v1/update
X-Holochain-Signature: Rl0zgv+t2aBVHX2hrvx7OwZZnssA4n3WMp3i
X-ZeroTier-Signature: xgTafxZtsb4DzWij4mk40ONC2QlHQ1UfB+FMC

{
  "instant": 1568784840568,
  "holochain_public_key": "HcSCi4Kkx3srC3xgdy8IuhXk6mIVd7o98rKnj6nr8ODohnrooj9t9K9ufdvph5z",
  "zerotier_public_key": "a96ff4f85d7ae4fada61e769304dba12c7cf151a4fc208a170f9c9b61344f15ea79fa9c42277c19fd83d96427fc60adc1d4ddd0cdb8b4c9cd7d15c428c296870"
}
```

Endpoint is idempotent, so Agent is designed to run periodically, at the very
least on each boot. This makes loss of Internal DNS state much less of an
issue, since Agents will naturally repopulate it (subject to how often it ends
up sending requests).

### Gateway

Gateway dispatches unaltered TCP traffic by TLS SNI that is resolved using
system-wide DNS resolver, which is normally set to [Internal
DNS](#internal-dns) `POST /v1/dns-query` passed through [dnscrypt-proxy][].

For best performance, socket redirection should use [`splice(2)`][splice].

Dispatch is only allowed for hostnames that end with `.holohost.net`.

Hostname convention is `<uuid>.<provider>-<region>.gateway.holo.host`:

- `uuid` is a random UUIDv4
- `provider` is provider name in lowercase, with spaces replaced by hyphens
- `region` is provider-specific region or data center short name

Examples:

- `ebc225ff-2ccb-4fac-a09e-593532c7c9fd.maxihost-mh1.gateway.holo.host` for
  Gateway deployed to [Maxihost MH1][maxihost-mh1] São Paulo (Brazil) data
  center
- `bb3f6ab0-2fc5-40b8-9c5e-58005b35b6c7.ovh-bhs.gateway.holo.host` for Gateway
  deployed to OVH Beauharnois (Canada) region
  
  *(This uses region because OVH doesn't allow to pick specific data center
    during provisioning)*
- `af83955e-b4c2-49f7-a6d8-0763d89e097b.packet-nrt1.gateway.holo.host` for
  Gateway instance deployed to Packet NRT1 Tokyo (Japan) data center

[dnscrypt-proxy]: https://github.com/DNSCrypt/dnscrypt-proxy
[maxihost-mh1]: https://www.maxihost.com/regions/sao-paulo-mh1
[splice]: http://man7.org/linux/man-pages/man2/splice.2.html

#### How do HoloPorts terminate TLS?

HoloPorts leverage their residential connection and IP address to register a
[Let's Encrypt][letsencrypt] account, which is the only endpoint that Let's
Encrypt rate limits per IP address. Challenge is then sent to proxied IP
address without any limitations.

Gateway doesn't need to decrypt user traffic thanks to [SNI][wikipedia-sni]
that is sent in cleartext. It contains enough information to dispatch request
to a specific HoloPort.

[letsencrypt]: https://letsencrypt.org
[wikipedia-sni]: https://en.wikipedia.org/wiki/Server_Name_Indication

### Internal DNS

[DNS-over-HTTPS][wikipedia-dns-over-https] resolver and HTTP `POST /v1/update`
server implemented as two [Cloudflare Workers][cloudflare-workers].

Canonical URL is <https://internal-dns.holo.host>.

There are two endpoints:

- `POST /v1/update` is an endpoint that adds Holochain public key to internal
  ZeroTier IPv4 address pair to KV Store.

  See [Agent](#agent) docs for request docs. Response doesn't have a body.
  Status codes:

  - `200 OK` if keys and signatures are valid and entry didn't change
  - `201 Created` if keys and signatures are valid and entry didn't exist
    before or had a different IPv4 address
  - `400 Bad Request` if `instant` is older than 15 minutes from now (time
    subject to change)
  - `401 Unauthorized` if `Holochain-Public-Key` and/or `ZeroTier-Public-Key`
    HTTP header is missing
  - `403 Forbidden` if any of the following is true:
    + Holochain public key doesn't pass Holochain hashcash (if any?)
    + Holochain signature fails to validate
    + ZeroTier public key doesn't pass [ZeroTier hashcash][zerotier-identity-hashcash]
    + ZeroTier signature fails to validate

  Note that `zerotier_public_key` is sent without address: in order to derive
  address, see [ZeroTier Identity hash source code][zerotier-identity-hash].

- `POST /v1/dns-query` is a [DNS-over-HTTPS][wikipedia-dns-over-https] resolver
  endpoint.

  It accepts `A` queries, looks up KV Store populated by `POST /v1/update` and
  responds with ZeroTier IPv4 address and TTL hardcoded to 900 seconds (time
  subject to change).

  Both request and response is in `application/dns-message` DNS wire format.
  For encoder/decoder, see [dns-packet][]. Also see [RFC 8484][rfc8484].

[cloudflare-workers]: https://workers.cloudflare.com
[dns-packet]: https://github.com/mafintosh/dns-packet
[rfc8484]: https://tools.ietf.org/html/rfc8484
[wikipedia-dns-over-https]: https://en.wikipedia.org/wiki/DNS_over_HTTPS
[zerotier-identity-hash]: https://git.io/JeZaa
[zerotier-identity-hashcash]: https://git.io/JeZyl

## Alternatives

### [Cloudflare Argo Tunnel][argo-tunnel]

Prohibitively expensive at $0.1/GB, and [accounts are limited to 1000 tunnels
per account][argo-tunnel-max]. It's also not obvious if it supports passing
through end-to-end encrypted TLS traffic.

### [Cloudflare Magic Transit][magic-transit]

I believe it doesn't work for us because GRE tunneling can't punch through NAT
without explicit setup on every router. We also don't know the price.

Additionally, we won't be able to use Magic Transit on L7, only L3, while
keeping end-to-end encryption.

### Kong/Zato infrastructure

There is a number of issues with legacy infrastructure:

- Publicly accessible Zato endpoints are vulnerable to abuse:
  + `/holo-init-cloudflare-dns-create` is vulnerable to denial of service by
    sending in thousands of requests to create a specific DNS entry.
    
    It is possible, but challenging to protect against this, by adding in a
    Hashcash-like scheme that incorporates current timestamp in ms.  However,
    this proposal doesn't require Cloudflare API access at all: we can do with
    with single static wildcard CNAME record.
  + `/holo-init-proxy-route-create` does not validate ownership of public key and 
    can be used to launch social engineering attacks by claiming addresses such as 
    <http://www.hydrа.holohost.net>.

    This is a huge issue: if someone learns your public key, they can claim
    what's essentially your Holo identity before you do. There is also no way
    to change the record, since there's no ownership validation. It's either
    publicly rewritable (thankfully, it's not) or the record is set in stone,
    which means:
      * if user loses ZeroTier key, they will never be able to use Holo hosting
	with that Holochain key again
      * if ZeroTier resets ZeroTier address to IPv4 address mapping, we will
	never be able to recover

    This also allows for social engineering attacks, substituting individual
    symbols with lookalikes (including those from another script).

    This also means that this endpoint is currently vulnerable to denial of
    service, because it's cheap to send requests to create proxy routes, while
    routing table and Zato capacity is limited.

    On the other hand, this proposal requires sending and proving ownership of
    keys that are computationally hard to create, and we don't have to update
    if mapping already exists. Internal DNS is backed by high capacity
    eventually consistent key-value store that scales to millions of keys, with
    very fast reads.

  + `/holo-init-proxy-service-create` does not validate target IP address and
    as a result will act as open proxy for anyone in the internet.

- Since Zato API is not idempotent, `holo-init` can only run once during first
  boot only, which means that public key to ZeroTier address mapping becomes
  part of precious durable state that we can't afford to lose.

- Kong is a highly coupled design and doesn't scale horizontally as well as
  Holo Router. In particular, Kong clusters have to be able to reach each other
  on LAN (see [Kong clustering docs][kong-clustering]) which incurs additional
  failure modes for geodistributed deployment.
  
  Mapping of public keys to ZeroTier addresses and actual proxying
  is done through the same entity, which is going to be extremely challenging to
  deploy worldwide.

  This alone blocks us from having any kind of reliability, geographically
  distributed proxies, or pushing more traffic than ~100 Mbps.

- Both Kong and Zato are simply way too complex for what we're trying to do.
  There are way too many moving parts, and it's hard to reason about, and
  control state of.
  
  Using this stack in production capacity will require significant operational
  overhead of managing our own key-value store and horizontally scaling Zato.
  With Holo Router, we delegate what's essentially a key-value store with
  admission requirements to Cloudflare and the only part we have to scale
  ourselves is Gateways.

- Currently HTTPS is not being handled at all. Reverse proxy design will
  require double wildcard certificate issued for Kong server.

- As a result of being built on top of Nginx, Kong can't act as SNI
  peek-and-splice proxy. It can only reverse proxy, which incurs a lot of
  overhead (running Lua, parsing HTTP, copying packets to and back from
  userspace) and allows Holo to see all traffic in cleartext.

- Kong is built on top of Nginx that is not memory-safe. Most high-severity
  vulnerabilities are caused by lack of memory safety. See [Nginx
  CVEs][nginx-cves].

- Kong and Zato do not have Nix derivations and NixOS services and should we
  want to deploy these on top of NixOS in the future, and it would be
  challenging to package them.

[argo-tunnel]: https://www.cloudflare.com/products/argo-tunnel/
[argo-tunnel-max]: https://developers.cloudflare.com/argo-tunnel/faq/#what-is-the-maximum-number-of-tunnels-that-can-be-run-per-account
[kong-clustering]: https://git.io/JeZM0
[magic-transit]: https://www.cloudflare.com/magic-transit/
[nginx-cves]: https://www.cvedetails.com/vulnerability-list/vendor_id-10048/product_id-17956/Nginx-Nginx.html

## Cost of ownership

Optimizing cost of ownership was one of the driving forces behind this redesign.

Providers with metered egress are not suitable for proxying, even for alpha
launch. Just 1 Gbps of traffic, which corresponds to 10 average fully saturated
residential connections, equals to 324 TB/mo. Which on its own will cost:

- AWS: $25538.56/mo (after discount: $0.15/GB * 10TB + $0.10/GB * 40TB +
  $0.08/GB * 100TB + $0.07/GB * 100TB + $0.06/GB * 74TB)
- Argo Tunnel: $33177.60/mo
- OVH: €94.99/mo ([Infra-1][ovh-infra-1], baseline 1 Gbps, burst 2 Gbps)
- Packet: anywhere from $1658.88/mo to $16588.8/mo depending on discount

OVH is the only service on the list that offers unlimited bandwidth. We will
likely have to use a mix, especially for regions with traditionally expensive
egress (South America, Africa, Oceania).

[ovh-infra-1]: https://www.ovh.ie/dedicated_servers/infra/infra-1/

## Next steps

- We should decide which domain to use for Holo hosting. Regardless of choice,
  domain should be only used for that purpose only, for reasons listed in
  [github.io introduction][github-io].

  It will be challenging to get separate staging/development prefixes added to
  [Public Suffix List][public-suffix-list] (discouraged by upstream), so it
  will probably be one domain for all networks.

  Currently, `holohost.net` is roughly used for hosting only: there are a few
  resources (proxy, resolver) that can be painlessly moved over to `holo.host`.
  This document proceeds with that choice in mind.

- We need to add `holohost.net` to [Public Suffix List][public-suffix-list] in
  order to lift Let's Encrypt limit of [50 certificates per Registered Domain
  per week][letsencrypt-certificates-per-registered-domain].

- We should set up `CNAME *` Gateway DNS records for Gateway
  deployments. There is no need to change Cloudflare DNS for each node anymore.

  Geodistribution is out of scope for now, see [Future work](#future-work)
  section.

[github-io]: https://github.blog/2013-04-05-new-github-pages-domain-github-io/

## Future work

This contains items that are explicitly out of scope for this proposal, and
describe various ways this work can be extended further.

- It would be nice to eventually support IPv6 in Internal DNS.

- This proposal doesn't authenticate access to Internal DNS, which means that
  anyone can map Holochain public keys to internal ZeroTier IPv4 addresses.
  This can be used for selective denial-of-service, so I believe it would be
  best to authenticate access to the service with a secret query parameter
  (we're limited to whatever [DNS Stamps format][dns-stamps] supports).

- Hosts don't check that it's Gateway that makes the request, which allows any
  network member to check Holochain key against a HoloPort, and potentially,
  cause selective denial of service, or circumvention of authorization flows.

  Legacy infrastructure is also vulnerable to this.

- We should geographically distribute Gateways and do status checks to filter
  out unresponsive nodes. We're mostly constrained to anycast and DNS for this,
  with the latter being more flexible and more native to our current stack.

- Gateway should handle [ESNI][draft-ietf-tls-esni-04].

  At least some support should land in [Rustls][rustls] first. At minimum, ESNI
  variant should be added to [`ServerNamePayload`][rustls-server-name-payload]
  enum, but that will have us implement ESNI decryption in Gateway itself.

  By design, ESNI will have to be terminated on Gateway, while letting TLS pass
  through unterminated.

- Gateway is well-placed to attempt an active man-in-the-middle attack: it can
  pass ACME HTTP challenge, get certificate signed by CA, and transparently
  re-encrypt all of traffic.
  
  Long-term, if we keep TLS certificates signed by certificate authorities, we
  should create an auditable, tamper-evident log of certificates issued for
  HoloPorts.

  First line of defense is to set up a [CAA record][dns-caa-record] that
  restricts CA to Let's Encrypt only. This would require attacker to use that
  specific certificate authority.

  Part that requires much more work is setting up something a lot like
  [Certificate Transparency][certificate-transparency] or [Key
  Transparency][key-transparency]: an append-only, tamper-evident log of
  certificates issued for `holohost.net` addresses, with each entry signed by
  Holochain key. 

  HoloPort will send TLS certificate hash signed by Holochain key that matches
  domain name it is issued for to [Trillian][trillian] personality that will
  verify Holochain signature and commit payload to verifiable append-only log.

  There can be several Trillian deployments, some potentially controlled by
  third parties. HoloPort can try to send the same payload to all of them.
  We're good as long as any single one is online and doesn't attempt a
  split-view attack.

  Next, we will monitor Certificate Transparency logs, and when we see
  certificates issued for `holohost.net`, we will cross-reference these
  certificates with our transparency log, and react if our transparency log
  doesn't include the same certificate.

  Anyone else will be able to similarly validate and cross-reference two
  transparency logs.

- Since TLS requires certificate authorities to function and we communicate
  public Holochain key out-of-band, we can actually establish Diffie-Hellman
  key exchange during WebSocket session.
  
  This will alleviate the need for transparency log in the first place, because
  connections will effectively pin hostname Holochain public key.

  This implies a custom transport encryption scheme not handled by the browser.
  If used without HTTPS layered on top, metadata will be sent in cleartext
  (HTTP headers, SNI).

  This will either require significant changes to Holochain or a WebSocket
  proxy running on HoloPort that will wrap traffic in transport encryption
  protocol. Plus, transport protocol will have to be implemented on the client.

- ZeroTier may be replaced in the future for stability reasons (ask PJ Klimek).
  Long-term, Gateway can establish peer-to-peer connections directly, provided
  that peer-to-peer traffic can punch holes in NAT. For prior art, see
  [Crust][crust].

- This proposal paves the way for untrusted third-parties running their own
  Gateways, if desired. However, making Holochain key to ZeroTier address
  mapping public may be undesirable. A number of things will have to be
  addressed, especially transparency (see above) and removal of malicious or
  poorly connected nodes.

  Note that removing Gateway automatically as a result of failed transparency
  check can lead to denial-of-service attacks.

- Agents sign a payload with current Unix timestamp in order to prevent replay
  attacks, e.g. if ZeroTier resets its IPv4 mapping. This information can also
  be potentially used to filter out inactive HoloPorts.

[certificate-transparency]: https://www.certificate-transparency.org
[crust]: https://github.com/maidsafe/crust
[dns-caa-record]: https://support.cloudflare.com/hc/en-us/articles/115000310792-Configuring-CAA-Records-
[dns-stamps]: https://dnscrypt.info/stamps/
[draft-ietf-tls-esni-04]: https://tools.ietf.org/html/draft-ietf-tls-esni-04
[key-transparency]: https://github.com/google/keytransparency
[letsencrypt-certificates-per-registered-domain]: https://letsencrypt.org/docs/rate-limits/#certificates-per-registered-domain
[public-suffix-list]: https://github.com/publicsuffix/list
[rustls]: https://github.com/ctz/rustls
[rustls-server-name-payload]: https://docs.rs/rustls/0.15.1/rustls/internal/msgs/handshake/enum.ServerNamePayload.html
[trillian]: https://github.com/google/trillian
