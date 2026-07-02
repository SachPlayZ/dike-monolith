// Server-side: hit EC2 directly. Client-side: route through /api/proxy to avoid mixed-content blocks.
const SERVICES_URL =
  typeof window === "undefined"
    ? (process.env.NEXT_PUBLIC_DIKE_SERVICES_URL ?? "http://localhost:4000")
    : "/api/proxy";

export class ServiceUnavailableError extends Error {
  constructor(path: string) {
    super(`dike-services not reachable: GET ${path}. Start the service at ${SERVICES_URL}.`);
    this.name = "ServiceUnavailableError";
  }
}

export async function apiGet<T>(path: string): Promise<T> {
  const url = `${SERVICES_URL}${path}`;
  let res: Response;
  try {
    // Matches dike-services' INDEXER_POLL_INTERVAL_MS (5s) — no point caching
    // longer than the data can actually change underneath us.
    res = await fetch(url, { next: { revalidate: 5 } });
  } catch {
    throw new ServiceUnavailableError(path);
  }
  if (!res.ok) {
    if (res.status === 404) throw new Error(`Not found: ${path}`);
    throw new Error(`API error ${res.status}: ${path}`);
  }
  return res.json() as Promise<T>;
}

// Server-only: calls dike-services admin routes directly (never through
// /api/proxy, which blocks /admin) and attaches the shared admin key from a
// non-NEXT_PUBLIC_ env var so it never ends up in the client bundle.
export async function adminApiGet<T>(path: string): Promise<T> {
  if (typeof window !== "undefined") {
    throw new Error("adminApiGet is server-only");
  }
  const base = process.env.NEXT_PUBLIC_DIKE_SERVICES_URL ?? "http://localhost:4000";
  const apiKey = process.env.DIKE_ADMIN_API_KEY;
  const url = `${base}${path}`;
  let res: Response;
  try {
    res = await fetch(url, {
      cache: "no-store",
      headers: apiKey ? { "x-api-key": apiKey } : undefined,
    });
  } catch {
    throw new ServiceUnavailableError(path);
  }
  if (!res.ok) {
    if (res.status === 401) throw new Error("API error 401: admin api key required");
    if (res.status === 404) throw new Error(`Not found: ${path}`);
    throw new Error(`API error ${res.status}: ${path}`);
  }
  return res.json() as Promise<T>;
}
