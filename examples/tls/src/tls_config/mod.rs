mod config {

    use tokio_rustls::rustls::{Certificate, PrivateKey, ServerConfig};

    const CERT: &[u8] = include_bytes!("local.cert");
    const PKEY: &[u8] = include_bytes!("local.key");

    fn tls_acceptor_impl(cert_der: &[u8], key_der: &[u8]) -> ServerConfig {
        let key = PrivateKey(cert_der.into());
        let cert = Certificate(key_der.into());
        ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(vec![cert], key)
            .unwrap()
    }

    pub fn rustls_config() -> ServerConfig {
        tls_acceptor_impl(PKEY, CERT)
    }
}

pub use config::rustls_config;
