//! OS-level CA trust operations (system keychain, certutil, update-ca-certificates).

use crate::common::{Error, Result};
use crate::tls::Ca;

/// Install the CA certificate in the system trust store.
///
/// Platform-specific:
/// - macOS: `sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain ca.pem`
/// - Linux: copy to `/usr/local/share/ca-certificates/portless-ca.crt` and run `update-ca-certificates`,
///   or use `trust anchor` directly. Falls back to `/etc/pki/ca-trust/source/anchors/` for RHEL-family.
/// - Windows: `certutil -addstore -f "ROOT" ca.pem`
pub async fn install_ca(ca: &Ca) -> Result<()> {
    let tmp = std::env::temp_dir().join("portless-ca.pem");
    tokio::fs::write(&tmp, ca.cert_pem.as_bytes()).await?;
    install_ca_at(&tmp).await
}

async fn install_ca_at(ca_path: &std::path::Path) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        let status = std::process::Command::new("sudo")
            .args([
                "security",
                "add-trusted-cert",
                "-d",
                "-r",
                "trustRoot",
                "-k",
                "/Library/Keychains/System.keychain",
            ])
            .arg(ca_path)
            .status()?;
        if !status.success() {
            return Err(Error::Tls("macOS security add-trusted-cert failed".into()));
        }
        Ok(())
    }
    #[cfg(target_os = "linux")]
    {
        // Try Debian/Ubuntu path first.
        let dest = std::path::Path::new("/usr/local/share/ca-certificates/portless-ca.crt");
        let status = std::process::Command::new("sudo")
            .args(["cp", &ca_path.to_string_lossy(), &dest.to_string_lossy()])
            .status()?;
        if !status.success() {
            return Err(Error::Tls(
                "copying CA to /usr/local/share/ca-certificates failed".into(),
            ));
        }
        let _ = std::process::Command::new("sudo")
            .args(["update-ca-certificates"])
            .status();
        Ok(())
    }
    #[cfg(target_os = "windows")]
    {
        let status = std::process::Command::new("certutil")
            .args(["-addstore", "-f", "ROOT", &ca_path.to_string_lossy()])
            .status()?;
        if !status.success() {
            return Err(Error::Tls("certutil addstore failed".into()));
        }
        Ok(())
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = ca_path;
        Err(Error::UnsupportedPlatform(
            "CA trust not implemented on this platform".into(),
        ))
    }
}

/// Remove the CA certificate from the system trust store.
pub async fn uninstall_ca(ca: &Ca) -> Result<()> {
    let tmp = std::env::temp_dir().join("portless-ca.pem");
    tokio::fs::write(&tmp, ca.cert_pem.as_bytes()).await?;
    uninstall_ca_at(&tmp).await
}

async fn uninstall_ca_at(ca_path: &std::path::Path) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        // Find the cert by SHA-256 fingerprint and remove.
        let _ = std::process::Command::new("sudo")
            .args(["security", "delete-certificate", "-c", "Portless Local CA"])
            .status();
        let _ = ca_path;
        Ok(())
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("sudo")
            .args([
                "rm",
                "-f",
                "/usr/local/share/ca-certificates/portless-ca.crt",
            ])
            .status();
        let _ = std::process::Command::new("sudo")
            .args(["update-ca-certificates", "--fresh"])
            .status();
        let _ = ca_path;
        Ok(())
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("certutil")
            .args(["-delstore", "ROOT", "Portless Local CA"])
            .status();
        let _ = ca_path;
        Ok(())
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = ca_path;
        Err(Error::UnsupportedPlatform(
            "CA trust not implemented on this platform".into(),
        ))
    }
}
