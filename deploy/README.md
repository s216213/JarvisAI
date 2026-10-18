# OCI deploy notes (system Caddy)

Caddy runs **on the VM host** (`/etc/caddy/Caddyfile`). Docker only runs `postgres` + `brain`.

## One-time VM setup

1. Install Docker + Compose plugin.
2. Install Caddy (system package / official repo).
3. Create `~/jarvis` with:
   - `docker-compose.yml`
   - `apps/brain/.env` (secrets; never commit)
4. Copy reverse-proxy config:
   ```bash
   sudo cp ~/jarvis/deploy/caddy/Caddyfile /etc/caddy/Caddyfile
   # edit domain, then:
   sudo systemctl enable --now caddy
   sudo systemctl reload caddy
   ```
5. Open OCI Security List / NSG: **22**, **80**, **443** (not 8080 publicly).

## GitHub Actions secrets

- `OCI_HOST`, `OCI_USER`, `OCI_SSH_KEY`
- Image: `ghcr.io/<owner>/jarvis-brain`

Workflow builds only `apps/brain` and runs:

```bash
cd ~/jarvis
docker compose pull brain
docker compose up -d postgres brain
```

## Local check on VM

```bash
curl -s http://127.0.0.1:8080/health
curl -s https://api.example.com/health
```
