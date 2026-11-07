/** Hand-rolled mediator: one handler per command/query type. */

export interface Request<TResult = unknown> {
  readonly type: string;
  /** Phantom field so TResult is inferred; never set at runtime. */
  readonly __result?: TResult;
}

export type Handler<R extends Request> = (
  request: R,
) => Promise<NonNullable<R["__result"]>> | NonNullable<R["__result"]>;

export class Mediator {
  private readonly handlers = new Map<string, Handler<any>>();

  register<R extends Request>(type: R["type"], handler: Handler<R>): void {
    if (this.handlers.has(type)) throw new Error(`Handler already registered for '${type}'`);
    this.handlers.set(type, handler);
  }

  async send<R extends Request>(request: R): Promise<NonNullable<R["__result"]>> {
    const handler = this.handlers.get(request.type);
    if (!handler) throw new Error(`No handler registered for '${request.type}'`);
    return handler(request);
  }
}
