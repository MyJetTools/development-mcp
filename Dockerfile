# Runtime stage
# 24.04, not 22.04: the ONNX Runtime binary that `ort` links against is built
# against glibc >= 2.38 (it references __isoc23_strtoll). 22.04 ships glibc 2.35,
# where that symbol does not exist - the link fails there. The CI runner must
# stay on the same release for the same reason.
FROM ubuntu:24.04

# ca-certificates: the model weights are fetched from HuggingFace over TLS on
#   first start - without it the download fails with a certificate error.
# libgomp1: OpenMP runtime that the bundled ONNX Runtime links against.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libgomp1 \
    && rm -rf /var/lib/apt/lists/*

COPY ./target/release/development-mcp /app/development-mcp
WORKDIR /app

# Mount a volume here. Otherwise the ~450MB of model weights are re-downloaded
# on every container restart.
ENV FASTEMBED_CACHE_PATH=/app/model-cache
RUN mkdir -p /app/model-cache
VOLUME ["/app/model-cache"]

ENTRYPOINT ["/app/development-mcp"]
