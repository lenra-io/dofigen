use dofigen_lib::*;
use pretty_assertions_sorted::assert_eq_sorted;

#[test]
fn yaml_to_dockerfile_empty() {
    let yaml = "";

    let dofigen: Dofigen = DofigenContext::new()
        .parse_from_string(yaml)
        .map_err(Error::from)
        .unwrap();
    let dockerfile: String = generate_dockerfile(&dofigen).unwrap();

    assert_eq_sorted!(
        dockerfile,
        r#"# syntax=docker/dockerfile:1.7
# This file is generated by Dofigen v0.0.0
# See https://github.com/lenra-io/dofigen

# runtime
FROM scratch AS runtime
USER 1000:1000
"#
    );

    let dockerignore: String = generate_dockerignore(&dofigen);

    assert_eq_sorted!(
        dockerignore,
        "# This file is generated by Dofigen v0.0.0\n# See https://github.com/lenra-io/dofigen\n\n"
    );
}

#[test]
#[cfg(feature = "permissive")]
fn yaml_to_dockerfile_complexe() {
    let yaml = r#"
builders:
  builder:
    fromImage: ekidd/rust-musl-builder
    user: rust
    add: "."
    run:
    - ls -al
    - cargo build --release
    cache: /usr/local/cargo/registry
arg:
  TARGETPLATFORM: ""
  APP_NAME: template-rust
env:
  fprocess: /app
copy:
  - fromBuilder: builder
    paths: /home/rust/src/target/x86_64-unknown-linux-musl/release/${APP_NAME}
    target: /app
    chmod: "555"
  - fromImage: ghcr.io/openfaas/of-watchdog:0.9.6
    paths: /fwatchdog
    target: /fwatchdog
    chmod: 555
expose: 8080
healthcheck:
  interval: 3s
  cmd: "[ -e /tmp/.lock ] || exit 1"
cmd: "/fwatchdog"
ignores:
  - target
  - test
        "#;

    let dofigen: Dofigen = DofigenContext::new()
        .parse_from_string(yaml)
        .map_err(Error::from)
        .unwrap();
    let dockerfile: String = generate_dockerfile(&dofigen).unwrap();

    assert_eq_sorted!(
        dockerfile,
        r#"# syntax=docker/dockerfile:1.7
# This file is generated by Dofigen v0.0.0
# See https://github.com/lenra-io/dofigen

# builder
FROM ekidd/rust-musl-builder AS builder
COPY \
    --chown=rust \
    --link \
    "." "./"
USER rust
RUN \
    --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    <<EOF
ls -al
cargo build --release
EOF

# runtime
FROM scratch AS runtime
ARG APP_NAME=template-rust
ARG TARGETPLATFORM
ENV fprocess="/app"
COPY \
    --from=builder \
    --chown=1000:1000 \
    --chmod=555 \
    --link \
    "/home/rust/src/target/x86_64-unknown-linux-musl/release/${APP_NAME}" "/app"
COPY \
    --from=ghcr.io/openfaas/of-watchdog:0.9.6 \
    --chown=1000:1000 \
    --chmod=555 \
    --link \
    "/fwatchdog" "/fwatchdog"
USER 1000:1000
EXPOSE 8080
HEALTHCHECK \
    --interval=3s \
    CMD [ -e /tmp/.lock ] || exit 1
CMD ["/fwatchdog"]
"#
    );

    let dockerignore: String = generate_dockerignore(&dofigen);

    assert_eq_sorted!(dockerignore, "# This file is generated by Dofigen v0.0.0\n# See https://github.com/lenra-io/dofigen\n\ntarget\ntest\n");
}

#[ignore = "Flatten enum variant alias problem: https://github.com/serde-rs/serde/issues/2188"]
#[test]
#[cfg(feature = "permissive")]
fn using_dockerfile_overlap_aliases() {
    use std::collections::HashMap;

    #[cfg(not(feature = "permissive"))]
    let yaml = r#"
builders:
  builder:
    image: 
      path: ekidd/rust-musl-builder
    adds:
      - paths:
        - "*"
    script:
      - cargo build --release
artifacts:
  - builder: builder
    source:
      - /home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust
    destination: /app
"#;

    #[cfg(feature = "permissive")]
    let yaml = r#"
builders:
  builder:
    image: ekidd/rust-musl-builder
    adds:
      - "*"
    script:
      - cargo build --release
artifacts:
- builder: builder
  source: /home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust
  destination: /app
"#;
    let dofigen: Dofigen = DofigenContext::new().parse_from_string(yaml).unwrap();
    assert_eq_sorted!(
        dofigen,
        Dofigen {
            builders: HashMap::from([(
                "builder".into(),
                Stage {
                    from: FromContext::FromImage(ImageName {
                        path: String::from("ekidd/rust-musl-builder"),
                        ..Default::default()
                    }),
                    copy: vec![CopyResource::Copy(Copy {
                        paths: vec![String::from("*")].into(),
                        ..Default::default()
                    })],
                    run: Run {
                        run: vec![String::from("cargo build --release")].into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            )]),
            stage: Stage {
                copy: vec![CopyResource::Copy(Copy {
                    from: FromContext::FromBuilder(String::from("builder")),
                    paths: vec![String::from(
                        "/home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust"
                    )],
                    options: CopyOptions {
                        target: Some(String::from("/app")),
                        ..Default::default()
                    },
                    ..Default::default()
                })]
                .into(),
                ..Default::default()
            },
            ..Default::default()
        }
    );
}

#[test]
#[cfg(feature = "permissive")]
fn multiline_run_field() {
    let yaml = r#"
run:
  - |
    if [ "test" = "test" ]; then
      echo "Test"
    fi
"#;

    let dofigen: Dofigen = DofigenContext::new().parse_from_string(yaml).unwrap();
    let dockerfile: String = generate_dockerfile(&dofigen).unwrap();

    assert_eq_sorted!(
        dockerfile,
        r#"# syntax=docker/dockerfile:1.7
# This file is generated by Dofigen v0.0.0
# See https://github.com/lenra-io/dofigen

# runtime
FROM scratch AS runtime
USER 1000:1000
RUN <<EOF
if [ "test" = "test" ]; then
  echo "Test"
fi
EOF
"#
    );
}

#[ignore = "Not managed yet by serde: https://serde.rs/field-attrs.html#flatten"]
#[test]
#[cfg(feature = "permissive")]
fn combine_field_and_aliases() {
    #[cfg(not(feature = "permissive"))]
    let yaml = r#"
fromContext: 
  path: scratch
fromImage:
  path: alpine
"#;

    #[cfg(feature = "permissive")]
    let yaml = r#"
fromContext: scratch
fromImage: alpine
"#;
    let result = DofigenContext::new().parse_from_string(yaml);
    assert!(
        result.is_err(),
        "The parsing must fail since from and image are not compatible",
    );
}

#[ignore = "Not managed yet by serde: https://serde.rs/field-attrs.html#flatten"]
#[test]
#[cfg(feature = "permissive")]
fn fail_on_unknow_field() {
    #[cfg(not(feature = "permissive"))]
    let yaml = r#"
fromImage:
  path: alpine
test: Fake value
"#;

    #[cfg(feature = "permissive")]
    let yaml = r#"
fromImage: alpine
test: Fake value
"#;
    let result = DofigenContext::new().parse_from_string(yaml);
    assert!(
        result.is_err(),
        "The parsing must fail since 'test' is not a valid field"
    );

    // Check the error message
    let error = result.unwrap_err();
    let expected =
        "Error while deserializing the document at line 2, column 1: unknown field `test`";
    assert_eq_sorted!(
        &error.to_string().as_str()[..expected.len()],
        expected,
        "Wrong error message"
    );
}

#[test]
#[cfg(feature = "permissive")]
fn manage_plural_aliases() {
    #[cfg(not(feature = "permissive"))]
    let yaml = r#"
builders:
  builder:
    fromImage:
      path: ekidd/rust-musl-builder
    user:
      user: rust
    adds: 
      - paths:
          - "."
    run:
      - cargo build --release
    caches:
      - /usr/local/cargo/registry
envs:
  fprocess: /app
artifacts:
  - fromBuilder: builder
    source: /home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust
    target: /app
  - fromImage: 
      host: ghcr.io
      path: openfaas/of-watchdog
      tag: 0.9.6
    source: /fwatchdog
    target: /fwatchdog
ports:
  - port: 8080
ignore:
  - target
  - test
"#;

    #[cfg(feature = "permissive")]
    let yaml = r#"
builders:
  builder:
    fromImage: ekidd/rust-musl-builder
    user: rust
    adds: 
    - "."
    run:
    - cargo build --release
    caches:
    - /usr/local/cargo/registry
envs:
  fprocess: /app
artifacts:
  - fromBuilder: builder
    source: /home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust
    target: /app
  - fromImage: ghcr.io/openfaas/of-watchdog:0.9.6
    source: /fwatchdog
    target: /fwatchdog
ports:
  - 8080
ignore:
  - target
  - test
"#;

    let result = DofigenContext::new().parse_from_string(yaml);

    assert!(result.is_ok());
}

#[test]
#[cfg(feature = "permissive")]
fn artifact_copy_custom_user() {
    #[cfg(not(feature = "permissive"))]
    let yaml = r#"
builders:
  builder:
    fromImage:
      path: ekidd/rust-musl-builder
    user:
      user: rust
    copy:
      - paths: ["."]
    run:
      - cargo build --release
    cache:
      - /usr/local/cargo/registry
user:
  user: 1001
artifacts:
- fromBuilder: builder
  source: /home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust
  target: /app
run:
  - echo "coucou"
cache:
  - /tmp
"#;

    #[cfg(feature = "permissive")]
    let yaml = r#"
builders:
  builder:
    fromImage: ekidd/rust-musl-builder
    user: rust
    add: "."
    run: cargo build --release
    cache: /usr/local/cargo/registry
user: 1001
artifacts:
- fromBuilder: builder
  source: /home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust
  target: /app
run: echo "coucou"
cache: /tmp
"#;

    let dofigen: Dofigen = DofigenContext::new().parse_from_string(yaml).unwrap();
    let dockerfile: String = generate_dockerfile(&dofigen).unwrap();

    assert_eq_sorted!(
        dockerfile,
        r#"# syntax=docker/dockerfile:1.7
# This file is generated by Dofigen v0.0.0
# See https://github.com/lenra-io/dofigen

# builder
FROM ekidd/rust-musl-builder AS builder
COPY \
    --chown=rust \
    --link \
    "." "./"
USER rust
RUN \
    --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    cargo build --release

# runtime
FROM scratch AS runtime
COPY \
    --from=builder \
    --chown=1001 \
    --link \
    "/home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust" "/app"
USER 1001
RUN \
    --mount=type=cache,target=/tmp,uid=1001,sharing=locked \
    echo "coucou"
"#
    );
}

#[test]
fn bind_instead_of_copy() {
    #[cfg(not(feature = "permissive"))]
    let yaml = r#"
builders:
  builder:
    fromImage:
      path: clux/muslrust
      tag: stable
    workdir: /app
    bind:
      - source: Cargo.toml
        target: Cargo.toml
      - source: Cargo.lock
        target: Cargo.lock
      - source: src/
        target: src/
    run:
      # Build with musl to work with scratch
      - cargo build --release
      # copy the generated binary outside of the target directory. If not the other stages won't be able to find it since it's in a cache volume
      - mv target/x86_64-unknown-linux-musl/release/dofigen /tmp/
    cache:
      # Cargo cache
      - target: /home/rust/.cargo
      # build cache
      - target: target
workdir: /app
copy:
  - fromBuilder: builder
    paths: 
      - "/tmp/dofigen"
    target: "/bin/"
entrypoint:
  - /bin/dofigen
cmd:
  - --help
context:
  - "/src"
  - "/Cargo.*"
"#;

    #[cfg(feature = "permissive")]
    let yaml = r#"
builders:
  builder: 
    fromImage: clux/muslrust:stable
    workdir: /app
    bind:
      - Cargo.toml
      - Cargo.lock
      - src/ src/
    run:
      # Build with musl to work with scratch
      - cargo build --release
      # copy the generated binary outside of the target directory. If not the other stages won't be able to find it since it's in a cache volume
      - mv target/x86_64-unknown-linux-musl/release/dofigen /tmp/
    cache:
      # Cargo cache
      - /home/rust/.cargo
      # build cache
      - target
workdir: /app
artifacts:
  - fromBuilder: builder
    source: "/tmp/dofigen"
    target: "/bin/"
entrypoint: /bin/dofigen
cmd: --help
context:
  - "/src"
  - "/Cargo.*"
"#;

    let dofigen: Dofigen = DofigenContext::new().parse_from_string(yaml).unwrap();
    let dockerfile: String = generate_dockerfile(&dofigen).unwrap();

    assert_eq_sorted!(
        dockerfile,
        r#"# syntax=docker/dockerfile:1.7
# This file is generated by Dofigen v0.0.0
# See https://github.com/lenra-io/dofigen

# builder
FROM clux/muslrust:stable AS builder
WORKDIR /app
RUN \
    --mount=type=bind,target=Cargo.toml,source=Cargo.toml \
    --mount=type=bind,target=Cargo.lock,source=Cargo.lock \
    --mount=type=bind,target=src/,source=src/ \
    --mount=type=cache,target=/home/rust/.cargo,sharing=locked \
    --mount=type=cache,target=/app/target,sharing=locked \
    <<EOF
cargo build --release
mv target/x86_64-unknown-linux-musl/release/dofigen /tmp/
EOF

# runtime
FROM scratch AS runtime
WORKDIR /app
COPY \
    --from=builder \
    --chown=1000:1000 \
    --link \
    "/tmp/dofigen" "/bin/"
USER 1000:1000
ENTRYPOINT ["/bin/dofigen"]
CMD ["--help"]
"#
    );
}

#[ignore = "Not managed yet by serde because of multilevel flatten: https://serde.rs/field-attrs.html#flatten"]
#[test]
#[cfg(feature = "permissive")]
fn malformed_user_must_fail() {
    let yaml = r#"
user: wrong.user
"#;

    let result = DofigenContext::new().parse_from_string(yaml);
    assert!(
        result.is_err(),
        "The parsing must fail since user name is malformed",
    );
}
