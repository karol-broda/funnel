# use build-time variables to make the base images easily configurable.
ARG GOLANG_VERSION=1.24-alpine
ARG ALPINE_VERSION=latest

# builder stage to compile the application.
FROM golang:${GOLANG_VERSION} AS builder

# build-time arguments for go build.
ARG TARGETOS=linux
ARG TARGETARCH=amd64
ARG APP_NAME=funnel-server

# set the working directory.
WORKDIR /app

# copy the entire project context.
# this is not ideal for caching, but for this project it's fine.
COPY . .

# compile the application.
# the output binary is placed in /app/bin.
RUN --mount=type=cache,target=/go/pkg/mod \
    --mount=type=cache,target=/root/.cache/go-build \
    CGO_ENABLED=0 GOOS=${TARGETOS} GOARCH=${TARGETARCH} \
    go build -v -o /app/bin/${APP_NAME} ./cmd/server

# final stage to create the production image.
FROM alpine:${ALPINE_VERSION}

# application user and group.
# using ARG so it can be changed at build time (e.g., --build-arg APP_USER=myuser).
ARG APP_USER=funnel
ARG APP_GROUP=funnel
ARG APP_HOME=/var/lib/funnel

# environment variables for runtime configuration.
# these can be overridden when running the container (e.g., -e FUNNEL_PORT=8081).
ENV FUNNEL_HOST="0.0.0.0"
ENV FUNNEL_PORT="8080"
ENV FUNNEL_TLS_PORT="8443"
ENV APP_NAME=funnel-server

# create the application user and group.
RUN addgroup -S "${APP_GROUP}" && adduser -S "${APP_USER}" -G "${APP_GROUP}"

# create the application's home directory and set permissions.
# this is used for storing certificates.
RUN mkdir -p "${APP_HOME}/certs" && chown -R "${APP_USER}:${APP_GROUP}" "${APP_HOME}"

# copy the compiled binary from the builder stage.
COPY --from=builder /app/bin/${APP_NAME} /usr/local/bin/${APP_NAME}

# copy the entrypoint script and make it executable.
COPY docker/server/entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

# set the user to run the application.
USER ${APP_USER}

# expose the default ports.
EXPOSE ${FUNNEL_PORT}
EXPOSE ${FUNNEL_TLS_PORT}

# the entrypoint script is responsible for executing the command.
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]

# default command to run when the container starts.
# the entrypoint script will prepend "/usr/local/bin/funnel-server" to these arguments.
CMD ["/usr/local/bin/funnel-server", "--host", "0.0.0.0", "--port", "8080"] 