use crate::{serde_permissive::StringOrStruct, Copy, CopyResources, ImageName, ImageVersion};
use regex::Regex;
use serde::de::{value::Error, Error as DeError};
use std::str::FromStr;

macro_rules! impl_Stage {
    (for $($t:ty),+) => {
        $(impl From<StringOrStruct<$t>> for $t {
            fn from(s: StringOrStruct<$t>) -> Self {
                match s {
                    StringOrStruct::String(s) => s.parse().unwrap(),
                    StringOrStruct::Struct(s) => s,
                }
            }
        })*
    }
}

impl_Stage!(for ImageName, CopyResources, Copy);

impl Default for ImageVersion {
    fn default() -> Self {
        ImageVersion::Tag("latest".into())
    }
}

impl FromStr for CopyResources {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        print!("CopyResources: {}", s);
        Ok(CopyResources::Copy(s.parse().unwrap()))
    }
}

impl FromStr for Copy {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        print!("Copy: {}", s);
        Ok(Copy {
            paths: vec![s.into()],
            target: None,
            chown: None,
            chmod: None,
            exclude: None,
            link: None,
            parents: None,
            from: None,
        })
    }
}

impl FromStr for ImageName {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        print!("ImageName: {}", s);

        let regex = Regex::new(r"^(?:(?<host>[^:\/.]+(?:\.[^:\/.]+)+)(?::(?<port>\d{1,5}))?\/)?(?<path>[a-zA-Z0-9-]{1,63}(?:\/[a-zA-Z0-9-]{1,63})*)(?:(?<version_char>[:@])(?<version_value>[a-zA-Z0-9_.:-]{1,128}))?$").unwrap();
        let Some(captures) = regex.captures(s) else {
            return Err(Error::custom("Not matching image name pattern"));
        };
        Ok(ImageName {
            host: captures.name("host").map(|m| m.as_str().to_string()),
            port: captures.name("port").map(|m| m.as_str().parse().unwrap()),
            path: captures["path"].to_string(),
            version: match (
                captures.name("version_char").map(|m| m.as_str()),
                captures.name("version_value"),
            ) {
                (Some(":"), Some(value)) => ImageVersion::Tag(value.as_str().into()),
                (Some("@"), Some(value)) => ImageVersion::Digest(value.as_str().into()),
                (None, None) => ImageVersion::default(),
                _ => return Err(Error::custom("Invalid version format")),
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_name_from_str() {
        let input = "example/image";
        let result = ImageName::from_str(input).unwrap();
        assert!(result.host.is_none());
        assert_eq!(result.path, "example/image");
        assert!(result.port.is_none());
        assert_eq!(result.version, ImageVersion::Tag("latest".into()));
    }

    #[test]
    fn image_name_from_str_with_host() {
        let input = "docker.io/example/image";
        let result = ImageName::from_str(input).unwrap();
        assert_eq!(result.host, Some("docker.io".into()));
        assert_eq!(result.path, "example/image");
        assert!(result.port.is_none());
        assert_eq!(result.version, ImageVersion::Tag("latest".into()));
    }

    #[test]
    fn image_name_from_str_with_tag() {
        let input = "example/image:tag";
        let result = ImageName::from_str(input).unwrap();
        assert!(result.host.is_none());
        assert_eq!(result.path, "example/image");
        assert!(result.port.is_none());
        assert_eq!(result.version, ImageVersion::Tag("tag".into()));
    }

    #[test]
    fn image_name_from_str_with_digest() {
        let input = "example/image@sha256:my-sha";
        let result = ImageName::from_str(input).unwrap();
        assert!(result.host.is_none());
        assert_eq!(result.path, "example/image");
        assert!(result.port.is_none());
        assert_eq!(result.version, ImageVersion::Digest("sha256:my-sha".into()));
    }
}
