mod context;
mod insctruction;

use colored::{Color, Colorize};
use struct_patch::Patch;

use crate::{
    DockerFile, DockerFileCommand, DockerFileInsctruction, DockerFileLine, DockerIgnore,
    DockerIgnoreLine, Dofigen, Error, FromContext, MessageLevel, Result,
    parse::context::ParseContext,
};

impl Dofigen {
    pub fn from_dockerfile(
        dockerfile: DockerFile,
        dockerignore: Option<DockerIgnore>,
    ) -> Result<Self> {
        let mut context = ParseContext::default();

        if let Some(dockerignore) = dockerignore {
            context.parse_dockerignore(dockerignore)?;
        }

        context.parse_dockerfile(dockerfile)?;

        Ok(context.dofigen.into())
    }
}

impl ParseContext {
    pub fn parse_dockerignore(&mut self, dockerignore: DockerIgnore) -> Result<()> {
        if !self.dofigen.ignore.is_empty() {
            return Err(Error::Custom(
                "A .dockerignore have already been parsed by this context".to_string(),
            ));
        }
        // TODO: If there is a negate pattern with **, then manage context field
        let ignores: Vec<String> = dockerignore
            .lines
            .iter()
            .filter(|line| {
                matches!(line, DockerIgnoreLine::Pattern(_))
                    || matches!(line, DockerIgnoreLine::NegatePattern(_))
            })
            .map(|line| match line {
                DockerIgnoreLine::Pattern(pattern) => pattern.clone(),
                DockerIgnoreLine::NegatePattern(pattern) => format!("!{pattern}"),
                _ => unreachable!(),
            })
            .collect();
        self.dofigen.ignore = ignores;
        Ok(())
    }

    pub fn parse_dockerfile(&mut self, dockerfile: DockerFile) -> Result<()> {
        if !self.stage_names.is_empty() {
            return Err(Error::Custom(
                "A Dockerfile have already been parsed by this context".to_string(),
            ));
        }
        let instructions: Vec<_> = dockerfile
            .lines
            .iter()
            .filter(|line| matches!(line, DockerFileLine::Instruction(_)))
            .collect();

        self.stage_names = instructions
            .iter()
            .filter(|&line| {
                matches!(
                    line,
                    DockerFileLine::Instruction(DockerFileInsctruction {
                        command: DockerFileCommand::FROM,
                        ..
                    })
                )
            })
            .map(|line| match line {
                DockerFileLine::Instruction(DockerFileInsctruction {
                    command: DockerFileCommand::FROM,
                    content,
                    ..
                }) => content,
                _ => unreachable!(),
            })
            .map(|from_content| split_from(from_content).1.unwrap_or("runtime").to_string())
            .collect();

        for line in instructions {
            self.apply(line)?;
        }

        // Get runtime informations
        let runtime_stage = self.current_stage.clone().ok_or(Error::Custom(
            "No FROM instruction found in Dockerfile".to_string(),
        ))?;
        let runtime_name = self
            .current_stage_name
            .clone()
            .unwrap_or("runtime".to_string());

        // Get base instructions in from builders
        let mut dofigen_patches = self
            .builder_dofigen_patches
            .remove(&runtime_name)
            .into_iter()
            .collect::<Vec<_>>();
        let mut searching_stage = runtime_stage.clone();
        while let FromContext::FromBuilder(builder_name) = searching_stage.from.clone() {
            if let Some(builder_dofigen_patch) = self.builder_dofigen_patches.remove(&builder_name)
            {
                dofigen_patches.insert(0, builder_dofigen_patch);
            }
            searching_stage = self
                .dofigen
                .builders
                .get(&builder_name)
                .ok_or(Error::Custom(format!(
                    "Builder '{}' not found",
                    builder_name
                )))?
                .clone();
        }

        // Apply merged patches
        if !dofigen_patches.is_empty() {
            dofigen_patches.iter().for_each(|dofigen_patch| {
                self.dofigen.apply(dofigen_patch.clone());
            });
        }

        self.apply_root()?;
        self.dofigen.stage = runtime_stage;

        // Handle lint messages
        self.messages.iter().for_each(|message| {
            eprintln!(
                "{}[path={}]: {}",
                match message.level {
                    MessageLevel::Error => "error".color(Color::Red).bold(),
                    MessageLevel::Warn => "warning".color(Color::Yellow).bold(),
                },
                message.path.join(".").color(Color::Blue).bold(),
                message.message
            );
        });

        Ok(())
    }
}

pub(crate) fn split_from(content: &str) -> (&str, Option<&str>) {
    let pos = content.to_lowercase().find(" as ");
    if let Some(pos) = pos {
        let (from, name) = content.split_at(pos);
        let name = name[4..].trim();
        (from, Some(name))
    } else {
        (content, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DofigenContext;
    use crate::GenerationContext;
    use crate::dockerfile_struct::*;
    use crate::dofigen_struct::*;
    use pretty_assertions_sorted::assert_eq_sorted;
    use std::collections::HashMap;

    #[test]
    fn php_dockerfile() {
        let dockerfile_content = r#"# syntax=docker/dockerfile:1.19.0
# This file is generated by Dofigen v0.0.0
# See https://github.com/lenra-io/dofigen

# get-composer
FROM composer:latest AS get-composer

# install-deps
FROM php:8.3-fpm-alpine AS install-deps
USER 0:0
RUN <<EOF
apt-get update
apk add --no-cache --update ca-certificates dcron curl git supervisor tar unzip nginx libpng-dev libxml2-dev libzip-dev icu-dev mysql-client
EOF

# install-php-ext
FROM install-deps AS install-php-ext
USER 0:0
RUN <<EOF
docker-php-ext-configure zip
docker-php-ext-install bcmath gd intl pdo_mysql zip
EOF

# runtime
FROM install-php-ext AS runtime
WORKDIR /
COPY \
    --from=get-composer \
    --chown=www-data \
    --link \
    "/usr/bin/composer" "/bin/"
ADD \
    --chown=www-data \
    --link \
    "https://github.com/pelican-dev/panel.git" "/tmp/pelican"
USER www-data
RUN <<EOF
cd /tmp/pelican
cp .env.example .env
mkdir -p bootstrap/cache/ storage/logs storage/framework/sessions storage/framework/views storage/framework/cache
chmod 777 -R bootstrap storage
composer install --no-dev --optimize-autoloader
rm -rf .env bootstrap/cache/*.php
mkdir -p /app/storage/logs/
chown -R nginx:nginx .
rm /usr/local/etc/php-fpm.conf
echo "* * * * * /usr/local/bin/php /app/artisan schedule:run >> /dev/null 2>&1" >> /var/spool/cron/crontabs/root
mkdir -p /var/run/php /var/run/nginx
mv .github/docker/default.conf /etc/nginx/http.d/default.conf
mv .github/docker/supervisord.conf /etc/supervisord.conf
EOF
"#;

        let yaml = r#"builders:
  install-deps:
    fromImage: php:8.3-fpm-alpine
    root:
      run:
      - apt-get update
      - >-
        apk add --no-cache --update
        ca-certificates
        dcron
        curl
        git
        supervisor
        tar
        unzip
        nginx
        libpng-dev
        libxml2-dev
        libzip-dev
        icu-dev
        mysql-client
  install-php-ext:
    fromBuilder: install-deps
    root:
      run:
      # - docker-php-ext-configure gd --with-freetype --with-jpeg
      # - docker-php-ext-install -j$(nproc) gd zip intl curl mbstring mysqli
        - docker-php-ext-configure zip
        - docker-php-ext-install bcmath gd intl pdo_mysql zip
  get-composer:
    name: composer
    fromImage: composer:latest
fromBuilder: install-php-ext
workdir: /
user:
  user: www-data
copy:
- fromBuilder: get-composer
  paths: "/usr/bin/composer"
  target: "/bin/"
  chown:
    user: www-data
  link: true
- repo: 'https://github.com/pelican-dev/panel.git'
  target: '/tmp/pelican'
  chown:
    user: www-data
  link: true
run:
  - cd /tmp/pelican
  - cp .env.example .env
  - mkdir -p bootstrap/cache/ storage/logs storage/framework/sessions storage/framework/views storage/framework/cache
  - chmod 777 -R bootstrap storage
  - composer install --no-dev --optimize-autoloader
  - rm -rf .env bootstrap/cache/*.php
  - mkdir -p /app/storage/logs/
  - chown -R nginx:nginx .
  - rm /usr/local/etc/php-fpm.conf
  - echo "* * * * * /usr/local/bin/php /app/artisan schedule:run >> /dev/null 2>&1" >> /var/spool/cron/crontabs/root
  - mkdir -p /var/run/php /var/run/nginx
  - mv .github/docker/default.conf /etc/nginx/http.d/default.conf
  - mv .github/docker/supervisord.conf /etc/supervisord.conf
"#;

        let dockerfile: DockerFile = dockerfile_content.parse().unwrap();

        let result = Dofigen::from_dockerfile(dockerfile, None);

        let dofigen_from_dockerfile = result.unwrap();

        assert_eq_sorted!(dofigen_from_dockerfile, Dofigen {
                builders: HashMap::from([
                    ("get-composer".to_string(), Stage {
                        from: FromContext::FromImage(
                            ImageName {
                                path: "composer".to_string(),
                                version: Some(
                                    ImageVersion::Tag(
                                        "latest".to_string(),
                                    ),
                                ),
                ..Default::default()
                            },
                        ),
                ..Default::default()
                    }),
                    ("install-deps".to_string(), Stage {
                        from: FromContext::FromImage(
                            ImageName {
                                path: "php".to_string(),
                                version: Some(
                                    ImageVersion::Tag(
                                        "8.3-fpm-alpine".to_string(),
                                    ),
                                ),
                ..Default::default()
                            },
                        ),
                        root: Some(
                            Run {
                                run: vec![
                                    "apt-get update".to_string(),
                                    "apk add --no-cache --update ca-certificates dcron curl git supervisor tar unzip nginx libpng-dev libxml2-dev libzip-dev icu-dev mysql-client".to_string(),
                                ],
                ..Default::default()
                            },
                        ),
                ..Default::default()
                    }),
                    ("install-php-ext".to_string(), Stage {
                        from: FromContext::FromBuilder(
                            "install-deps".to_string(),
                        ),
                        root: Some(
                            Run {
                                run: vec![
                                    "docker-php-ext-configure zip".to_string(),
                                    "docker-php-ext-install bcmath gd intl pdo_mysql zip".to_string(),
                                ],
                ..Default::default()
                            },
                        ),
                ..Default::default()
                    })
                    ]),
                stage: Stage {
                    from: FromContext::FromBuilder(
                        "install-php-ext".to_string(),
                    ),
                    user: Some(
                        User {
                            user: "www-data".to_string(),
                            group: None,
                        },
                    ),
                    workdir: Some(
                        "/".to_string(),
                    ),
                    copy: vec![
                        CopyResource::Copy(
                            Copy {
                                from: FromContext::FromBuilder(
                                    "get-composer".to_string(),
                                ),
                                paths: vec![
                                    "/usr/bin/composer".to_string(),
                                ],
                                options: CopyOptions {
                                   target: Some(
                                       "/bin/".to_string(),
                                   ),
                                   chown: Some(
                                       User {
                                           user: "www-data".to_string(),
                                           group: None,
                                       },
                                   ),
                                   link: Some(
                                       true,
                                   ),
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                        ),
                        CopyResource::AddGitRepo(
                            AddGitRepo {
                                repo: "https://github.com/pelican-dev/panel.git".to_string(),
                                options: CopyOptions {
                                   target: Some(
                                       "/tmp/pelican".to_string(),
                                   ),
                                   chown: Some(
                                       User {
                                           user: "www-data".to_string(),
                                           group: None,
                                       },
                                   ),
                                   link: Some(
                                       true,
                                   ),
                ..Default::default()
                                },
                ..Default::default()
                            },
                        ),
                    ],
                    run: Run {
                        run: vec![
                            "cd /tmp/pelican".to_string(),
                            "cp .env.example .env".to_string(),
                            "mkdir -p bootstrap/cache/ storage/logs storage/framework/sessions storage/framework/views storage/framework/cache".to_string(),
                            "chmod 777 -R bootstrap storage".to_string(),
                            "composer install --no-dev --optimize-autoloader".to_string(),
                            "rm -rf .env bootstrap/cache/*.php".to_string(),
                            "mkdir -p /app/storage/logs/".to_string(),
                            "chown -R nginx:nginx .".to_string(),
                            "rm /usr/local/etc/php-fpm.conf".to_string(),
                            "echo \"* * * * * /usr/local/bin/php /app/artisan schedule:run >> /dev/null 2>&1\" >> /var/spool/cron/crontabs/root".to_string(),
                            "mkdir -p /var/run/php /var/run/nginx".to_string(),
                            "mv .github/docker/default.conf /etc/nginx/http.d/default.conf".to_string(),
                            "mv .github/docker/supervisord.conf /etc/supervisord.conf".to_string(),
                        ],
                ..Default::default()
                    },
                                ..Default::default()
                },
                ..Default::default()
            });

        let dofigen_from_string: Dofigen = DofigenContext::new()
            .parse_from_string(yaml)
            .map_err(Error::from)
            .unwrap();

        assert_eq_sorted!(dofigen_from_dockerfile, dofigen_from_string);

        let mut context = GenerationContext::from(dofigen_from_string.clone());

        let generated_dockerfile = context.generate_dockerfile().unwrap();

        assert_eq_sorted!(dockerfile_content.to_string(), generated_dockerfile);
    }
}
