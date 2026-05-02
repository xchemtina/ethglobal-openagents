/**
 * Session helpers — lightweight JWT stored in an HTTP-only cookie.
 * We avoid a heavy auth library to stay edge-compatible.
 */
import { SignJWT, jwtVerify } from "jose"
import { cookies } from "next/headers"

const COOKIE_NAME = "__crucible_session"
const COOKIE_MAX_AGE = 60 * 60 * 24 * 30 // 30 days in seconds

export interface SessionUser {
  sub:      string   // Crucible user UUID (or temp ORCID-derived id before DB)
  orcid:    string   // e.g. "0000-0002-1825-0097"
  name:     string
  email?:   string
  role:     string
  verified: boolean
}

function getSecret(): Uint8Array {
  const secret = process.env.SESSION_SECRET
  if (!secret) throw new Error("SESSION_SECRET environment variable is not set")
  return new TextEncoder().encode(secret)
}

/** Sign a new session JWT and return the cookie value string */
export async function signSession(user: SessionUser): Promise<string> {
  return new SignJWT({ ...user })
    .setProtectedHeader({ alg: "HS256" })
    .setIssuedAt()
    .setExpirationTime("30d")
    .sign(getSecret())
}

/** Verify and decode a session JWT. Returns null if invalid. */
export async function verifySession(token: string): Promise<SessionUser | null> {
  try {
    const { payload } = await jwtVerify(token, getSecret())
    return payload as unknown as SessionUser
  } catch {
    return null
  }
}

/** Read the current session from the request cookies. Returns null if unauthenticated. */
export async function getSession(): Promise<SessionUser | null> {
  const cookieStore = await cookies()
  const token = cookieStore.get(COOKIE_NAME)?.value
  if (!token) return null
  return verifySession(token)
}

/** Build the Set-Cookie header value for a new session */
export function buildSessionCookie(jwt: string, isSecure: boolean): string {
  const parts = [
    `${COOKIE_NAME}=${jwt}`,
    `HttpOnly`,
    `Path=/`,
    `Max-Age=${COOKIE_MAX_AGE}`,
    `SameSite=Lax`,
  ]
  if (isSecure) parts.push("Secure")
  return parts.join("; ")
}

/** Build the Set-Cookie header value that expires the session */
export function buildLogoutCookie(): string {
  return `${COOKIE_NAME}=; HttpOnly; Path=/; Max-Age=0; SameSite=Lax`
}
