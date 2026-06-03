use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

use buildkit_frontend::oci::*;
use buildkit_llb::ops::platform::Platform;
use chrono::{DateTime, Utc};
use dofigen_lib::{Dofigen, Healthcheck, Port, PortProtocol, User};

/// OCI label carrying the image creation date (RFC 3339).
const LABEL_CREATED: &str = "org.opencontainers.image.created";
/// OCI label carrying the image author(s).
const LABEL_AUTHORS: &str = "org.opencontainers.image.authors";

pub trait ImageSpecificationExt {
    fn image_specification(&self, platform: &Platform) -> ImageSpecification;
}

impl ImageSpecificationExt for Dofigen {
    fn image_specification(&self, platform: &Platform) -> ImageSpecification {
        let labels = &self.stage.label;

        ImageSpecification {
            // Use the `org.opencontainers.image.created` label if present and
            // valid, otherwise fall back to the current build date.
            created: Some(
                labels
                    .get(LABEL_CREATED)
                    .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
                    .map(|date| date.with_timezone(&Utc))
                    .unwrap_or_else(Utc::now),
            ),
            // Extract the author from the `org.opencontainers.image.authors` label.
            author: labels.get(LABEL_AUTHORS).cloned(),

            architecture: arch_from_str(&platform.architecture),
            os: os_from_str(&platform.os),
            variant: if platform.variant.is_empty() {
                None
            } else {
                Some(platform.variant.clone())
            }, //platform.variant.clone(),

            os_version: None,
            os_features: None,

            config: Some(ImageConfig {
                entrypoint: into_optional_vec(&self.entrypoint),
                cmd: into_optional_vec(&self.cmd),
                env: into_btree_map(&self.stage.env),
                user: self.stage.user.as_ref().map(User::to_string),
                working_dir: self.stage.workdir.clone().map(String::into),

                labels: into_btree_map(&self.stage.label),
                volumes: into_optional_mapped_vec(&self.volume, String::into),
                exposed_ports: into_optional_mapped_vec(&self.expose, Port::to_exposed_port),
                healthcheck: self.healthcheck.as_ref().map(to_healthcheck),
                // The stop signal will be managed in a later phase of Dofigen.
                stop_signal: None,
                ..Default::default()
            }),

            // TODO: we should be able to generate the rootfs and history from the stages
            rootfs: None,
            history: None,
        }
    }
}

/// Translates a Dofigen [`Healthcheck`] into its OCI representation.
fn to_healthcheck(healthcheck: &Healthcheck) -> buildkit_frontend::oci::Healthcheck {
    buildkit_frontend::oci::Healthcheck {
        // `CMD-SHELL` runs the command in the image's default shell, which
        // matches the shell form used by the Dockerfile generator (`CMD <cmd>`).
        test: Some(vec!["CMD-SHELL".to_string(), healthcheck.cmd.clone()]),
        interval: healthcheck.interval.as_deref().and_then(parse_go_duration),
        timeout: healthcheck.timeout.as_deref().and_then(parse_go_duration),
        start_period: healthcheck.start.as_deref().and_then(parse_go_duration),
        start_interval: None,
        retries: healthcheck.retries.map(u32::from),
    }
}

/// Parses a Go-style duration string (e.g. `30s`, `1m30s`, `500ms`) into a
/// [`Duration`]. Returns `None` when the string is empty or malformed.
///
/// Supported units: `ns`, `us`/`µs`, `ms`, `s`, `m`, `h`.
fn parse_go_duration(input: &str) -> Option<Duration> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }

    let bytes = input.as_bytes();
    let mut total_nanos: f64 = 0.0;
    let mut index = 0;

    while index < bytes.len() {
        // Parse the numeric part (digits and an optional decimal point).
        let number_start = index;
        while index < bytes.len() && (bytes[index].is_ascii_digit() || bytes[index] == b'.') {
            index += 1;
        }
        if index == number_start {
            return None;
        }
        let number: f64 = input[number_start..index].parse().ok()?;

        // Parse the unit (everything up to the next digit).
        let unit_start = index;
        while index < bytes.len() && !bytes[index].is_ascii_digit() && bytes[index] != b'.' {
            index += 1;
        }
        let multiplier: f64 = match &input[unit_start..index] {
            "ns" => 1.0,
            "us" | "µs" | "μs" => 1_000.0,
            "ms" => 1_000_000.0,
            "s" => 1_000_000_000.0,
            "m" => 60.0 * 1_000_000_000.0,
            "h" => 3_600.0 * 1_000_000_000.0,
            _ => return None,
        };

        total_nanos += number * multiplier;
    }

    Some(Duration::from_nanos(total_nanos as u64))
}

trait ToExposedPort {
    fn to_exposed_port(self) -> ExposedPort;
}

impl ToExposedPort for Port {
    fn to_exposed_port(self) -> ExposedPort {
        // Default port is TCP
        if PortProtocol::Tcp == self.protocol.unwrap_or(PortProtocol::Tcp) {
            ExposedPort::Tcp(self.port)
        } else {
            ExposedPort::Udp(self.port)
        }
    }
}

fn into_btree_map(map: &HashMap<String, String>) -> Option<BTreeMap<String, String>> {
    if map.is_empty() {
        None
    } else {
        Some(map.clone().into_iter().collect())
    }
}

fn into_optional_vec(vec: &Vec<String>) -> Option<Vec<String>> {
    if vec.is_empty() {
        None
    } else {
        Some(vec.clone())
    }
}

fn into_optional_mapped_vec<T, O, F>(vec: &Vec<T>, f: F) -> Option<Vec<O>>
where
    T: Clone,
    F: Fn(T) -> O,
{
    if vec.is_empty() {
        None
    } else {
        Some(vec.clone().into_iter().map(f).collect())
    }
}

fn arch_from_str(arch: &str) -> Architecture {
    match arch {
        "amd64" => Architecture::Amd64,
        "arm64" | "aarch64" => Architecture::ARM64,
        "arm" => Architecture::ARM,
        "386" | "i386" => Architecture::I386,
        "ppc64le" => Architecture::PPC64le,
        "ppc64" => Architecture::PPC64,
        "mips64le" => Architecture::Mips64le,
        "mips64" => Architecture::Mips64,
        "mipsle" => Architecture::Mipsle,
        "mips" => Architecture::Mips,
        "s390x" => Architecture::S390x,
        _ => Architecture::Amd64,
    }
}

fn os_from_str(os: &str) -> OperatingSystem {
    match os {
        "linux" => OperatingSystem::Linux,
        "windows" => OperatingSystem::Windows,
        "darwin" => OperatingSystem::Darwin,
        "freebsd" => OperatingSystem::Freebsd,
        "dragonfly" => OperatingSystem::Dragonfly,
        "netbsd" => OperatingSystem::Netbsd,
        "openbsd" => OperatingSystem::Openbsd,
        "plan9" => OperatingSystem::Plan9,
        "solaris" => OperatingSystem::Solaris,
        _ => OperatingSystem::Linux,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_duration_units() {
        assert_eq!(parse_go_duration("30s"), Some(Duration::from_secs(30)));
        assert_eq!(parse_go_duration("500ms"), Some(Duration::from_millis(500)));
        assert_eq!(parse_go_duration("2h"), Some(Duration::from_secs(7200)));
        assert_eq!(parse_go_duration("100ns"), Some(Duration::from_nanos(100)));
        assert_eq!(parse_go_duration("5us"), Some(Duration::from_micros(5)));
    }

    #[test]
    fn parse_duration_compound() {
        assert_eq!(parse_go_duration("1m30s"), Some(Duration::from_secs(90)));
        assert_eq!(
            parse_go_duration("2h45m"),
            Some(Duration::from_secs(2 * 3600 + 45 * 60))
        );
        assert_eq!(parse_go_duration("1.5h"), Some(Duration::from_secs(5400)));
    }

    #[test]
    fn parse_duration_invalid() {
        assert_eq!(parse_go_duration(""), None);
        assert_eq!(parse_go_duration("abc"), None);
        assert_eq!(parse_go_duration("10"), None);
        assert_eq!(parse_go_duration("10x"), None);
    }

    #[test]
    fn healthcheck_translation() {
        let healthcheck = Healthcheck {
            cmd: "curl -f http://localhost/ || exit 1".to_string(),
            interval: Some("30s".to_string()),
            timeout: Some("5s".to_string()),
            start: Some("10s".to_string()),
            retries: Some(3),
        };

        let oci = to_healthcheck(&healthcheck);

        assert_eq!(
            oci.test,
            Some(vec![
                "CMD-SHELL".to_string(),
                "curl -f http://localhost/ || exit 1".to_string()
            ])
        );
        assert_eq!(oci.interval, Some(Duration::from_secs(30)));
        assert_eq!(oci.timeout, Some(Duration::from_secs(5)));
        assert_eq!(oci.start_period, Some(Duration::from_secs(10)));
        assert_eq!(oci.start_interval, None);
        assert_eq!(oci.retries, Some(3));
    }
}
