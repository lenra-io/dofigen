# syntax=docker/dockerfile:1.4

# builder
FROM ekidd/rust-musl-builder as builder
ADD --link . ./
RUN \
	--mount=type=cache,uid=1000,gid=1000,target=/home/rust/.cargo\
	--mount=type=cache,uid=1000,gid=1000,target=/home/rust/src/target\
	cargo build --release --target=x86_64-unknown-linux-musl && \
	mv target/x86_64-unknown-linux-musl/release/dofigen ../

# runtime
FROM scratch as runtime
WORKDIR /app
COPY --link --from=builder "/home/rust/dofigen" "/bin/"
ENTRYPOINT ["/bin/dofigen"]
CMD ["--help"]
