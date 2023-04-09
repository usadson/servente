// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use std::{time::Duration, sync::Arc};

use crate::Middleware;

#[derive(Clone)]
pub struct ServenteConfig {
    #[cfg(feature = "rustls")]
    pub tls_config: std::sync::Arc<rustls::ServerConfig>,

    #[cfg(feature = "tls-boring")]
    pub tls_config: boring::ssl::SslAcceptor,

    pub settings: ServenteSettings,
}

impl ServenteConfig {
    pub fn new() -> ServenteConfigBuilder<&'static [&'static str]> {
        ServenteConfigBuilder {
            alpn_list: determine_alpn_protocols()
        }
    }
}

pub struct ServenteConfigBuilder<T> {
    alpn_list: T,
}

impl<T> ServenteConfigBuilder<T>
        where T: AsRef<[&'static str]> {
    pub fn build(self, settings: ServenteSettings) -> ServenteConfig {
        #[cfg(not(any(feature = "rustls", feature = "tls-boring")))]
        { _ = self.alpn_list }

        ServenteConfig {
            #[cfg(feature = "rustls")]
            tls_config: std::sync::Arc::new(create_tls_config_rustls(self.alpn_list.as_ref())),

            #[cfg(feature = "tls-boring")]
            tls_config: create_tls_config_boring(self.alpn_list.as_ref()),

            settings,
        }
    }

    pub fn with_alpn_list<U>(self, list: U) -> ServenteConfigBuilder<U> {
        ServenteConfigBuilder::<U> {
            alpn_list: list,
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

    /// TODO: file serving should become middleware too.
    pub middleware: Vec<Arc<dyn Middleware>>,
}

impl ServenteSettings {
    pub fn new(handler_controller: crate::handler::HandlerController) -> Self {
        Self {
            handler_controller,
            read_headers_timeout: Duration::from_secs(10),
            read_body_timeout: Duration::from_secs(60),
            middleware: Vec::new()
        }
    }
}

#[cfg(feature = "tls-boring")]
fn create_tls_config_boring(alpn_list: &[&'static str]) -> boring::ssl::SslAcceptor {
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
    ssl_builder.set_alpn_protos(&determine_alpn_protocols_boring(alpn_list)).expect("Failed to set ALPN protocols");
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
fn create_tls_config_rustls(alpn_list: &[&'static str]) -> rustls::ServerConfig {
    let cert_data = servente_self_signed_cert::load_certificate_locations();

    let mut tls_config = rustls::ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(cert_data.certs, cert_data.private_key)
        .expect("Failed to build rustls configuration!");

    // https://www.iana.org/assignments/tls-extensiontype-values/tls-extensiontype-values.xhtml#alpn-protocol-ids
    tls_config.alpn_protocols = alpn_list.iter().map(|str| str.as_bytes().to_owned()).collect();
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
fn determine_alpn_protocols_boring(protocols: &[&'static str]) -> Vec<u8> {
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
