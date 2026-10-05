# CI — monorepo (multiple services in one GitHub repo)

Do **NOT** use `ci-utils` / `build.rs` for CI. Create the Dockerfile and the workflows manually per
service. (`build.rs` is still used in a monorepo for proto files and CSS — just never with
`CiGenerator`.)

Each service gets its own tag pattern `{service-name}-*` and **two** workflow files:

| File | Trigger | What it does |
|---|---|---|
| `release-{service-name}.yaml` | tag `{service-name}-*` | builds the service and pushes the runtime image — ~2 min |
| `build-{service-name}-docker.yaml` | `workflow_dispatch` | bakes the dependency graph into a builder image — ~10 min, run by hand |

Generate both. A release workflow without its builder image works, it is just five times slower.

## Dockerfile — `{service-dir}/Dockerfile`

```dockerfile
FROM ubuntu:22.04
COPY ./target/release/{service-name} ./target/release/{service-name}
ENTRYPOINT ["./target/release/{service-name}"]
```

## Workflow — `.github/workflows/release-{service-name}.yaml`

```yaml
name: Release App
on:
  push:
    tags:
      - "{service-name}-*"

env:
  IMAGE_NAME: ghcr.io/{repo-org}/{service-name}
  BUILD_IMAGE: ghcr.io/{repo-org}/{service-name}-build-docker
  DIR: {service-dir}

jobs:
  build:
    runs-on: ubuntu-22.04
    steps:
      - uses: actions/checkout@v6.0.2

      - name: Get the version
        id: get_version
        run: |
          TAG="${{ github.ref_name }}"
          VERSION="${TAG##*-}"
          echo "VERSION=$VERSION" >> "$GITHUB_OUTPUT"

      - name: Updating version
        run: |
          cd ${DIR}
          sed -i -e 's/^version = .*/version = "${{ steps.get_version.outputs.VERSION }}"/' Cargo.toml

      - name: Docker login
        run: |
          echo "${{ secrets.PUBLISH_TOKEN }}" | docker login https://ghcr.io -u "${{ github.actor }}" --password-stdin

      - name: Pull the builder image
        id: builder
        continue-on-error: true
        run: docker pull ${BUILD_IMAGE}:latest

      - name: Build (warm, in the builder image)
        if: steps.builder.outcome == 'success'
        run: |
          docker run --rm \
            -v "${PWD}:/src" \
            -w /src/${DIR} \
            ${BUILD_IMAGE}:latest \
            bash -c "cargo build --release \
              && mkdir -p target/release \
              && cp /build/target/release/${DIR} target/release/${DIR}"

      - uses: actions-rust-lang/setup-rust-toolchain@v1.15.2
        if: steps.builder.outcome != 'success'
        with:
          toolchain: stable
          rustflags: ""

      - name: Install Protoc
        if: steps.builder.outcome != 'success'
        run: sudo apt-get update && sudo apt-get install -y protobuf-compiler

      - name: Build (cold, on the runner)
        if: steps.builder.outcome != 'success'
        run: |
          export GIT_HUB_TOKEN="${{ secrets.PUBLISH_TOKEN }}"
          cd ${DIR}
          cargo build --release

      - name: Docker Build and Publish
        run: |
          cd ${DIR}
          docker build -t ${IMAGE_NAME}:${{ steps.get_version.outputs.VERSION }} .
          docker push ${IMAGE_NAME}:${{ steps.get_version.outputs.VERSION }}
```

`${DIR}` is expanded by the **runner** before `docker run`, which is why the inner `bash -c` is in
double quotes — the container has no `DIR`.

The `cp` line uses `${DIR}` as the **binary** name, and the runtime Dockerfile copies
`./target/release/{service-name}`. That only lines up while `{service-dir}` and `{service-name}` are
the same string, which is the normal case. If they differ, add a separate `BIN: {service-name}` to
the workflow `env:` and use `${BIN}` for the binary in both the `cp` and the Dockerfile — otherwise
the warm build succeeds and the `docker build` fails on a missing file.

## Builder image — `.github/workflows/build-{service-name}-docker.yaml`

A release used to spend **thirteen of its fourteen minutes** rebuilding a dependency graph that had
not changed. This workflow bakes that graph into a Docker image once, by hand; every release then
mounts fresh sources over it and compiles only the delta. Measured on `accumulator-grpc`,
`ubuntu-22.04`: release **9 m 55 s → 2 m 27 s** (pull 41 s, compile 83 s, image 9 s); baking the
image itself takes ~9 m 36 s and is *not* run per release.

`workflow_dispatch` **only** — there is no Docker daemon on the dev machine, so a builder image can
only be tested where it is built, and manual dispatch lets you iterate on it without burning release
tags. It writes the builder Dockerfile inline with a heredoc, over the runtime `Dockerfile`, in the
runner's throwaway checkout — so the service folder keeps exactly one Dockerfile, the runtime one.

```yaml
name: Build {service-name}-build-docker

on:
  workflow_dispatch:

env:
  IMAGE_NAME: ghcr.io/{repo-org}/{service-name}-build-docker
  DIR: {service-dir}

jobs:
  build:
    runs-on: ubuntu-22.04
    steps:
      - uses: actions/checkout@v6.0.2

      - name: Docker login
        run: |
          echo "${{ secrets.PUBLISH_TOKEN }}" | docker login https://ghcr.io -u "${{ github.actor }}" --password-stdin

      # Quoted heredoc (<<'DOCKERFILE') — unquoted, the runner's shell would expand $PATH in
      # `ENV PATH=...:$PATH` and bake the RUNNER's PATH into the image.
      - name: Generate the builder Dockerfile
        run: |
          cat > ${DIR}/Dockerfile <<'DOCKERFILE'
          FROM ubuntu:22.04

          ENV DEBIAN_FRONTEND=noninteractive
          ENV CARGO_HOME=/usr/local/cargo
          ENV CARGO_TARGET_DIR=/build/target
          ENV PATH=/usr/local/cargo/bin:$PATH

          RUN apt-get update && apt-get install -y --no-install-recommends \
                  build-essential \
                  ca-certificates \
                  curl \
                  git \
                  libprotobuf-dev \
                  libssl-dev \
                  pkg-config \
                  protobuf-compiler \
              && rm -rf /var/lib/apt/lists/*

          RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
              | sh -s -- -y --default-toolchain stable --profile minimal

          COPY . /src

          RUN cd /src/{service-dir} && cargo build --release

          # Emptied on purpose: an image run WITHOUT the release's mount would otherwise compile the
          # sources baked at build time and look like it worked.
          RUN rm -rf /src && mkdir -p /src

          WORKDIR /src/{service-dir}
          DOCKERFILE
          cat ${DIR}/Dockerfile

      - name: Build and push the builder image
        run: |
          docker build -f ${DIR}/Dockerfile -t ${IMAGE_NAME}:latest .
          docker push ${IMAGE_NAME}:latest
```

**Load-bearing rules.** Each of these silently turns a warm build into a cold one — no error, just
the old fourteen minutes:

1. **`CARGO_HOME` and `CARGO_TARGET_DIR` must live OUTSIDE `/src`.** The release bind-mounts its
   checkout over `/src`, and a bind mount *hides* whatever the image had underneath. A target dir
   inside `/src` disappears at exactly the moment it is supposed to be reused.
2. **Bake at the same absolute path the release mounts** (`/src`). Cargo fingerprints record
   absolute paths; the same sources at another path are a cold build wearing a warm image's name.
3. **The base must match the runtime image's base** (`ubuntu:22.04`). The binary is copied into the
   runtime image, not rebuilt there — a builder on a newer base links it against a newer glibc and
   the container dies at start-up on a symbol lookup.
4. **`Cargo.lock` must be committed** — un-ignore it per service in `.gitignore`. With the lock
   ignored CI resolves fresh and takes the newest semver-compatible release of every transitive
   crate, so one patch published anywhere in a ~400-crate graph invalidates the whole image. Warm
   hits become a lottery you cannot even measure. Bonus: a broken transitive dependency now
   reproduces locally instead of only in CI.
5. **Build context is the repo root**, not the service folder — sibling crates arrive as path
   dependencies and `build.rs` reads `../proto-files/`.
6. **Pre-install what the graph needs**, and it is more than protoc: `build-essential` (jemalloc is
   a configure+make build), `libprotobuf-dev` (protoc without the well-known types is not protoc),
   `pkg-config`, `libssl-dev`, `git`, `ca-certificates`.

**Do not pass a build secret to the builder image.** All MyJetTools / my-ai-utils git dependencies
are public. A token passed as `--build-arg` is written into the image history, where anyone who can
pull the image can read it back. If a private dependency ever appears, use a BuildKit secret mount
— never `--build-arg`.

**Compile-time secrets need `-e` on the warm step, or they vanish silently.** If the service bakes
values into the binary (`ci_utils::bake_compile_time_secret("X")`, or `add_compile_time_secret` on a
`CiGenerator` build), note that the cold step `export`s them into the runner's shell while
`docker run` starts a container with none of the runner's environment. `build.rs` then sees nothing
— and it cannot even complain, because the guard that fails a release build keys off
`GITHUB_ACTIONS=true`, which is also absent inside the container. The result is a green workflow
shipping a binary whose `option_env!` returned `None`. Pass each secret explicitly:

```yaml
          docker run --rm \
            -e ENCRYPTION_KEY="${{ secrets.ENCRYPTION_KEY }}" \
            -e GITHUB_ACTIONS=true \
            -v "${PWD}:/src" \
```

`-e` sets the environment of one container run — it is not written into any image layer, so this is
not the `--build-arg` mistake above. Keep `-e GITHUB_ACTIONS=true` so a missing secret still turns
the step red instead of shipping quietly.

Only the service crate and its path-dependency siblings recompile each release: `actions/checkout`
stamps every file with the checkout time, so cargo considers all of them dirty regardless. That is
fine and expected — they are small. The dependency sources keep their baked mtimes inside the
image's `CARGO_HOME`, and that graph is the thirteen minutes.

**Do not use the GitHub Actions cache for this.** It is scoped **by ref**: a run reads caches
written by its own ref or by the default branch, and every release runs on a fresh `{service-name}-*`
tag — so each build writes hundreds of megabytes under a ref nothing will ever read from again. With
~28 services sharing one 10 GB repo quota they also evict each other. `cache-workspaces` on
`setup-rust-toolchain` looks like it is working and is not: the tell is a cache step that finishes in
three seconds. A registry image has no ref scoping — written once, read by every tag afterwards.

## Adopting the builder image for a service

1. Add both workflow files (copy from an adopted service, substitute the name — the files differ by
   nothing else).
2. Un-ignore its lock: `!{service-dir}/Cargo.lock` in `.gitignore`, then commit the lock. Verify
   with `cargo check --locked` first — a lock that does not satisfy the manifests fails the build.
3. Run `build-{service-name}-docker.yaml` by hand and let it finish.
4. Release as usual. Confirm in the run that *Build (warm, …)* ran and the cold steps were skipped.

**Not for Dioxus WASM builds** (`dashboards-ui` and friends) — own toolchain, own `dx build`, built
inside the `ghcr.io/my-jet-tools/dioxus-docker` container instead. Different problem, different fix: see the
Dioxus client-side bootstrap guide (`get_dioxus_client_side_bootstrap_guide`, topic `ci`) for that workflow.

**Placeholders to replace:**
- `{service-name}` — crate name from `Cargo.toml` (e.g. `price-feed-binance`)
- `{service-dir}` — directory name; keep it identical to `{service-name}` unless there is a reason
  not to, otherwise add a `BIN` env var as described above
- `{repo-org}` — GitHub org or repo path for docker image (e.g. `my-margin-trading`)

**Release:** `gh release create {service-name}-0.1.0 --title "{service-name}-0.1.0" --notes ""` —
this creates both the release and the tag. Full flow, re-deploys and troubleshooting: the release
guide (`get_release_guide`).

**Rules:**
- One workflow file per service: `release-{service-name}.yaml`, plus one
  `build-{service-name}-docker.yaml` for its builder image
- Tag pattern: `{service-name}-*` — version is extracted after the last `-`
- Dockerfile lives inside the service directory, not at repo root — and it is the **runtime** one;
  the builder Dockerfile is generated inline by the dispatch workflow and never committed
- A release never breaks because of the builder: the pull step is `continue-on-error` and the cold
  steps behind the `if:` are the exact build used before the image existed. The worst outcome of a
  bad image is the time we used to pay anyway
- Re-bake the builder image by hand when the graph moves — a MyJetTools tag bumped, a dependency
  added, `Cargo.lock` updated. Nothing else needs it
