import { AppShell } from "@/components/AppShell";
import { FlagsEditor } from "@/components/FlagsEditor";
import { brainFetch } from "@/lib/brain";
import { requireSuperadmin } from "@/lib/session";
import type { FeatureFlag } from "@/lib/protocol";

export const instant = false;

export default async function FlagsPage() {
  const user = await requireSuperadmin();
  const res = await brainFetch("/v1/admin/flags");
  const flags: FeatureFlag[] = res.ok ? await res.json() : [];

  return (
    <AppShell user={user}>
      <h1 className="text-2xl">Feature flags</h1>
      <p className="mt-1 text-sm text-[var(--muted)]">Runtime toggles without redeploy.</p>
      <div className="mt-6">
        <FlagsEditor initial={flags} />
      </div>
    </AppShell>
  );
}
