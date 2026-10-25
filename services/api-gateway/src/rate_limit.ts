import type { Request, Response, NextFunction } from "express";

/** Fixed-window limiter for one gateway instance. Proxy headers are trusted only
 * when Express has been explicitly configured with trusted proxy addresses. */
export function rateLimit(opts: {
  windowMs: number;
  max: number;
  maxKeys?: number;
  now?: () => number;
}): (req: Request, res: Response, next: NextFunction) => void {
  const maxKeys = opts.maxKeys ?? 10_000;
  for (const [name, value] of Object.entries({ windowMs: opts.windowMs, max: opts.max, maxKeys })) {
    if (!Number.isSafeInteger(value) || value <= 0) {
      throw new Error(`${name} must be a positive safe integer`);
    }
  }
  const hits = new Map<string, { count: number; reset: number }>();
  const clock = opts.now ?? Date.now;
  let nextSweep = 0;

  function prune(now: number) {
    for (const [key, bucket] of hits) {
      if (now >= bucket.reset) hits.delete(key);
    }
    nextSweep = now + opts.windowMs;
  }

  return (req, res, next) => {
    // Express defaults to trust proxy=false; never read raw X-Forwarded-For here.
    const ip = req.ip || req.socket.remoteAddress || "unknown";
    const now = clock();
    if (now >= nextSweep) prune(now);
    let bucket = hits.get(ip);
    if (!bucket || now >= bucket.reset) {
      if (!bucket && hits.size >= maxKeys) prune(now);
      if (!bucket && hits.size >= maxKeys) {
        res.setHeader("Retry-After", String(Math.ceil(opts.windowMs / 1000)));
        res.status(429).json({ error: "rate_limit_capacity", message: "Retry after the current rate-limit window." });
        return;
      }
      bucket = { count: 0, reset: now + opts.windowMs };
      hits.set(ip, bucket);
    }
    bucket.count = Math.min(bucket.count + 1, opts.max + 1);
    res.setHeader("X-RateLimit-Limit", String(opts.max));
    res.setHeader("X-RateLimit-Remaining", String(Math.max(0, opts.max - bucket.count)));
    if (bucket.count > opts.max) {
      res.setHeader("Retry-After", String(Math.max(1, Math.ceil((bucket.reset - now) / 1000))));
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
