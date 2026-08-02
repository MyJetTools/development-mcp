# Runtime stage
FROM ubuntu:22.04


COPY ./target/release/development-mcp /app/development-mcp
WORKDIR /app
ENTRYPOINT ["/app/development-mcp"]
