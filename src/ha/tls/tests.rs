//! The trust decisions of `ha::tls`, exercised against a real TLS
//! server rather than asserted on: each handshake test below binds a
//! tokio-rustls listener on a loopback port, hands the client the
//! `ClientConfig` that [`ws_connector`] actually builds, and watches
//! the handshake succeed or fail.
//!
//! The fixtures were generated with OpenSSL 3 (`openssl req -x509 ...`,
//! valid until 2046) and are test keys with no life outside this file:
//!
//! - `BROKEN_*`: the issue #102 certificate. A plain self-signed
//!   `openssl req -x509` cert, which OpenSSL 3 stamps with
//!   `basicConstraints = critical, CA:TRUE` by default. Presented as a
//!   server certificate, webpki rejects it as `CaUsedAsEndEntity` - no
//!   root store can fix that.
//! - `HOME_CA_CERT` and `LEAF_*`: a properly run home CA. The leaf is
//!   `CA:FALSE`, carries `subjectAltName=DNS:localhost`, and is signed
//!   by the CA - the setup `TlsOptions::ca_file` exists for.

use std::sync::Arc;

use rustls_pki_types::pem::PemObject;
use rustls_pki_types::{CertificateDer, PrivateKeyDer, ServerName};

use super::{TlsOptions, http_client, ws_connector};

const BROKEN_CERT: &[u8] = b"-----BEGIN CERTIFICATE-----
MIIBmTCCAT+gAwIBAgIUCQtjtNhb8DVtxLRq2oS8AnmeOwkwCgYIKoZIzj0EAwIw
FDESMBAGA1UEAwwJbG9jYWxob3N0MB4XDTI2MDkxNTA4MzY1OVoXDTQ2MDkxMDA4
MzY1OVowFDESMBAGA1UEAwwJbG9jYWxob3N0MFkwEwYHKoZIzj0CAQYIKoZIzj0D
AQcDQgAE17W2peXJkYh9Zq1L5fc4hbE5wxTzG7av4F2S38oIchXWb19CoQOxUcX/
GJP9OHtQtj9/d+NcbfqVFI6Nd9K/uaNvMG0wHQYDVR0OBBYEFJEh2bOBE3MXF5JB
T8naSG3NZBUmMB8GA1UdIwQYMBaAFJEh2bOBE3MXF5JBT8naSG3NZBUmMA8GA1Ud
EwEB/wQFMAMBAf8wGgYDVR0RBBMwEYIJbG9jYWxob3N0hwR/AAABMAoGCCqGSM49
BAMCA0gAMEUCIHteTvZCkwx99eRPRFT1cCpx+GW4RpPU1KPos5N3zh3fAiEA5M+1
LuygiOaax/88++SkkDP2zH/2Pq+THXb61Id8wf0=
-----END CERTIFICATE-----
";

const BROKEN_KEY: &[u8] = b"-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgKfkWh+jOgFtFxq5Q
FjBFD2llqDYRSJAZdnA24njlctKhRANCAATXtbal5cmRiH1mrUvl9ziFsTnDFPMb
tq/gXZLfyghyFdZvX0KhA7FRxf8Yk/04e1C2P39341xt+pUUjo130r+5
-----END PRIVATE KEY-----
";

const HOME_CA_CERT: &[u8] = b"-----BEGIN CERTIFICATE-----
MIIBlDCCATugAwIBAgIUC45u1HMB07cYoqbJr+Cjovx2ShUwCgYIKoZIzj0EAwIw
IDEeMBwGA1UEAwwVU25hcGRhc2ggVGVzdCBIb21lIENBMB4XDTI2MDkxNTA4MzY1
OVoXDTQ2MDkxMDA4MzY1OVowIDEeMBwGA1UEAwwVU25hcGRhc2ggVGVzdCBIb21l
IENBMFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEQorQTnbP+uKy4pZGKitPzp8v
rP56dgzgLvkJKyRcdO+F+CmpJqmk/9qICLnPyKG4nGk9n0dbQMayKQ/VR5u6DKNT
MFEwHQYDVR0OBBYEFADBclpzvZgdnmM0MlTa2pwQ+uzIMB8GA1UdIwQYMBaAFADB
clpzvZgdnmM0MlTa2pwQ+uzIMA8GA1UdEwEB/wQFMAMBAf8wCgYIKoZIzj0EAwID
RwAwRAIgIJSjWOtNTUkll5b7m3LRWudWA+Eb9rzCKyJYjH44u5ACIDt2v1To6l7b
NpfVbayv42+3d28kWtA0WFJN/mMdMN2x
-----END CERTIFICATE-----
";

const LEAF_CERT: &[u8] = b"-----BEGIN CERTIFICATE-----
MIIBojCCAUigAwIBAgIUIQu+iKcKBqO6s80SMl6uVt+Oc9owCgYIKoZIzj0EAwIw
IDEeMBwGA1UEAwwVU25hcGRhc2ggVGVzdCBIb21lIENBMB4XDTI2MDkxNTA4MzY1
OVoXDTQ2MDkxMDA4MzY1OVowFDESMBAGA1UEAwwJbG9jYWxob3N0MFkwEwYHKoZI
zj0CAQYIKoZIzj0DAQcDQgAEodah8b/5mEsDrBcZ/Q9sBo0E4TywmXr5jo+LLbiZ
YwvFGdnDybWO09BovGaTCnSscV8CE70FYE+yxu/2As4dVaNsMGowDAYDVR0TAQH/
BAIwADAaBgNVHREEEzARgglsb2NhbGhvc3SHBH8AAAEwHQYDVR0OBBYEFBwnHd+g
NHzRJfPBIWtexi+zOuXcMB8GA1UdIwQYMBaAFADBclpzvZgdnmM0MlTa2pwQ+uzI
MAoGCCqGSM49BAMCA0gAMEUCIHeSdWo/v3VG6S+LTcNR1YZtQZ3I81GfcMYVH3AY
IGV0AiEAwDF2CpdHjkALAJCnBtveMz2R0d8QHTxlttnAhDc53AE=
-----END CERTIFICATE-----
";

const LEAF_KEY: &[u8] = b"-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgZToFZ/TCcguNjpjD
JoZPSP6JX4HiEH83eOjKeCwWiuyhRANCAASh1qHxv/mYSwOsFxn9D2wGjQThPLCZ
evmOj4stuJljC8UZ2cPJtY7T0Gi8ZpMKdKxxXwITvQVgT7LG7/YCzh1V
-----END PRIVATE KEY-----
";

/// A PEM bundle as a file on disk, the way `TlsOptions::ca_file` names
/// one.
fn pem_file(pem: &[u8]) -> tempfile::NamedTempFile {
    let file = tempfile::NamedTempFile::new().expect("temp file");
    std::fs::write(file.path(), pem).expect("write PEM");
    file
}

fn options_with_ca(pem: &[u8]) -> (TlsOptions, tempfile::NamedTempFile) {
    let file = pem_file(pem);
    let options = TlsOptions {
        ca_file: Some(file.path().to_path_buf()),
        accept_invalid_certs: false,
    };
    (options, file)
}

/// One full TLS handshake on loopback: a tokio-rustls server presenting
/// `cert_pem`/`key_pem`, against the client config [`ws_connector`]
/// builds for `tls`. Returns the client's verdict.
async fn handshake(tls: &TlsOptions, cert_pem: &[u8], key_pem: &[u8]) -> Result<(), String> {
    let certs: Vec<CertificateDer<'static>> = CertificateDer::pem_slice_iter(cert_pem)
        .collect::<Result<_, _>>()
        .expect("server certificate parses");
    let key = PrivateKeyDer::from_pem_slice(key_pem).expect("server key parses");

    let server_config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .expect("server protocol setup")
    .with_no_client_auth()
    .with_single_cert(certs, key)
    .expect("server certificate usable");

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback");
    let addr = listener.local_addr().expect("local addr");
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(server_config));

    // The server's own outcome is deliberately ignored: a client that
    // rejects the certificate closes the socket mid-handshake, which
    // surfaces server-side as an unremarkable IO error.
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let _ = acceptor.accept(stream).await;
    });

    let client_config = match ws_connector(tls).expect("connector builds") {
        Some(tokio_tungstenite::Connector::Rustls(config)) => config,
        other => panic!("expected a rustls connector, got {:?}", other.is_some()),
    };

    let tcp = tokio::net::TcpStream::connect(addr).await.expect("connect");
    let result = tokio_rustls::TlsConnector::from(client_config)
        .connect(ServerName::try_from("localhost").expect("name"), tcp)
        .await;

    server.await.expect("server task");
    result.map(|_| ()).map_err(|e| e.to_string())
}

/// Untouched options mean tungstenite's stock connector - webpki roots,
/// full validation - so default setups keep the code path they had
/// before this module existed.
#[test]
fn default_options_use_the_stock_connector() {
    assert!(
        ws_connector(&TlsOptions::default())
            .expect("builds")
            .is_none()
    );
}

/// The issue #102 reproduction. The certificate is broken *as a
/// certificate* - `CA:TRUE` on a leaf - so even a client that trusts a
/// CA of its own rejects it, with exactly the error from the user's
/// log.
#[tokio::test]
async fn the_issue_102_certificate_is_rejected_by_a_validating_client() {
    let (options, _file) = options_with_ca(HOME_CA_CERT);

    let err = handshake(&options, BROKEN_CERT, BROKEN_KEY)
        .await
        .expect_err("a CA certificate must not pass as a server certificate");

    assert!(
        err.contains("CaUsedAsEndEntity"),
        "expected CaUsedAsEndEntity, got: {err}"
    );
}

/// And trusting the broken certificate itself does not help - it goes
/// into the root store fine (it *is* a CA, says so itself) and is then
/// rejected in the server slot all the same. This is the case the
/// danger toggle exists for.
#[tokio::test]
async fn trusting_the_issue_102_certificate_does_not_fix_it() {
    let (options, _file) = options_with_ca(BROKEN_CERT);

    let err = handshake(&options, BROKEN_CERT, BROKEN_KEY)
        .await
        .expect_err("no root store can bless a CA:TRUE leaf");

    assert!(
        err.contains("CaUsedAsEndEntity"),
        "expected CaUsedAsEndEntity, got: {err}"
    );
}

/// The danger toggle connects to it.
#[tokio::test]
async fn the_danger_toggle_accepts_the_issue_102_certificate() {
    let options = TlsOptions {
        ca_file: None,
        accept_invalid_certs: true,
    };

    handshake(&options, BROKEN_CERT, BROKEN_KEY)
        .await
        .expect("accept_invalid_certs connects to any certificate");
}

/// The right fix for a properly run home CA: hand Snapdash the CA
/// bundle and a leaf it signed validates normally.
#[tokio::test]
async fn a_home_ca_bundle_trusts_a_leaf_it_signed() {
    let (options, _file) = options_with_ca(HOME_CA_CERT);

    handshake(&options, LEAF_CERT, LEAF_KEY)
        .await
        .expect("a leaf signed by the trusted CA validates");
}

/// Without the bundle the same leaf is rejected - the CA file is doing
/// the trusting, not some accident of the test setup.
#[tokio::test]
async fn the_home_ca_leaf_is_untrusted_without_the_bundle() {
    // A validating client that trusts an unrelated CA, because the
    // default options short-circuit to the stock connector and build
    // no config to hand the test server.
    let (options, _file) = options_with_ca(BROKEN_CERT);

    let err = handshake(&options, LEAF_CERT, LEAF_KEY)
        .await
        .expect_err("an unknown issuer must not validate");

    assert!(
        err.contains("UnknownIssuer"),
        "expected UnknownIssuer, got: {err}"
    );
}

/// A trust the user asked for and cannot have must be an error, never a
/// silent downgrade - for both clients, which share the options but not
/// the parser.
#[test]
fn a_missing_ca_file_fails_both_clients() {
    let options = TlsOptions {
        ca_file: Some("/nonexistent/ca.pem".into()),
        accept_invalid_certs: false,
    };

    assert!(ws_connector(&options).is_err());
    assert!(http_client(&options).is_err());
}

/// An empty or non-PEM bundle parses to zero certificates, which would
/// change nothing while looking configured.
#[test]
fn a_bundle_with_no_certificates_fails_both_clients() {
    let file = pem_file(b"this is not a certificate\n");
    let options = TlsOptions {
        ca_file: Some(file.path().to_path_buf()),
        accept_invalid_certs: false,
    };

    assert!(ws_connector(&options).is_err());
    assert!(http_client(&options).is_err());
}

/// The reqwest side of the same trust: a valid bundle builds, and the
/// danger toggle builds without one.
#[test]
fn the_http_client_builds_for_every_valid_option_shape() {
    assert!(http_client(&TlsOptions::default()).is_ok());

    let (with_ca, _file) = options_with_ca(HOME_CA_CERT);
    assert!(http_client(&with_ca).is_ok());

    let insecure = TlsOptions {
        ca_file: None,
        accept_invalid_certs: true,
    };
    assert!(http_client(&insecure).is_ok());
}
