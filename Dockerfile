# Runtime stage
FROM ubuntu:22.04


COPY ./target/release/devops-mcp /app/devops-mcp
WORKDIR /app
ENTRYPOINT ["/app/devops-mcp"]
