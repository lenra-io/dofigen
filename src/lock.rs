use crate::{dofigen_struct::*, DofigenContext, Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const DOCKER_HUB_HOST: &str = "registry.hub.docker.com";
pub(crate) const DEFAULT_NAMESPACE: &str = "library";
const DEFAULT_TAG: &str = "latest";
const DEFAULT_PORT: u16 = 443;

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, PartialOrd, Eq)]
pub struct DockerTag {
    // TODO: replace with a date type
    pub tag_last_pushed: String,
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
            host: self.host.clone().or(Some(DOCKER_HUB_HOST.to_string())),
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
    pub image: String,

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
                (host, None)
            };
            for (namespace, repositories) in namespaces {
                for (repository, tags) in repositories {
                    for (tag, digest) in tags {
                        images.insert(
                            ImageName {
                                host: Some(host.clone()),
                                port,
                                path: format!("{}/{}", namespace, repository),
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

    pub fn from_context(effective_image: &Image, context: &DofigenContext) -> Result<LockFile> {
        let mut images = HashMap::new();
        for (image, docker_tag) in context.used_image_tags() {
            let host = format!("{}:{}", image.host.unwrap(), image.port.unwrap());
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
            image: serde_yaml::to_string(effective_image).map_err(Error::from)?,
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

impl Lock for Image {
    fn lock(&self, context: &mut DofigenContext) -> Result<Self> {
        Ok(Self {
            builders: self.builders.lock(context)?,
            stage: self.stage.lock(context)?,
            ..self.clone()
        })
    }
}

impl Lock for Stage {
    fn lock(&self, context: &mut DofigenContext) -> Result<Self> {
        Ok(Self {
            from: self.from.lock(context)?,
            ..self.clone()
        })
    }
}

impl Lock for ImageName {
    fn lock(&self, context: &mut DofigenContext) -> Result<Self> {
        match self.version.clone() {
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

impl Ord for DockerTag {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.tag_last_pushed.cmp(&other.tag_last_pushed)
    }
}

impl Ord for ResourceVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.hash.cmp(&other.hash)
    }
}