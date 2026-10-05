# CI — static-hosting Dockerfile, cache busting, GitHub Actions workflow

## Dockerfile — Static Hosting

Client-side Dioxus compiles to static WASM + HTML + JS + CSS. Use `ghcr.io/my-jet-tools/web-app-host` to serve:

```dockerfile
FROM ghcr.io/my-jet-tools/web-app-host:0.1.0
ARG BUILD_VERSION
ENV BUILD_VERSION=$BUILD_VERSION

ARG RFC3339_TIME
ENV COMPILE_TIME=$RFC3339_TIME

WORKDIR /app
COPY ./target/dx/your-project-name/release/web/public ./wwwroot
```

## build.py — Cache Busting

Place `build.py` in project root. CI runs it after `dx build` to append random query strings to `.wasm`, `.js`, `.css` references in `index.html`:

```python
import random
import string
import argparse

def replace_wasm_with_random_string(file_path):
    def generate_random_string(length=16):
        characters = string.ascii_letters + string.digits
        return ''.join(random.choice(characters) for _ in range(length))

    with open(file_path, 'r') as file:
        content = file.read()

    updated_content = content.replace('.wasm', f'.wasm?id={generate_random_string()}')
    updated_content = updated_content.replace('.js', f'.js?id={generate_random_string()}')
    updated_content = updated_content.replace('.css', f'.css?id={generate_random_string()}')

    with open(file_path, 'w') as file:
        file.write(updated_content)

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Cache-bust static assets in index.html")
    parser.add_argument("file_path", help="Path to the index.html file")
    args = parser.parse_args()
    replace_wasm_with_random_string(args.file_path)
```

## CI Workflow (GitHub Actions)

Tag trigger: `your-project-name-*`. Two jobs: build (in `dioxus-docker` container) and publish (Docker image to GHCR).

> **This is the whole CI story for a Dioxus WASM client — use the workflow below verbatim.**
> The pre-baked builder image from the app-bootstrap guide (`get_app_bootstrap_guide`, topic `ci-monorepo`: `{service-name}-build-docker`,
> `build-{service-name}-docker.yaml`) is for **native Rust monorepo services only**. It does not
> apply here: this build runs `dx build --release --web` with its own toolchain inside
> `ghcr.io/my-jet-tools/dioxus-docker`, which already ships the toolchain the build needs. Do not add a
> builder-image workflow, a `docker pull`/warm-cold pair of steps, or `CARGO_TARGET_DIR` juggling to
> a Dioxus client project.

```yaml
name: Release App
on:
  push:
    tags:
      - "your-project-name-*"

env:
  IMAGE_NAME: ghcr.io/your-org/your-project-name
  DIR: your-project-name
  APP_NAME: your-project-name
  DELIVER_NAME: your-project-name

jobs:
  build:
    runs-on: ubuntu-22.04
    container:
      image: ghcr.io/my-jet-tools/dioxus-docker:0.7.10
    steps:
      - uses: actions/checkout@v6.0.2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

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

      - run: |
          export GIT_HUB_TOKEN="${{ secrets.PUBLISH_TOKEN }}"
          cd ${DIR}
          dx build --release --web
          ls ./target/dx/${APP_NAME}/release/web/public
          python3 build.py ./target/dx/${APP_NAME}/release/web/public/index.html

      - name: Zip and upload
        run: |
          cd ${DIR}
          FILE_NAME="https://jetdev.eu/file/${DELIVER_NAME}-build.zip"
          apt install zip
          zip -r data.zip ./target/dx/${APP_NAME}/release/web ./Dockerfile
          curl -X 'POST' $FILE_NAME -H 'accept: */*' -H 'Content-Type: multipart/form-data' -F 'file=@data.zip;type=application/zip'

  publish:
    runs-on: ubuntu-22.04
    needs: build
    steps:
      - uses: actions/checkout@v6.0.2

      - name: Download Build Artifacts
        run: |
          cd ${DIR}
          FILE_NAME="https://jetdev.eu/file/${DELIVER_NAME}-build.zip"
          curl -L -o data.zip $FILE_NAME
          unzip -o data.zip

      - name: Get the version
        id: get_version
        run: |
          TAG="${{ github.ref_name }}"
          VERSION="${TAG##*-}"
          echo "VERSION=$VERSION" >> "$GITHUB_OUTPUT"

      - name: Docker login
        run: |
          echo "${{ secrets.PUBLISH_TOKEN }}" | docker login https://ghcr.io -u "${{ github.actor }}" --password-stdin

      - name: Docker Build and Publish
        run: |
          cd ${DIR}
          docker build -t ${IMAGE_NAME}:${{ steps.get_version.outputs.VERSION }} .
          docker push ${IMAGE_NAME}:${{ steps.get_version.outputs.VERSION }}
```
