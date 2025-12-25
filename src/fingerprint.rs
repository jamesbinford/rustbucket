use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::config::FingerprintConfig;

/// Version pools for realistic randomization
const UBUNTU_VERSIONS: &[&str] = &["20.04.6", "22.04.3", "22.04.4", "24.04"];
const KERNEL_VERSIONS: &[&str] = &["5.4.0", "5.15.0", "6.5.0", "6.8.0"];
const APACHE_VERSIONS: &[&str] = &["2.4.41", "2.4.52", "2.4.57", "2.4.58"];
const NGINX_VERSIONS: &[&str] = &["1.18.0", "1.22.1", "1.24.0"];
const VSFTPD_VERSIONS: &[&str] = &["3.0.3", "3.0.5"];
const OPENSSH_VERSIONS: &[&str] = &["8.2p1", "8.9p1", "9.0p1", "9.3p1"];
const HOSTNAME_PREFIXES: &[&str] = &["srv", "web", "app", "prod", "node", "host"];

/// Generated server fingerprint - persisted per instance for consistency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerFingerprint {
    /// Hostname (e.g., "srv-prod-01")
    pub hostname: String,
    /// OS version string (e.g., "Ubuntu 22.04.3 LTS")
    pub os_version: String,
    /// Kernel version (e.g., "5.15.0-91-generic")
    pub kernel_version: String,
    /// Full SSH welcome banner
    pub ssh_banner: String,
    /// Full uname -a output
    pub uname_output: String,
    /// HTTP Server header value (e.g., "Apache/2.4.57 (Ubuntu)")
    pub http_server: String,
    /// FTP welcome banner (e.g., "220 (vsFTPd 3.0.5)")
    pub ftp_banner: String,
    /// SMTP hostname (e.g., "mail.srv-prod-01.internal")
    pub smtp_hostname: String,
    /// SMTP greeting banner
    pub smtp_banner: String,
    /// SMTP EHLO response
    pub smtp_ehlo: String,
}

impl ServerFingerprint {
    /// Generate a new random fingerprint, using config overrides where provided
    pub fn generate(config: &FingerprintConfig) -> Self {
        let mut rng = rand::thread_rng();

        // Generate hostname
        let hostname = config.hostname.clone().unwrap_or_else(|| {
            let prefix = HOSTNAME_PREFIXES.choose(&mut rng).unwrap_or(&"srv");
            let suffix: String = (0..4)
                .map(|_| rng.gen_range(b'0'..=b'9') as char)
                .collect();
            format!("{}-{}", prefix, suffix)
        });

        // Pick Ubuntu version
        let ubuntu_version = UBUNTU_VERSIONS.choose(&mut rng).unwrap_or(&"22.04.3");
        let os_version = format!("Ubuntu {} LTS", ubuntu_version);

        // Generate kernel version with patch number
        let kernel_base = KERNEL_VERSIONS.choose(&mut rng).unwrap_or(&"5.15.0");
        let kernel_patch: u32 = rng.gen_range(50..150);
        let kernel_version = format!("{}-{}-generic", kernel_base, kernel_patch);

        // SSH banner
        let openssh_version = OPENSSH_VERSIONS.choose(&mut rng).unwrap_or(&"8.9p1");
        let ssh_banner = format!(
            "Welcome to {} (GNU/Linux {} x86_64)\r\n\r\n",
            os_version, kernel_version
        );

        // uname -a output
        let uname_output = format!(
            "Linux {} {} #{}-Ubuntu SMP x86_64 GNU/Linux\n",
            hostname,
            kernel_version,
            rng.gen_range(60..100)
        );

        // HTTP server header
        let http_server = config.http_server.clone().unwrap_or_else(|| {
            // 70% Apache, 30% nginx
            if rng.gen_bool(0.7) {
                let apache = APACHE_VERSIONS.choose(&mut rng).unwrap_or(&"2.4.52");
                format!("Apache/{} (Ubuntu)", apache)
            } else {
                let nginx = NGINX_VERSIONS.choose(&mut rng).unwrap_or(&"1.18.0");
                format!("nginx/{}", nginx)
            }
        });

        // FTP banner
        let ftp_version = config.ftp_version.clone().unwrap_or_else(|| {
            VSFTPD_VERSIONS.choose(&mut rng).unwrap_or(&"3.0.3").to_string()
        });
        let ftp_banner = format!("220 (vsFTPd {})\r\n", ftp_version);

        // SMTP hostname and banners
        let smtp_hostname = config.smtp_hostname.clone().unwrap_or_else(|| {
            format!("mail.{}.internal", hostname)
        });
        let smtp_banner = format!("220 {} ESMTP Postfix (Ubuntu)\r\n", smtp_hostname);
        let smtp_ehlo = format!(
            "250-{} Hello\r\n250-PIPELINING\r\n250-SIZE 10240000\r\n250-VRFY\r\n250-ETRN\r\n250-STARTTLS\r\n250-AUTH PLAIN LOGIN\r\n250 8BITMIME\r\n",
            smtp_hostname
        );

        Self {
            hostname,
            os_version,
            kernel_version,
            ssh_banner,
            uname_output,
            http_server,
            ftp_banner,
            smtp_hostname,
            smtp_banner,
            smtp_ehlo,
        }
    }

    /// Generate default fingerprint (for when fingerprinting is disabled)
    pub fn default_static() -> Self {
        Self {
            hostname: "ubuntu-server".to_string(),
            os_version: "Ubuntu 22.04.1 LTS".to_string(),
            kernel_version: "5.15.0-56-generic".to_string(),
            ssh_banner: "Welcome to Ubuntu 22.04.1 LTS (GNU/Linux 5.15.0-56-generic x86_64)\r\n\r\n".to_string(),
            uname_output: "Linux ubuntu-server 5.15.0-56-generic #62-Ubuntu SMP x86_64 GNU/Linux\n".to_string(),
            http_server: "Apache/2.4.52 (Ubuntu)".to_string(),
            ftp_banner: "220 (vsFTPd 3.0.3)\r\n".to_string(),
            smtp_hostname: "mail.example.com".to_string(),
            smtp_banner: "220 mail.example.com ESMTP Postfix (Ubuntu)\r\n".to_string(),
            smtp_ehlo: "250-mail.example.com Hello\r\n250-PIPELINING\r\n250-SIZE 10240000\r\n250-VRFY\r\n250-ETRN\r\n250-STARTTLS\r\n250-AUTH PLAIN LOGIN\r\n250 8BITMIME\r\n".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_fingerprint() {
        let config = FingerprintConfig::default();
        let fp = ServerFingerprint::generate(&config);

        // Verify fields are populated
        assert!(!fp.hostname.is_empty());
        assert!(fp.os_version.contains("Ubuntu"));
        assert!(fp.kernel_version.contains("-generic"));
        assert!(fp.ssh_banner.contains("Welcome to"));
        assert!(fp.uname_output.contains("Linux"));
        assert!(!fp.http_server.is_empty());
        assert!(fp.ftp_banner.starts_with("220"));
        assert!(fp.smtp_banner.starts_with("220"));
    }

    #[test]
    fn test_generate_with_overrides() {
        let config = FingerprintConfig {
            enabled: true,
            hostname: Some("custom-host".to_string()),
            http_server: Some("nginx/1.25.0".to_string()),
            ftp_version: Some("3.0.5".to_string()),
            smtp_hostname: Some("mail.custom.com".to_string()),
        };
        let fp = ServerFingerprint::generate(&config);

        assert_eq!(fp.hostname, "custom-host");
        assert_eq!(fp.http_server, "nginx/1.25.0");
        assert!(fp.ftp_banner.contains("3.0.5"));
        assert_eq!(fp.smtp_hostname, "mail.custom.com");
    }

    #[test]
    fn test_default_static() {
        let fp = ServerFingerprint::default_static();

        assert_eq!(fp.hostname, "ubuntu-server");
        assert!(fp.os_version.contains("22.04.1"));
        assert!(fp.kernel_version.contains("5.15.0-56"));
    }

    #[test]
    fn test_fingerprint_serialization() {
        let fp = ServerFingerprint::default_static();
        let json = serde_json::to_string(&fp).unwrap();

        assert!(json.contains("ubuntu-server"));
        assert!(json.contains("Apache/2.4.52"));

        let parsed: ServerFingerprint = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.hostname, fp.hostname);
    }

    #[test]
    fn test_uniqueness() {
        let config = FingerprintConfig::default();
        let fp1 = ServerFingerprint::generate(&config);
        let fp2 = ServerFingerprint::generate(&config);

        // Very unlikely to generate identical fingerprints
        assert_ne!(fp1.hostname, fp2.hostname);
    }
}
