# TLS trust is user-configurable, in two deliberate steps

Snapdash validated Home Assistant's certificate against the bundled webpki roots and offered no way to trust anything else.
For a public hostname that is correct; for the LAN setups Home Assistant actually lives on it is a dead end, and issue #102 is what it looks like from the outside: `invalid peer certificate: Other(OtherError(CaUsedAsEndEntity))` and no way forward.
We have decided to make the trust configurable, as exactly two options owned by `ha::tls` and carried inside `HaConnectionConfig`.

## The two options, and why two

`ca_file` trusts a PEM bundle of the user's own root CAs in addition to the built-in roots.
This is the correct fix for a properly run home CA, and it keeps full validation: hostname, expiry, and chain are all still checked, just against one more root.

`accept_invalid_certs` switches certificate validation off entirely, behind a toggle that Settings presents with a standing warning rather than a transient status message.
It exists because a CA file cannot fix a certificate that is broken *as a certificate*.
The certificate from issue #102 is the default output of `openssl req -x509`, which OpenSSL 3 stamps with `basicConstraints = critical, CA:TRUE`.
webpki rejects a CA certificate presented in the server slot no matter which store trusts it, so for the most common self-signed setup in the wild the trusted-CA option is provably not enough - `trusting_the_issue_102_certificate_does_not_fix_it` in `ha::tls::tests` is that proof.

One option was not enough and three would be a lie: there is no middle trust between "validate against these roots" and "do not validate" that rustls can actually deliver.

## Both clients are configured from one module

The REST client and the WebSocket take different TLS plumbing (reqwest's builder versus a rustls `ClientConfig` handed to tungstenite), which is exactly how the two could drift apart.
Both are built in `ha::tls` from the same `TlsOptions`, so a trust decision the user makes is made once.

The options live inside `HaConnectionConfig`, which the WS subscription is keyed on.
Changing either field therefore re-keys the subscription and reconnects with the new trust, with no reconnect logic added anywhere.

## What is given up when validation is off

The connection is still encrypted, and handshake signatures are still verified against whatever key the peer presented.
What is gone is knowing whose key that is: anyone who can answer on the configured URL can present any certificate and receive the long-lived token.
That is why the toggle's warning is permanent UI next to the setting rather than a log line, and why `accept_invalid_certs` deliberately overrides `ca_file` instead of combining with it - a user who has switched validation off has no residual trust for a CA file to add.

## Consequences

A misconfigured `ca_file` (missing, unreadable, empty of certificates) fails the connection with an error naming the file, never a silent fall-back to the built-in roots.
The webpki roots stay bundled; the system trust store remains unread, so a CA installed in the OS keychain still needs to be named in `ca_file`.
Untouched configurations take the exact code path they took before this decision: `ws_connector` answers `None` and tungstenite builds its own stock connector.
