const SERVICES_URL =
  process.env.NEXT_PUBLIC_DIKE_SERVICES_URL ?? "http://localhost:4000";

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
    res = await fetch(url, { next: { revalidate: 30 } });
  } catch {
    throw new ServiceUnavailableError(path);
  }
  if (!res.ok) {
    if (res.status === 404) throw new Error(`Not found: ${path}`);
    throw new Error(`API error ${res.status}: ${path}`);
  }
  return res.json() as Promise<T>;
}
