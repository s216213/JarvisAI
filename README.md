# Jarvis AI

Personal Iron Man–style agent: Rust brain on Oracle Cloud, Next.js admin/chat on Vercel, later Windows (C#) and Android (Flutter).

## Monorepo

| Path | Role | Deploy |
|------|------|--------|
| `apps/brain` | Rust Axum orchestrator (edition 2024, MSRV 1.94) | Docker + GitHub Actions → OCI VM |
| `apps/web` | Next.js 16 UI + BFF | Vercel (`rootDirectory=apps/web`) |
| `packages/protocol` | Shared JSON Schema + TS types | consumed by web |
| `clients/*` | Desktop / Android (later) | — |

## Local development

### Brain

```bash
# Postgres (Docker) or any Postgres 16
cd apps/brain
cp .env.example .env
# set DATABASE_URL, JWT_SECRET, optional LLM keys, bootstrap superadmin
cargo run
```

Health: `GET http://localhost:8080/health`

### Web

```bash
cd apps/web
cp .env.example .env.local
pnpm install
pnpm dev
```

## Architecture (MVP)

- **BFF** on Vercel: HttpOnly cookies, REST proxy, WS ticket
- **WSS** browser → OCI brain with short-lived ticket
- **LLM**: Groq → Gemini → OpenRouter + quota guardian
- **Memory**: profile / facts / thread summaries (Postgres)
- **Persona**: versioned system prompt editable by superadmin
- **Devices**: capability registry + tool hop protocol

## OCI deploy

1. Copy `docker-compose.yml` + brain `.env` to `~/jarvis` on the VM (Docker: `postgres` + `brain` only).
2. Use **host Caddy**: copy `deploy/caddy/Caddyfile` → `/etc/caddy/Caddyfile`, set your domain, `systemctl reload caddy`.
3. GitHub secrets: `OCI_HOST`, `OCI_USER`, `OCI_SSH_KEY`.
4. Push to `main` under `apps/brain/**` → GHCR build + `docker compose up -d postgres brain`.

See [deploy/README.md](deploy/README.md).

## Vercel

- Root Directory: `apps/web`
- Env: `BRAIN_URL` (server), `NEXT_PUBLIC_BRAIN_WS_URL` (WSS), `SESSION_SECRET`
