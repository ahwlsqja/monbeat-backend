---
id: T01
parent: S03
milestone: M001
provides:
  - AuthModule with GitHub OAuth + JWT strategy + JwtAuthGuard
  - configuration.ts extended with github, jwt, gemini config keys
  - 9 auth unit tests
key_files:
  - backend/src/auth/auth.module.ts
  - backend/src/auth/auth.service.ts
  - backend/src/auth/auth.controller.ts
  - backend/src/auth/jwt.strategy.ts
  - backend/src/auth/jwt-auth.guard.ts
  - backend/src/auth/github.strategy.ts
  - backend/src/config/configuration.ts
  - backend/src/app.module.ts
key_decisions:
  - JwtStrategy uses fallback string for secretOrKey to satisfy passport-jwt type requirement (config still provides real value at runtime)
patterns_established:
  - AuthGuard convenience classes (JwtAuthGuard, GithubAuthGuard) wrapping Passport strategies
  - AuthService.login() returns { accessToken, user } — controller returns raw, TransformInterceptor wraps
observability_surfaces:
  - GET /api/auth/me — returns current user profile from JWT
  - AuthService logs user upsert (githubId, username, userId)
  - JwtStrategy logs token validation failures
duration: 12m
verification_result: passed
completed_at: 2026-03-22
blocker_discovered: false
---

# T01: Build AuthModule with GitHub OAuth, JWT strategies, and JwtAuthGuard

**Implemented AuthModule with GitHub OAuth code-exchange, JWT signing/validation, JwtAuthGuard, and 3 HTTP endpoints; extended configuration.ts with github/jwt/gemini config keys; 9 unit tests passing.**

## What Happened

Installed all 8 S03 npm dependencies (6 prod: @nestjs/passport, @nestjs/jwt, passport, passport-github2, passport-jwt, @google/generative-ai; 2 dev: @types/passport-github2, @types/passport-jwt). Extended `configuration.ts` with `github.clientId`, `github.clientSecret`, `jwt.secret`, and `gemini.apiKey` config keys from environment variables.

Built the complete auth module: JwtStrategy extracts Bearer tokens from Authorization header and validates by looking up the user in Prisma via the `sub` claim. GithubStrategy configures passport-github2 with clientId/clientSecret from config and delegates to AuthService for user upsert. AuthService.validateOrCreateUser() performs a Prisma upsert on the User table using `githubId` as the unique key, and login() signs a JWT with `{ sub, githubId, username }` payload. AuthController exposes three endpoints: `GET /auth/github` (OAuth redirect), `GET /auth/github/callback` (token exchange), `GET /auth/me` (JWT-protected profile). JwtAuthGuard is exported from AuthModule for use by PaymasterModule in T03.

Registered AuthModule in AppModule. Wrote 9 unit tests across 2 test files covering user creation, update, missing fields, JWT signing, controller callback, and profile endpoint.

## Verification

- `npm run build` — exit 0, zero TypeScript errors
- `npm test -- --testPathPattern=auth` — 9 tests pass across 2 suites
- `npm test` — 43 total tests pass (34 existing + 9 new), no regressions
- `grep -q "AuthModule" backend/src/app.module.ts` — confirmed registered
- `grep -q "jwt.secret" backend/src/config/configuration.ts` — confirmed
- `test -f backend/src/auth/jwt-auth.guard.ts` — confirmed exists

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cd backend && npm run build` | 0 | ✅ pass | 3s |
| 2 | `cd backend && npm test -- --testPathPattern=auth` | 0 | ✅ pass | 4s |
| 3 | `cd backend && npm test` | 0 | ✅ pass | 8s |
| 4 | `grep -q "AuthModule" backend/src/app.module.ts` | 0 | ✅ pass | <1s |
| 5 | `grep -q "jwt.secret" backend/src/config/configuration.ts` | 0 | ✅ pass | <1s |
| 6 | `test -f backend/src/auth/jwt-auth.guard.ts` | 0 | ✅ pass | <1s |

## Diagnostics

- **Auth pipeline inspection:** `GET /api/auth/me` with valid JWT returns user profile `{ id, githubId, username, avatarUrl, deployCount }`. Returns 401 without JWT.
- **User upsert logging:** AuthService logs `User upsert: githubId=..., username=..., userId=...` on every GitHub OAuth login.
- **JWT validation failures:** JwtStrategy logs warning when token references a non-existent user ID.
- **Config verification:** All 4 new config keys (`github.clientId`, `github.clientSecret`, `jwt.secret`, `gemini.apiKey`) available via ConfigService dot-notation.

## Deviations

- JwtStrategy `secretOrKey` uses `|| 'fallback-secret'` to satisfy passport-jwt's strict TypeScript type requiring non-undefined value. The configuration already provides a default in `configuration.ts`, so this is belt-and-suspenders — at runtime, `configService.get('jwt.secret')` always returns a string.

## Known Issues

None.

## Files Created/Modified

- `backend/package.json` — added 8 npm dependencies (6 prod + 2 dev)
- `backend/src/config/configuration.ts` — extended with github, jwt, gemini config sections
- `backend/src/app.module.ts` — added AuthModule import
- `backend/src/auth/auth.module.ts` — AuthModule with PassportModule + JwtModule configuration
- `backend/src/auth/auth.service.ts` — validateOrCreateUser + login methods
- `backend/src/auth/auth.controller.ts` — 3 auth endpoints (github, github/callback, me)
- `backend/src/auth/jwt.strategy.ts` — JWT Passport strategy with Prisma user lookup
- `backend/src/auth/jwt-auth.guard.ts` — JWT AuthGuard convenience class (exported)
- `backend/src/auth/github.strategy.ts` — GitHub OAuth Passport strategy
- `backend/src/auth/github-auth.guard.ts` — GitHub AuthGuard convenience class
- `backend/src/auth/dto/auth-response.dto.ts` — AuthResponseDto + AuthUserDto interfaces
- `backend/test/auth.service.spec.ts` — 5 AuthService unit tests
- `backend/test/auth.controller.spec.ts` — 4 AuthController unit tests
