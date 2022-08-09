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
[![Contributors][contributors-shield]][contributors-url]
[![Forks][forks-shield]][forks-url]
[![Stargazers][stars-shield]][stars-url]
[![Issues][issues-shield]][issues-url]
[![MIT License][license-shield]][license-url]

# Dofigen

Dofigen is a Dockerfile generator using a simplyfied description in YAML or JSON format.
It defines default values and behaviors that makes the creation of Dockerfiles simpler.

A french DevOps said about it:
> C'est une bouffée Dofigen dans ce monde de con...teneurs.

[Report Bug](https://github.com/lenra-io/dofigen/issues)
·
[Request Feature](https://github.com/lenra-io/dofigen/issues)

<!-- GETTING STARTED -->
## Getting Started

### Prerequisites

Install Dofigen using one of the next possibilities.

#### Cargo install

First install Cargo, the Rust package manager: https://doc.rust-lang.org/cargo/getting-started/installation.html

Then use the next command to install dofigen:

```bash
cargo install dofigen
```

#### Download the binary

You can download the Dofigen binary from [the release page](https://github.com/lenra-io/docs/releases) and add it to your path environment variable.

#### Use it with Docker

You can run Dofigen directly from it Docker image with the next command:

```bash
docker run --rm -it -v $(pwd):/app lenra/dofigen
```

<p align="right">(<a href="#top">back to top</a>)</p>

### How to use it

Use the help options to understand how to use it:

```bash
$ dofigen --help
dofigen 0.1.0
Dofigen is a Dockerfile generator using a simplyfied description in YAML or JSON format

USAGE:
    dofigen [OPTIONS] [INPUT_FILE]

ARGS:
    <INPUT_FILE>    The input file Dofigen file. Default reads stdin

OPTIONS:
    -d, --dockerfile <DOCKERFILE>    The output Dockerfile file [default: Dockerfile]
    -f, --format <FORMAT>            The input format [default: yaml] [possible values: json, yaml]
    -h, --help                       Print help information
    -i, --ignorefile <IGNOREFILE>    The output .dockerignore file [default: .dockerignore]
    -o, --output                     Writes the Dockerfile to the stdout
    -V, --version                    Print version information
```

### Image descriptor

The image descriptor structure has been created to be simplier than the Dockerfile's one.

### Image

The image is the main element. It defines the runtime stage of the Dockerfile:

| Field            | Type             | Description                   |
|------------------|------------------|-------------------------------|
| `image`          | String?          | The `FROM` Docker image       |
| `user`           | String?          | The runtime user (default `1000`) |
| `workdir`        | String?          | The runtime work directory    |
| `envs`           | Map<String, String>? | The runtime environment variables |
| `artifacts`      | [Artifact](#artifact)[]? | Defines artifacts to copy from builders |
| `adds`           | String[]?        | Paths of elements to add at build time to the workdir |
| `root`           | [Root](#root)?   | Actions made using the `root` user |
| `script`         | String[]?        | Script commands to execute    |
| `caches`         | String[]?        | Paths in the image stage to cache during the `script` execution |
| `builders`       | [Builder](#builder)[]? | Build stages executed before the runtime stage and not in the final Docker image. Mostly to generate artifacts |
| `ports`          | int[]?           | The list of exposed ports of the Docker image |
| `healthcheck`    | [Healthcheck](#healthcheck)? | The Docker image healthcheck definition. |
| `entrypoint`     | String[]?        | The Docker image `ENTRYPOINT` parts |
| `cmd`            | String[]?        | The Docker image `CMD` parts  |
| `ignores`        | String[]?        | Paths to generate the `.dockerignore` file |

### Builder

The builders are stages executed before the runtime stage and not in the final Docker image. Mostly to generate artifacts :

| Field            | Type             | Description                   |
|------------------|------------------|-------------------------------|
| `name`           | String?          | The builder name. If not defined, a name is defined with the given pattern: `builder-<position in the builders list starting at 0>` |
| `image`          | String?          | The `FROM` Docker image of the builder |
| `user`           | String?          | The builder user              |
| `workdir`        | String?          | The builder work directory    |
| `envs`           | Map<String, String>? | The builder environment variables |
| `artifacts`      | [Artifact](#artifact)[]? | Defines artifacts to copy from previous builders |
| `adds`           | String[]?        | Paths of elements to add at build time to the workdir |
| `root`           | [Root](#root)?   | Actions made using the `root` user |
| `script`         | String[]?        | Script commands to execute    |
| `caches`         | String[]?        | Paths in the image stage to cache during the `script` execution |

### Artifact

Artifacts are element copied from a previous build to the current stage :

| Field            | Type             | Description                   |
|------------------|------------------|-------------------------------|
| `builder`        | String           | The builder name from which the artifact will be copied |
| `source`         | String           | The source of the artifact in the given builder |
| `destination`    | String           | The destination path in the current stage |

### Root

Actions made using the `root` user :

| Field            | Type             | Description                   |
|------------------|------------------|-------------------------------|
| `script`         | String[]?        | Script commands to execute    |
| `caches`         | String[]?        | Paths in the image stage to cache during the `script` execution |

### Healthcheck

The Docker image's healthcheck definition. It defines when the container is not healthy :

| Field            | Type             | Description                   |
|------------------|------------------|-------------------------------|
| `cmd`            | String           | The command executed to check the container health |
| `interval`       | String?          | The command execution interval (default `30s`) |
| `timeout`        | String?          | The command execution timeout (default `30s`) |
| `start`          | String?          | The duration before starting the commmand execution at container start (default `0s`) |
| `retries`        | int?             | The number of retries before defining the container as unhealthy (default `3`) |

<p align="right">(<a href="#top">back to top</a>)</p>

<!-- CONTRIBUTING -->
## Contributing

Contributions are what make the open source community such an amazing place to learn, inspire, and create. Any contributions you make are **greatly appreciated**.

If you have a suggestion that would make this better, please open an issue with the tag "enhancement" or "bug".
Don't forget to give the project a star! Thanks again!

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
