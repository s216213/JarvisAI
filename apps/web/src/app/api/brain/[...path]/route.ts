import { NextRequest, NextResponse } from "next/server";
import {
  ACCESS_COOKIE,
  REFRESH_COOKIE,
  brainFetch,
  brainUrl,
  getAccessToken,
  getRefreshToken,
} from "@/lib/brain";

async function refreshAccess(): Promise<{ access: string; refresh: string; user: unknown } | null> {
  const refresh = await getRefreshToken();
  if (!refresh) return null;
  const res = await fetch(`${brainUrl()}/v1/auth/refresh`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ refresh_token: refresh }),
  });
  if (!res.ok) return null;
  const data = await res.json();
  return { access: data.access_token, refresh: data.refresh_token, user: data.user };
}

async function proxy(req: NextRequest, pathParts: string[]) {
  const path = `/v1/${pathParts.join("/")}${req.nextUrl.search}`;
  let access = await getAccessToken();

  const init: RequestInit = {
    method: req.method,
    body: ["GET", "HEAD"].includes(req.method) ? undefined : await req.text(),
  };

  let res = await brainFetch(path, init, access);
  if (res.status === 401) {
    const rotated = await refreshAccess();
    if (rotated) {
      access = rotated.access;
      res = await brainFetch(path, init, access);
      const data = await res.json().catch(() => ({}));
      const response = NextResponse.json(data, { status: res.status });
      const secure = process.env.NODE_ENV === "production";
      response.cookies.set(ACCESS_COOKIE, rotated.access, {
        httpOnly: true,
        sameSite: "lax",
        secure,
        path: "/",
        maxAge: 60 * 15,
      });
      response.cookies.set(REFRESH_COOKIE, rotated.refresh, {
        httpOnly: true,
        sameSite: "lax",
        secure,
        path: "/",
        maxAge: 60 * 60 * 24 * 30,
      });
      return response;
    }
  }

  const data = await res.json().catch(() => ({}));
  return NextResponse.json(data, { status: res.status });
}

type Ctx = { params: Promise<{ path: string[] }> };

export async function GET(req: NextRequest, ctx: Ctx) {
  const { path } = await ctx.params;
  return proxy(req, path);
}
export async function POST(req: NextRequest, ctx: Ctx) {
  const { path } = await ctx.params;
  return proxy(req, path);
}
export async function PUT(req: NextRequest, ctx: Ctx) {
  const { path } = await ctx.params;
  return proxy(req, path);
}
export async function DELETE(req: NextRequest, ctx: Ctx) {
  const { path } = await ctx.params;
  return proxy(req, path);
}
