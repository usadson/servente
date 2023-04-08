// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

#[cfg(feature = "tls-rustls")]
use rustls::{Certificate, PrivateKey};

#[cfg(feature = "tls-boring")]
#[derive(Clone, Debug)]
pub struct Certificate(pub Vec<u8>);

#[cfg(feature = "tls-boring")]
#[derive(Clone, Debug)]
pub struct PrivateKey(pub Vec<u8>);

#[derive(Debug)]
pub struct CertificateData {
    pub certs: Vec<Certificate>,
    pub private_key: PrivateKey,
}

#[must_use]
fn load_data_from_vec_u8(certificate: Vec<u8>, private_key: Vec<u8>) -> CertificateData {
    CertificateData {
        certs: vec![Certificate(certificate)],
        private_key: PrivateKey(private_key),
    }
}

#[must_use]
pub fn load_certificate_locations() -> CertificateData {
    let subject_alt_names = vec![
        "localhost".to_string()
    ];

    if let Ok(certificate) = std::fs::read(".servente/cert.der") {
        let private_key = std::fs::read(".servente/key.der").unwrap();
        return load_data_from_vec_u8(certificate, private_key);
    }

    let cert = rcgen::generate_simple_self_signed(subject_alt_names).unwrap();

    let certificate_data = cert.serialize_der().unwrap();
    let private_key_data = cert.serialize_private_key_der();

    _ = std::fs::create_dir(".servente");
    _ = std::fs::write(".servente/cert.der", &certificate_data);
    _ = std::fs::write(".servente/key.der", &private_key_data);

    load_data_from_vec_u8(certificate_data, private_key_data)
}
