---
alwaysApply: true
---
# Single-VM Unix-Socket Setup

How microservices are wired together when the entire system runs on **one host**. No TCP between internal services, no service mesh, no Kubernetes — every service-to-service hop is a unix-socket file on the host filesystem.

This pattern applies to all our production hosts. If you're writing a compose file for a new service or a new product, follow this exactly.

## Why unix sockets, not TCP

- **No port allocation.** With dozens of services on one host, TCP would force a port registry. Unix sockets are paths — no collisions.
- **Lower latency, lower overhead.** Kernel never touches the TCP stack; bytes go straight between processes.
- **No accidental external exposure.** A unix socket cannot be reached from outside the host — internal traffic is sealed by default. Only services that *must* be public (REST API gateways, admin UIs behind the reverse proxy) bind a TCP port.
- **Process-isolation still works.** Containers see only the directories we mount; service A in product X cannot reach service B in product Y unless we explicitly mount the same socket dir into both.

## Two scopes: `system` and `<product>`

Sockets live under `~/unix-sockets/` on the host. Two scopes:

| Scope                         | Purpose                                                                                       | Lifetime                          |
| ----------------------------- | --------------------------------------------------------------------------------------------- | --------------------------------- |
| `~/unix-sockets/system/`      | Cross-product infrastructure: settings-service, logger, certbot/CA — **one copy per host**.    | Live as long as the host does.    |
| `~/unix-sockets/<product>/`   | One subdirectory per product (`margin-trading`, `virtual-fans`, `crypto-processing`, …). Holds product-internal sockets and product-private infrastructure (this product's SB broker). | Lives as long as the product stack does. |

This split exists because multiple products run side-by-side on one host. Each product gets its own SB broker, its own service inventory, its own settings namespace — but they all share the system-level settings-service / logger.

## Directory layout

```
~/unix-sockets/
├── system/                                ← cross-product
│   ├── settings.http-sock                 ← settings-service HTTP
│   ├── logger.http-sock                   ← my-logger UI/API
│   └── …                                  ← certbot, CA, etc.
│
├── margin-trading/                        ← one product
│   ├── system/                            ← product-private infra
│   │   └── …                              ← THIS product's SB broker socket(s)
│   ├── http/                              ← HTTP sockets of services in this product
│   │   ├── margin-engine                  ← file named exactly as <service-name>
│   │   ├── price-feed-binance
│   │   └── …
│   └── grpc/                              ← gRPC sockets of services in this product
│       ├── margin-engine
│       └── …
│
└── virtual-fans/                          ← another product, fully parallel
    ├── system/
    ├── http/
    └── grpc/
```

**Socket file naming:**
- A service's HTTP socket: `~/unix-sockets/<product>/http/<service-name>` — no extension, the file is named exactly after the service.
- A service's gRPC socket: `~/unix-sockets/<product>/grpc/<service-name>`.
- System-level sockets (settings, logger): conventionally have a `.http-sock` / `.tcp-sock` suffix because they're set up manually, not via service-sdk's defaults.

## The 4 standard volume mounts

**Every** service container mounts the same 4 directories:

```yaml
volumes:
- ~/unix-sockets/<product>/http:/root/http                      # my HTTP socket goes here
- ~/unix-sockets/<product>/grpc:/root/grpc                      # my gRPC socket goes here
- ~/unix-sockets/<product>/system:/root/product-system-sockets  # this product's SB / shared infra
- ~/unix-sockets/system:/root/system-sockets                    # host-wide infra (settings, logger)
```

| Mount path inside container       | What's there                                                                                       | Used for                                                                                                                              |
| --------------------------------- | -------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `/root/http/`                     | Empty when the container starts; service-sdk creates `<service-name>` socket here on boot.         | Other services in the **same product** open it as `http+unix:///root/http/<service-name>:…`.                                          |
| `/root/grpc/`                     | Same idea but for the gRPC server (only when service has gRPC).                                    | gRPC clients in the same product dial `grpc+unix:///root/grpc/<service-name>`.                                                        |
| `/root/product-system-sockets/`   | Sockets of product-private infra: this product's SB broker, optionally a product-private Postgres. | The service reaches its own product's SB on these paths.                                                                              |
| `/root/system-sockets/`           | settings-service, logger, certbot, CA — host-wide.                                                 | Always how settings get loaded: `SETTINGS_URL=http+unix://root/system-sockets/settings.http-sock:/settings/<product>/<service>`.       |

**Cross-product traffic is intentionally impossible** through this layout. Service in product X mounts only its own `<product>/{http,grpc,system}` plus the global `system/`. To talk to product Y, it would have to go through a system-level component (SB, settings) — and that's by design.

## service-sdk env vars

The settings shape every service-sdk container expects:

```yaml
environment:
- SETTINGS_URL=http+unix://root/system-sockets/settings.http-sock:/settings/<product>/<service>
- UNIX_SOCKET=1
- ENV_INFO=HOME
```

| Env var          | Effect                                                                                                                                                                                                                  |
| ---------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `SETTINGS_URL`   | The settings reader uses this URL to fetch the YAML template. `http+unix://root/system-sockets/settings.http-sock` is the unix-socket "host"; everything after `:` is the URL path: `/settings/<product>/<service>`.   |
| `UNIX_SOCKET=1`  | Tells service-sdk to start its HTTP listener on `/root/http/<service-name>` (and the gRPC listener on `/root/grpc/<service-name>` when the `grpc` feature is enabled). `1` = both TCP and unix; `ONLY` = unix-only.    |
| `ENV_INFO=HOME`  | Label propagated to logs / telemetry / SB queue suffixes. Identifies which datacenter / environment the service is running in.                                                                                          |

**TCP ports stay commented out** for internal services. Compose files keep `# ports:` as a hint — uncomment only if the service must be reached from off-host (REST gateways, admin UIs behind the reverse proxy).

## Memory and logging caps

Standard per-service compose footer:

```yaml
deploy:
  resources:
    limits:
      memory: 64Mb          # bump per-service if profile demands it (128/256Mb)
logging:
  options:
    max-size: "512Kb"
    max-file: "1"
networks:
- docker_net

networks:
  docker_net:
    external: true
```

The 64Mb default is intentional — forces small, focused services. Bump to 128–256Mb only when a measured profile demands it (in-memory caches, heavyweight crypto, etc.) — document the reason.

`docker_net` is a manually-created bridge network; release-mcp will not create it. Networks aren't load-bearing for unix-socket traffic but matter for cases where the service does call out via TCP (gRPC over TCP to another host, WebSocket to an external feed).

## Onboarding a new product

When a brand-new product (e.g. `telegram-trading`) lands on the host:

1. **Create the directory tree** on the host (or let the first stack do it via the volume mount):
   ```bash
   mkdir -p ~/unix-sockets/<product>/{http,grpc,system}
   ```
2. **Stand up the product's SB broker stack** under `<product>/system/my-sb` (and `my-sb-persistence`). Use the existing `margin-trading/system/my-sb` as the template. The SB broker's unix socket will live under `~/unix-sockets/<product>/system/`.
3. **Register the product in settings-service** — create a product entry with the product id matching the directory name.
4. **All subsequent service stacks** in this product copy the standard compose template with `<product>` substituted.

## Onboarding a new service in an existing product

1. **Copy compose from a sibling** in the same product (`margin-trading/binance-price-feed` is a good background-worker template; `virtual-fans/chats-grpc` for a service with a gRPC API).
2. **Substitute** the service name in `image`, `hostname`, `container_name`, and `SETTINGS_URL` path.
3. **Add a settings template** in settings-service under `product_id=<product>`, `template_id=<service-name>` — mirrors any other service in the product.
4. **Define a new `~/unix-sockets/<product>/http/<service-name>` consumer** in whichever service needs to call this one over HTTP (or `grpc/<service-name>` for gRPC). The consuming service already mounts that directory — it just needs to know the path.

## Background-worker template (no inbound API)

For a service that has no public API and is driven by an external feed (or an SB subscription) — like `price-feed-binance`, `margin-engine`, or `telegram-ingest`:

```yaml
services:
  <service-name>:
    image: ghcr.io/<org>/<service-name>:<version>
    hostname: <service-name>
    container_name: <service-name>
    restart: always
    environment:
    - SETTINGS_URL=http+unix://root/system-sockets/settings.http-sock:/settings/<product>/<service-name>
    - ENV_INFO=HOME
    - UNIX_SOCKET=1
#    ports:
#    - <host-port>:8888    # only if external HTTP/gRPC clients must reach this service
    volumes:
    - ~/unix-sockets/<product>/http:/root/http
    - ~/unix-sockets/<product>/grpc:/root/grpc
    - ~/unix-sockets/<product>/system:/root/product-system-sockets
    - ~/unix-sockets/system:/root/system-sockets
    deploy:
      resources:
        limits:
          memory: 64Mb
    logging:
      options:
        max-size: "512Kb"
        max-file: "1"
    networks:
    - docker_net

networks:
  docker_net:
    external: true
```

Even though there's no API, the 4 volume mounts and the `UNIX_SOCKET=1` env stay — service-sdk's built-in `/api/isalive` and `/metrics` still serve on the unix socket, and operators can `curl --unix-socket ~/unix-sockets/<product>/http/<service-name> http://localhost/api/isalive` to probe liveness.

## Gateway-service template (TCP exposed)

For services that must be reachable from outside the VM — a public REST API, an admin UI behind the reverse proxy, a gRPC endpoint open to other hosts — uncomment the `ports:` block:

```yaml
    ports:
    - <host-port>:8888
```

Otherwise the file is identical to the background-worker template above. These services still mount the 4 standard volumes, still read settings over `system-sockets`, still publish their own unix socket — the TCP listener is an *additional* surface, not a replacement.

## Anti-patterns

- **Mounting another product's `<product>` directory.** If you find yourself reaching across products via unix socket — stop. Cross-product traffic goes through SB (or, very occasionally, through a system-level read-model). A direct mount couples two products at the filesystem level and breaks the isolation that justifies the split.
- **Binding TCP ports for internal-only services.** Defeats the purpose of unix-socket layout and reintroduces the port-allocation problem.
- **Renaming the socket directories per-service.** Names like `http`, `grpc`, `system`, `system-sockets` are conventions baked into service-sdk's defaults. Renaming forces every other service to know the exception.
- **Using `~/unix-sockets/system/` as a dumping ground for one-product infra.** That directory is reserved for genuinely cross-product components (settings-service, logger, certbot). Product-private things go in `<product>/system/`, not in the global `system/`.
- **Storing service state in the unix-socket volume.** Sockets are volatile communication endpoints, not durable storage. State that must survive container restart lives in a separate bind-mount (Postgres data dir, session files, etc.).

## How this fits with the rest of the stack

- **Settings:** Always read via `SETTINGS_URL=http+unix://root/system-sockets/settings.http-sock:/settings/<product>/<service>`. See `application-architecture-best-practices` for the `SettingsReader` derive pattern that consumes this URL.
- **Service Bus:** Each product runs its own SB broker. The unix-socket path to that broker is mounted under `/root/product-system-sockets/`. Cross-product SB is not standard — define a separate broker per product.
- **Logger / telemetry:** Always system-level. The logger socket lives under `/root/system-sockets/` and the `seq_conn_string` setting points to it.
- **Reverse proxy / TLS:** Sits in front and translates external HTTPS → internal HTTP. The reverse proxy's upstream connection can itself be a unix socket (when the upstream service is on the same host), so even gateway services can avoid TCP entirely. That's a separate concern; see the reverse-proxy doc.
