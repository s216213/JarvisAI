import { AppShell } from "@/components/AppShell";
import { PersonaEditor } from "@/components/PersonaEditor";
import { brainFetch } from "@/lib/brain";
import { requireSuperadmin } from "@/lib/session";
import type { PersonaVersion } from "@/lib/protocol";

export const instant = false;

export default async function PersonaPage() {
  const user = await requireSuperadmin();
  const res = await brainFetch("/v1/persona");
  const versions: PersonaVersion[] = res.ok ? await res.json() : [];

  return (
    <AppShell user={user}>
      <h1 className="text-2xl">Persona versions</h1>
      <p className="mt-1 text-sm text-[var(--muted)]">
        Edit Jarvis system prompt, activate, or roll back without redeploy.
      </p>
      <div className="mt-6">
        <PersonaEditor initial={versions} />
      </div>
    </AppShell>
  );
}
