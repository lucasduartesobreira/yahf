mod config {

    use rcgen::generate_simple_self_signed;
    use tokio_rustls::rustls::{Certificate, PrivateKey, ServerConfig};

    fn gen_certs_and_server_config() -> ServerConfig {
        let subject_alt_names = vec!["hello.world.example".to_string(), "localhost".to_string()];

        let cert = generate_simple_self_signed(subject_alt_names).unwrap();
        let key = PrivateKey(cert.serialize_private_key_der());
        let cert = Certificate(cert.serialize_der().unwrap());
        ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(vec![cert], key)
            .unwrap()
    }

    pub fn rustls_config() -> ServerConfig {
        gen_certs_and_server_config()
    }
}

pub use config::rustls_config;
