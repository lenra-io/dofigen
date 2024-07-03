use crate::{Add, AddGitRepo, Chown, Copy, CopyResources, ImageName, ImageVersion, Result};

pub trait DockerfileGenerator {
    fn to_dockerfile_content(&self) -> Result<String>;
}

impl DockerfileGenerator for ImageName {
    fn to_dockerfile_content(&self) -> Result<String> {
        let mut registry = String::new();
        if let Some(host) = &self.host {
            registry.push_str(host);
            if self.port.is_some() {
                registry.push_str(":");
                registry.push_str(self.port.unwrap().to_string().as_str());
            }
            registry.push_str("/");
        }
        let mut version = String::new();
        match &self.version {
            Some(ImageVersion::Tag(tag)) => {
                version.push_str(":");
                version.push_str(tag);
            }
            Some(ImageVersion::Digest(digest)) => {
                version.push_str("@");
                version.push_str(digest);
            }
            None => {}
        }
        Ok(format!("{registry}{path}{version}", path = self.path))
    }
}

impl DockerfileGenerator for CopyResources {
    fn to_dockerfile_content(&self) -> Result<String> {
        match self {
            CopyResources::Copy(copy) => copy.to_dockerfile_content(),
            CopyResources::Add(add_web_file) => add_web_file.to_dockerfile_content(),
            CopyResources::AddGitRepo(add_git_repo) => add_git_repo.to_dockerfile_content(),
        }
    }
}

impl DockerfileGenerator for Copy {
    fn to_dockerfile_content(&self) -> Result<String> {
        let paths = self.paths.clone().to_vec().join(" ");
        let target = self.target.clone().unwrap_or("./".to_string());
        let mut options = String::new();
        push_conditional_str_option(&mut options, "from", &self.from);
        push_chown_option(&mut options, &self.chown);
        push_conditional_str_option(&mut options, "chmod", &self.chmod);
        if let Some(exclude) = &self.exclude {
            for path in exclude.clone().to_vec() {
                push_str_option(&mut options, "exclude", &path);
            }
        }
        push_bool_option(&mut options, "link", &self.link.unwrap_or(true));
        push_conditional_bool_option(&mut options, "parents", &self.parents);
        Ok(format!("COPY{options} {paths} {target}"))
    }
}

impl DockerfileGenerator for Add {
    fn to_dockerfile_content(&self) -> Result<String> {
        let urls = self.paths.clone().to_vec().join(" ");
        let mut options = String::new();
        push_chown_option(&mut options, &self.chown);
        push_conditional_str_option(&mut options, "chmod", &self.chmod);
        push_bool_option(&mut options, "link", &self.link.unwrap_or(true));
        Ok(format!(
            "ADD{options} {urls} {target}",
            target = self.target.clone().unwrap_or(".".to_string())
        ))
    }
}

impl DockerfileGenerator for AddGitRepo {
    fn to_dockerfile_content(&self) -> Result<String> {
        let mut options = String::new();
        push_chown_option(&mut options, &self.chown);
        push_conditional_str_option(&mut options, "chmod", &self.chmod);
        if let Some(exclude) = &self.exclude {
            for path in exclude.clone().to_vec() {
                push_str_option(&mut options, "exclude", &path);
            }
        }
        push_bool_option(&mut options, "link", &self.link.unwrap_or(true));
        Ok(format!(
            "ADD{options} {repo} {target}",
            repo = self.repo,
            target = self.target.clone().unwrap_or(".".to_string())
        ))
    }
}

// Push option functions

fn push_chown_option(options: &mut String, chown: &Option<Chown>) {
    if let Some(c) = chown {
        options.push_str(" --chown=");
        options.push_str(c.user.as_str());
        if let Some(group) = &c.group {
            options.push_str(":");
            options.push_str(group);
        }
    }
}

fn push_conditional_str_option(options: &mut String, name: &str, value: &Option<String>) {
    if let Some(v) = value {
        push_str_option(options, name, v);
    }
}

fn push_str_option(options: &mut String, name: &str, value: &String) {
    options.push_str(" --");
    options.push_str(name);
    options.push_str("=");
    options.push_str(value);
}

fn push_conditional_bool_option(options: &mut String, name: &str, value: &Option<bool>) {
    if let Some(v) = value {
        push_bool_option(options, name, v);
    }
}

fn push_bool_option(options: &mut String, name: &str, &value: &bool) {
    if value {
        options.push_str(" --");
        options.push_str(name);
    }
}
