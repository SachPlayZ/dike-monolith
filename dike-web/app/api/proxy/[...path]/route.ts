import { NextRequest, NextResponse } from "next/server";

const SERVICES_URL = process.env.NEXT_PUBLIC_DIKE_SERVICES_URL ?? "http://localhost:4000";

function joinUrl(pathSegments: string[], search: string) {
  const base = SERVICES_URL.endsWith("/") ? SERVICES_URL.slice(0, -1) : SERVICES_URL;
  const path = pathSegments.join("/");
  return `${base}/${path}${search}`;
}

export async function GET(
  request: NextRequest,
  context: { params: Promise<{ path: string[] }> },
) {
  const { path } = await context.params;
  if (path[0] === "admin" || path[0] === "metrics") {
    return NextResponse.json({ error: "protected path unavailable through public proxy" }, { status: 403 });
  }
  const upstream = await fetch(joinUrl(path, request.nextUrl.search), {
    headers: {
      accept: request.headers.get("accept") ?? "application/json",
    },
    cache: "no-store",
  }).catch(() => null);

  if (!upstream) {
    return NextResponse.json({ error: "dike-services unavailable" }, { status: 503 });
  }

  const body = await upstream.text();
  return new NextResponse(body, {
    status: upstream.status,
    headers: {
      "content-type": upstream.headers.get("content-type") ?? "application/json; charset=utf-8",
      "cache-control": "no-store",
    },
  });
}
