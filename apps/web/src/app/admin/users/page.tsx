import { AppShell } from "@/components/AppShell";
import { InviteForm } from "@/components/InviteForm";
import { brainFetch } from "@/lib/brain";
import { requireSuperadmin } from "@/lib/session";

export const instant = false;

export default async function UsersAdminPage() {
  const user = await requireSuperadmin();
  const res = await brainFetch("/v1/admin/users");
  const users = res.ok ? await res.json() : [];

  return (
    <AppShell user={user}>
      <h1 className="text-2xl">Users & invites</h1>
      <p className="mt-1 text-sm text-[var(--muted)]">
        Invite-only access. Invited users get chat, not superadmin tools.
      </p>

      <div className="mt-6 grid gap-4 lg:grid-cols-2">
        <section className="panel p-4">
          <h2 className="text-sm text-[var(--muted)]">Accounts</h2>
          <ul className="mt-3 space-y-2 text-sm">
            {(users as { id: string; email: string; role: string; display_name: string | null }[]).map(
              (u) => (
                <li key={u.id} className="flex justify-between border-b border-[var(--line)] py-2">
                  <span>
                    {u.display_name ?? u.email}
                    <span className="ml-2 text-[var(--muted)]">{u.email}</span>
                  </span>
                  <span className="text-[var(--accent)]">{u.role}</span>
                </li>
              ),
            )}
          </ul>
        </section>
        <section className="panel p-4">
          <h2 className="text-sm text-[var(--muted)]">Create invite</h2>
          <InviteForm />
        </section>
      </div>
    </AppShell>
  );
}
