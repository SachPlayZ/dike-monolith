import "dotenv/config";

const serviceUrl = process.env.SPONSOR_SMOKE_SERVICE_URL ?? "http://localhost:4000";
const signedInnerXdr = process.env.SPONSOR_SMOKE_SIGNED_INNER_XDR;

if (!signedInnerXdr) {
  throw new Error("SPONSOR_SMOKE_SIGNED_INNER_XDR must contain a wallet-signed assembled Dike transaction XDR.");
}

const response = await fetch(`${serviceUrl}/sponsorship/transactions`, {
  method: "POST",
  headers: { "content-type": "application/json", accept: "application/json" },
  body: JSON.stringify({ signedTransactionXdr: signedInnerXdr }),
});
const payload = await response.json().catch(() => ({}));

if (!response.ok) {
  console.error(JSON.stringify({ status: response.status, payload }));
  process.exitCode = 1;
} else {
  console.log(JSON.stringify({ sponsored: true, ...payload }));
}
