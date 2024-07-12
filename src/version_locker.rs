use crate::{Error, ImageName, ImageVersion, Result};

const DOCKER_HUB_HOST: &str = "registry.hub.docker.com";
const DEFAULT_NAMESPACE: &str = "library";
const DEFAULT_TAG: &str = "latest";

#[derive(Debug, serde::Deserialize)]
pub struct Tag {
    // TODO: replace with a date type
    pub tag_last_pushed: String,
    pub digest: String,
}

impl ImageName {
    pub async fn load_digest(&self) -> Result<Tag> {
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
        println!("{}", request_url);
        let response = reqwest::get(&request_url).await.map_err(Error::from)?;

        let tag: Tag = response.json().await.map_err(Error::from)?;
        println!("{:?}", tag);
        Ok(tag)
    }
}
