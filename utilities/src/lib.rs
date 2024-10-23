use tokio_rustls::rustls::client::danger::{
    HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier,
};
use tokio_rustls::rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use tokio_rustls::rustls::{DigitallySignedStruct, Error, SignatureScheme};

#[derive(Debug)]
pub struct InsecureCertVerifier {}

impl ServerCertVerifier for InsecureCertVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer,
        _intermediates: &[CertificateDer],
        _server_name: &ServerName,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PKCS1_SHA1,
            SignatureScheme::ECDSA_SHA1_Legacy,
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::ECDSA_NISTP521_SHA512,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::ED25519,
            SignatureScheme::ED448,
        ]
    }
}

use trust_dns_resolver::config::{ResolverConfig, ResolverOpts};
use trust_dns_resolver::Resolver;

pub mod json_row_parser;
pub mod json_row_stream;
pub mod raw_json_row_streamer;

pub fn try_lookup_srv(addr: String) -> Result<Vec<String>, String> {
    // NOTE: resolver is going to build its own runtime, which is a pain...
    // Construct a new Resolver with default configuration options
    let resolver = Resolver::new(ResolverConfig::default(), ResolverOpts::default())
        .map_err(|e| e.to_string())?;
    let mut address = addr;
    if !address.starts_with("_couchbases._tcp.") {
        address = format!("_couchbases._tcp.{}", address);
    }

    let response = match resolver.srv_lookup(address) {
        Ok(k) => k,
        Err(e) => return Err(e.to_string()),
    };

    let mut addresses: Vec<String> = Vec::new();
    for a in response.iter() {
        // The addresses get suffixed with a . so we have to remove this to later match the address
        // with the addresses in the alternate addresses in the config.
        let mut host = a.target().to_string();
        if let Some(prefix) = host.strip_suffix('.') {
            host = prefix.to_string();
        }
        addresses.push(host);
    }

    Ok(addresses)
}
