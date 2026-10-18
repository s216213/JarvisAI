import { NextResponse } from "next/server";
import { brainFetch, getAccessToken, getRefreshToken, brainUrl, ACCESS_COOKIE, REFRESH_COOKIE } from "@/lib/brain";

async function ensureAccess(): Promise<string | null> {
  let access = await getAccessToken();
  if (access) return access;

  const refresh = await getRefreshToken();
  if (!refresh) return null;

  const res = await fetch(`${brainUrl()}/v1/auth/refresh`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ refresh_token: refresh }),
  });
  if (!res.ok) return null;
  const data = await res.json();
  return data.access_token as string;
}

export async function POST() {
  const access = await ensureAccess();
  if (!access) {
    return NextResponse.json({ code: "unauthorized" }, { status: 401 });
  }

  const res = await brainFetch("/v1/auth/ws-ticket", { method: "POST" }, access);
  const data = await res.json();
  if (!res.ok) {
    return NextResponse.json(data, { status: res.status });
  }

  const response = NextResponse.json(data);
  // Refresh access cookie if we just rotated via refresh path
  const current = await getAccessToken();
  if (!current) {
    const secure = process.env.NODE_ENV === "production";
    response.cookies.set(ACCESS_COOKIE, access, {
      httpOnly: true,
      sameSite: "lax",
      secure,
      path: "/",
      maxAge: 60 * 15,
    });
  }
  // silence unused import when cookie already present
  void REFRESH_COOKIE;
  return response;
}
