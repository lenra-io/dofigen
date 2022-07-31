# syntax=docker/dockerfile:1.4

# builder
FROM ekidd/rust-musl-builder as builder
ADD --link . ./
RUN \
	--mount=type=cache,uid=1000,gid=1000,target=/usr/local/cargo/registry\
	--mount=type=cache,uid=1000,gid=1000,target=/home/rust/src/target\
	ls -al && \
	cargo build --release && \
	cp target/x86_64-unknown-linux-musl/release/dofigen ../

# runtime
FROM scratch as runtime
WORKDIR /app
COPY --link --from=builder "/home/rust/dofigen" "/"
ENTRYPOINT ["/dofigen"]
