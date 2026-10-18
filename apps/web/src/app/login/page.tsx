"use client";

import { FormEvent, useState } from "react";
import { useRouter } from "next/navigation";
import Link from "next/link";

export default function LoginPage() {
  const router = useRouter();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    setLoading(true);
    setError(null);
    const res = await fetch("/api/auth/login", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ email, password }),
    });
    setLoading(false);
    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      setError(data.message ?? "Login failed");
      return;
    }
    router.push("/chat");
    router.refresh();
  }

  return (
    <main className="flex min-h-full flex-1 items-center justify-center px-4 py-16">
      <div className="panel w-full max-w-md p-8">
        <p className="brand text-sm text-[var(--accent)]">JARVIS</p>
        <h1 className="mt-2 text-3xl">Sign in</h1>
        <p className="mt-2 text-sm text-[var(--muted)]">
          Superadmin console and text chat for your personal agent.
        </p>
        <form onSubmit={onSubmit} className="mt-8 space-y-4">
          <label className="block text-sm text-[var(--muted)]">
            Email
            <input
              className="input mt-1"
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              required
            />
          </label>
          <label className="block text-sm text-[var(--muted)]">
            Password
            <input
              className="input mt-1"
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              required
            />
          </label>
          {error && <p className="text-sm text-[var(--danger)]">{error}</p>}
          <button className="btn w-full" disabled={loading} type="submit">
            {loading ? "Authenticating…" : "Enter"}
          </button>
        </form>
        <p className="mt-6 text-sm text-[var(--muted)]">
          Have an invite?{" "}
          <Link className="text-[var(--accent)]" href="/invite">
            Accept invite
          </Link>
        </p>
      </div>
    </main>
  );
}
