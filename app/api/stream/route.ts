// Streams dike-services' /stream SSE endpoint through to the browser.
// Kept same-origin (same reasoning as /api/proxy) to avoid mixed-content
// blocks when this app is served over https but dike-services isn't.
export const dynamic = "force-dynamic";

const SERVICES_URL = process.env.NEXT_PUBLIC_DIKE_SERVICES_URL ?? "http://localhost:4000";

export async function GET() {
  const upstream = await fetch(`${SERVICES_URL}/stream`, {
    headers: { accept: "text/event-stream" },
    cache: "no-store",
  }).catch(() => null);

  if (!upstream || !upstream.body) {
    return new Response("dike-services unavailable", { status: 503 });
  }

  return new Response(upstream.body, {
    status: 200,
    headers: {
      "Content-Type": "text/event-stream",
      "Cache-Control": "no-cache, no-transform",
      Connection: "keep-alive",
    },
  });
}
