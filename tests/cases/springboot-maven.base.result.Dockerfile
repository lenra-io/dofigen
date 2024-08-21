# This file is generated by Dofigen v0.0.0
# See https://github.com/lenra-io/dofigen

# syntax=docker/dockerfile:1.7

# maven-builder
FROM maven:3.9-eclipse-temurin-17-alpine AS maven-builder
WORKDIR /app
COPY \
    --link \
    "." "./"
USER 0:0
RUN \
    --mount=type=cache,target=/root/.m2,uid=0,gid=0 \
    --mount=type=cache,target=/app/target,uid=0,gid=0 \
    <<EOF
mvn package -DskipTests
mv target/*.jar app.jar
EOF

# runtime
FROM eclipse-temurin:17-jre-alpine AS runtime
COPY \
    --from=maven-builder \
    --chown=1000:1000 \
    --link \
    "/app/app.jar" "app.jar"
USER 1000:1000
CMD ["java", "-jar", "app.jar"]
