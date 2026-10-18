"use client";

import { FormEvent, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import Link from "next/link";
import { Suspense } from "react";

function InviteForm() {
  const router = useRouter();
  const params = useSearchParams();
  const [token, setToken] = useState(params.get("token") ?? "");
  const [password, setPassword] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    setLoading(true);
    setError(null);
    const res = await fetch("/api/auth/invite", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        token,
        password,
        display_name: displayName || null,
      }),
    });
    setLoading(false);
    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      setError(data.message ?? "Invite failed");
      return;
    }
    router.push("/chat");
    router.refresh();
  }

  return (
    <form onSubmit={onSubmit} className="mt-8 space-y-4">
      <label className="block text-sm text-[var(--muted)]">
        Invite token
        <input
          className="input mt-1"
          value={token}
          onChange={(e) => setToken(e.target.value)}
          required
        />
      </label>
      <label className="block text-sm text-[var(--muted)]">
        Display name
        <input
          className="input mt-1"
          value={displayName}
          onChange={(e) => setDisplayName(e.target.value)}
        />
      </label>
      <label className="block text-sm text-[var(--muted)]">
        Password
        <input
          className="input mt-1"
          type="password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          minLength={8}
          required
        />
      </label>
      {error && <p className="text-sm text-[var(--danger)]">{error}</p>}
      <button className="btn w-full" disabled={loading} type="submit">
        {loading ? "Creating account…" : "Accept invite"}
      </button>
    </form>
  );
}

export default function InvitePage() {
  return (
    <main className="flex min-h-full flex-1 items-center justify-center px-4 py-16">
      <div className="panel w-full max-w-md p-8">
        <p className="brand text-sm text-[var(--accent)]">JARVIS</p>
        <h1 className="mt-2 text-3xl">Accept invite</h1>
        <Suspense fallback={<p className="mt-8 text-[var(--muted)]">Loading…</p>}>
          <InviteForm />
        </Suspense>
        <p className="mt-6 text-sm text-[var(--muted)]">
          Already registered?{" "}
          <Link className="text-[var(--accent)]" href="/login">
            Sign in
          </Link>
        </p>
      </div>
    </main>
  );
}
