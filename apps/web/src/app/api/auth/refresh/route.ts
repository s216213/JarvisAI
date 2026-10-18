import { NextResponse } from "next/server";
import {
  ACCESS_COOKIE,
  REFRESH_COOKIE,
  brainUrl,
  getRefreshToken,
} from "@/lib/brain";

export async function POST() {
  const refresh = await getRefreshToken();
  if (!refresh) {
    return NextResponse.json({ code: "unauthorized" }, { status: 401 });
  }

  const res = await fetch(`${brainUrl()}/v1/auth/refresh`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ refresh_token: refresh }),
  });
  const data = await res.json();
  if (!res.ok) {
    const response = NextResponse.json(data, { status: res.status });
    response.cookies.set(ACCESS_COOKIE, "", { httpOnly: true, path: "/", maxAge: 0 });
    response.cookies.set(REFRESH_COOKIE, "", { httpOnly: true, path: "/", maxAge: 0 });
    return response;
  }

  const response = NextResponse.json({ user: data.user });
  const secure = process.env.NODE_ENV === "production";
  response.cookies.set(ACCESS_COOKIE, data.access_token, {
    httpOnly: true,
    sameSite: "lax",
    secure,
    path: "/",
    maxAge: 60 * 15,
  });
  response.cookies.set(REFRESH_COOKIE, data.refresh_token, {
    httpOnly: true,
    sameSite: "lax",
    secure,
    path: "/",
    maxAge: 60 * 60 * 24 * 30,
  });
  return response;
}
