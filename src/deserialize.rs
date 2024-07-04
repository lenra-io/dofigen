use crate::{
    serde_permissive::StringOrStruct, Add, AddGitRepo, Chown, Copy, CopyResources, ImageName,
    ImageVersion, StageFrom,
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

impl_Stage!(for ImageName, StageFrom, CopyResources, Copy);

const GIT_HTTP_REPO_REGEX: &str = "https?://(?:.+@)?[a-zA-Z0-9_-]+(?:\\.[a-zA-Z0-9_-]+)+/[a-zA-Z0-9_-]+/[a-zA-Z0-9_-]+\\.git(?:#[a-zA-Z0-9_/.-]*(?::[a-zA-Z0-9_/-]+)?)?";
const GIT_SSH_REPO_REGEX: &str = "[a-zA-Z0-9_-]+@[a-zA-Z0-9_-]+(?:\\.[a-zA-Z0-9_-]+)+:[a-zA-Z0-9_.-]+/[a-zA-Z0-9_.-]+(?:#[a-zA-Z0-9_/.-]+)?(?::[a-zA-Z0-9_/-]+)?";
const URL_REGEX: &str = "https?://(?:.+@)?[a-zA-Z0-9_-]+(?:\\.[a-zA-Z0-9_-]+)+(/[a-zA-Z0-9_.-]+)*";

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

impl FromStr for CopyResources {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let parts_regex = format!(
            r"^(?:(?<git>(?:{git_http}|{git_ssh}))|(?<url>{url})|\S+)(?: (?:{git_http}|{git_ssh}|{url}|\S+))*(?: \S+)?$",
            git_http = GIT_HTTP_REPO_REGEX,
            git_ssh = GIT_SSH_REPO_REGEX,
            url = URL_REGEX
        );
        let regex = Regex::new(parts_regex.as_str()).unwrap();
        let Some(captures) = regex.captures(s) else {
            return Err(Error::custom("Not matching copy resources pattern"));
        };
        if captures.name("git").is_some() {
            return Ok(CopyResources::AddGitRepo(s.parse().unwrap()));
        }
        if captures.name("url").is_some() {
            return Ok(CopyResources::Add(s.parse().unwrap()));
        }
        Ok(CopyResources::Copy(s.parse().unwrap()))
    }
}

impl FromStr for Copy {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let mut parts: Vec<String> = s.split(" ").map(|s| s.to_string()).collect();
        let target = if parts.len() > 1 { parts.pop() } else { None };
        Ok(Copy {
            paths: parts.clone(),
            target: target,
            ..Default::default()
        })
    }
}

impl FromStr for AddGitRepo {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let (repo, target) = match &s.split(" ").collect::<Vec<&str>>().as_slice() {
            &[repo, target] => (repo.to_string(), Some(target.to_string())),
            &[repo] => (repo.to_string(), None),
            _ => return Err(Error::custom("Invalid add git repo format")),
        };
        Ok(AddGitRepo {
            repo: repo,
            target: target,
            ..Default::default()
        })
    }
}

impl FromStr for Add {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let mut parts: Vec<String> = s.split(" ").map(|s| s.to_string()).collect();
        let target = if parts.len() > 1 { parts.pop() } else { None };
        Ok(Add {
            paths: parts,
            target: target,
            ..Default::default()
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
            let result = Copy::from_str("src").unwrap();
            assert_eq!(result.paths, vec!["src".to_string()]);
            assert!(result.target.is_none());
            assert!(result.chown.is_none());
            assert!(result.chmod.is_none());
            assert!(result.exclude.is_none());
            assert!(result.link.is_none());
            assert!(result.parents.is_none());
            assert!(result.from.is_none());
        }

        #[test]
        fn with_target_option() {
            let result = Copy::from_str("src /app").unwrap();
            assert_eq!(result.paths, vec!["src".to_string()]);
            assert_eq!(result.target, Some("/app".to_string()));
            assert!(result.chown.is_none());
            assert!(result.chmod.is_none());
            assert!(result.exclude.is_none());
            assert!(result.link.is_none());
            assert!(result.parents.is_none());
            assert!(result.from.is_none());
        }

        #[test]
        fn with_multiple_sources_and_target() {
            let result = Copy::from_str("src1 src2 /app").unwrap();
            assert_eq!(result.paths, vec!["src1".to_string(), "src2".to_string()]);
            assert_eq!(result.target, Some("/app".to_string()));
            assert!(result.chown.is_none());
            assert!(result.chmod.is_none());
            assert!(result.exclude.is_none());
            assert!(result.link.is_none());
            assert!(result.parents.is_none());
            assert!(result.from.is_none());
        }
    }

    mod add_git_repo {
        use super::*;

        #[test]
        fn ssh() {
            let result = AddGitRepo::from_str("git@github.com:lenra-io/dofigen.git").unwrap();
            assert_eq!(result.repo, "git@github.com:lenra-io/dofigen.git");
            assert!(result.target.is_none());
            assert!(result.chown.is_none());
            assert!(result.chmod.is_none());
            assert!(result.exclude.is_none());
            assert!(result.link.is_none());
            assert!(result.keep_git_dir.is_none());
        }

        #[test]
        fn ssh_with_target() {
            let result = AddGitRepo::from_str("git@github.com:lenra-io/dofigen.git /app").unwrap();
            assert_eq!(result.repo, "git@github.com:lenra-io/dofigen.git");
            assert_eq!(result.target, Some("/app".to_string()));
            assert!(result.chown.is_none());
            assert!(result.chmod.is_none());
            assert!(result.exclude.is_none());
            assert!(result.link.is_none());
            assert!(result.keep_git_dir.is_none());
        }

        #[test]
        fn http() {
            let result = AddGitRepo::from_str("https://github.com/lenra-io/dofigen.git").unwrap();
            assert_eq!(result.repo, "https://github.com/lenra-io/dofigen.git");
            assert!(result.target.is_none());
            assert!(result.chown.is_none());
            assert!(result.chmod.is_none());
            assert!(result.exclude.is_none());
            assert!(result.link.is_none());
            assert!(result.keep_git_dir.is_none());
        }

        #[test]
        fn http_with_target() {
            let result =
                AddGitRepo::from_str("https://github.com/lenra-io/dofigen.git /app").unwrap();
            assert_eq!(result.repo, "https://github.com/lenra-io/dofigen.git");
            assert_eq!(result.target, Some("/app".to_string()));
            assert!(result.chown.is_none());
            assert!(result.chmod.is_none());
            assert!(result.exclude.is_none());
            assert!(result.link.is_none());
            assert!(result.keep_git_dir.is_none());
        }
    }

    mod add {
        use super::*;

        #[test]
        fn simple() {
            let result =
                Add::from_str("https://github.com/lenra-io/dofigen/raw/main/README.md").unwrap();
            assert_eq!(
                result.paths,
                vec!["https://github.com/lenra-io/dofigen/raw/main/README.md".to_string()]
            );
            assert!(result.target.is_none());
            assert!(result.chown.is_none());
            assert!(result.chmod.is_none());
            assert!(result.link.is_none());
        }

        #[test]
        fn with_target_option() {
            let result =
                Add::from_str("https://github.com/lenra-io/dofigen/raw/main/README.md /app")
                    .unwrap();
            assert_eq!(
                result.paths,
                vec!["https://github.com/lenra-io/dofigen/raw/main/README.md".to_string()]
            );
            assert_eq!(result.target, Some("/app".to_string()));
            assert!(result.chown.is_none());
            assert!(result.chmod.is_none());
            assert!(result.link.is_none());
        }

        #[test]
        fn with_multiple_sources_and_target() {
            let result = Add::from_str("https://github.com/lenra-io/dofigen/raw/main/README.md https://github.com/lenra-io/dofigen/raw/main/LICENSE /app").unwrap();
            assert_eq!(
                result.paths,
                vec![
                    "https://github.com/lenra-io/dofigen/raw/main/README.md".to_string(),
                    "https://github.com/lenra-io/dofigen/raw/main/LICENSE".to_string()
                ]
            );
            assert_eq!(result.target, Some("/app".to_string()));
            assert!(result.chown.is_none());
            assert!(result.chmod.is_none());
            assert!(result.link.is_none());
        }
    }

    mod copy_resources {
        use super::*;

        #[test]
        fn copy() {
            let result = CopyResources::from_str("src").unwrap();
            assert_eq!(result, CopyResources::Copy(Copy::from_str("src").unwrap()));
        }

        #[test]
        fn add_git_repo_ssh() {
            let result = CopyResources::from_str("git@github.com:lenra-io/dofigen.git").unwrap();
            assert_eq!(
                result,
                CopyResources::AddGitRepo(
                    AddGitRepo::from_str("git@github.com:lenra-io/dofigen.git").unwrap()
                )
            );
        }

        #[test]
        fn add_git_repo_http() {
            let result =
                CopyResources::from_str("https://github.com/lenra-io/dofigen.git").unwrap();
            assert_eq!(
                result,
                CopyResources::AddGitRepo(
                    AddGitRepo::from_str("https://github.com/lenra-io/dofigen.git").unwrap()
                )
            );
        }

        #[test]
        fn add() {
            let result =
                CopyResources::from_str("https://github.com/lenra-io/dofigen/raw/main/README.md")
                    .unwrap();
            assert_eq!(
                result,
                CopyResources::Add(
                    Add::from_str("https://github.com/lenra-io/dofigen/raw/main/README.md")
                        .unwrap()
                )
            );
        }
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
