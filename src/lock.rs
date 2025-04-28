use crate::{dofigen_struct::*, DofigenContext, Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub(crate) const DOCKER_HUB_HOST: &str = "registry.hub.docker.com";
pub(crate) const DOCKER_IO_HOST: &str = "docker.io";
pub(crate) const DEFAULT_NAMESPACE: &str = "library";
pub(crate) const DEFAULT_TAG: &str = "latest";
pub(crate) const DEFAULT_PORT: u16 = 443;

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, PartialOrd, Eq)]
pub struct DockerTag {
    pub digest: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, PartialOrd, Eq)]
pub struct ResourceVersion {
    pub hash: String,
    pub content: String,
}

impl ImageName {
    pub fn fill(&self) -> Self {
        Self {
            host: self.host.clone().or(Some(DOCKER_IO_HOST.to_string())),
            port: self.port.clone().or(Some(DEFAULT_PORT)),
            version: self
                .version
                .clone()
                .or(Some(ImageVersion::Tag(DEFAULT_TAG.to_string()))),
            ..self.clone()
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LockFile {
    /// The effective Dofigen configuration
    pub effective: String,

    /// The digests of the images used in the Dofigen file
    /// The first level key is the host
    /// The second level key is the namespace
    /// The third level key is the repository
    /// The fourth level key is the tag
    pub images: HashMap<String, HashMap<String, HashMap<String, HashMap<String, DockerTag>>>>,

    /// The files used in the Dofigen file for 'extend' fields
    pub resources: HashMap<String, ResourceVersion>,
}

impl LockFile {
    fn images(&self) -> HashMap<ImageName, DockerTag> {
        let mut images = HashMap::new();
        for (host, namespaces) in self.images.clone() {
            let (host, port) = if host.contains(":") {
                let mut parts = host.split(":");
                (
                    parts.next().unwrap().to_string(),
                    Some(parts.next().unwrap().parse().unwrap()),
                )
            } else {
                (host, Some(DEFAULT_PORT))
            };
            // In order to do not create breaking changes, we replace the Docker hub host with docker.io
            let host = if host == DOCKER_HUB_HOST {
                DOCKER_IO_HOST.to_string()
            } else {
                host
            };
            for (namespace, repositories) in namespaces {
                for (repository, tags) in repositories {
                    let path = if namespace == DEFAULT_NAMESPACE {
                        repository.clone()
                    } else {
                        format!("{}/{}", namespace, repository)
                    };
                    for (tag, digest) in tags {
                        images.insert(
                            ImageName {
                                host: Some(host.clone()),
                                port,
                                path: path.clone(),
                                version: Some(ImageVersion::Tag(tag)),
                            },
                            digest,
                        );
                    }
                }
            }
        }
        images
    }

    fn resources(&self) -> HashMap<Resource, ResourceVersion> {
        self.resources
            .clone()
            .into_iter()
            .map(|(path, content)| (path.parse().unwrap(), content))
            .collect()
    }

    pub fn to_context(&self) -> DofigenContext {
        DofigenContext::from(self.resources(), self.images())
    }

    pub fn from_context(effective: &Dofigen, context: &DofigenContext) -> Result<LockFile> {
        let mut images = HashMap::new();
        for (image, docker_tag) in context.used_image_tags() {
            let host = image
                .host
                .ok_or(Error::Custom("Image host is not set".to_string()))?;
            let port = image
                .port
                .ok_or(Error::Custom("Image port is not set".to_string()))?;
            let host = if port == DEFAULT_PORT {
                host
            } else {
                format!("{}:{}", host, port)
            };
            let (namespace, repository) = if image.path.contains("/") {
                let mut parts = image.path.split("/");
                let namespace = parts.next().unwrap();
                let repository = parts.collect::<Vec<&str>>().join("/");
                (namespace, repository)
            } else {
                (DEFAULT_NAMESPACE, image.path)
            };
            let tag = match image.version.unwrap() {
                ImageVersion::Tag(tag) => Ok(tag),
                _ => Err(Error::Custom("Image version is not a tag".to_string())),
            }?;
            images
                .entry(host)
                .or_insert_with(HashMap::new)
                .entry(namespace.to_string())
                .or_insert_with(HashMap::new)
                .entry(repository.to_string())
                .or_insert_with(HashMap::new)
                .insert(tag, docker_tag);
        }

        let files = context
            .used_resource_contents()
            .iter()
            .map(|(resource, content)| (resource.to_string(), content.clone()))
            .collect();

        Ok(LockFile {
            effective: serde_yaml::to_string(effective).map_err(Error::from)?,
            images,
            resources: files,
        })
    }
}

pub trait Lock: Sized {
    fn lock(&self, context: &mut DofigenContext) -> Result<Self>;
}

impl<T> Lock for Option<T>
where
    T: Lock,
{
    fn lock(&self, context: &mut DofigenContext) -> Result<Self> {
        match self {
            Some(t) => Ok(Some(t.lock(context)?)),
            None => Ok(None),
        }
    }
}

impl<T> Lock for Vec<T>
where
    T: Lock,
{
    fn lock(&self, context: &mut DofigenContext) -> Result<Self> {
        self.iter().map(|t| t.lock(context)).collect()
    }
}

impl<K, V> Lock for HashMap<K, V>
where
    K: Eq + std::hash::Hash + Clone,
    V: Lock,
{
    fn lock(&self, context: &mut DofigenContext) -> Result<Self> {
        self.iter()
            .map(|(key, value)| {
                value
                    .lock(context)
                    .map(|locked_value| (key.clone(), locked_value))
            })
            .collect()
    }
}

impl Lock for Dofigen {
    fn lock(&self, context: &mut DofigenContext) -> Result<Self> {
        let mut stage = self.stage.lock(context)?;
        if !context.no_default_labels {
            stage.label.insert(
                "io.dofigen.version".into(),
                env!("CARGO_PKG_VERSION").into(),
            );
        }
        Ok(Self {
            builders: self.builders.lock(context)?,
            stage,
            ..self.clone()
        })
    }
}

impl Lock for Stage {
    fn lock(&self, context: &mut DofigenContext) -> Result<Self> {
        let mut label = self.label.clone();
        let from = match &self.from {
            FromContext::FromImage(image_name) => {
                let image_name_filled = image_name.fill();
                let version = image_name_filled.version.clone().ok_or(Error::Custom(
                    "Version must be set in filled image name".into(),
                ))?;
                FromContext::FromImage(match version {
                    ImageVersion::Tag(_) => {
                        label.insert(
                            "org.opencontainers.image.base.name".into(),
                            image_name_filled.to_string(),
                        );
                        let locked = image_name.lock(context)?;
                        match &locked.version {
                            Some(ImageVersion::Digest(digest)) => {
                                label.insert(
                                    "org.opencontainers.image.base.digest".into(),
                                    digest.clone(),
                                );
                            }
                            _ => unreachable!("Version must be a digest in locked image name"),
                        }
                        locked
                    }
                    ImageVersion::Digest(digest) => {
                        label.insert(
                            "org.opencontainers.image.base.digest".into(),
                            digest.clone(),
                        );
                        image_name_filled
                    }
                })
            }
            from => from.clone(),
        };
        Ok(Self {
            from,
            label,
            copy: self.copy.lock(context)?,
            run: self.run.lock(context)?,
            root: self
                .root
                .as_ref()
                .map(|root| root.lock(context))
                .transpose()?,
            ..self.clone()
        })
    }
}

impl Lock for FromContext {
    fn lock(&self, context: &mut DofigenContext) -> Result<Self> {
        match self {
            Self::FromImage(image_name) => Ok(Self::FromImage(image_name.lock(context)?)),
            other => Ok(other.clone()),
        }
    }
}

impl Lock for ImageName {
    fn lock(&self, context: &mut DofigenContext) -> Result<Self> {
        match &self.version {
            Some(ImageVersion::Digest(_)) => Ok(self.clone()),
            _ => Ok(Self {
                version: Some(ImageVersion::Digest(
                    context.get_image_tag(self)?.digest.clone(),
                )),
                ..self.clone()
            }),
        }
    }
}

impl Lock for CopyResource {
    fn lock(&self, context: &mut DofigenContext) -> Result<Self> {
        match self {
            Self::Copy(resource) => Ok(Self::Copy(resource.lock(context)?)),
            other => Ok(other.clone()),
        }
    }
}

impl Lock for Copy {
    fn lock(&self, context: &mut DofigenContext) -> Result<Self> {
        Ok(Self {
            from: self.from.lock(context)?,
            ..self.clone()
        })
    }
}

impl Lock for Run {
    fn lock(&self, context: &mut DofigenContext) -> Result<Self> {
        Ok(Self {
            bind: self.bind.lock(context)?,
            cache: self.cache.lock(context)?,
            ..self.clone()
        })
    }
}

impl Lock for Bind {
    fn lock(&self, context: &mut DofigenContext) -> Result<Self> {
        Ok(Self {
            from: self.from.lock(context)?,
            ..self.clone()
        })
    }
}

impl Lock for Cache {
    fn lock(&self, context: &mut DofigenContext) -> Result<Self> {
        Ok(Self {
            from: self.from.lock(context)?,
            ..self.clone()
        })
    }
}

impl Ord for DockerTag {
    fn cmp(&self, _other: &Self) -> std::cmp::Ordering {
        panic!("DockerTag cannot be ordered")
    }
}

impl Ord for ResourceVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.hash.cmp(&other.hash)
    }
}
