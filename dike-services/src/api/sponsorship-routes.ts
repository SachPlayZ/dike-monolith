import type { FastifyInstance } from "fastify";
import { parseSponsorshipRequest } from "../sponsorship/request.js";
import { publicSponsorshipError, sponsorshipHttpStatus } from "../sponsorship/types.js";
import type { FeeSponsorshipService } from "../sponsorship/service.js";

export function registerSponsorshipRoutes(
  app: FastifyInstance<any, any, any, any, any>,
  service: FeeSponsorshipService,
) {
  app.get("/sponsorship/status", async (_request, reply) => reply.send(service.status()));

  app.post("/sponsorship/transactions", { bodyLimit: 200_000 }, async (request, reply) => {
    try {
      const body = parseSponsorshipRequest(request.body);
      const result = await service.sponsor(body, request.ip);
      return reply.code(200).send(result);
    } catch (error) {
      const safe = publicSponsorshipError(error);
      return reply.code(sponsorshipHttpStatus(safe.code)).send({ error: safe });
    }
  });
}
