# Jarvis AI

Personal Iron Man–style agent: Rust brain on Oracle Cloud, Next.js admin/chat on Vercel, later Windows (C#) and Android (Flutter).

## Monorepo

| Path | Role | Deploy |
|------|------|--------|
| `apps/brain` | Rust Axum orchestrator (edition 2024; local SQLite, OCI Postgres) | Docker + GitHub Actions → OCI VM |
| `apps/web` | Next.js 16 UI + BFF | Vercel (`rootDirectory=apps/web`) |
| `packages/protocol` | Shared JSON Schema + TS types | consumed by web |
| `clients/*` | Desktop / Android (later) | — |

## Local development

### Brain

```bash
cd apps/brain
cp .env.example .env
# Local default: DATABASE_URL=sqlite:./data/jarvis.db (file DB, no Postgres)
# OCI/prod: DATABASE_URL=postgres://...
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

1. On the VM create `~/jarvis/.env` (from `apps/brain/.env.example`).
2. Host Caddy: `deploy/caddy/Caddyfile` → `/etc/caddy/Caddyfile`, set domain, `systemctl reload caddy`.
3. GitHub secrets: `OCI_HOST`, `OCI_USER`, `OCI_SSH_KEY`.
4. Push / run **Deploy Brain** — Actions syncs `docker-compose.yml` and runs `docker compose up -d postgres brain`.

See [deploy/README.md](deploy/README.md).

## Vercel

- Root Directory: `apps/web`
- Env: `BRAIN_URL` (server), `NEXT_PUBLIC_BRAIN_WS_URL` (WSS), `SESSION_SECRET`
