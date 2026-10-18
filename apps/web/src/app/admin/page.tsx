import { AppShell } from "@/components/AppShell";
import { brainFetch } from "@/lib/brain";
import { requireSuperadmin } from "@/lib/session";

export const instant = false;

export default async function AdminPage() {
  const user = await requireSuperadmin();
  const [providersRes, toolsRes, auditRes] = await Promise.all([
    brainFetch("/v1/admin/providers"),
    brainFetch("/v1/admin/tools"),
    brainFetch("/v1/admin/audit"),
  ]);
  const providers = providersRes.ok ? await providersRes.json() : null;
  const tools = toolsRes.ok ? await toolsRes.json() : null;
  const audit = auditRes.ok ? await auditRes.json() : [];

  return (
    <AppShell user={user}>
      <h1 className="text-2xl">Superadmin overview</h1>
      <p className="mt-1 text-sm text-[var(--muted)]">
        Devtools, provider health, and recent audit events.
      </p>

      <div className="mt-6 grid gap-4 md:grid-cols-2">
        <section className="panel p-4">
          <h2 className="text-sm text-[var(--muted)]">LLM providers</h2>
          <ul className="mt-3 space-y-2 text-sm">
            {(providers?.providers ?? []).map(
              (p: { name: string; state: string; requests: number; errors: number }) => (
                <li key={p.name} className="flex justify-between border-b border-[var(--line)] py-2">
                  <span>
                    {p.name} · <span className="text-[var(--accent)]">{p.state}</span>
                  </span>
                  <span className="text-[var(--muted)]">
                    {p.requests} req / {p.errors} err
                  </span>
                </li>
              ),
            )}
          </ul>
          <pre className="mt-3 overflow-auto text-xs text-[var(--muted)]">
            {JSON.stringify(providers?.configured ?? {}, null, 2)}
          </pre>
        </section>

        <section className="panel p-4">
          <h2 className="text-sm text-[var(--muted)]">Tool catalog</h2>
          <ul className="mt-3 space-y-2 text-sm">
            {(tools?.tools ?? []).map(
              (t: { name: string; capability: string; trust: string }) => (
                <li key={t.name} className="border-b border-[var(--line)] py-2">
                  <div>{t.name}</div>
                  <div className="text-xs text-[var(--muted)]">
                    {t.capability} · {t.trust}
                  </div>
                </li>
              ),
            )}
          </ul>
        </section>
      </div>

      <section className="panel mt-4 p-4">
        <h2 className="text-sm text-[var(--muted)]">Recent audit</h2>
        <ul className="mt-3 space-y-2 text-sm">
          {(audit as { id: string; action: string; created_at: string }[]).slice(0, 15).map((a) => (
            <li key={a.id} className="flex justify-between border-b border-[var(--line)] py-2">
              <span>{a.action}</span>
              <span className="text-[var(--muted)]">{new Date(a.created_at).toLocaleString()}</span>
            </li>
          ))}
        </ul>
      </section>
    </AppShell>
  );
}
