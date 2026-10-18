"use client";

import { FormEvent, useState } from "react";
import type { PersonaVersion } from "@/lib/protocol";

export function PersonaEditor({ initial }: { initial: PersonaVersion[] }) {
  const [versions, setVersions] = useState(initial);
  const [label, setLabel] = useState("");
  const [prompt, setPrompt] = useState("");
  const [message, setMessage] = useState<string | null>(null);

  async function create(e: FormEvent) {
    e.preventDefault();
    setMessage(null);
    const res = await fetch("/api/brain/persona", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ label, system_prompt: prompt, activate: true }),
    });
    const data = await res.json();
    if (!res.ok) {
      setMessage(data.message ?? "Create failed");
      return;
    }
    setVersions((v) => [{ ...data }, ...v.map((x) => ({ ...x, is_active: false }))]);
    setLabel("");
    setPrompt("");
    setMessage(`Activated ${data.label}`);
  }

  async function activate(id: string) {
    setMessage(null);
    const res = await fetch(`/api/brain/persona/${id}/activate`, { method: "POST" });
    const data = await res.json();
    if (!res.ok) {
      setMessage(data.message ?? "Activate failed");
      return;
    }
    setVersions((v) => v.map((x) => ({ ...x, is_active: x.id === id })));
    setMessage(`Rolled to ${data.label}`);
  }

  return (
    <div className="grid gap-4 lg:grid-cols-2">
      <form onSubmit={create} className="panel space-y-3 p-4">
        <h2 className="text-sm text-[var(--muted)]">New version</h2>
        <input
          className="input"
          placeholder="label (e.g. concise-v2)"
          value={label}
          onChange={(e) => setLabel(e.target.value)}
          required
        />
        <textarea
          className="input min-h-48"
          placeholder="System prompt"
          value={prompt}
          onChange={(e) => setPrompt(e.target.value)}
          required
        />
        <button className="btn" type="submit">
          Save & activate
        </button>
        {message && <p className="text-sm text-[var(--accent)]">{message}</p>}
      </form>

      <div className="space-y-3">
        {versions.map((v) => (
          <article key={v.id} className="panel p-4">
            <div className="flex items-center justify-between gap-2">
              <h3 className="font-medium">
                {v.label}{" "}
                {v.is_active && <span className="text-xs text-[var(--ok)]">ACTIVE</span>}
              </h3>
              {!v.is_active && (
                <button className="btn-ghost text-xs" type="button" onClick={() => activate(v.id)}>
                  Activate
                </button>
              )}
            </div>
            <p className="mt-2 max-h-40 overflow-auto whitespace-pre-wrap text-sm text-[var(--muted)]">
              {v.system_prompt}
            </p>
          </article>
        ))}
      </div>
    </div>
  );
}
