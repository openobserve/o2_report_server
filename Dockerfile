# syntax=docker/dockerfile:1

FROM public.ecr.aws/debian/debian:trixie-slim AS runtime
RUN apt-get update -y && apt-get install -y --no-install-recommends ca-certificates chromium && \
    update-ca-certificates
COPY ./bin/report-generator /
RUN ["/report-generator", "init-dir", "-p", "/data/"]
CMD ["/report-generator"]
