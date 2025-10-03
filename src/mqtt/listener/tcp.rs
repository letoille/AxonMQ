use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;

use rustls_pemfile::{certs, pkcs8_private_keys};
use tokio::net::TcpListener;
use tokio_rustls::{TlsAcceptor, rustls::ServerConfig};
use tracing::{debug, error, info};

use crate::mqtt::helper::BrokerHelper;
use crate::operator::helper::Helper as OperatorHelper;

use super::shared::process_client;

pub fn spawn_tcp_listener(
    host: String,
    port: u16,
    broker_helper: BrokerHelper,
    operator_helper: OperatorHelper,
) {
    tokio::spawn(async move {
        let addr = format!("{}:{}", host, port);
        let listener = match TcpListener::bind(&addr).await {
            Ok(l) => l,
            Err(e) => {
                error!("failed to bind TCP listener on {}: {}", addr, e);
                return;
            }
        };
        info!("MQTT TCP listening on {}", addr);

        loop {
            let (stream, addr) = listener.accept().await.unwrap();
            let broker_helper = broker_helper.clone();
            let operator_helper = operator_helper.clone();
            tokio::spawn(process_client(stream, addr, broker_helper, operator_helper));
        }
    });
}

pub fn spawn_tls_listener(
    host: String,
    port: u16,
    cert_path: String,
    key_path: String,
    broker_helper: BrokerHelper,
    operator_helper: OperatorHelper,
) {
    tokio::spawn(async move {
        let addr = format!("{}:{}", host, port);
        let tls_acceptor = match load_tls_acceptor(&cert_path, &key_path) {
            Ok(acceptor) => acceptor,
            Err(e) => {
                error!("failed to load TCP/TLS config for {}: {}", addr, e);
                return;
            }
        };

        let listener = match TcpListener::bind(&addr).await {
            Ok(l) => l,
            Err(e) => {
                error!("failed to bind TCP/TLS listener on {}: {}", addr, e);
                return;
            }
        };
        info!("MQTT TCP/TLS listening on {}", addr);

        loop {
            let (stream, addr) = listener.accept().await.unwrap();
            let acceptor = tls_acceptor.clone();
            let broker_helper = broker_helper.clone();
            let operator_helper = operator_helper.clone();

            tokio::spawn(async move {
                match acceptor.accept(stream).await {
                    Ok(tls_stream) => {
                        process_client(tls_stream, addr, broker_helper, operator_helper).await;
                    }
                    Err(e) => {
                        debug!("TLS handshake error from {}: {}", addr, e);
                    }
                }
            });
        }
    });
}

pub fn load_tls_acceptor(cert_path: &str, key_path: &str) -> std::io::Result<TlsAcceptor> {
    let certs_file =
        File::open(cert_path).map_err(|e| std::io::Error::new(std::io::ErrorKind::NotFound, e))?;
    let mut certs_reader = BufReader::new(certs_file);
    let certs = certs(&mut certs_reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

    let key_file =
        File::open(key_path).map_err(|e| std::io::Error::new(std::io::ErrorKind::NotFound, e))?;
    let mut key_reader = BufReader::new(key_file);

    let key = pkcs8_private_keys(&mut key_reader).next().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "no private key found in key file",
        )
    })??;

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, rustls::pki_types::PrivateKeyDer::Pkcs8(key))
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    Ok(TlsAcceptor::from(Arc::new(config)))
}
