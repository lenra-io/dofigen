<div id="top"></div>
<!--
*** Thanks for checking out the Best-README-Template. If you have a suggestion
*** that would make this better, please fork the repo and create a pull request
*** or simply open an issue with the tag "enhancement".
*** Don't forget to give the project a star!
*** Thanks again! Now go create something AMAZING! :D
-->



<!-- PROJECT SHIELDS -->
<!--
*** I'm using markdown "reference style" links for readability.
*** Reference links are enclosed in brackets [ ] instead of parentheses ( ).
*** See the bottom of this document for the declaration of the reference variables
*** for contributors-url, forks-url, etc. This is an optional, concise syntax you may use.
*** https://www.markdownguide.org/basic-syntax/#reference-style-links
-->


<div align="center">

[![Contributors][contributors-shield]][contributors-url]
[![Forks][forks-shield]][forks-url]
[![Stargazers][stars-shield]][stars-url]
[![Issues][issues-shield]][issues-url]
[![MIT License][license-shield]][license-url]
[![Crates.io][crate-shield]][crate-url]

<img alt="Dofigen logo" src="./logo.svg" width="256" style="margin: 20px 0" /> 

# Dofigen

</div>

Dofigen is a Dockerfile generator using a simplified description in YAML or JSON format.
It defines default values and behaviors that simplify the creation of Dockerfiles.

Dofigen is also made to use the Buildkit, which is now the default Docker build engine, optimizations that speed-up the Docker image build by parallelizing the layer builds.

A french DevOps said about it:
> C'est une bouffée, Dofigen, dans ce monde de con...teneurs.

[Report Bug](https://github.com/lenra-io/dofigen/issues)
·
[Request Feature](https://github.com/lenra-io/dofigen/issues)

<!-- GETTING STARTED -->
## Getting Started

### Prerequisites

Install Dofigen using one of the following options.

#### Use it with Docker

You can run Dofigen directly from its Docker image with the following command:

```bash
docker run --rm -it -v $(pwd):/app lenra/dofigen --help
```

#### Arch Linux

[dofigen](https://aur.archlinux.org/packages/dofigen) is available as an AUR package.

You can install it using an AUR helper (e.g. `paru`):

```bash
paru -S dofigen
```

#### Cargo

First install Cargo, the Rust package manager: https://doc.rust-lang.org/cargo/getting-started/installation.html

Then use the following command to install dofigen:

```bash
cargo install dofigen
```

#### Homebrew

You can install Dofigen using Homebrew:

```bash
brew tap lenra-io/tools
brew install dofigen
```

Or:

```bash
brew install lenra-io/tools/dofigen
```

#### Download the binary

You can download the Dofigen binary from [the release page](https://github.com/lenra-io/dofigen/releases) and add it to your path environment variable.

<p align="right">(<a href="#top">back to top</a>)</p>

### How to use it

To generate a Dockerfile, you need to create a Dofigen file `dofigen.yml` and run the next command:

```bash
dofigen gen
```

Use the help options to understand how to override default behaviors:

```bash
$ dofigen gen --help
Generate the Dockerfile and .dockerignore files

Usage: dofigen generate [OPTIONS]

Options:
  -f, --file <FILE>      The input Dofigen file. Default search for the next files: dofigen.yml, dofigen.yaml, dofigen.json Use "-" to read from stdin
      --offline          The command won't load data from any URL. This disables extending file from URL and loading image tag
  -o, --output <OUTPUT>  The output Dockerfile file Define to - to write to stdout [default: Dockerfile]
  -l, --locked           Locked version of the dofigen definition
  -h, --help             Print help
```

To look further use the help command:

```bash
dofigen --help
```


### Dofigen descriptor

The structure of the Dofigen descriptor was created to be simpler than the Dockerfile.

The JSON Schema of the Dofigen descriptor is available [here](./docs/dofigen.schema.json).

Here is an example to generate the Dofigen Dockerfile:

```yaml
builders:
  muslrust:
    fromImage: clux/muslrust:stable
    workdir: /app
    bind:
      - Cargo.toml
      - Cargo.lock
      - src/
    run:
      # Build with musl to work with scratch
      - cargo build --release
      # copy the generated binary outside of the target directory. If not the other stages won't be able to find it since it's in a cache volume
      - mv target/x86_64-unknown-linux-musl/release/dofigen /tmp/
    cache:
      # Cargo cache
      - /home/rust/.cargo
      # build cache
      - /app/target

# Runtime
workdir: /app
copy:
  - fromBuilder: muslrust
    paths: "/tmp/dofigen"
    target: "/bin/"
entrypoint: dofigen
cmd: --help
context:
  - "/src"
  - "/Cargo.*"
```

### Extending external files

You can extend the Dofigen file with external files using the `extend` attribute.
You can find some Dofigen templates on the [Dofigen Hub repository](https://github.com/lenra-io/dofigen-hub).
Here is an example of extending a Dofigen file:

```yaml
extend:
  - https://raw.githubusercontent.com/lenra-io/dofigen/main/dofigen.yml
```

You can also override or merge the structure of the extended files:

```yaml
extend:
  - https://raw.githubusercontent.com/lenra-io/dofigen/main/dofigen.yml
user: 1001
```

### The lock file

Dofigen generates a lock file to keep the version of the Dofigen descriptor used to generate the Dockerfile.
The lock file also keep the loaded resources and images tags to rebuild the Dockerfile with the same versions.
To update the images and resources, you can use the `dofigen update` command.
To regenerate the Dockerfile with the same versions, you can use the `dofigen gen --locked` command.

<p align="right">(<a href="#top">back to top</a>)</p>

<!-- CONTRIBUTING -->
## Contributing

Contributions are what make the open source community such an amazing place to learn, inspire, and create. Any contributions you make are **greatly appreciated**.

If you have a suggestion that would make this better, please open an issue with the tag "enhancement" or "bug".
Don't forget to give the project a star! Thanks again!

### Tests

To run the tests, use the following command:

```bash
cargo test
```

#### Test coverage

To generate the test coverage, use the following commands:

```bash
# Generate the coverage report
RUSTFLAGS="-C instrument-coverage" \
  RUSTDOCFLAGS="-C instrument-coverage" \
  LLVM_PROFILE_FILE="target/coverage/profiles/cargo-test-%p-%m.profraw" \
  cargo test
# Convert to lcov format
grcov target/coverage/profiles/ --binary-path ./target/debug/deps/ -s . -t lcov --branch --ignore-not-existing -o target/coverage/lcov.info
# Generate the HTML report
grcov target/coverage/profiles/ --binary-path ./target/debug/deps/ -s . -t html --branch --ignore-not-existing -o target/coverage/html
```

### Generate the JSON Schema

To generate the JSON schema of the Dofigen file structure, use the following command:

```bash
cargo run -F json_schema -- schema
```

<p align="right">(<a href="#top">back to top</a>)</p>



<!-- LICENSE -->
## License

Distributed under the **MIT** License. See [LICENSE](./LICENSE) for more information.

<p align="right">(<a href="#top">back to top</a>)</p>



<!-- CONTACT -->
## Contact

Lenra - [@lenra_dev](https://twitter.com/lenra_dev) - contact@lenra.io

Project Link: [https://github.com/lenra-io/dofigen](https://github.com/lenra-io/dofigen)

<p align="right">(<a href="#top">back to top</a>)</p>


<!-- MARKDOWN LINKS & IMAGES -->
<!-- https://www.markdownguide.org/basic-syntax/#reference-style-links -->
[contributors-shield]: https://img.shields.io/github/contributors/lenra-io/dofigen.svg?style=for-the-badge
[contributors-url]: https://github.com/lenra-io/dofigen/graphs/contributors
[forks-shield]: https://img.shields.io/github/forks/lenra-io/dofigen.svg?style=for-the-badge
[forks-url]: https://github.com/lenra-io/dofigen/network/members
[stars-shield]: https://img.shields.io/github/stars/lenra-io/dofigen.svg?style=for-the-badge
[stars-url]: https://github.com/lenra-io/dofigen/stargazers
[issues-shield]: https://img.shields.io/github/issues/lenra-io/dofigen.svg?style=for-the-badge
[issues-url]: https://github.com/lenra-io/dofigen/issues
[license-shield]: https://img.shields.io/github/license/lenra-io/dofigen.svg?style=for-the-badge
[license-url]: https://github.com/lenra-io/dofigen/blob/master/LICENSE.txt
[crate-shield]: https://img.shields.io/crates/v/dofigen.svg?style=for-the-badge
[crate-url]: https://crates.io/crates/dofigen
