import type { ErrorRequestHandler, RequestHandler, Request, Response, NextFunction } from "express";

const decompressionErrors = new Set(["Z_DATA_ERROR", "Z_BUF_ERROR"]);

/** Express 4 does not forward rejected async handlers to error middleware. */
export function asyncRoute(handler: (req: Request, res: Response, next: NextFunction) => Promise<void>): RequestHandler {
  return (req, res, next) => {
    Promise.resolve().then(() => handler(req, res, next)).catch(next);
  };
}

export const errorHandler: ErrorRequestHandler = (error, _req, res, next) => {
  if (res.headersSent) return next(error);
  const invalidBody = error?.type === "entity.parse.failed" ||
    (error?.status === 400 && decompressionErrors.has(error?.code));
  const status = error?.type === "entity.too.large" ? 413 : invalidBody ? 400 : 500;
  // Do not expose paths, request bodies, worker output, or provider credentials.
  res.status(status).json({ error: status === 500 ? "internal_error" : "invalid_request", message: status === 500 ? "The request could not be completed." : "Invalid JSON request body." });
};
