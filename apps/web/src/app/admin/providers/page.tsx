import { AppShell } from "@/components/AppShell";
import { brainFetch } from "@/lib/brain";
import { requireSuperadmin } from "@/lib/session";

export const instant = false;

export default async function ProvidersPage() {
  const user = await requireSuperadmin();
  const res = await brainFetch("/v1/admin/providers");
  const data = res.ok ? await res.json() : { providers: [], configured: {} };

  return (
    <AppShell user={user}>
      <h1 className="text-2xl">LLM providers & quota</h1>
      <p className="mt-1 text-sm text-[var(--muted)]">
        Circuit breaker state, request counters, and configured keys (presence only).
      </p>
      <div className="mt-6 grid gap-4 md:grid-cols-3">
        {(data.providers as {
          name: string;
          state: string;
          requests: number;
          errors: number;
          latency_ms_ema: number | null;
          cooling_until: string | null;
        }[]).map((p) => (
          <section key={p.name} className="panel p-4">
            <h2 className="brand text-[var(--accent)]">{p.name}</h2>
            <dl className="mt-3 space-y-2 text-sm">
              <div className="flex justify-between">
                <dt className="text-[var(--muted)]">state</dt>
                <dd>{p.state}</dd>
              </div>
              <div className="flex justify-between">
                <dt className="text-[var(--muted)]">requests</dt>
                <dd>{p.requests}</dd>
              </div>
              <div className="flex justify-between">
                <dt className="text-[var(--muted)]">errors</dt>
                <dd>{p.errors}</dd>
              </div>
              <div className="flex justify-between">
                <dt className="text-[var(--muted)]">latency ema</dt>
                <dd>{p.latency_ms_ema ? `${p.latency_ms_ema.toFixed(0)} ms` : "—"}</dd>
              </div>
            </dl>
          </section>
        ))}
      </div>
      <pre className="panel mt-4 overflow-auto p-4 text-xs text-[var(--muted)]">
        {JSON.stringify(data.configured, null, 2)}
      </pre>
    </AppShell>
  );
}
