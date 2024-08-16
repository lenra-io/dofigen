use crate::{dofigen_struct::*, Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const DOCKER_HUB_HOST: &str = "registry.hub.docker.com";
const DEFAULT_NAMESPACE: &str = "library";
const DEFAULT_TAG: &str = "latest";
const DEFAULT_PORT: u16 = 443;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DockerTag {
    // TODO: replace with a date type
    pub tag_last_pushed: String,
    pub digest: String,
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

    fn load_digest(&self) -> Result<DockerTag> {
        let tag = match self.version.as_ref() {
            Some(ImageVersion::Tag(tag)) => tag,
            None => DEFAULT_TAG,
            _ => {
                return Err(Error::Custom("Image version is not a tag".to_string()));
            }
        };

        let mut repo = self.path.clone();
        let namespace = if repo.contains("/") {
            let mut parts = self.path.split("/");
            let ret = parts.next().unwrap();
            repo = parts.collect::<Vec<&str>>().join("/");
            ret
        } else {
            DEFAULT_NAMESPACE
        };
        let request_url = format!(
            "https://{host}/v2/namespaces/{namespace}/repositories/{repo}/tags/{tag}",
            host = self.host.as_ref().unwrap_or(&DOCKER_HUB_HOST.to_string()),
            namespace = namespace,
            repo = repo,
            tag = tag
        );
        let response = reqwest::blocking::get(&request_url).map_err(Error::from)?;

        let tag: DockerTag = response.json().map_err(Error::from)?;

        Ok(tag)
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
}

pub struct LockContext {
    pub images: HashMap<ImageName, DockerTag>,
}

impl From<LockFile> for LockContext {
    fn from(lockfile: LockFile) -> Self {
        let mut images = HashMap::new();
        for (host, namespaces) in lockfile.images {
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
        Self { images }
    }
}

impl LockContext {
    pub fn to_lockfile(self, effective_image: &Image) -> Result<LockFile> {
        let mut images = HashMap::new();
        for (image, docker_tag) in self.images {
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
                ImageVersion::Tag(tag) => tag,
                _ => DEFAULT_TAG.to_string(),
            };
            images
                .entry(host)
                .or_insert_with(HashMap::new)
                .entry(namespace.to_string())
                .or_insert_with(HashMap::new)
                .entry(repository.to_string())
                .or_insert_with(HashMap::new)
                .insert(tag, docker_tag);
        }
        Ok(LockFile {
            image: serde_yaml::to_string(effective_image).map_err(Error::from)?,
            images,
        })
    }
}

pub trait Lock: Sized {
    fn lock(&self, context: &mut LockContext) -> Result<Self>;
}

impl<T> Lock for Option<T>
where
    T: Lock,
{
    fn lock(&self, context: &mut LockContext) -> Result<Self> {
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
    fn lock(&self, context: &mut LockContext) -> Result<Self> {
        self.iter().map(|t| t.lock(context)).collect()
    }
}

impl Lock for Image {
    fn lock(&self, context: &mut LockContext) -> Result<Self> {
        Ok(Self {
            builders: self.builders.lock(context)?,
            stage: self.stage.lock(context)?,
            ..self.clone()
        })
    }
}

impl Lock for Stage {
    fn lock(&self, context: &mut LockContext) -> Result<Self> {
        Ok(Self {
            from: self.from.lock(context)?,
            ..self.clone()
        })
    }
}

impl Lock for ImageName {
    fn lock(&self, context: &mut LockContext) -> Result<Self> {
        match self.version.clone() {
            Some(ImageVersion::Digest(_)) => Ok(self.clone()),
            _ => {
                let filled = self.fill();
                if let Some(tag) = context.images.get(&filled) {
                    Ok(Self {
                        version: Some(ImageVersion::Digest(tag.digest.clone())),
                        ..self.clone()
                    })
                } else {
                    let tag = filled.load_digest()?;
                    context.images.insert(filled, tag.clone());
                    Ok(Self {
                        version: Some(ImageVersion::Digest(tag.digest)),
                        ..self.clone()
                    })
                }
            }
        }
    }
}
