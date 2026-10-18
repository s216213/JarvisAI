# OCI deploy notes (system Caddy)

Caddy runs **on the VM host** (`/etc/caddy/Caddyfile`). Docker only runs `postgres` + `brain`.

## One-time VM setup

1. Install Docker + Compose plugin.
2. Install Caddy on the host.
3. Create deploy dir and secrets file:
   ```bash
   mkdir -p ~/jarvis
   # copy apps/brain/.env.example → ~/jarvis/.env and fill secrets
   nano ~/jarvis/.env
   ```
4. Host Caddy (edit domain first):
   ```bash
   sudo cp /path/to/repo/deploy/caddy/Caddyfile /etc/caddy/Caddyfile
   sudo systemctl enable --now caddy
   sudo systemctl reload caddy
   ```
5. OCI Security List / NSG: **22**, **80**, **443** (not 8080 publicly).

`docker-compose.yml` is synced automatically by GitHub Actions into `~/jarvis/`.

## Required on VM before first deploy

| Path | Purpose |
|------|---------|
| `~/jarvis/.env` | Brain secrets (`JWT_SECRET`, LLM keys, bootstrap admin, `CORS_ORIGINS`) |
| `/etc/caddy/Caddyfile` | `reverse_proxy 127.0.0.1:8080` |

## GitHub Actions secrets

- `OCI_HOST`, `OCI_USER`, `OCI_SSH_KEY`

## Local check on VM

```bash
curl -s http://127.0.0.1:8080/health
curl -s https://api.example.com/health
```
