use crate::error::AppError;
use std::collections::HashSet;
use std::env;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::thread;
use std::time::{Duration, Instant};

pub const MDNS_PAIRING_SERVICE: &str = "_adb-tls-pairing._tcp";
pub const MDNS_CONNECT_SERVICE: &str = "_adb-tls-connect._tcp";

#[derive(Debug, Clone)]
pub struct Adb {
    executable: PathBuf,
}

#[derive(Debug, Clone)]
pub struct MdnsService {
    pub name: String,
    pub service_type: String,
    pub endpoint: String,
}

impl Adb {
    pub fn resolve(explicit: Option<PathBuf>) -> Result<Self, AppError> {
        if let Some(path) = explicit {
            return if path.exists() {
                Ok(Self { executable: path })
            } else {
                Err(AppError::unsupported(format!(
                    "adb was not found at {}",
                    path.display()
                )))
            };
        }

        if let Ok(path) = which::which(adb_binary_name()) {
            return Ok(Self { executable: path });
        }

        for candidate in common_adb_paths() {
            if candidate.exists() {
                return Ok(Self {
                    executable: candidate,
                });
            }
        }

        Err(AppError::unsupported(
            "adb was not found. Install Android platform-tools, add adb to PATH, or pass --adb-path.",
        ))
    }

    pub fn version_text(&self) -> Result<String, AppError> {
        self.run_text(&["version"])
    }

    pub fn mdns_check(&self) -> Result<String, AppError> {
        self.run_text(&["mdns", "check"])
    }

    pub fn list_mdns_services(&self) -> Result<Vec<MdnsService>, AppError> {
        let text = self.run_text(&["mdns", "services"])?;
        Ok(parse_mdns_services(&text))
    }

    pub fn list_pairing_services(&self) -> Result<Vec<MdnsService>, AppError> {
        Ok(self
            .list_mdns_services()?
            .into_iter()
            .filter(|service| service.service_type == MDNS_PAIRING_SERVICE)
            .collect())
    }

    pub fn wait_for_connect_service(
        &self,
        pairing_endpoint: &str,
        timeout: Duration,
    ) -> Result<Option<MdnsService>, AppError> {
        let start = Instant::now();
        let pairing_host = endpoint_host(pairing_endpoint);

        while start.elapsed() < timeout {
            let service = self.list_mdns_services()?.into_iter().find(|service| {
                service.service_type == MDNS_CONNECT_SERVICE
                    && endpoint_host(&service.endpoint) == pairing_host
            });
            if service.is_some() {
                return Ok(service);
            }
            thread::sleep(Duration::from_secs(1));
        }

        Ok(None)
    }

    /// Names of the pairing services adb already knows about.
    ///
    /// Captured before the QR code is displayed so that any pairing service
    /// appearing afterwards can be attributed to the scan we just triggered.
    pub fn pairing_service_names(&self) -> Result<HashSet<String>, AppError> {
        Ok(self
            .list_pairing_services()?
            .into_iter()
            .map(|service| service.name)
            .collect())
    }

    /// Wait for the pairing service created by scanning our QR code.
    ///
    /// Prefers an exact name match, but falls back to any pairing service that
    /// was not present in `baseline`. adb does not always advertise the service
    /// under the name we embedded in the payload, and insisting on an exact
    /// match there means ignoring the very service the user just created while
    /// their phone sits waiting on the pairing screen.
    pub fn wait_for_pairing_service(
        &self,
        service_name: &str,
        baseline: &HashSet<String>,
        timeout: Duration,
    ) -> Result<MdnsService, AppError> {
        let start = Instant::now();
        let grace = fallback_grace(timeout);
        let mut observed = Vec::new();

        while start.elapsed() < timeout {
            observed = self.list_mdns_services()?;
            let services = observed
                .iter()
                .filter(|service| service.service_type == MDNS_PAIRING_SERVICE);

            // Only consider an unrecognised service once the exact name has had
            // time to show up; a concurrent pairing session on the same device
            // would otherwise be picked over the one the user just scanned.
            let allow_fallback = start.elapsed() >= grace;
            if let Some(service) =
                select_pairing_service(services, service_name, baseline, allow_fallback)
            {
                return Ok(service);
            }

            thread::sleep(Duration::from_secs(1));
        }

        Err(AppError::timeout(format!(
            "timed out waiting for pairing service `{service_name}` to appear in adb mDNS discovery\n{}",
            describe_observed(&observed)
        )))
    }

    pub fn pair(&self, endpoint: &str, code: &str) -> Result<(), AppError> {
        self.run_text(&["pair", endpoint, code])?;
        Ok(())
    }

    pub fn connect(&self, endpoint: &str) -> Result<(), AppError> {
        let output = self.run_text(&["connect", endpoint])?;
        if output.starts_with("connected to ") || output.starts_with("already connected to ") {
            Ok(())
        } else {
            Err(AppError::adb(format!(
                "adb connect {endpoint} failed: {output}"
            )))
        }
    }

    pub fn device_set(&self) -> Result<HashSet<String>, AppError> {
        let text = self.run_text(&["devices"])?;
        Ok(parse_devices(&text).into_iter().collect())
    }

    pub fn wait_for_device(
        &self,
        baseline: &HashSet<String>,
        timeout: Duration,
    ) -> Result<bool, AppError> {
        let start = Instant::now();

        while start.elapsed() < timeout {
            let current = self.device_set()?;
            if current
                .iter()
                .any(|line| line.contains("\tdevice") && !baseline.contains(line))
            {
                return Ok(true);
            }

            thread::sleep(Duration::from_secs(1));
        }

        Ok(false)
    }

    fn run_text(&self, args: &[&str]) -> Result<String, AppError> {
        let output = self.run(args)?;
        let mut text = String::new();
        text.push_str(&String::from_utf8_lossy(&output.stdout));
        if !output.stderr.is_empty() {
            if !text.is_empty() && !text.ends_with('\n') {
                text.push('\n');
            }
            text.push_str(&String::from_utf8_lossy(&output.stderr));
        }

        if output.status.success() {
            Ok(text.trim().to_string())
        } else {
            Err(AppError::adb(format!(
                "adb {} failed: {}",
                args.join(" "),
                text.trim()
            )))
        }
    }

    fn run(&self, args: &[&str]) -> Result<Output, AppError> {
        Command::new(&self.executable)
            .args(args)
            .output()
            .map_err(|err| {
                AppError::adb(format!(
                    "failed to execute adb at {}: {err}",
                    self.executable.display()
                ))
            })
    }
}

/// Pick the pairing service that belongs to this run: an exact name match
/// first, otherwise any pairing service that was not already advertised.
fn select_pairing_service<'a, I>(
    services: I,
    service_name: &str,
    baseline: &HashSet<String>,
    allow_fallback: bool,
) -> Option<MdnsService>
where
    I: Iterator<Item = &'a MdnsService>,
{
    let mut fallback = None;

    for service in services {
        if service.name == service_name {
            return Some(service.clone());
        }
        if allow_fallback && fallback.is_none() && !baseline.contains(&service.name) {
            fallback = Some(service.clone());
        }
    }

    fallback
}

/// How long to hold out for an exact name match before considering any other
/// newly advertised pairing service.
fn fallback_grace(timeout: Duration) -> Duration {
    (timeout / 3).min(Duration::from_secs(15))
}

/// Render what adb actually discovered, so a timeout is diagnosable from a
/// single run rather than needing a separate debugging session.
fn describe_observed(services: &[MdnsService]) -> String {
    if services.is_empty() {
        return "adb reported no mDNS services at all. Check that the device is on the same \
network, that Wireless debugging is still open on the pairing screen, and that a firewall \
is not blocking mDNS (UDP 5353)."
            .to_string();
    }

    let mut text = String::from("adb last reported these mDNS services:");
    for service in services {
        text.push_str(&format!(
            "\n  {} {} {}",
            service.name, service.service_type, service.endpoint
        ));
    }
    text
}

pub fn parse_mdns_services(output: &str) -> Vec<MdnsService> {
    output
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("List of discovered") {
                return None;
            }

            let mut parts = trimmed.split_whitespace();
            let name = parts.next()?;
            let service_type = parts.next()?;
            let endpoint = parts.next()?;
            let _ = endpoint.rsplit_once(':')?;

            Some(MdnsService {
                name: name.to_string(),
                service_type: service_type.to_string(),
                endpoint: endpoint.to_string(),
            })
        })
        .collect()
}

pub fn parse_devices(output: &str) -> Vec<String> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| {
            !line.is_empty()
                && !line.starts_with("List of devices attached")
                && !line.starts_with('*')
        })
        .map(ToOwned::to_owned)
        .collect()
}

fn endpoint_host(endpoint: &str) -> &str {
    endpoint
        .rsplit_once(':')
        .map(|(host, _)| host.trim_matches(['[', ']']))
        .unwrap_or(endpoint)
}

fn adb_binary_name() -> &'static str {
    if cfg!(windows) {
        "adb.exe"
    } else {
        "adb"
    }
}

fn common_adb_paths() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let binary = adb_binary_name();

    for env_name in ["ANDROID_SDK_ROOT", "ANDROID_HOME"] {
        if let Ok(root) = env::var(env_name) {
            candidates.push(PathBuf::from(root).join("platform-tools").join(binary));
        }
    }

    if let Some(home) = home_dir() {
        candidates.push(home.join("Library/Android/sdk/platform-tools").join(binary));
        candidates.push(home.join("Android/Sdk/platform-tools").join(binary));
        candidates.push(
            home.join("AppData/Local/Android/Sdk/platform-tools")
                .join(binary),
        );
    }

    if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
        candidates.push(
            PathBuf::from(local_app_data)
                .join("Android/Sdk/platform-tools")
                .join(binary),
        );
    }

    candidates
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("USERPROFILE").map(PathBuf::from))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mdns_services_output() {
        let services = parse_mdns_services(
            "List of discovered mdns services\nadb-qr-test\t_adb-tls-pairing._tcp\t192.168.0.5:37123\nignored\t_adb-tls-connect._tcp\t192.168.0.5:43210\n",
        );

        assert_eq!(services.len(), 2);
        assert_eq!(services[0].name, "adb-qr-test");
        assert_eq!(services[0].endpoint, "192.168.0.5:37123");
    }

    #[test]
    fn parses_devices_output() {
        let devices = parse_devices(
            "List of devices attached\nadb-ABC123._adb-tls-connect._tcp\tdevice\nemulator-5554\tdevice\n\n",
        );

        assert_eq!(devices.len(), 2);
        assert!(devices[0].contains("adb-ABC123"));
    }

    fn pairing(name: &str) -> MdnsService {
        MdnsService {
            name: name.to_string(),
            service_type: MDNS_PAIRING_SERVICE.to_string(),
            endpoint: "192.168.0.5:37123".to_string(),
        }
    }

    #[test]
    fn prefers_the_exact_service_name() {
        let services = [pairing("studio-other"), pairing("studio-wanted")];
        let selected =
            select_pairing_service(services.iter(), "studio-wanted", &HashSet::new(), true)
                .expect("hit");

        assert_eq!(selected.name, "studio-wanted");
    }

    #[test]
    fn falls_back_to_a_newly_advertised_service() {
        let services = [pairing("studio-wanted (2)")];
        let selected =
            select_pairing_service(services.iter(), "studio-wanted", &HashSet::new(), true)
                .expect("fallback");

        assert_eq!(selected.name, "studio-wanted (2)");
    }

    #[test]
    fn ignores_services_present_before_the_scan() {
        let baseline: HashSet<String> = ["studio-stale".to_string()].into_iter().collect();
        let services = [pairing("studio-stale")];

        assert!(
            select_pairing_service(services.iter(), "studio-wanted", &baseline, true).is_none()
        );
    }

    #[test]
    fn fallback_is_withheld_during_the_grace_period() {
        let services = [pairing("studio-someone-else")];

        assert!(
            select_pairing_service(services.iter(), "studio-wanted", &HashSet::new(), false)
                .is_none()
        );
    }

    #[test]
    fn exact_match_still_wins_during_the_grace_period() {
        let services = [pairing("studio-wanted")];
        let selected =
            select_pairing_service(services.iter(), "studio-wanted", &HashSet::new(), false)
                .expect("exact match");

        assert_eq!(selected.name, "studio-wanted");
    }

    #[test]
    fn fallback_grace_is_capped() {
        assert_eq!(
            fallback_grace(Duration::from_secs(90)),
            Duration::from_secs(15)
        );
        assert_eq!(
            fallback_grace(Duration::from_secs(9)),
            Duration::from_secs(3)
        );
    }

    #[test]
    fn timeout_diagnostics_list_observed_services() {
        let described = describe_observed(&[pairing("studio-seen")]);
        assert!(described.contains("studio-seen"));
        assert!(described.contains(MDNS_PAIRING_SERVICE));

        assert!(describe_observed(&[]).contains("5353"));
    }

    #[test]
    fn extracts_ipv4_and_ipv6_endpoint_hosts() {
        assert_eq!(endpoint_host("192.168.0.5:37123"), "192.168.0.5");
        assert_eq!(endpoint_host("[fe80::1234]:37123"), "fe80::1234");
    }
}
