// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use anyhow::bail;
use rustls::client::{ServerCertVerifier, ServerCertVerified};
use tokio::io::AsyncReadExt;

/// A [`rustls::client::ServerCertVerifier`] which trusts all certificates.
pub struct UntrustedCertificateServerCertVerifier {
}

impl ServerCertVerifier for UntrustedCertificateServerCertVerifier {
    fn verify_server_cert(&self,
            _: &rustls::Certificate,
            _: &[rustls::Certificate],
            _: &rustls::ServerName,
            _: &mut dyn Iterator<Item = &[u8]>,
            _: &[u8],
            _:std::time::SystemTime,
        ) -> Result<rustls::client::ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }
}

pub async fn read_to_crlf<R>(reader: &mut R) -> anyhow::Result<String>
        where R: AsyncReadExt + Unpin {
    let mut data = Vec::new();

    let mut was_carriage_return = false;
    loop {
        let byte = reader.read_u8().await?;

        if was_carriage_return {
            if byte != b'\n' {
                bail!("CR not followed by LN: {byte}");
            }

            break;
        }

        if byte == b'\r' {
            was_carriage_return = true;
        } else {
            data.push(byte);
        }
    }

    Ok(String::from_utf8(data)?)
}
