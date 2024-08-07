use crate::dofigen_struct::*;
use crate::serde_permissive::ParsableStruct;
use regex::Regex;
use serde::de::{value::Error, Error as DeError};
use std::str::FromStr;
use struct_patch::Patch;
use url::Url;

const GIT_HTTP_REPO_REGEX: &str = "https?://(?:.+@)?[a-zA-Z0-9_-]+(?:\\.[a-zA-Z0-9_-]+)+/[a-zA-Z0-9_-]+/[a-zA-Z0-9_-]+\\.git(?:#[a-zA-Z0-9_/.-]*(?::[a-zA-Z0-9_/-]+)?)?";
const GIT_SSH_REPO_REGEX: &str = "[a-zA-Z0-9_-]+@[a-zA-Z0-9_-]+(?:\\.[a-zA-Z0-9_-]+)+:[a-zA-Z0-9_.-]+/[a-zA-Z0-9_.-]+(?:#[a-zA-Z0-9_/.-]+)?(?::[a-zA-Z0-9_/-]+)?";
const URL_REGEX: &str = "https?://(?:.+@)?[a-zA-Z0-9_-]+(?:\\.[a-zA-Z0-9_-]+)+(/[a-zA-Z0-9_.-]+)*";

macro_rules! impl_parsable_patch {
    ($struct:ty, $patch:ty, $param:ident, $expression:expr) => {
        impl Patch<ParsableStruct<$patch>> for $struct {
            fn apply(&mut self, patch: ParsableStruct<$patch>) {
                self.apply(patch.0);
            }

            fn into_patch(self) -> ParsableStruct<$patch> {
                ParsableStruct(self.into_patch())
            }

            fn into_patch_by_diff(self, previous_struct: Self) -> ParsableStruct<$patch> {
                ParsableStruct(self.into_patch_by_diff(previous_struct))
            }

            fn new_empty_patch() -> ParsableStruct<$patch> {
                ParsableStruct(Self::new_empty_patch())
            }
        }

        impl From<ParsableStruct<$patch>> for $struct {
            fn from(value: ParsableStruct<$patch>) -> Self {
                value.0.into()
            }
        }

        impl FromStr for $patch {
            type Err = Error;

            fn from_str($param: &str) -> std::result::Result<Self, Self::Err> {
                $expression
            }
        }
    };
}

impl_parsable_patch!(ImageName, ImageNamePatch, s, {
    let regex = Regex::new(r"^(?:(?<host>[^:\/.]+(?:\.[^:\/.]+)+)(?::(?<port>\d{1,5}))?\/)?(?<path>[a-zA-Z0-9-]{1,63}(?:\/[a-zA-Z0-9-]{1,63})*)(?:(?<version_char>[:@])(?<version_value>[a-zA-Z0-9_.:-]{1,128}))?$").unwrap();
    let Some(captures) = regex.captures(s) else {
        return Err(Error::custom("Not matching image name pattern"));
    };
    Ok(ImageNamePatch {
        host: Some(captures.name("host").map(|m| m.as_str().into())),
        port: Some(captures.name("port").map(|m| m.as_str().parse().unwrap())),
        path: Some(captures["path"].into()),
        version: Some(
            match (
                captures.name("version_char").map(|m| m.as_str()),
                captures.name("version_value"),
            ) {
                (Some(":"), Some(value)) => Some(ImageVersion::Tag(value.as_str().into())),
                (Some("@"), Some(value)) => Some(ImageVersion::Digest(value.as_str().into())),
                (None, None) => None,
                _ => return Err(Error::custom("Invalid version format")),
            },
        ),
    })
});

impl_parsable_patch!(CopyResource, CopyResourcePatch, s, {
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
        return Ok(CopyResourcePatch::AddGitRepo(s.parse().unwrap()));
    }
    if captures.name("url").is_some() {
        return Ok(CopyResourcePatch::Add(s.parse().unwrap()));
    }
    Ok(CopyResourcePatch::Copy(s.parse().unwrap()))
});

impl_parsable_patch!(Copy, CopyPatch, s, {
    let mut parts: Vec<String> = s.split(" ").map(|s| s.into()).collect();
    let target = if parts.len() > 1 { parts.pop() } else { None };
    Ok(Self {
        paths: Some(parts.into()),
        options: Some(CopyOptionsPatch {
            target: Some(target),
            chmod: Some(None),
            chown: Some(None),
            link: Some(None),
        }),
        from: Some(None),
        exclude: Some(vec![].into()),
        parents: Some(None),
    })
});

impl_parsable_patch!(AddGitRepo, AddGitRepoPatch, s, {
    let (repo, target) = match &s.split(" ").collect::<Vec<&str>>().as_slice() {
        &[repo, target] => (repo.to_string(), Some(target.to_string())),
        &[repo] => (repo.to_string(), None),
        _ => return Err(Error::custom("Invalid add git repo format")),
    };
    Ok(Self {
        repo: Some(repo),
        options: Some(CopyOptionsPatch {
            target: Some(target),
            chmod: Some(None),
            chown: Some(None),
            link: Some(None),
        }),
        exclude: Some(vec![].into()),
        keep_git_dir: Some(None),
    })
});

impl_parsable_patch!(Add, AddPatch, s, {
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
    Ok(Self {
        files: Some(parts.into()),
        options: Some(CopyOptionsPatch {
            target: Some(target),
            chmod: Some(None),
            chown: Some(None),
            link: Some(None),
        }),
        checksum: Some(None),
    })
});

impl_parsable_patch!(User, UserPatch, s, {
    let regex = Regex::new(r"^(?<user>[a-zA-Z0-9_]+)(?::(?<group>[a-zA-Z0-9_]+))?$").unwrap();
    let Some(captures) = regex.captures(s) else {
        return Err(Error::custom("Not matching chown pattern"));
    };
    Ok(Self {
        user: Some(captures["user"].into()),
        group: Some(captures.name("group").map(|m| m.as_str().into())),
    })
});

impl_parsable_patch!(Port, PortPatch, s, {
    let regex = Regex::new(r"^(?<port>\d+)(?:/(?<protocol>(tcp|udp)))?$").unwrap();
    let Some(captures) = regex.captures(s) else {
        return Err(Error::custom("Not matching chown pattern"));
    };
    Ok(Self {
        port: Some(captures["port"].parse().map_err(Error::custom)?),
        protocol: Some(captures.name("protocol").map(|m| match m.as_str() {
            "tcp" => PortProtocol::Tcp,
            "udp" => PortProtocol::Udp,
            _ => unreachable!(),
        })),
    })
});

#[cfg(test)]
mod test_from_str {
    use super::*;
    use pretty_assertions_sorted::assert_eq_sorted;

    mod image_name {
        use pretty_assertions_sorted::assert_eq_sorted;

        use super::*;

        #[test]
        fn simple() {
            let input = "example/image";
            let result = ImageNamePatch::from_str(input).unwrap();
            assert_eq_sorted!(result.host, Some(None));
            assert_eq_sorted!(result.path, Some("example/image".into()));
            assert_eq_sorted!(result.port, Some(None));
            assert_eq_sorted!(result.version, Some(None));
        }

        #[test]
        fn with_host() {
            let input = "docker.io/example/image";
            let result = ImageNamePatch::from_str(input).unwrap();
            assert_eq_sorted!(result.host, Some(Some("docker.io".into())));
            assert_eq_sorted!(result.path, Some("example/image".into()));
            assert_eq_sorted!(result.port, Some(None));
            assert_eq_sorted!(result.version, Some(None));
        }

        #[test]
        fn with_tag() {
            let input = "example/image:tag";
            let result = ImageNamePatch::from_str(input).unwrap();
            assert_eq_sorted!(result.host, Some(None));
            assert_eq_sorted!(result.path, Some("example/image".into()));
            assert_eq_sorted!(result.port, Some(None));
            assert_eq_sorted!(result.version, Some(Some(ImageVersion::Tag("tag".into()))));
        }

        #[test]
        fn with_digest() {
            let input = "example/image@sha256:my-sha";
            let result = ImageNamePatch::from_str(input).unwrap();
            assert_eq_sorted!(result.host, Some(None));
            assert_eq_sorted!(result.path, Some("example/image".into()));
            assert_eq_sorted!(result.port, Some(None));
            assert_eq_sorted!(
                result.version,
                Some(Some(ImageVersion::Digest("sha256:my-sha".into())))
            );
        }

        #[test]
        fn full() {
            let input = "registry.my-host.io:5001/example/image:stable";
            let result = ImageNamePatch::from_str(input).unwrap();
            assert_eq_sorted!(result.host, Some(Some("registry.my-host.io".into())));
            assert_eq_sorted!(result.path, Some("example/image".into()));
            assert_eq_sorted!(result.port, Some(Some(5001)));
            assert_eq_sorted!(
                result.version,
                Some(Some(ImageVersion::Tag("stable".into())))
            );
        }
    }

    mod copy {

        use super::*;

        #[test]
        fn simple() {
            let result = CopyPatch::from_str("src").unwrap();
            assert_eq_sorted!(
                result,
                CopyPatch {
                    paths: Some(vec!["src".to_string()].into_patch()),
                    options: Some(CopyOptionsPatch {
                        target: Some(None),
                        chown: Some(None),
                        chmod: Some(None),
                        link: Some(None),
                    }),
                    exclude: Some(vec![].into()),
                    parents: Some(None),
                    from: Some(None),
                }
            );
        }

        #[test]
        fn with_target_option() {
            let result = CopyPatch::from_str("src /app").unwrap();
            assert_eq_sorted!(
                result,
                CopyPatch {
                    paths: Some(vec!["src".to_string()].into_patch()),
                    options: Some(CopyOptionsPatch {
                        target: Some(Some("/app".into())),
                        chown: Some(None),
                        chmod: Some(None),
                        link: Some(None),
                    }),
                    exclude: Some(vec![].into()),
                    parents: Some(None),
                    from: Some(None),
                }
            );
        }

        #[test]
        fn with_multiple_sources_and_target() {
            let result = CopyPatch::from_str("src1 src2 /app").unwrap();
            assert_eq_sorted!(
                result,
                CopyPatch {
                    paths: Some(vec!["src1".to_string(), "src2".to_string()].into_patch()),
                    options: Some(CopyOptionsPatch {
                        target: Some(Some("/app".into())),
                        chown: Some(None),
                        chmod: Some(None),
                        link: Some(None),
                    }),
                    exclude: Some(vec![].into()),
                    parents: Some(None),
                    from: Some(None),
                }
            );
        }
    }

    mod add_git_repo {

        use super::*;

        #[test]
        fn ssh() {
            let result = AddGitRepoPatch::from_str("git@github.com:lenra-io/dofigen.git").unwrap();
            assert_eq_sorted!(
                result,
                AddGitRepoPatch {
                    repo: Some("git@github.com:lenra-io/dofigen.git".into()),
                    options: Some(CopyOptionsPatch {
                        target: Some(None),
                        chown: Some(None),
                        chmod: Some(None),
                        link: Some(None),
                    }),
                    exclude: Some(vec![].into()),
                    keep_git_dir: Some(None),
                }
            );
        }

        #[test]
        fn ssh_with_target() {
            let result =
                AddGitRepoPatch::from_str("git@github.com:lenra-io/dofigen.git /app").unwrap();
            assert_eq_sorted!(
                result,
                AddGitRepoPatch {
                    repo: Some("git@github.com:lenra-io/dofigen.git".into()),
                    options: Some(CopyOptionsPatch {
                        target: Some(Some("/app".into())),
                        chown: Some(None),
                        chmod: Some(None),
                        link: Some(None),
                    }),
                    exclude: Some(vec![].into()),
                    keep_git_dir: Some(None),
                }
            );
        }

        #[test]
        fn http() {
            let result =
                AddGitRepoPatch::from_str("https://github.com/lenra-io/dofigen.git").unwrap();
            assert_eq_sorted!(
                result,
                AddGitRepoPatch {
                    repo: Some("https://github.com/lenra-io/dofigen.git".into()),
                    options: Some(CopyOptionsPatch {
                        target: Some(None),
                        chown: Some(None),
                        chmod: Some(None),
                        link: Some(None),
                    }),
                    exclude: Some(vec![].into()),
                    keep_git_dir: Some(None),
                }
            );
        }

        #[test]
        fn http_with_target() {
            let result =
                AddGitRepoPatch::from_str("https://github.com/lenra-io/dofigen.git /app").unwrap();
            assert_eq_sorted!(
                result,
                AddGitRepoPatch {
                    repo: Some("https://github.com/lenra-io/dofigen.git".into()),
                    options: Some(CopyOptionsPatch {
                        target: Some(Some("/app".into())),
                        chown: Some(None),
                        chmod: Some(None),
                        link: Some(None),
                    }),
                    exclude: Some(vec![].into()),
                    keep_git_dir: Some(None),
                }
            );
        }
    }

    mod add {
        use struct_patch::Patch;

        use crate::{CopyOptions, Resource};

        use super::*;

        #[test]
        fn simple() {
            let result =
                AddPatch::from_str("https://github.com/lenra-io/dofigen/raw/main/README.md")
                    .unwrap();
            assert_eq_sorted!(
                result,
                Add {
                    files: vec![Resource::Url(
                        "https://github.com/lenra-io/dofigen/raw/main/README.md"
                            .parse()
                            .unwrap()
                    )],
                    options: CopyOptions::default(),
                    ..Default::default()
                }
                .into_patch()
            );
        }

        #[test]
        fn with_target_option() {
            let result =
                AddPatch::from_str("https://github.com/lenra-io/dofigen/raw/main/README.md /app")
                    .unwrap();
            assert_eq_sorted!(
                result,
                Add {
                    files: vec![Resource::Url(
                        "https://github.com/lenra-io/dofigen/raw/main/README.md"
                            .parse()
                            .unwrap()
                    )],
                    options: CopyOptions {
                        target: Some("/app".into()),
                        ..Default::default()
                    },
                    ..Default::default()
                }
                .into_patch()
            );
        }

        #[test]
        fn with_multiple_sources_and_target() {
            let result = AddPatch::from_str("https://github.com/lenra-io/dofigen/raw/main/README.md https://github.com/lenra-io/dofigen/raw/main/LICENSE /app").unwrap();
            assert_eq_sorted!(
                result,
                Add {
                    files: vec![
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
                    ],
                    options: CopyOptions {
                        target: Some("/app".into()),
                        ..Default::default()
                    },
                    ..Default::default()
                }
                .into_patch()
            );
        }
    }

    mod copy_resources {
        use super::*;

        #[test]
        fn copy() {
            let result = CopyResourcePatch::from_str("src").unwrap();
            assert_eq_sorted!(
                result,
                CopyResourcePatch::Copy(CopyPatch::from_str("src").unwrap())
            );
        }

        #[test]
        fn add_git_repo_ssh() {
            let result =
                CopyResourcePatch::from_str("git@github.com:lenra-io/dofigen.git").unwrap();
            assert_eq_sorted!(
                result,
                CopyResourcePatch::AddGitRepo(
                    AddGitRepoPatch::from_str("git@github.com:lenra-io/dofigen.git").unwrap()
                )
            );
        }

        #[test]
        fn add_git_repo_http() {
            let result =
                CopyResourcePatch::from_str("https://github.com/lenra-io/dofigen.git").unwrap();
            assert_eq_sorted!(
                result,
                CopyResourcePatch::AddGitRepo(
                    AddGitRepoPatch::from_str("https://github.com/lenra-io/dofigen.git").unwrap()
                )
            );
        }

        #[test]
        fn add() {
            let result = CopyResourcePatch::from_str(
                "https://github.com/lenra-io/dofigen/raw/main/README.md",
            )
            .unwrap();
            assert_eq_sorted!(
                result,
                CopyResourcePatch::Add(
                    AddPatch::from_str("https://github.com/lenra-io/dofigen/raw/main/README.md")
                        .unwrap()
                )
            );
        }
    }

    mod user {
        use pretty_assertions_sorted::assert_eq_sorted;

        use super::*;

        #[test]
        fn user() {
            let result = UserPatch::from_str("user").unwrap();

            assert_eq_sorted!(result.user, Some("user".into()));
            assert_eq_sorted!(result.group, Some(None));
        }

        #[test]
        fn with_group() {
            let result = UserPatch::from_str("user:group").unwrap();

            assert_eq_sorted!(result.user, Some("user".into()));
            assert_eq_sorted!(result.group, Some(Some("group".into())));
        }

        #[test]
        fn uid() {
            let result = UserPatch::from_str("1000").unwrap();

            assert_eq_sorted!(result.user, Some("1000".into()));
            assert_eq_sorted!(result.group, Some(None));
        }

        #[test]
        fn uid_with_gid() {
            let result = UserPatch::from_str("1000:1000").unwrap();

            assert_eq_sorted!(result.user, Some("1000".into()));
            assert_eq_sorted!(result.group, Some(Some("1000".into())));
        }

        #[test]
        fn invalid() {
            let result = UserPatch::from_str("user:group:extra");

            assert!(result.is_err());
        }
    }

    mod port {

        use super::*;

        #[test]
        fn simple() {
            let result = PortPatch::from_str("80").unwrap();

            assert_eq_sorted!(result.port, Some(80));
            assert_eq_sorted!(result.protocol, Some(None));
        }

        #[test]
        fn with_tcp_protocol() {
            let result = PortPatch::from_str("80/tcp").unwrap();

            assert_eq_sorted!(result.port, Some(80));
            assert_eq_sorted!(result.protocol, Some(Some(PortProtocol::Tcp)));
        }

        #[test]
        fn with_udp_protocol() {
            let result = PortPatch::from_str("80/udp").unwrap();

            assert_eq_sorted!(result.port, Some(80));
            assert_eq_sorted!(result.protocol, Some(Some(PortProtocol::Udp)));
        }

        #[test]
        fn invalid() {
            let result = PortPatch::from_str("80/invalid");

            assert!(result.is_err());
        }
    }
}
