use std::collections::HashMap;

use dofigen_lib::*;
use pretty_assertions_sorted::assert_eq_sorted;

#[cfg(feature = "permissive")]
#[test]
fn badly_ordered_stages_and_no_runtime() {
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
user: www-data
copy:
- fromBuilder: get-composer
  paths: "/usr/bin/composer"
  target: "/bin/"
- repo: 'https://github.com/pelican-dev/panel.git'
  target: '/tmp/pelican'
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

    let dofigen: Dofigen = DofigenContext::new()
        .parse_from_string(yaml)
        .map_err(Error::from)
        .unwrap();

    assert_eq_sorted!(dofigen, Dofigen {
        builders: HashMap::from([
          ( "install-deps".to_string(), Stage {
                from: FromContext::FromImage(ImageName {
                  path: "php".to_string(),
                  version: Some(ImageVersion::Tag("8.3-fpm-alpine".to_string())),
                  ..Default::default()
                }),
                root: Some(Run {run:vec![
                    "apt-get update".to_string(),
                    "apk add --no-cache --update ca-certificates dcron curl git supervisor tar unzip nginx libpng-dev libxml2-dev libzip-dev icu-dev mysql-client".to_string(),
                ],
                ..Default::default()
                }),
                ..Default::default()
            }),
            ( "install-php-ext".to_string(), Stage {
                from: FromContext::FromBuilder("install-deps".to_string()),
                root: Some(Run {
                    run: vec![
                        "docker-php-ext-configure zip".to_string(),
                        "docker-php-ext-install bcmath gd intl pdo_mysql zip".to_string(),
                    ],
                ..Default::default()
                }),
                ..Default::default()
            }),
            ( "get-composer".to_string(), Stage {
                from: FromContext::FromImage(ImageName {
                    path: "composer".to_string(),
                    version: Some(ImageVersion::Tag("latest".to_string())),
                    ..Default::default()
                }),
                ..Default::default()
            }),
        ]),
        stage: Stage {
        from: FromContext::FromBuilder("install-php-ext".to_string()),
        workdir: Some("/".to_string()),
        user: Some(User::new_without_group("www-data".into())),
        copy: vec![
            CopyResource::Copy(Copy {
                from: FromContext::FromBuilder("get-composer".to_string()),
                paths: vec!["/usr/bin/composer".to_string()],
                options: CopyOptions {
                target: Some("/bin/".to_string()),
                ..Default::default()
                },
                ..Default::default()
            }),
            CopyResource::AddGitRepo(AddGitRepo {
                repo: "https://github.com/pelican-dev/panel.git".to_string(),
                options: CopyOptions {
                target: Some("/tmp/pelican".to_string()),
                ..Default::default()
                },
                ..Default::default()
            }),
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

    let dockerfile: String = generate_dockerfile(&dofigen).unwrap();

    assert_eq_sorted!(
        dockerfile,
        r#"# This file is generated by Dofigen v0.0.0
# See https://github.com/lenra-io/dofigen

# syntax=docker/dockerfile:1.7

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
"#
    );
}
