import { createServer, type IncomingMessage, type Server } from "node:http";
import type { Mediator } from "../core/mediator.ts";

const readJson = async (req: IncomingMessage) => {
  let body = "";
  for await (const chunk of req) body += chunk;
  return body ? JSON.parse(body) : {};
};

/** Single endpoint: POST /api/<RequestType> with the request as JSON body. */
export function startServer(mediator: Mediator, port: number): Promise<Server> {
  const server = createServer(async (req, res) => {
    const match = /^\/api\/(\w+)$/.exec(req.url ?? "");
    if (req.method !== "POST" || !match) {
      res.writeHead(404).end();
      return;
    }
    try {
      const result = await mediator.send({ ...(await readJson(req)), type: match[1]! });
      res.writeHead(200, { "content-type": "application/json" }).end(JSON.stringify(result));
    } catch (err) {
      res.writeHead(400, { "content-type": "application/json" });
      res.end(JSON.stringify({ error: (err as Error).message }));
    }
  });
  return new Promise((resolve) => server.listen(port, "127.0.0.1", () => resolve(server)));
}
