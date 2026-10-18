import { cookies } from "next/headers";

const ACCESS_COOKIE = "jarvis_access";
const REFRESH_COOKIE = "jarvis_refresh";

export { ACCESS_COOKIE, REFRESH_COOKIE };

export function brainUrl(): string {
  const url = process.env.BRAIN_URL ?? "http://127.0.0.1:8080";
  return url.replace(/\/$/, "");
}

export async function getAccessToken(): Promise<string | null> {
  const jar = await cookies();
  return jar.get(ACCESS_COOKIE)?.value ?? null;
}

export async function getRefreshToken(): Promise<string | null> {
  const jar = await cookies();
  return jar.get(REFRESH_COOKIE)?.value ?? null;
}

export async function brainFetch(
  path: string,
  init: RequestInit = {},
  token?: string | null,
): Promise<Response> {
  const access = token === undefined ? await getAccessToken() : token;
  const headers = new Headers(init.headers);
  if (!headers.has("Content-Type") && init.body) {
    headers.set("Content-Type", "application/json");
  }
  if (access) {
    headers.set("Authorization", `Bearer ${access}`);
  }
  return fetch(`${brainUrl()}${path}`, { ...init, headers, cache: "no-store" });
}

export type SessionUser = {
  id: string;
  email: string;
  role: string;
  display_name: string | null;
};
