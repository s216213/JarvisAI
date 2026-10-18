"use client";

import { useState } from "react";
import type { FeatureFlag } from "@/lib/protocol";

export function FlagsEditor({ initial }: { initial: FeatureFlag[] }) {
  const [flags, setFlags] = useState(initial);
  const [message, setMessage] = useState<string | null>(null);

  async function save(flag: FeatureFlag, enabled: boolean, valueText: string) {
    setMessage(null);
    let value: unknown = flag.value;
    try {
      value = JSON.parse(valueText);
    } catch {
      setMessage("Value must be valid JSON");
      return;
    }
    const res = await fetch(`/api/brain/admin/flags/${encodeURIComponent(flag.key)}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ enabled, value }),
    });
    const data = await res.json();
    if (!res.ok) {
      setMessage(data.message ?? "Update failed");
      return;
    }
    setFlags((prev) => prev.map((f) => (f.key === data.key ? data : f)));
    setMessage(`Updated ${data.key}`);
  }

  return (
    <div className="space-y-4">
      {message && <p className="text-sm text-[var(--accent)]">{message}</p>}
      {flags.map((flag) => (
        <FlagRow key={flag.key} flag={flag} onSave={save} />
      ))}
    </div>
  );
}

function FlagRow({
  flag,
  onSave,
}: {
  flag: FeatureFlag;
  onSave: (flag: FeatureFlag, enabled: boolean, valueText: string) => Promise<void>;
}) {
  const [enabled, setEnabled] = useState(flag.enabled);
  const [valueText, setValueText] = useState(JSON.stringify(flag.value, null, 2));

  return (
    <div className="panel p-4">
      <div className="flex items-center justify-between gap-3">
        <h2 className="font-medium">{flag.key}</h2>
        <label className="flex items-center gap-2 text-sm text-[var(--muted)]">
          <input type="checkbox" checked={enabled} onChange={(e) => setEnabled(e.target.checked)} />
          enabled
        </label>
      </div>
      <textarea
        className="input mt-3 min-h-24 font-mono text-xs"
        value={valueText}
        onChange={(e) => setValueText(e.target.value)}
      />
      <button className="btn mt-3" type="button" onClick={() => onSave(flag, enabled, valueText)}>
        Save
      </button>
    </div>
  );
}
