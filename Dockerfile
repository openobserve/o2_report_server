# syntax=docker/dockerfile:1

FROM public.ecr.aws/debian/debian:trixie-slim AS runtime
RUN apt-get update -y && \
    apt-get install -y --no-install-recommends ca-certificates chromium && \
    apt-get install -y --only-upgrade libbrotli1 brotli && \
    update-ca-certificates && \
    rm -rf /var/lib/apt/lists/*
COPY ./bin/report-generator /
RUN ["/report-generator", "init-dir", "-p", "/data/"]
CMD ["/report-generator"]
