FROM {{base-image}} AS chef

RUN cargo install cargo-chef
WORKDIR /app

COPY Cargo.toml Cargo.lock ./

RUN cargo chef prepare --recipe-path recipe.json
RUN cargo chef cook --recipe-path recipe.json

COPY . .

RUN cargo build --release

FROM debian:bookworm-slim AS runtime
WORKDIR /app
COPY --from=chef /app/target/release/{{project-name}} /usr/local/bin
COPY --from=chef /app/docker/entrypoint.sh /usr/local/bin
RUN chmod +x /usr/local/bin/entrypoint.sh

LABEL org.opencontainers.image.authors="{{authors}}"
LABEL org.opencontainers.image.description="{{project-description}}"
LABEL org.opencontainers.image.source="https://github.com/{{gh-username}}/{{project-name}}"

ENV RUST_LOG="gadget=info"
ENV BIND_ADDR="0.0.0.0"
ENV BIND_PORT=9632
ENV BLUEPRINT_ID=0
ENV SERVICE_ID=0

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]