import type { Request, Response, NextFunction } from "express";

/**
 * Tiny in-memory token bucket per IP. Fine for single-instance deploy;
 * replace with Redis when multi-node.
 */
export function rateLimit(opts: {
  windowMs: number;
  max: number;
}): (req: Request, res: Response, next: NextFunction) => void {
  const hits = new Map<string, { count: number; reset: number }>();

  return (req, res, next) => {
    const ip =
      (req.headers["x-forwarded-for"] as string | undefined)?.split(",")[0]?.trim() ||
      req.socket.remoteAddress ||
      "unknown";
    const now = Date.now();
    let bucket = hits.get(ip);
    if (!bucket || now > bucket.reset) {
      bucket = { count: 0, reset: now + opts.windowMs };
      hits.set(ip, bucket);
    }
    bucket.count += 1;
    res.setHeader("X-RateLimit-Limit", String(opts.max));
    res.setHeader(
      "X-RateLimit-Remaining",
      String(Math.max(0, opts.max - bucket.count)),
    );
    if (bucket.count > opts.max) {
      res.status(429).json({
        error: "rate_limited",
        message: `max ${opts.max} requests per ${opts.windowMs / 1000}s`,
        retry_after_ms: bucket.reset - now,
      });
      return;
    }
    next();
  };
}
