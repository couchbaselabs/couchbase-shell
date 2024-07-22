use crate::client::capella_ca::CAPELLA_CERT;
use crate::client::ClientError;
use crate::ClusterTlsConfig;
use log::{debug, error};
use rustls_pemfile::{read_all, Item};
use std::convert::TryFrom;
use std::fs;
use std::io::BufReader;
use std::sync::Arc;
use tokio_rustls::rustls::crypto::{aws_lc_rs::default_provider, CryptoProvider};
use tokio_rustls::rustls::{ClientConfig, RootCertStore};

#[derive(Clone)]
pub struct RustTlsConfig {
    config: ClientConfig,
    // We need to hold onto these so that we can later rewrite them into the config file if needed.
    accept_all_certs: bool,
    cert_path: Option<String>,
}

impl RustTlsConfig {
    pub fn new(
        accept_all_certs: bool,
        cert_path: Option<String>,
    ) -> Result<RustTlsConfig, ClientError> {
        let _ = CryptoProvider::install_default(default_provider());
        let builder = ClientConfig::builder();
        if accept_all_certs {
            let config = builder
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(utilities::InsecureCertVerifier {}))
                .with_no_client_auth();

            return Ok(RustTlsConfig {
                config,
                accept_all_certs,
                cert_path,
            });
        }

        let mut root_cert_store = RootCertStore::empty();
        if let Some(path) = cert_path.clone() {
            // If the user has provided a cert path then use it.
            // If any errors occur then consider this as fatal and return the error.
            let cert = fs::read(path).map_err(ClientError::from)?;
            let mut reader = BufReader::new(&cert[..]);
            let items = read_all(&mut reader).map(|item| {
                item.map_err(|e| ClientError::RequestFailed {
                    reason: Some(format!("Failed to read cert file {}", e)),
                    key: None,
                })
            });
            for item in items {
                match item? {
                    Item::X509Certificate(c) => {
                        root_cert_store
                            .add(c)
                            .map_err(|e| ClientError::RequestFailed {
                                reason: Some(format!("Failed to add cert to root store {}", e)),
                                key: None,
                            })?
                    }
                    _ => {
                        return Err(ClientError::RequestFailed {
                            reason: Some("Unsupported certificate format".to_string()),
                            key: None,
                        })
                    }
                }
            }
        } else {
            debug!("Adding webpki tls server roots");
            root_cert_store = RootCertStore {
                roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
            };

            debug!("Adding Capella root CA to trust store");
            let mut reader = BufReader::new(CAPELLA_CERT.as_bytes());
            match read_all(&mut reader).next().unwrap() {
                Ok(item) => {
                    // There is only 1 item in the capella cert.
                    match &item {
                        Item::X509Certificate(c) => match root_cert_store.add(c.to_owned()) {
                            Ok(()) => {}
                            Err(e) => {
                                error!("Failed to add root capella cert to root store {}", e);
                            }
                        },
                        _ => {
                            error!(
                                "Failed to read capella certificate, unsupported certificate format"
                            );
                        }
                    };
                }
                Err(e) => {
                    error!("Failed to read capella certificate, {}", e);
                }
            };
        };
        let config = builder
            .with_root_certificates(root_cert_store)
            .with_no_client_auth();

        Ok(RustTlsConfig {
            config,
            accept_all_certs,
            cert_path,
        })
    }

    pub fn config(&self) -> ClientConfig {
        self.config.clone()
    }

    pub fn accept_all_certs(&self) -> bool {
        self.accept_all_certs
    }

    pub fn cert_path(&self) -> Option<String> {
        self.cert_path.clone()
    }
}

impl TryFrom<ClusterTlsConfig> for RustTlsConfig {
    type Error = ClientError;

    fn try_from(tls_config: ClusterTlsConfig) -> Result<Self, Self::Error> {
        RustTlsConfig::new(
            tls_config.accept_all_certs(),
            tls_config.cert_path().clone(),
        )
    }
}
