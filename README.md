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
It defines default values and behaviers that make the creation of Dockerfiles simpler.

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

You can download the Dofigen binary from [the release page](https://github.com/lenra-io/docs/releases) and add it to you path environment variable.

#### Use it with Docker

You can run Dofigen directly from it Docker image with the next command:

```bash
docker run --rm -it -v $(pwd):/app lenra/dofigen
```

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