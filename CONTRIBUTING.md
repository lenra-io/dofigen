# Contributing to Dofigen

Contributions are what make the open source community such an amazing place to learn, inspire, and create. Any contributions you make are **greatly appreciated** - and don't forget to give the project a ⭐!

## Where to start

- Browse the [good first issues](https://github.com/lenra-io/dofigen/labels/good%20first%20issue): they are scoped and don't require deep knowledge of the codebase.
- Have an idea or found a bug? [Open an issue](https://github.com/lenra-io/dofigen/issues/new/choose) with the `enhancement` or `bug` label.
- **Comment on the issue before you start** so we can assign it to you and avoid duplicate work.

## Project structure

- `src/` - the library (`dofigen_lib`) and the CLI (`dofigen`).
  - Dockerfile parsing (`FROM`, `RUN`, `COPY`, …)
  - the Dofigen configuration model (YAML/JSON) and the patch/extends system
  - the Dockerfile generation
- `docs/` - documentation and the generated JSON Schema.
- `tests/` - integration tests.

## Development setup

Dofigen is written in Rust (**edition 2024**).

```bash
# Build
cargo build

# Run tests
cargo test
```

## Coding conventions

The CI enforces formatting, so please run this before opening a PR:

```bash
# Format (checked in CI)
cargo fmt --all
```

### Commits

We use [Conventional Commits](https://www.conventionalcommits.org), and releases are managed automatically with [convco](https://convco.github.io). Commit messages must follow the convention, e.g.:

```
feat(parse): handle STOPSIGNAL command
fix(lock): keep a stable field order in the lock file
docs: document the patch system
```

## Pull request workflow

1. Fork the repository and create a branch from `main`.
2. Make your changes, with tests when it makes sense.
3. Run `cargo fmt --all` and `cargo test` locally.
4. Push and open a Pull Request describing what and why. Link the related issue (e.g. `Closes #123`).

## Tests

To run the tests:

```bash
cargo test
```

### Test coverage

To generate the test coverage report:

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

## Generate the JSON Schema

To generate the JSON schema of the Dofigen file structure:

```bash
# Generate the JSON Schema
cargo run -F json_schema -- schema > docs/dofigen.schema.json
# Download the SchemaStore's Prettier configuration
curl -O -L -f -s -H 'Accept: application/vnd.github.v3.raw' https://github.com/SchemaStore/schemastore/raw/refs/heads/master/.prettierrc.cjs
# Install Prettier if you don't have it
npm i -g prettier prettier-plugin-sort-json prettier-plugin-toml
# Format the JSON Schema
npx prettier --config .prettierrc.cjs --write docs/dofigen.schema.json
```
