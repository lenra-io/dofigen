use crate::{
    Add, AddGitRepo, Copy, CopyOptions, CopyResource, ImageName, ImageVersion, Port, PortProtocol,
    Resource, User,
};
use regex::Regex;
use serde::de::{value::Error, Error as DeError};
use std::str::FromStr;
use url::Url;

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
            host: captures.name("host").map(|m| m.as_str().into()),
            port: captures.name("port").map(|m| m.as_str().parse().unwrap()),
            path: Some(captures["path"].into()),
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

impl FromStr for CopyResource {
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
            return Ok(CopyResource::AddGitRepo(s.parse().unwrap()));
        }
        if captures.name("url").is_some() {
            return Ok(CopyResource::Add(s.parse().unwrap()));
        }
        Ok(CopyResource::Copy(s.parse().unwrap()))
    }
}

impl FromStr for Copy {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let mut parts: Vec<String> = s.split(" ").map(|s| s.into()).collect();
        let target = if parts.len() > 1 { parts.pop() } else { None };
        Ok(Copy {
            paths: Some(parts.into()),
            options: CopyOptions {
                target: target,
                ..Default::default()
            },
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
            repo: Some(repo),
            options: CopyOptions {
                target: target,
                ..Default::default()
            },
            ..Default::default()
        })
    }
}

impl FromStr for Add {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let mut parts: Vec<_> = s.split(" ").collect();
        let target = if parts.len() > 1 {
            parts.pop().map(str::to_string)
        } else {
            None
        };
        let parts: Vec<_> = parts
            .iter()
            .map(|s| {
                Url::parse(s)
                    .map(Resource::Url)
                    .ok()
                    .unwrap_or(Resource::File(s.into()))
            })
            .collect();
        Ok(Add {
            files: Some(parts.into()),
            options: CopyOptions {
                target: target,
                ..Default::default()
            },
            ..Default::default()
        })
    }
}

impl FromStr for User {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let regex = Regex::new(r"^(?<user>[a-zA-Z0-9_]+)(?::(?<group>[a-zA-Z0-9_]+))?$").unwrap();
        let Some(captures) = regex.captures(s) else {
            return Err(Error::custom("Not matching chown pattern"));
        };
        Ok(User {
            user: Some(captures["user"].into()),
            group: captures.name("group").map(|m| m.as_str().into()),
        })
    }
}

impl FromStr for Port {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let regex = Regex::new(r"^(?<port>\d+)(?:/(?<protocol>(tcp|udp)))?$").unwrap();
        let Some(captures) = regex.captures(s) else {
            return Err(Error::custom("Not matching chown pattern"));
        };
        Ok(Port {
            port: Some(captures["port"].parse().map_err(Error::custom)?),
            protocol: captures.name("protocol").map(|m| match m.as_str() {
                "tcp" => PortProtocol::Tcp,
                "udp" => PortProtocol::Udp,
                _ => unreachable!(),
            }),
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
            assert_eq!(result.path.unwrap(), "example/image");
            assert!(result.port.is_none());
            assert!(result.version.is_none());
        }

        #[test]
        fn with_host() {
            let input = "docker.io/example/image";
            let result = ImageName::from_str(input).unwrap();
            assert_eq!(result.host, Some("docker.io".into()));
            assert_eq!(result.path.unwrap(), "example/image");
            assert!(result.port.is_none());
            assert!(result.version.is_none());
        }

        #[test]
        fn with_tag() {
            let input = "example/image:tag";
            let result = ImageName::from_str(input).unwrap();
            assert!(result.host.is_none());
            assert_eq!(result.path.unwrap(), "example/image");
            assert!(result.port.is_none());
            assert_eq!(result.version, Some(ImageVersion::Tag("tag".into())));
        }

        #[test]
        fn with_digest() {
            let input = "example/image@sha256:my-sha";
            let result = ImageName::from_str(input).unwrap();
            assert!(result.host.is_none());
            assert_eq!(result.path.unwrap(), "example/image");
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
            assert_eq!(result.path.unwrap(), "example/image");
            assert_eq!(result.port, Some(5001));
            assert_eq!(result.version, Some(ImageVersion::Tag("stable".into())));
        }
    }

    fn check_empty_copy_options(options: &CopyOptions) {
        assert!(options.target.is_none());
        assert!(options.chown.is_none());
        assert!(options.chmod.is_none());
        assert!(options.link.is_none());
    }

    mod copy {
        use super::*;

        #[test]
        fn simple() {
            let result = Copy::from_str("src").unwrap();
            assert_eq!(result.paths.unwrap(), vec!["src".into()].into());
            check_empty_copy_options(&result.options);
            assert!(result.exclude.is_none());
            assert!(result.parents.is_none());
            assert!(result.from.is_none());
        }

        #[test]
        fn with_target_option() {
            let result = Copy::from_str("src /app").unwrap();
            assert_eq!(result.paths.unwrap(), vec!["src".into()].into());
            assert_eq!(result.options.target, Some("/app".into()));
            assert!(result.options.chown.is_none());
            assert!(result.options.chmod.is_none());
            assert!(result.options.link.is_none());
            assert!(result.exclude.is_none());
            assert!(result.parents.is_none());
            assert!(result.from.is_none());
        }

        #[test]
        fn with_multiple_sources_and_target() {
            let result = Copy::from_str("src1 src2 /app").unwrap();
            assert_eq!(
                result.paths.unwrap(),
                vec!["src1".into(), "src2".into()].into()
            );
            assert_eq!(result.options.target, Some("/app".into()));
            assert!(result.options.chown.is_none());
            assert!(result.options.chmod.is_none());
            assert!(result.options.link.is_none());
            assert!(result.exclude.is_none());
            assert!(result.parents.is_none());
            assert!(result.from.is_none());
        }
    }

    mod add_git_repo {
        use super::*;

        #[test]
        fn ssh() {
            let result = AddGitRepo::from_str("git@github.com:lenra-io/dofigen.git").unwrap();
            assert_eq!(result.repo.unwrap(), "git@github.com:lenra-io/dofigen.git");
            check_empty_copy_options(&result.options);
            assert!(result.exclude.is_none());
            assert!(result.keep_git_dir.is_none());
        }

        #[test]
        fn ssh_with_target() {
            let result = AddGitRepo::from_str("git@github.com:lenra-io/dofigen.git /app").unwrap();
            assert_eq!(result.repo.unwrap(), "git@github.com:lenra-io/dofigen.git");
            assert_eq!(result.options.target, Some("/app".into()));
            assert!(result.options.chown.is_none());
            assert!(result.options.chmod.is_none());
            assert!(result.options.link.is_none());
            assert!(result.exclude.is_none());
            assert!(result.keep_git_dir.is_none());
        }

        #[test]
        fn http() {
            let result = AddGitRepo::from_str("https://github.com/lenra-io/dofigen.git").unwrap();
            assert_eq!(
                result.repo.unwrap(),
                "https://github.com/lenra-io/dofigen.git"
            );
            check_empty_copy_options(&result.options);
            assert!(result.exclude.is_none());
            assert!(result.keep_git_dir.is_none());
        }

        #[test]
        fn http_with_target() {
            let result =
                AddGitRepo::from_str("https://github.com/lenra-io/dofigen.git /app").unwrap();
            assert_eq!(
                result.repo.unwrap(),
                "https://github.com/lenra-io/dofigen.git"
            );
            assert_eq!(result.options.target, Some("/app".into()));
            assert!(result.options.chown.is_none());
            assert!(result.options.chmod.is_none());
            assert!(result.options.link.is_none());
            assert!(result.exclude.is_none());
            assert!(result.keep_git_dir.is_none());
        }
    }

    mod add {
        use crate::Resource;

        use super::*;

        #[test]
        fn simple() {
            let result =
                Add::from_str("https://github.com/lenra-io/dofigen/raw/main/README.md").unwrap();
            assert_eq!(
                result.files.unwrap(),
                vec![Resource::Url(
                    "https://github.com/lenra-io/dofigen/raw/main/README.md"
                        .parse()
                        .unwrap()
                )]
                .into()
            );
            check_empty_copy_options(&result.options);
        }

        #[test]
        fn with_target_option() {
            let result =
                Add::from_str("https://github.com/lenra-io/dofigen/raw/main/README.md /app")
                    .unwrap();
            assert_eq!(
                result.files.unwrap(),
                vec![Resource::Url(
                    "https://github.com/lenra-io/dofigen/raw/main/README.md"
                        .parse()
                        .unwrap()
                )]
                .into()
            );
            assert_eq!(result.options.target, Some("/app".into()));
            assert!(result.options.chown.is_none());
            assert!(result.options.chmod.is_none());
            assert!(result.options.link.is_none());
        }

        #[test]
        fn with_multiple_sources_and_target() {
            let result = Add::from_str("https://github.com/lenra-io/dofigen/raw/main/README.md https://github.com/lenra-io/dofigen/raw/main/LICENSE /app").unwrap();
            assert_eq!(
                result.files.unwrap(),
                vec![
                    Resource::Url(
                        "https://github.com/lenra-io/dofigen/raw/main/README.md"
                            .parse()
                            .unwrap()
                    ),
                    Resource::Url(
                        "https://github.com/lenra-io/dofigen/raw/main/LICENSE"
                            .parse()
                            .unwrap()
                    )
                ]
                .into()
            );
            assert_eq!(result.options.target, Some("/app".into()));
            assert!(result.options.chown.is_none());
            assert!(result.options.chmod.is_none());
            assert!(result.options.link.is_none());
        }
    }

    mod copy_resources {
        use super::*;

        #[test]
        fn copy() {
            let result = CopyResource::from_str("src").unwrap();
            assert_eq!(result, CopyResource::Copy(Copy::from_str("src").unwrap()));
        }

        #[test]
        fn add_git_repo_ssh() {
            let result = CopyResource::from_str("git@github.com:lenra-io/dofigen.git").unwrap();
            assert_eq!(
                result,
                CopyResource::AddGitRepo(
                    AddGitRepo::from_str("git@github.com:lenra-io/dofigen.git").unwrap()
                )
            );
        }

        #[test]
        fn add_git_repo_http() {
            let result = CopyResource::from_str("https://github.com/lenra-io/dofigen.git").unwrap();
            assert_eq!(
                result,
                CopyResource::AddGitRepo(
                    AddGitRepo::from_str("https://github.com/lenra-io/dofigen.git").unwrap()
                )
            );
        }

        #[test]
        fn add() {
            let result =
                CopyResource::from_str("https://github.com/lenra-io/dofigen/raw/main/README.md")
                    .unwrap();
            assert_eq!(
                result,
                CopyResource::Add(
                    Add::from_str("https://github.com/lenra-io/dofigen/raw/main/README.md")
                        .unwrap()
                )
            );
        }
    }

    mod user {
        use super::*;

        #[test]
        fn user() {
            let result = User::from_str("user").unwrap();

            assert_eq!(result.user.unwrap(), "user");
            assert!(result.group.is_none());
        }

        #[test]
        fn with_group() {
            let result = User::from_str("user:group").unwrap();

            assert_eq!(result.user.unwrap(), "user");
            assert_eq!(result.group, Some("group".into()));
        }

        #[test]
        fn uid() {
            let result = User::from_str("1000").unwrap();

            assert_eq!(result.user.unwrap(), "1000");
            assert!(result.group.is_none());
        }

        #[test]
        fn uid_with_gid() {
            let result = User::from_str("1000:1000").unwrap();

            assert_eq!(result.user.unwrap(), "1000");
            assert_eq!(result.group, Some("1000".into()));
        }

        #[test]
        fn invalid() {
            let result = User::from_str("user:group:extra");

            assert!(result.is_err());
        }
    }

    mod port {
        use super::*;

        #[test]
        fn simple() {
            let result = Port::from_str("80").unwrap();

            assert_eq!(result.port.unwrap(), 80);
            assert!(result.protocol.is_none());
        }

        #[test]
        fn with_tcp_protocol() {
            let result = Port::from_str("80/tcp").unwrap();

            assert_eq!(result.port.unwrap(), 80);
            assert_eq!(result.protocol, Some(PortProtocol::Tcp));
        }

        #[test]
        fn with_udp_protocol() {
            let result = Port::from_str("80/udp").unwrap();

            assert_eq!(result.port.unwrap(), 80);
            assert_eq!(result.protocol, Some(PortProtocol::Udp));
        }

        #[test]
        fn invalid() {
            let result = Port::from_str("80/invalid");

            assert!(result.is_err());
        }
    }
}
