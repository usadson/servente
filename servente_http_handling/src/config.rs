// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use std::time::Duration;

#[derive(Clone)]
pub struct ServenteConfig {
    #[cfg(feature = "rustls")]
    pub tls_config: std::sync::Arc<rustls::ServerConfig>,

    #[cfg(feature = "tls-boring")]
    pub tls_config: boring::ssl::SslAcceptor,

    pub settings: ServenteSettings,
}

impl ServenteConfig {

    pub fn new(settings: ServenteSettings) -> Self {
        Self {
            #[cfg(feature = "rustls")]
            tls_config: std::sync::Arc::new(create_tls_config_rustls()),

            #[cfg(feature = "tls-boring")]
            tls_config: create_tls_config_boring(),

            settings
        }
    }

}

unsafe impl Send for ServenteConfig {}
unsafe impl Sync for ServenteConfig {}

#[derive(Clone)]
pub struct ServenteSettings {
    pub handler_controller: crate::handler::HandlerController,

    /// If the client doesn't transmit the full request-line and headers within
    /// this time, the request is terminated.
    pub read_headers_timeout: Duration,

    /// If the client doesn't transmit the full body within
    /// this time, the request is terminated.
    pub read_body_timeout: Duration,
}

#[cfg(feature = "tls-boring")]
fn create_tls_config_boring() -> boring::ssl::SslAcceptor {
    use boring::ssl;

    let cert_data = servente_self_signed_cert::load_certificate_locations();

    let private_key = boring::pkey::PKey::private_key_from_pkcs8(&cert_data.private_key.0)
        .expect("Failed to load PKCS#8 DER private key");

    let server = ssl::SslMethod::tls_server();
    let mut ssl_builder = boring::ssl::SslAcceptor::mozilla_modern(server)
        .expect("Failed to setup BoringSSL with Mozilla Modern Configuration");

    ssl_builder.set_default_verify_paths().expect("Failed to set default verify paths");
    ssl_builder.set_verify(ssl::SslVerifyMode::NONE);
    ssl_builder.enable_ocsp_stapling();
    ssl_builder.set_alpn_protos(&determine_alpn_protocols_boring()).expect("Failed to set ALPN protocols");
    ssl_builder.set_private_key(&private_key).expect("Failed to set the private key");

    let mut certs = cert_data.certs.iter()
        .map(|der_data| {
            boring::x509::X509::from_der(&der_data.0).unwrap()
        });

    ssl_builder.set_certificate(&certs.next().unwrap())
            .expect("Failed to set the certificate");

    for cert in certs {
        ssl_builder.add_extra_chain_cert(cert)
            .expect("Failed to append chain certificates");
    }

    ssl_builder.build()
}

#[cfg(feature = "rustls")]
fn create_tls_config_rustls() -> rustls::ServerConfig {
    let cert_data = servente_self_signed_cert::load_certificate_locations();

    let mut tls_config = rustls::ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(cert_data.certs, cert_data.private_key)
        .expect("Failed to build rustls configuration!");

    // https://www.iana.org/assignments/tls-extensiontype-values/tls-extensiontype-values.xhtml#alpn-protocol-ids
    tls_config.alpn_protocols = determine_alpn_protocols().iter().map(|str| str.as_bytes().to_owned()).collect();
    tls_config.send_half_rtt_data = true;

    #[cfg(feature = "ktls")]
    {
        tls_config.enable_secret_extraction = true;
    }

    tls_config
}

const fn determine_alpn_protocols() -> &'static [&'static str] {
    &[
        #[cfg(feature = "http3")]
        "h3",

        #[cfg(feature = "http2")]
        "h2",

        "http/1.1",
    ]
}

#[cfg(feature = "tls-boring")]
fn determine_alpn_protocols_boring() -> Vec<u8> {
    let protocols = determine_alpn_protocols();
    let length = protocols.iter()
        .map(|str| 1 + str.len())
        .sum();

    let mut result = Vec::with_capacity(length);
    for str in protocols {
        result.push(str.len() as u8);
        result.extend_from_slice(str.as_bytes());
    }

    debug_assert_eq!(result.len(), length);
    result
}
