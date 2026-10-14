//! The PC's TLS identity: a self-signed certificate made once and kept. Phones remember its
//! SHA-256 fingerprint when they pair and accept nothing else afterwards, so nobody on the
//! network can read the traffic or pose as the PC later.

use std::path::Path;
use std::sync::Arc;

use rustls::ServerConfig;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use sha2::{Digest, Sha256};

pub struct Identity {
    pub cert: Vec<u8>,
    key: Vec<u8>,
}

impl Identity {
    /// Loads `cert.der`/`key.der` from `dir`, or makes and saves a new pair.
    pub fn load_or_create(dir: &Path, name: &str) -> Result<Self, String> {
        let (cert_path, key_path) = (dir.join("remote-cert.der"), dir.join("remote-key.der"));
        if let (Ok(cert), Ok(key)) = (std::fs::read(&cert_path), std::fs::read(&key_path)) {
            return Ok(Self { cert, key });
        }
        let made = rcgen::generate_simple_self_signed(vec![name.to_string(), "localhost".into()])
            .map_err(|e| e.to_string())?;
        let identity = Self {
            cert: made.cert.der().to_vec(),
            key: made.signing_key.serialize_der(),
        };
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        write_private(&key_path, &identity.key)?;
        std::fs::write(&cert_path, &identity.cert).map_err(|e| e.to_string())?;
        Ok(identity)
    }

    /// SHA-256 of the certificate, lowercase hex: what phones pin.
    pub fn fingerprint(&self) -> String {
        Sha256::digest(&self.cert)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    pub fn server_config(&self) -> Result<Arc<ServerConfig>, String> {
        let config =
            ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
                .with_safe_default_protocol_versions()
                .map_err(|e| e.to_string())?
                .with_no_client_auth()
                .with_single_cert(
                    vec![CertificateDer::from(self.cert.clone())],
                    PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(self.key.clone())),
                )
                .map_err(|e| e.to_string())?;
        Ok(Arc::new(config))
    }
}

/// Writes a file only the current user can read (Unix; Windows keeps it in the user's own
/// app data folder, which other accounts can't read either).
fn write_private(path: &Path, bytes: &[u8]) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)
            .map_err(|e| e.to_string())?;
        f.write_all(bytes).map_err(|e| e.to_string())
    }
    #[cfg(not(unix))]
    {
        std::fs::write(path, bytes).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_identity_is_made_once_and_kept() {
        let dir = tempfile::tempdir().unwrap();
        let first = Identity::load_or_create(dir.path(), "DESKTOP-TEST").unwrap();
        let again = Identity::load_or_create(dir.path(), "DESKTOP-TEST").unwrap();
        assert_eq!(first.fingerprint(), again.fingerprint());
        assert_eq!(first.fingerprint().len(), 64);
        assert!(first.server_config().is_ok());
    }
}
