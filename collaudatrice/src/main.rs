// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use std::{sync::Arc, time::Instant};

use analyze::ServerAnalysis;
use clap::Parser;
use owo_colors::OwoColorize;

use crate::io::UntrustedCertificateServerCertVerifier;

mod analyze;
mod io;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
   /// The host to connect to.
   #[arg(long)]
   host: String,

   /// The port number to connect to.
   #[arg(short, long, default_value_t = 443)]
   port: u16,
}

#[derive(Debug)]
pub struct Configuration {
    pub args: Args,
    pub rustls_client_config: Arc<rustls::ClientConfig>,
    pub allow_untrusted_certificates: bool,
}

impl Configuration {
    pub fn new(args: Args) -> anyhow::Result<Self> {
        let mut roots = rustls::RootCertStore::empty();
        for cert in rustls_native_certs::load_native_certs()? {
            roots
                .add(&rustls::Certificate(cert.0))
                .unwrap();
        }

        let rustls_client_config = rustls::ClientConfig::builder()
                .with_safe_defaults()
                .with_root_certificates(roots)
                .with_no_client_auth();

        Ok(Self {
            args,
            rustls_client_config: Arc::new(rustls_client_config),
            allow_untrusted_certificates: false,
        })
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let start = Instant::now();
    println!("+=== collaudatrice ===+");

    let mut config = Configuration::new(Args::parse())?;

    println!("{} {} {}:{}", "I:".blue(), "Analyzing basic server information on".yellow(),
        config.args.host, config.args.port);

    let analysis = run_analysis(&mut config).await?;

    println!("{} {}", "II:".blue(), "Indexing server");
    // TODO
    _ = analysis;

    println!();
    println!("Finished in {} seconds", start.elapsed().as_secs_f64());

    Ok(())
}

fn print_analysis(analysis: &ServerAnalysis) {
    println!("  {}: {}", "Server Name".green(), analysis.server_product_name);
    print!("  {}: ", "IPv4 Address".green());
    match analysis.ipv4_address {
        Some(address) => println!("{}", address),
        None => println!("{}", "MISSING / NOT RESOLVED BY DNS".red().bold())
    };

    print!("  {}: ", "IPv6 Address".green());
    match analysis.ipv6_address {
        Some(address) => println!("{}", address),
        None => println!("{}", "MISSING / NOT RESOLVED BY DNS".red().bold())
    };

    println!();
}

/// Run the analysis, initially trying with certificate verification, but
/// trying without if the previous failed.
async fn run_analysis(config: &mut Configuration) -> anyhow::Result<ServerAnalysis> {
    let analysis = match analyze::analyze_server(config).await {
        Ok(analysis) => analysis,
        Err(_) => {
            config.allow_untrusted_certificates = true;
            config.rustls_client_config = Arc::new(
                rustls::ClientConfig::builder()
                    .with_safe_defaults()
                    .with_custom_certificate_verifier(Arc::new(UntrustedCertificateServerCertVerifier{}))
                    .with_no_client_auth()
            );
            println!("  {}: {}", "Warning".green(), "Failed using the trusted certificate root, using untrusted".red().bold());
            analyze::analyze_server(config).await?
        }
    };

    print_analysis(&analysis);

    Ok(analysis)
}
