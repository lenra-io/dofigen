# Dofigen Buildkit frontend

To generate the Dofigen Buildkit frontend, use the following command:

```bash
# Build the frontend
cargo build --release --bin dofigen-frontend --no-default-features -F frontend -F permissive
# Generate the Dockerfile
dofigen gen -f frontend.dofigen.yml -o frontend.Dockerfile
# Build the frontend image
docker build -t localhost:5000/dofigen:frontend-local -f frontend.Dockerfile .
# Push the frontend image
docker push localhost:5000/dofigen:frontend-local
```

Short for rebuild:

```bash
cargo build --release --bin dofigen-frontend --no-default-features -F frontend -F permissive && docker build -t localhost:5000/dofigen:frontend-local --push -f frontend.Dockerfile .
```

To use the frontend with `buildctl`, use the next commands:

```bash
# Start the buildkit daemon in a container
docker run --rm -d --name buildkitd --privileged --network host moby/buildkit:latest
# Define the daemon to use
export BUILDKIT_HOST=docker-container://buildkitd
```

You can then see the build logs with the following command:

```bash
docker logs buildkitd -f
```

To build an file using the frontend, use the following command:

```bash
buildctl build --frontend=gateway.v0 --opt source=localhost:5000/dofigen:frontend-local --local context=. --local dockerfile=. --opt filename=./test.Dockerfile --trace test.log
```

