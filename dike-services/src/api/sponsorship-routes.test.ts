import Fastify from "fastify";
import { describe, expect, it } from "vitest";
import { registerSponsorshipRoutes } from "./sponsorship-routes.js";

describe("sponsorship routes", () => {
  it("returns a stable success payload", async () => {
    const app = Fastify();
    registerSponsorshipRoutes(app, {
      sponsor: async () => ({ innerHash: "inner", outerHash: "outer", status: "SUCCESS" }),
    } as never);
    const response = await app.inject({
      method: "POST",
      url: "/sponsorship/transactions",
      payload: { signedTransactionXdr: Buffer.from("signed").toString("base64") },
    });
    expect(response.statusCode).toBe(200);
    expect(response.json()).toMatchObject({ outerHash: "outer" });
  });

  it("rejects malformed payloads before sponsorship", async () => {
    const app = Fastify();
    registerSponsorshipRoutes(app, { sponsor: async () => { throw new Error("must not call"); } } as never);
    const response = await app.inject({ method: "POST", url: "/sponsorship/transactions", payload: {} });
    expect(response.statusCode).toBe(400);
    expect(response.json().error.code).toBe("MALFORMED_XDR");
  });
});
