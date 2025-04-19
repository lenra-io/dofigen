use colored::{Color, Colorize};
use serde::Deserialize;

use crate::{
    lock::{DockerTag, ResourceVersion, DEFAULT_NAMESPACE, DOCKER_HUB_HOST},
    Dofigen, DofigenPatch, Error, Extend, ImageName, ImageVersion, Resource, Result,
};
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::Read,
    str::FromStr,
};

const MAX_LOAD_STACK_SIZE: usize = 10;

/// The representation of the Dofigen execution context
pub struct DofigenContext {
    pub offline: bool,
    pub update_file_resources: bool,
    pub update_url_resources: bool,
    pub update_docker_tags: bool,
    pub display_updates: bool,

    // Load resources
    load_resource_stack: Vec<Resource>,
    resources: HashMap<Resource, ResourceVersion>,
    used_resources: HashSet<Resource>,

    // Images tags
    images: HashMap<ImageName, DockerTag>,
    used_images: HashSet<ImageName>,
}

impl DofigenContext {
    //////////  Resource management  //////////

    pub(crate) fn current_resource(&self) -> Option<&Resource> {
        self.load_resource_stack.last()
    }

    pub(crate) fn push_resource_stack(&mut self, resource: Resource) -> Result<()> {
        let present = self.load_resource_stack.contains(&resource);
        self.load_resource_stack.push(resource);

        // check for circular dependencies
        if present {
            return Err(Error::Custom(format!(
                "Circular dependency detected while loading resource {}",
                self.load_resource_stack
                    .iter()
                    .map(|r| format!("{:?}", r))
                    .collect::<Vec<_>>()
                    .join(" -> "),
            )));
        }

        // check the stack size
        if self.load_resource_stack.len() > MAX_LOAD_STACK_SIZE {
            return Err(Error::Custom(format!(
                "Max load stack size exceeded while loading resource {}",
                self.load_resource_stack
                    .iter()
                    .map(|r| format!("{:?}", r))
                    .collect::<Vec<_>>()
                    .join(" -> "),
            )));
        }
        Ok(())
    }

    pub(crate) fn pop_resource_stack(&mut self) -> Option<Resource> {
        self.load_resource_stack.pop()
    }

    /// Get the content of a resource from cache if possible
    pub(crate) fn get_resource_content(&mut self, resource: Resource) -> Result<String> {
        let load = match resource {
            Resource::File(_) => self.update_file_resources,
            Resource::Url(_) => self.update_url_resources,
        } || !self.resources.contains_key(&resource);

        let version = if load {
            let version = self.load_resource_version(&resource)?;
            let previous = self.resources.insert(resource.clone(), version.clone());

            // display update
            if self.display_updates {
                let resource_name = resource.to_string();
                if let Some(previous) = previous {
                    if previous.hash != version.hash {
                        println!(
                            "{:>20} {} {} -> {}",
                            "Update resource".color(Color::Green).bold(),
                            resource_name,
                            previous.hash,
                            version.hash
                        );
                    }
                } else {
                    println!(
                        "{:>20} {} {}",
                        "Add resource".color(Color::Blue).bold(),
                        resource_name,
                        version.hash
                    );
                }
            }

            version
        } else {
            self.resources[&resource].clone()
        };

        let content = version.content.clone();
        self.used_resources.insert(resource);
        Ok(content)
    }

    /// Load the content of a resource
    fn load_resource_version(&self, resource: &Resource) -> Result<ResourceVersion> {
        let content = match resource.clone() {
            Resource::File(path) => fs::read_to_string(path.clone())
                .map_err(|err| Error::Custom(format!("Could not read file {:?}: {}", path, err)))?,
            Resource::Url(url) => {
                if self.offline {
                    return Err(Error::Custom(
                        "Offline mode can't load URL resources".to_string(),
                    ));
                }
                reqwest::blocking::get(url.as_ref())
                    .map_err(Error::from)?
                    .error_for_status()?
                    .text()
                    .map_err(Error::from)?
            }
        };
        let version = ResourceVersion {
            hash: sha256::digest(content.clone()),
            content: content.clone(),
        };
        Ok(version)
    }

    fn clean_unused_resources(&mut self) {
        for resource in self.resources.clone().keys() {
            if !self.used_resources.contains(resource) {
                let version = self.resources.remove(resource).unwrap();
                if self.display_updates {
                    println!(
                        "{:>20} {} {}",
                        "Remove image".color(Color::Red).bold(),
                        resource.to_string(),
                        version.hash
                    );
                }
            }
        }
    }

    //////////  Image management  //////////

    pub(crate) fn get_image_tag(&mut self, image: &ImageName) -> Result<DockerTag> {
        let image = image.fill();

        let tag = if self.update_docker_tags || !self.images.contains_key(&image) {
            let tag = self.load_image_tag(&image)?;
            let previous = self.images.insert(image.clone(), tag.clone());

            // display update
            if self.display_updates {
                let image_name = image.to_string();
                if let Some(previous) = previous {
                    if previous.digest != tag.digest {
                        println!(
                            "{:>20} {} {} -> {}",
                            "Update image".color(Color::Green).bold(),
                            image_name,
                            previous.digest,
                            tag.digest
                        );
                    }
                } else {
                    println!(
                        "{:>20} {} {}",
                        "Add image".color(Color::Blue).bold(),
                        image_name,
                        tag.digest
                    );
                }
            }

            tag
        } else {
            self.images[&image].clone()
        };

        self.used_images.insert(image.clone());
        Ok(tag)
    }

    fn load_image_tag(&mut self, image: &ImageName) -> Result<DockerTag> {
        if self.offline {
            return Err(Error::Custom(
                "Offline mode can't load image tag".to_string(),
            ));
        }

        let tag = match image
            .version
            .clone()
            .ok_or(Error::Custom("No version found for image".into()))?
        {
            ImageVersion::Tag(tag) => tag,
            _ => {
                return Err(Error::Custom("Image version is not a tag".to_string()));
            }
        };

        let host = image
            .host
            .clone()
            .ok_or(Error::Custom("No host found for image".into()))?;
        let port = image
            .port
            .clone()
            .ok_or(Error::Custom("No port found for image".into()))?;
        let port = if port == 443 {
            String::new()
        } else {
            format!(":{port}")
        };

        let client = reqwest::blocking::Client::new();

        let docker_tag = if self.load_from_api(host.as_str()) {
            let mut repo = image.path.clone();
            let namespace = if repo.contains("/") {
                let mut parts = image.path.split("/");
                let ret = parts.next().unwrap();
                repo = parts.collect::<Vec<&str>>().join("/");
                ret
            } else {
                DEFAULT_NAMESPACE
            };
            let request_url = format!(
                "https://{DOCKER_HUB_HOST}/v2/namespaces/{namespace}/repositories/{repo}/tags/{tag}",
                namespace = namespace,
                repo = repo,
                tag = tag
            );
            let response = client.get(&request_url).send().map_err(Error::from)?;

            let response: DockerHubTagResponse = response.json().map_err(Error::from)?;
            DockerTag {
                digest: response
                    .digest
                    .or(response.images.get(0).map(|img| img.digest.clone()))
                    .ok_or(Error::Custom("No digest found in response".to_string()))?,
            }
        } else {
            let request_url = format!(
                "https://{host}{port}/v2/{path}/manifests/{tag}",
                path = image.path,
                tag = tag
            );
            let response = client.head(&request_url).send().map_err(Error::from)?;

            let digest = response
                .headers()
                .get("Docker-Content-Digest")
                .ok_or(Error::Custom("No digest found in response".to_string()))?;
            let digest = digest
                .to_str()
                .map_err(|err| Error::display(err))?
                .to_string();

            DockerTag { digest }
        };

        Ok(docker_tag)
    }

    fn load_from_api(&self, host: &str) -> bool {
        host == DOCKER_HUB_HOST || host == "docker.io"
    }

    fn clean_unused_images(&mut self) {
        for image in self.images.clone().keys() {
            if !self.used_images.contains(image) {
                let tag = self.images.remove(image).unwrap();
                if self.display_updates {
                    println!(
                        "{:>20} {} {}",
                        "Remove image".color(Color::Red).bold(),
                        image.to_string(),
                        tag.digest
                    );
                }
            }
        }
    }

    /// The tags of the locked images
    pub(crate) fn get_locked_images_map(&self) -> HashMap<String, String> {
        self.images
            .iter()
            .map(|(image, tag)| {
                if let Some(ImageVersion::Tag(tag_name)) = &image.version {
                    let locked_image = ImageName {
                        version: Some(ImageVersion::Digest(tag.digest.clone())),
                        ..image.clone()
                    };
                    return (locked_image.to_string(), tag_name.clone());
                } else {
                    unreachable!("Image can only be a tag here")
                }
            })
            .collect()
    }

    //////////  Getters  //////////

    pub(crate) fn used_resource_contents(&self) -> HashMap<Resource, ResourceVersion> {
        self.used_resources
            .iter()
            .map(|res| (res.clone(), self.resources[res].clone()))
            .collect()
    }

    pub(crate) fn used_image_tags(&self) -> HashMap<ImageName, DockerTag> {
        self.used_images
            .iter()
            .map(|image| (image.clone(), self.images[image].clone()))
            .collect()
    }

    //////////  Comparison  //////////

    pub fn image_updates(
        &self,
        previous: &DofigenContext,
    ) -> Vec<UpdateCommand<ImageName, DockerTag>> {
        let mut updates = vec![];

        let mut previous_images = previous.images.clone();
        let current_images = self.used_image_tags();

        for (image, tag) in current_images {
            if let Some(previous_tag) = previous_images.remove(&image) {
                if tag.digest != previous_tag.digest {
                    updates.push(UpdateCommand::Update(image, tag, previous_tag))
                }
            } else {
                updates.push(UpdateCommand::Add(image, tag))
            }
        }

        for (image, tag) in previous_images {
            updates.push(UpdateCommand::Remove(image, tag));
        }

        updates.sort();

        updates
    }

    pub fn resource_updates(
        &self,
        previous: &DofigenContext,
    ) -> Vec<UpdateCommand<Resource, ResourceVersion>> {
        let mut updates = vec![];

        let mut previous_resources = previous.resources.clone();
        let current_resources = self.used_resource_contents();

        for (resource, version) in current_resources
            .into_iter()
            .filter(|(r, _)| matches!(r, Resource::Url(_)))
        {
            if let Some(previous_version) = previous_resources.remove(&resource) {
                if version.hash != previous_version.hash {
                    updates.push(UpdateCommand::Update(resource, version, previous_version))
                }
            } else {
                updates.push(UpdateCommand::Add(resource, version))
            }
        }

        for (resource, version) in previous_resources
            .into_iter()
            .filter(|(r, _)| matches!(r, Resource::Url(_)))
        {
            updates.push(UpdateCommand::Remove(resource, version));
        }

        updates.sort();

        updates
    }

    //////////  Dofigen parsing  //////////

    /// Parse an Dofigen from a string.
    ///
    /// # Examples
    ///
    /// Basic struct
    ///
    /// ```
    /// use dofigen_lib::*;
    /// use pretty_assertions_sorted::assert_eq_sorted;
    ///
    /// let yaml = "
    /// fromImage:
    ///   path: ubuntu
    /// ";
    /// let image: Dofigen = DofigenContext::new().parse_from_string(yaml).unwrap();
    /// assert_eq_sorted!(
    ///     image,
    ///     Dofigen {
    ///       stage: Stage {
    ///         from: ImageName {
    ///             path: String::from("ubuntu"),
    ///             ..Default::default()
    ///         }.into(),
    ///         ..Default::default()
    ///       },
    ///      ..Default::default()
    ///     }
    /// );
    /// ```
    ///
    /// Advanced image with builders and artifacts
    ///
    /// ```
    /// use dofigen_lib::*;
    /// use pretty_assertions_sorted::assert_eq_sorted;
    /// use std::collections::HashMap;
    ///
    /// let yaml = r#"
    /// builders:
    ///   builder:
    ///     fromImage:
    ///       path: ekidd/rust-musl-builder
    ///     copy:
    ///       - paths: ["*"]
    ///     run:
    ///       - cargo build --release
    /// fromImage:
    ///     path: ubuntu
    /// copy:
    ///   - fromBuilder: builder
    ///     paths:
    ///       - /home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust
    ///     target: /app
    /// "#;
    /// let image: Dofigen = DofigenContext::new().parse_from_string(yaml).unwrap();
    /// assert_eq_sorted!(
    ///     image,
    ///     Dofigen {
    ///         builders: HashMap::from([
    ///             ("builder".to_string(), Stage {
    ///                 from: ImageName { path: "ekidd/rust-musl-builder".into(), ..Default::default() }.into(),
    ///                 copy: vec![CopyResource::Copy(Copy{paths: vec!["*".into()].into(), ..Default::default()}).into()].into(),
    ///                 run: Run {
    ///                     run: vec!["cargo build --release".parse().unwrap()].into(),
    ///                     ..Default::default()
    ///                 },
    ///                 ..Default::default()
    ///             }),
    ///         ]),
    ///         stage: Stage {
    ///             from: ImageName {
    ///                 path: "ubuntu".into(),
    ///                 ..Default::default()
    ///             }.into(),
    ///             copy: vec![CopyResource::Copy(Copy{
    ///                 from: FromContext::FromBuilder(String::from("builder")),
    ///                 paths: vec![String::from(
    ///                     "/home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust"
    ///                 )],
    ///                 options: CopyOptions {
    ///                     target: Some(String::from("/app")),
    ///                     ..Default::default()
    ///                 },
    ///                 ..Default::default()
    ///             })].into(),
    ///             ..Default::default()
    ///         },
    ///         ..Default::default()
    ///     }
    /// );
    /// ```
    pub fn parse_from_string(&mut self, input: &str) -> Result<Dofigen> {
        self.merge_extended_image(
            serde_yaml::from_str(input).map_err(|err| Error::Deserialize(err))?,
        )
    }

    /// Parse an Dofigen from an IO stream.
    ///
    /// # Examples
    ///
    /// Basic struct
    ///
    /// ```
    /// use dofigen_lib::*;
    /// use pretty_assertions_sorted::assert_eq_sorted;
    ///
    /// let yaml = "
    /// fromImage:
    ///   path: ubuntu
    /// ";
    ///
    /// let image: Dofigen = DofigenContext::new().parse_from_reader(yaml.as_bytes()).unwrap();
    /// assert_eq_sorted!(
    ///     image,
    ///     Dofigen {
    ///         stage: Stage {
    ///             from: ImageName {
    ///                 path: String::from("ubuntu"),
    ///                 ..Default::default()
    ///             }.into(),
    ///             ..Default::default()
    ///         },
    ///         ..Default::default()
    ///     }
    /// );
    /// ```
    ///
    /// Advanced image with builders and artifacts
    ///
    /// ```
    /// use dofigen_lib::*;
    /// use pretty_assertions_sorted::assert_eq_sorted;
    /// use std::collections::HashMap;
    ///
    /// let yaml = r#"
    /// builders:
    ///   builder:
    ///     fromImage:
    ///       path: ekidd/rust-musl-builder
    ///     copy:
    ///       - paths: ["*"]
    ///     run:
    ///       - cargo build --release
    /// fromImage:
    ///     path: ubuntu
    /// copy:
    ///   - fromBuilder: builder
    ///     paths:
    ///       - /home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust
    ///     target: /app
    /// "#;
    /// let image: Dofigen = DofigenContext::new().parse_from_reader(yaml.as_bytes()).unwrap();
    /// assert_eq_sorted!(
    ///     image,
    ///     Dofigen {
    ///         builders: HashMap::from([
    ///             ("builder".to_string(), Stage {
    ///                 from: ImageName { path: "ekidd/rust-musl-builder".into(), ..Default::default() }.into(),
    ///                 copy: vec![CopyResource::Copy(Copy{paths: vec!["*".into()].into(), ..Default::default()}).into()].into(),
    ///                 run: Run {
    ///                     run: vec!["cargo build --release".parse().unwrap()].into(),
    ///                     ..Default::default()
    ///                 },
    ///                 ..Default::default()
    ///             }),
    ///         ]),
    ///         stage: Stage {
    ///             from: ImageName {
    ///                 path: String::from("ubuntu"),
    ///                 ..Default::default()
    ///             }.into(),
    ///             copy: vec![CopyResource::Copy(Copy {
    ///                 from: FromContext::FromBuilder(String::from("builder")),
    ///                 paths: vec![String::from(
    ///                     "/home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust"
    ///                 )],
    ///                 options: CopyOptions {
    ///                     target: Some(String::from("/app")),
    ///                     ..Default::default()
    ///                 },
    ///                 ..Default::default()
    ///             })].into(),
    ///             ..Default::default()
    ///         },
    ///         ..Default::default()
    ///     }
    /// );
    /// ```
    pub fn parse_from_reader<R: Read>(&mut self, reader: R) -> Result<Dofigen> {
        self.merge_extended_image(
            serde_yaml::from_reader(reader).map_err(|err| Error::Deserialize(err))?,
        )
    }

    /// Parse an Dofigen from a Resource (File or Url)
    ///
    /// # Example
    ///
    /// ```
    /// use dofigen_lib::*;
    /// use pretty_assertions_sorted::assert_eq_sorted;
    /// use std::path::PathBuf;
    ///
    /// let dofigen: Dofigen = DofigenContext::new().parse_from_resource(Resource::File(PathBuf::from("tests/cases/simple.yml"))).unwrap();
    /// assert_eq_sorted!(
    ///     dofigen,
    ///     Dofigen {
    ///         stage: Stage {
    ///             from: ImageName {
    ///                 path: String::from("alpine"),
    ///                 ..Default::default()
    ///             }.into(),
    ///             ..Default::default()
    ///         },
    ///         ..Default::default()
    ///     }
    /// );
    /// ```
    pub fn parse_from_resource(&mut self, resource: Resource) -> Result<Dofigen> {
        let dofigen = resource.load(self)?;
        self.merge_extended_image(dofigen)
    }

    fn merge_extended_image(&mut self, dofigen: Extend<DofigenPatch>) -> Result<Dofigen> {
        Ok(dofigen.merge(self)?.into())
    }

    pub fn clean_unused(&mut self) {
        self.clean_unused_resources();
        self.clean_unused_images();
    }

    //////////  Constructors  //////////

    pub fn new() -> Self {
        Self {
            offline: false,
            update_docker_tags: false,
            update_file_resources: true,
            update_url_resources: false,
            display_updates: true,
            load_resource_stack: vec![],
            resources: HashMap::new(),
            used_resources: HashSet::new(),
            images: HashMap::new(),
            used_images: HashSet::new(),
        }
    }

    pub fn from(
        resources: HashMap<Resource, ResourceVersion>,
        images: HashMap<ImageName, DockerTag>,
    ) -> Self {
        Self {
            offline: false,
            update_docker_tags: false,
            update_file_resources: true,
            update_url_resources: false,
            display_updates: true,
            load_resource_stack: vec![],
            resources,
            used_resources: HashSet::new(),
            images,
            used_images: HashSet::new(),
        }
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq, PartialOrd, Eq)]
pub struct DockerHubTagResponse {
    pub digest: Option<String>,
    images: Vec<DockerTag>,
}

#[derive(PartialEq, PartialOrd, Eq)]
pub enum UpdateCommand<K, V> {
    Update(K, V, V),
    Add(K, V),
    Remove(K, V),
}

impl<K, V> Ord for UpdateCommand<K, V>
where
    K: Ord,
    V: Ord,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (
                UpdateCommand::Add(a, _)
                | UpdateCommand::Update(a, _, _)
                | UpdateCommand::Remove(a, _),
                UpdateCommand::Add(b, _)
                | UpdateCommand::Update(b, _, _)
                | UpdateCommand::Remove(b, _),
            ) => a.cmp(b),
        }
    }
}

impl Ord for ImageName {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.to_string().cmp(&other.to_string())
    }
}

impl Ord for Resource {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Resource::File(a), Resource::File(b)) => a.cmp(b),
            (Resource::Url(a), Resource::Url(b)) => a.cmp(b),
            (Resource::File(_), Resource::Url(_)) => std::cmp::Ordering::Less,
            (Resource::Url(_), Resource::File(_)) => std::cmp::Ordering::Greater,
        }
    }
}

impl FromStr for Resource {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        if s.starts_with("http://") || s.starts_with("https://") {
            Ok(Resource::Url(s.parse().map_err(Error::display)?))
        } else {
            Ok(Resource::File(s.into()))
        }
    }
}
