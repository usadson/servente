use tokio::{
    net::TcpListener,
    task,
};

use rustls::{
    ConfigBuilder,
    server::{
        AllowAnyAnonymousOrAuthenticatedClient, AllowAnyAuthenticatedClient, NoClientAuth,
    }, ServerConfig
};
use tokio_rustls::TlsAcceptor;

use std::{io, sync::Arc};

async fn process_socket<T>(socket: T, tls_config: Arc<ServerConfig>) {
    let mut connection = rustls::ServerConnection::new(tls_config).unwrap();

    connection.
}

pub async fn start(address: &str, tls_config: Arc<ServerConfig>) -> io::Result<()> {
    let listener = TcpListener::bind(address).await?;
    let acceptor = TlsAcceptor::from(tls_config);

    loop {
        let (socket, address) = listener.accept().await?;
        let tls_config = Arc::clone(&tls_config);
        task::spawn(async {
            process_socket(socket, tls_config).await;
        });
    }
}
