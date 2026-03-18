## Release Guide — How to deploy services

### Single-repo (one service = one GitHub repo)

Create a GitHub release — tag and title are the version number:

```bash
gh release create 0.1.0 --title "0.1.0" --notes ""
```

The workflow triggers on any tag (`tags: "*"`), builds the service, and pushes the Docker image.

---

### Monorepo (multiple services in one GitHub repo)

Each service has its own tag pattern: `{service-name}-{version}`.

#### Create a release

```bash
gh release create {service-name}-{version} --title "{service-name}-{version}" --notes ""
```

Example:
```bash
gh release create price-feed-binance-0.1.0 --title "price-feed-binance-0.1.0" --notes ""
```

This creates both the GitHub release and the tag. The workflow file `release-{service-name}.yaml` triggers on tags matching `{service-name}-*` and builds only that service.

#### Check build status

```bash
# List recent runs
gh run list --limit 5

# Watch a specific run
gh run watch {run-id}

# View logs of a failed run
gh run view {run-id} --log-failed
```

#### Re-deploy the same version

Delete the release + tag, then create again:

```bash
gh release delete {service-name}-{version} --yes --cleanup-tag
gh release create {service-name}-{version} --title "{service-name}-{version}" --notes ""
```

---

### Prerequisites

Before the first release of a new service, make sure these files are committed and pushed:

1. **Dockerfile** — in the service directory (`{service-dir}/Dockerfile`)
2. **Workflow file** — `.github/workflows/release-{service-name}.yaml`

If the workflow file is not in the repo at the time the tag is created, GitHub will not trigger the build.

### Version extraction

The workflow extracts the version from the tag by stripping the `{service-name}-` prefix:

```yaml
TAG="${{ github.ref_name }}"          # e.g. margin-engine-0.2.1
VERSION="${TAG##*-}"                   # e.g. 0.2.1
```

This version is used for:
- Updating `version` in `Cargo.toml` during build
- Tagging the Docker image: `ghcr.io/{org}/{service-name}:{version}`
