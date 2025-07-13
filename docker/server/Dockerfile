ARG ALPINE_VERSION=latest

FROM alpine:${ALPINE_VERSION}

ARG APP_USER=funnel
ARG APP_GROUP=funnel
ARG APP_HOME=/var/lib/funnel

ENV FUNNEL_HOST="0.0.0.0"
ENV FUNNEL_PORT="8080"
ENV FUNNEL_TLS_PORT="8443"
ENV APP_NAME=funnel-server

RUN addgroup -S "${APP_GROUP}" && adduser -S "${APP_USER}" -G "${APP_GROUP}"
RUN mkdir -p "${APP_HOME}/certs" && chown -R "${APP_USER}:${APP_GROUP}" "${APP_HOME}"

# goreleaser places the binary (named 'server' from the builds.binary config)
# and the entrypoint.sh (from the dockers.extra_files config)
# in the root of the build context.
COPY server /usr/local/bin/funnel-server
COPY docker/server/entrypoint.sh /usr/local/bin/entrypoint.sh

RUN chmod +x /usr/local/bin/entrypoint.sh

USER ${APP_USER}

EXPOSE ${FUNNEL_PORT}
EXPOSE ${FUNNEL_TLS_PORT}

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]

CMD ["/usr/local/bin/funnel-server", "--host", "0.0.0.0", "--port", "8080"] 