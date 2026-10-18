# OCI deploy notes

1. On the VM, create `~/jarvis` with:
   - `docker-compose.yml` (from repo root)
   - `deploy/caddy/Caddyfile`
   - `apps/brain/.env` (secrets; never commit)

2. Set `JARVIS_DOMAIN` for Caddy (or edit Caddyfile).

3. GitHub Actions secrets:
   - `OCI_HOST`, `OCI_USER`, `OCI_SSH_KEY`
   - Image pushes to GHCR as `ghcr.io/<owner>/jarvis-brain`

4. Workflow only builds `apps/brain` Docker context (not the whole monorepo).
