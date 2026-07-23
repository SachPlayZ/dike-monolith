const RESERVED_HOSTS = new Set(["localhost", "example.com", "www.example.com"]);

function isPrivateIpv4(hostname: string) {
  const octets = hostname.split(".").map(Number);
  if (octets.length !== 4 || octets.some((part) => !Number.isInteger(part))) return false;
  return (
    octets[0] === 10 ||
    octets[0] === 127 ||
    (octets[0] === 169 && octets[1] === 254) ||
    (octets[0] === 172 && octets[1] >= 16 && octets[1] <= 31) ||
    (octets[0] === 192 && octets[1] === 168)
  );
}

export function getReferenceUrlError(value: string, label = "URL"): string | null {
  const trimmed = value.trim();
  if (!trimmed) return `${label} is required`;

  try {
    const url = new URL(trimmed);
    const hostname = url.hostname.toLowerCase();
    if (url.protocol !== "https:") return `${label} must use HTTPS`;
    if (url.username || url.password) return `${label} must not contain credentials`;
    if (
      RESERVED_HOSTS.has(hostname) ||
      hostname.endsWith(".localhost") ||
      hostname.endsWith(".example.com") ||
      hostname === "::1" ||
      isPrivateIpv4(hostname)
    ) {
      return `${label} must be publicly accessible`;
    }
    return null;
  } catch {
    return `${label} must be a valid HTTPS URL`;
  }
}

export function safeReferenceUrl(value: string | null | undefined): string | null {
  if (!value || getReferenceUrlError(value) !== null) return null;
  return value.trim();
}
