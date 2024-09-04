[![Stargazers][stars-shield]][stars-url]
[![Issues][issues-shield]][issues-url]
[![MIT License][license-shield]][license-url]

# Dofigen

Dofigen is a Dockerfile generator using a simplified description in YAML or JSON format.
It defines default values and behaviors that simplify the creation of Dockerfiles.

Dofigen is also made to use the Buildkit optimizations that speed-up the Docker image build by parallelizing the layer builds.
It uses the [`--link` option](https://docs.docker.com/engine/reference/builder/#benefits-of-using---link) when adding files and the [`--mount=type=cache` option](https://docs.docker.com/engine/reference/builder/#run---mounttypecache) when running scripts (when you define `caches` attribute).
You can use Buildkit with the [`docker buildx build` subcommand](https://docs.docker.com/engine/reference/commandline/buildx_build/) like this: 

```bash
docker buildx build --cache-to=type=local,dest=.dockercache --cache-from=type=local,src=.dockercache -t my-app:latest --load .
```

A french DevOps said about it:
> C'est une bouffée, Dofigen, dans ce monde de con...teneurs.

[Report Bug](https://github.com/lenra-io/dofigen/issues)
·
[Request Feature](https://github.com/lenra-io/dofigen/issues)

<!-- GETTING STARTED -->
## Getting Started

Use it with Docker

You can run Dofigen directly from its Docker image with the following command:

```bash
docker run --rm -it -v $(pwd):/app lenra/dofigen
```

See the full documentation on the [GitHub repository](https://github.com/lenra-io/dofigen/).


<!-- MARKDOWN LINKS & IMAGES -->
<!-- https://www.markdownguide.org/basic-syntax/#reference-style-links -->
[stars-shield]: https://img.shields.io/github/stars/lenra-io/dofigen.svg?style=for-the-badge
[stars-url]: https://github.com/lenra-io/dofigen/stargazers
[issues-shield]: https://img.shields.io/github/issues/lenra-io/dofigen.svg?style=for-the-badge
[issues-url]: https://github.com/lenra-io/dofigen/issues
[license-shield]: https://img.shields.io/github/license/lenra-io/dofigen.svg?style=for-the-badge
[license-url]: https://github.com/lenra-io/dofigen/blob/master/LICENSE.txt
