use std::collections::{BTreeMap, HashMap};

use buildkit_frontend::oci::*;
use buildkit_llb::ops::platform::Platform;
use dofigen_lib::{Dofigen, Port, PortProtocol, User};

pub trait ImageSpecificationExt {
    fn image_specification(&self, platform: &Platform) -> ImageSpecification;
}

impl ImageSpecificationExt for Dofigen {
    fn image_specification(&self, platform: &Platform) -> ImageSpecification {
        ImageSpecification {
            // TODO: manage created date based on labels or use the current date if not specified
            created: None,
            // TODO: manage author based on labels
            author: None,

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
                // TODO: Manage stop signals
                stop_signal: None,
                ..Default::default()
            }),

            // TODO: handle healthcheck

            // TODO: we should be able to generate the rootfs and history from the stages
            rootfs: None,
            history: None,
        }
    }
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
