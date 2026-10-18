"use client";

import Link from "next/link";
import { usePathname, useRouter } from "next/navigation";
import type { SessionUser } from "@/lib/brain";

export function AppShell({
  user,
  children,
}: {
  user: SessionUser;
  children: React.ReactNode;
}) {
  const pathname = usePathname();
  const router = useRouter();
  const isAdmin = user.role === "superadmin";

  async function logout() {
    await fetch("/api/auth/logout", { method: "POST" });
    router.push("/login");
    router.refresh();
  }

  const links = [
    { href: "/chat", label: "Chat" },
    ...(isAdmin
      ? [
          { href: "/admin", label: "Overview" },
          { href: "/admin/users", label: "Users" },
          { href: "/admin/flags", label: "Flags" },
          { href: "/admin/providers", label: "Providers" },
          { href: "/admin/persona", label: "Persona" },
        ]
      : []),
  ];

  return (
    <div className="flex min-h-full flex-1 flex-col">
      <header className="border-b border-[var(--line)] bg-[color-mix(in_srgb,var(--bg-elevated)_88%,transparent)] px-4 py-3 backdrop-blur">
        <div className="mx-auto flex max-w-6xl items-center justify-between gap-4">
          <div className="flex items-center gap-6">
            <Link href="/chat" className="brand text-lg text-[var(--accent)]">
              JARVIS
            </Link>
            <nav className="flex flex-wrap gap-3 text-sm text-[var(--muted)]">
              {links.map((l) => (
                <Link
                  key={l.href}
                  href={l.href}
                  className={
                    pathname === l.href || pathname.startsWith(l.href + "/")
                      ? "text-[var(--text)]"
                      : "hover:text-[var(--text)]"
                  }
                >
                  {l.label}
                </Link>
              ))}
            </nav>
          </div>
          <div className="flex items-center gap-3 text-sm">
            <span className="text-[var(--muted)]">
              {user.display_name ?? user.email}
              {isAdmin ? " · superadmin" : ""}
            </span>
            <button className="btn-ghost" type="button" onClick={logout}>
              Logout
            </button>
          </div>
        </div>
      </header>
      <div className="mx-auto w-full max-w-6xl flex-1 px-4 py-6">{children}</div>
    </div>
  );
}
