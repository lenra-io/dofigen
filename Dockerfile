# syntax=docker/dockerfile:1.11
# This file is generated by Dofigen v0.0.0
# See https://github.com/lenra-io/dofigen

# runtime
FROM scratch AS runtime
ARG TARGETPLATFORM
WORKDIR /app
COPY \
    --chown=1000:1000 \
    --chmod=555 \
    --link \
    "builds/${TARGETPLATFORM}/dofigen" "/bin/dofigen"
USER 1000:1000
ENTRYPOINT ["/bin/dofigen"]
CMD ["--help"]
