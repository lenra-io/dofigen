use std::collections::{BTreeMap, HashMap};

use buildkit_frontend::oci::*;
use dofigen_lib::{Dofigen, Port, PortProtocol, User};

pub trait ImageSpecificationExt {
    fn image_specification(&self) -> ImageSpecification;
}

impl ImageSpecificationExt for Dofigen {
    fn image_specification(&self) -> ImageSpecification {
        ImageSpecification {
            created: None,
            author: None,

            architecture: Architecture::Amd64,
            os: OperatingSystem::Linux,

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
            }),

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
