use crate::{
    serde_permissive::StringOrStruct, Chown, Copy, CopyResources, ImageName, ImageVersion,
};
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

impl FromStr for CopyResources {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        // todo!("implement CopyResources from_str");
        Ok(CopyResources::Copy(s.parse().unwrap()))
    }
}

impl FromStr for Copy {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
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

impl FromStr for Chown {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let regex = Regex::new(r"^(?<user>[a-zA-Z0-9_]+)(?::(?<group>[a-zA-Z0-9_]+))?$").unwrap();
        let Some(captures) = regex.captures(s) else {
            return Err(Error::custom("Not matching chown pattern"));
        };
        Ok(Chown {
            user: captures["user"].into(),
            group: captures.name("group").map(|m| m.as_str().into()),
        })
    }
}

impl FromStr for ImageName {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
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
                (Some(":"), Some(value)) => Some(ImageVersion::Tag(value.as_str().into())),
                (Some("@"), Some(value)) => Some(ImageVersion::Digest(value.as_str().into())),
                (None, None) => None,
                _ => return Err(Error::custom("Invalid version format")),
            },
        })
    }
}

#[cfg(test)]
mod test_from_str {
    use super::*;

    mod image_name {
        use super::*;

        #[test]
        fn simple() {
            let input = "example/image";
            let result = ImageName::from_str(input).unwrap();
            assert!(result.host.is_none());
            assert_eq!(result.path, "example/image");
            assert!(result.port.is_none());
            assert!(result.version.is_none());
        }

        #[test]
        fn with_host() {
            let input = "docker.io/example/image";
            let result = ImageName::from_str(input).unwrap();
            assert_eq!(result.host, Some("docker.io".into()));
            assert_eq!(result.path, "example/image");
            assert!(result.port.is_none());
            assert!(result.version.is_none());
        }

        #[test]
        fn with_tag() {
            let input = "example/image:tag";
            let result = ImageName::from_str(input).unwrap();
            assert!(result.host.is_none());
            assert_eq!(result.path, "example/image");
            assert!(result.port.is_none());
            assert_eq!(result.version, Some(ImageVersion::Tag("tag".into())));
        }

        #[test]
        fn with_digest() {
            let input = "example/image@sha256:my-sha";
            let result = ImageName::from_str(input).unwrap();
            assert!(result.host.is_none());
            assert_eq!(result.path, "example/image");
            assert!(result.port.is_none());
            assert_eq!(
                result.version,
                Some(ImageVersion::Digest("sha256:my-sha".into()))
            );
        }

        #[test]
        fn full() {
            let input = "registry.my-host.io:5001/example/image:stable";
            let result = ImageName::from_str(input).unwrap();
            assert_eq!(result.host, Some("registry.my-host.io".into()));
            assert_eq!(result.path, "example/image");
            assert_eq!(result.port, Some(5001));
            assert_eq!(result.version, Some(ImageVersion::Tag("stable".into())));
        }
    }

    mod copy {
        use super::*;

        #[test]
        fn simple() {
            let input = "src";
            let result = Copy::from_str(input).unwrap();
            assert_eq!(result.paths, vec!["src".to_string()]);
            assert!(result.target.is_none());
            assert!(result.chown.is_none());
            assert!(result.chmod.is_none());
            assert!(result.exclude.is_none());
            assert!(result.link.is_none());
            assert!(result.parents.is_none());
            assert!(result.from.is_none());
        }

        // #[test]
        // fn with_target_option() {
        //     let input = "src /app";
        //     let result = Copy::from_str(input).unwrap();
        //     assert_eq!(result.paths, vec!["src".to_string()]);
        //     assert_eq!(result.target, Some("/app".to_string()));
        //     assert!(result.chown.is_none());
        //     assert!(result.chmod.is_none());
        //     assert!(result.exclude.is_none());
        //     assert!(result.link.is_none());
        //     assert!(result.parents.is_none());
        //     assert!(result.from.is_none());
        // }

        // #[test]
        // fn with_multiple_sources_and_target() {
        //     let input = "src1 src2 /app";
        //     let result = Copy::from_str(input).unwrap();
        //     assert_eq!(result.paths, vec!["src1".to_string(), "src2".to_string()]);
        //     assert_eq!(result.target, Some("/app".to_string()));
        //     assert!(result.chown.is_none());
        //     assert!(result.chmod.is_none());
        //     assert!(result.exclude.is_none());
        //     assert!(result.link.is_none());
        //     assert!(result.parents.is_none());
        //     assert!(result.from.is_none());
        // }

        // #[test]
        // fn with_chown_option() {
        //     let input = "src --chown=user:group";
        //     let result = Copy::from_str(input).unwrap();
        //     assert_eq!(result.paths, vec!["src".to_string()]);
        //     assert!(result.target.is_none());
        //     assert_eq!(result.chown, Some("user:group".parse().unwrap()));
        //     assert!(result.chmod.is_none());
        //     assert!(result.exclude.is_none());
        //     assert!(result.link.is_none());
        //     assert!(result.parents.is_none());
        //     assert!(result.from.is_none());
        // }

        // #[test]
        // fn with_chmod_option() {
        //     let input = "src --chmod=755";
        //     let result = Copy::from_str(input).unwrap();
        //     assert_eq!(result.paths, vec!["src".to_string()]);
        //     assert!(result.target.is_none());
        //     assert!(result.chown.is_none());
        //     assert_eq!(result.chmod, Some("755".to_string()));
        //     assert!(result.exclude.is_none());
        //     assert!(result.link.is_none());
        //     assert!(result.parents.is_none());
        //     assert!(result.from.is_none());
        // }

        // #[test]
        // fn with_exclude_option() {
        //     let input = "src --exclude=.git";
        //     let result = Copy::from_str(input).unwrap();
        //     assert_eq!(result.paths, vec!["src".to_string()]);
        //     assert!(result.target.is_none());
        //     assert!(result.chown.is_none());
        //     assert!(result.chmod.is_none());
        //     assert_eq!(result.exclude, Some(vec![".git".to_string()]));
        //     assert!(result.link.is_none());
        //     assert!(result.parents.is_none());
        //     assert!(result.from.is_none());
        // }
    }

    mod chown {
        use super::*;

        #[test]
        fn user() {
            let result = Chown::from_str("user").unwrap();

            assert_eq!(result.user, "user");
            assert!(result.group.is_none());
        }

        #[test]
        fn with_group() {
            let result = Chown::from_str("user:group").unwrap();

            assert_eq!(result.user, "user");
            assert_eq!(result.group, Some("group".into()));
        }

        #[test]
        fn uid() {
            let result = Chown::from_str("1000").unwrap();

            assert_eq!(result.user, "1000");
            assert!(result.group.is_none());
        }

        #[test]
        fn uid_with_gid() {
            let result = Chown::from_str("1000:1000").unwrap();

            assert_eq!(result.user, "1000");
            assert_eq!(result.group, Some("1000".into()));
        }

        #[test]
        fn invalid() {
            let result = Chown::from_str("user:group:extra");

            assert!(result.is_err());
        }
    }
}
