"use client";

import { FormEvent, useState } from "react";

export function InviteForm() {
  const [email, setEmail] = useState("");
  const [role, setRole] = useState("user");
  const [token, setToken] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    setError(null);
    setToken(null);
    const res = await fetch("/api/brain/admin/invites", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ email, role }),
    });
    const data = await res.json();
    if (!res.ok) {
      setError(data.message ?? "Failed to create invite");
      return;
    }
    setToken(data.token);
  }

  return (
    <form onSubmit={onSubmit} className="mt-3 space-y-3">
      <label className="block text-sm text-[var(--muted)]">
        Email
        <input className="input mt-1" type="email" value={email} onChange={(e) => setEmail(e.target.value)} required />
      </label>
      <label className="block text-sm text-[var(--muted)]">
        Role
        <select className="input mt-1" value={role} onChange={(e) => setRole(e.target.value)}>
          <option value="user">user</option>
          <option value="superadmin">superadmin</option>
        </select>
      </label>
      <button className="btn" type="submit">
        Create invite
      </button>
      {error && <p className="text-sm text-[var(--danger)]">{error}</p>}
      {token && (
        <p className="break-all text-sm text-[var(--ok)]">
          Invite token (share once): <code>{token}</code>
        </p>
      )}
    </form>
  );
}
