//! User-configurable TLS trust for the Home Assistant connection (#102).
//!
//! Out of the box both clients validate HA's certificate against the
//! bundled webpki roots, which is right for a public hostname and wrong
//! for most home setups: a LAN instance typically carries a certificate
//! from a home-grown CA, or a self-signed one no public root will ever
//! bless. Two escape hatches, in increasing order of bluntness:
//!
//! - [`TlsOptions::ca_file`] trusts the user's own CA bundle (PEM) in
//!   addition to the built-in roots. The right fix for a properly
//!   issued home CA.
//! - [`TlsOptions::accept_invalid_certs`] switches certificate
//!   validation off entirely. It exists because a CA file cannot fix a
//!   certificate that is broken *as a certificate*: the classic
//!   `openssl req -x509` self-signed cert carries `CA:TRUE` and is
//!   rejected as a leaf (`CaUsedAsEndEntity`) no matter which store
//!   trusts it. The connection stays encrypted but the peer is no
//!   longer authenticated, so the UI keeps this behind an explicit
//!   danger toggle.
//!
//! Both clients - reqwest for REST, tungstenite for the WebSocket - are
//! configured from here, so they can never disagree about what to
//! trust.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use rustls_pki_types::pem::PemObject;
use rustls_pki_types::{CertificateDer, ServerName, UnixTime};

/// What the user asked the connection to trust. Carried inside
/// [`crate::ha::HaConnectionConfig`], so changing either field re-keys
/// the WS subscription and reconnects with the new trust.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct TlsOptions {
    /// Extra root CAs (a PEM bundle) trusted alongside the webpki roots.
    pub ca_file: Option<PathBuf>,
    /// Skip certificate validation entirely. Overrides `ca_file`.
    pub accept_invalid_certs: bool,
}

/// One process-wide HTTP client per [`TlsOptions`], so service calls
/// reuse a pooled keep-alive connection.
///
/// Previously every call constructed `reqwest::Client::new()`, which
/// builds a fresh connection pool and therefore paid for a new TCP and
/// TLS handshake per tap. `Client` is internally `Arc`'d and explicitly
/// designed to be reused; the cache re-keys only when the user changes
/// the TLS settings, which tears down the old pool with the old trust.
static HTTP: Mutex<Option<(TlsOptions, reqwest::Client)>> = Mutex::new(None);

/// The shared REST client, built with the given trust.
///
/// Fails when `ca_file` is set but unreadable or not valid PEM - the
/// caller surfaces that, because a trust the user asked for and did not
/// get must never silently degrade to "connect anyway".
pub fn http_client(tls: &TlsOptions) -> Result<reqwest::Client> {
    let mut cached = HTTP.lock().expect("HTTP client cache poisoned");

    if let Some((key, client)) = cached.as_ref()
        && key == tls
    {
        return Ok(client.clone());
    }

    let mut builder = reqwest::Client::builder();

    if let Some(path) = &tls.ca_file {
        // reqwest wants its own `Certificate` type, so the bundle is
        // handed over as PEM bytes rather than through `read_ca_bundle`.
        // Both parses see the same file; a bundle one accepts and the
        // other rejects fails the build here, before anything connects.
        let pem = std::fs::read(path)
            .with_context(|| format!("cannot read CA bundle {}", path.display()))?;
        let certs = reqwest::Certificate::from_pem_bundle(&pem)
            .with_context(|| format!("invalid PEM in CA bundle {}", path.display()))?;
        // reqwest parses a PEM-free file to zero certificates without
        // complaint; `read_ca_bundle` documents why that must not pass.
        anyhow::ensure!(
            !certs.is_empty(),
            "no certificates found in {}",
            path.display()
        );
        for cert in certs {
            builder = builder.add_root_certificate(cert);
        }
    }

    if tls.accept_invalid_certs {
        builder = builder.danger_accept_invalid_certs(true);
    }

    let client = builder.build().context("failed to build the HTTP client")?;
    *cached = Some((tls.clone(), client.clone()));
    Ok(client)
}

/// The WebSocket connector for the given trust, or `None` for the
/// default one (webpki roots, full validation) so untouched setups keep
/// taking tungstenite's own path.
pub fn ws_connector(tls: &TlsOptions) -> Result<Option<tokio_tungstenite::Connector>> {
    // Built with an explicit provider rather than `ClientConfig::builder()`.
    // The implicit builder picks "the one enabled provider feature" and
    // panics at runtime if some other crate in the tree enables the
    // second one - a trap this decides not to be near.
    let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
    let builder = rustls::ClientConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()
        .context("TLS protocol setup")?;

    let config = if tls.accept_invalid_certs {
        builder
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(AcceptAnyServerCert { provider }))
            .with_no_client_auth()
    } else if let Some(path) = &tls.ca_file {
        let mut roots = rustls::RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        for cert in read_ca_bundle(path)? {
            roots
                .add(cert)
                .with_context(|| format!("unusable CA certificate in {}", path.display()))?;
        }
        builder.with_root_certificates(roots).with_no_client_auth()
    } else {
        return Ok(None);
    };

    Ok(Some(tokio_tungstenite::Connector::Rustls(Arc::new(config))))
}

fn read_ca_bundle(path: &Path) -> Result<Vec<CertificateDer<'static>>> {
    let certs: Vec<CertificateDer<'static>> = CertificateDer::pem_file_iter(path)
        .with_context(|| format!("cannot read CA bundle {}", path.display()))?
        .collect::<Result<_, _>>()
        .with_context(|| format!("invalid PEM in CA bundle {}", path.display()))?;

    // An empty bundle parses fine and would silently change nothing -
    // the one outcome worse than an error for a user debugging trust.
    anyhow::ensure!(
        !certs.is_empty(),
        "no certificates found in {}",
        path.display()
    );

    Ok(certs)
}

/// The verifier behind [`TlsOptions::accept_invalid_certs`]: any
/// certificate chain, any hostname.
///
/// Handshake signatures are still verified - that costs nothing and
/// keeps the session cryptographically tied to whatever key the peer
/// presented. What is given up is knowing *whose* key that is.
#[derive(Debug)]
struct AcceptAnyServerCert {
    provider: Arc<rustls::crypto::CryptoProvider>,
}

impl rustls::client::danger::ServerCertVerifier for AcceptAnyServerCert {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

#[cfg(test)]
mod tests;
