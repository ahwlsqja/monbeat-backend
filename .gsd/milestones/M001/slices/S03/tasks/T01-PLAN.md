---
estimated_steps: 5
estimated_files: 11
---

# T01: Build AuthModule with GitHub OAuth, JWT strategies, and JwtAuthGuard

**Slice:** S03 — Auth + Analysis + Paymaster Module
**Milestone:** M001

## Description

Install all S03 npm dependencies and build the complete AuthModule: GitHub OAuth code-exchange flow, JWT signing/validation, JwtAuthGuard, and three HTTP endpoints. This task also extends `configuration.ts` with the 4 new config keys needed by the entire slice (github.clientId, github.clientSecret, jwt.secret, gemini.apiKey). The module is foundational — T03 (Paymaster) depends on JwtAuthGuard from this task.

**Relevant skills:** None specific — standard NestJS Passport/JWT patterns.

**Key context from prior slices:**
- Backend lives in `backend/` subdirectory — all npm/nest commands run from there
- `ConfigModule` is `isGlobal: true` — inject ConfigService anywhere with `configService.get<string>('github.clientId')` using dot-notation
- `PrismaModule` is `@Global()` — inject PrismaService anywhere without importing
- `TransformInterceptor` auto-wraps responses as `{ success: true, data }` — controllers return raw data
- User model in Prisma has `githubId` (unique), `username`, `email`, `avatarUrl`, `deployCount`
- `HttpExceptionFilter` handles all errors — throw NestJS HTTP exceptions

## Steps

1. **Install npm dependencies:** In `backend/`, add production deps: `@nestjs/passport@^11`, `@nestjs/jwt@^11`, `passport@^0.7`, `passport-github2@^0.1.12`, `passport-jwt@^4.0.1`, `@google/generative-ai@^0.24.0`. Dev deps: `@types/passport-github2`, `@types/passport-jwt`. Run `npm install`.

2. **Extend configuration.ts:** Add `github: { clientId, clientSecret }`, `jwt: { secret }`, `gemini: { apiKey }` sections reading from `GITHUB_CLIENT_ID`, `GITHUB_CLIENT_SECRET`, `JWT_SECRET`, `GEMINI_API_KEY` environment variables.

3. **Create auth source files:**
   - `jwt.strategy.ts` — PassportStrategy extending `passport-jwt` Strategy, extracts Bearer token from Authorization header, validates by looking up user in Prisma via `sub` (userId). Injects ConfigService for `jwt.secret`.
   - `jwt-auth.guard.ts` — `AuthGuard('jwt')` convenience class.
   - `github.strategy.ts` — PassportStrategy extending `passport-github2` Strategy. Configures with `github.clientId`, `github.clientSecret`, callbackURL from config. The validate method receives GitHub profile, delegates to AuthService.
   - `auth.service.ts` — `validateOrCreateUser(githubProfile)`: upsert User in Prisma using `githubId` (unique). `login(user)`: sign JWT with `{ sub: user.id, githubId: user.githubId, username: user.username }` using JwtService.
   - `auth.controller.ts` — Three endpoints:
     - `GET /api/auth/github` — Uses `@UseGuards(GithubAuthGuard)` to redirect to GitHub OAuth page
     - `GET /api/auth/github/callback` — Uses `@UseGuards(GithubAuthGuard)`, receives authenticated user from Passport, calls `authService.login(user)`, returns `{ accessToken, user }`. Note: For SPA frontend, this could also accept a `code` query param and exchange manually — implement the Passport guard approach first as it's simpler.
     - `GET /api/auth/me` — Uses `@UseGuards(JwtAuthGuard)`, returns current user from `@Req() req.user`
   - `github-auth.guard.ts` — `AuthGuard('github')` convenience class.
   - `dto/auth-response.dto.ts` — Type for `{ accessToken: string, user: { id, githubId, username, avatarUrl, deployCount } }`.
   - `auth.module.ts` — Imports `PassportModule`, `JwtModule.registerAsync({ useFactory: (config) => ({ secret: config.get('jwt.secret'), signOptions: { expiresIn: '7d' } }), inject: [ConfigService] })`. Provides strategies + AuthService. Exports JwtAuthGuard, JwtStrategy (so other modules can use the guard).

4. **Register AuthModule in AppModule:** Add `AuthModule` to imports array in `backend/src/app.module.ts`.

5. **Write unit tests:** Create `backend/test/auth.service.spec.ts` and `backend/test/auth.controller.spec.ts`:
   - AuthService tests (≥3): `validateOrCreateUser` creates new user, `validateOrCreateUser` updates existing user, `login` returns valid JWT structure
   - AuthController tests (≥3): `GET /api/auth/me` returns user when JWT valid, `GET /api/auth/me` rejects without JWT (guard behavior), controller is defined
   - Mock PrismaService, JwtService, ConfigService

## Must-Haves

- [ ] All 8 npm packages installed (6 prod + 2 dev) and `npm run build` succeeds
- [ ] `configuration.ts` exports `github.clientId`, `github.clientSecret`, `jwt.secret`, `gemini.apiKey` from env vars
- [ ] JwtStrategy validates Bearer tokens and resolves userId from Prisma
- [ ] JwtAuthGuard is exported from AuthModule for use by other modules
- [ ] AuthService.login() signs JWT with sub/githubId/username payload
- [ ] AuthController exposes 3 endpoints: GET /auth/github, GET /auth/github/callback, GET /auth/me
- [ ] AuthModule registered in AppModule
- [ ] ≥6 unit tests passing

## Verification

- `cd backend && npm run build` — exit 0
- `cd backend && npm test -- --testPathPattern=auth` — ≥6 tests pass
- `grep -q "AuthModule" backend/src/app.module.ts` — module registered
- `grep -q "jwt.secret" backend/src/config/configuration.ts` — config extended
- `test -f backend/src/auth/jwt-auth.guard.ts` — guard file exists

## Observability Impact

- **Signals added:** AuthService logs user upsert operations with `githubId` and `username` (Logger). JwtStrategy logs token validation failures.
- **Inspection surfaces:** `GET /api/auth/me` — returns current user profile from JWT, confirming auth pipeline works end-to-end.
- **Failure visibility:** 401 Unauthorized on missing/invalid JWT (JwtAuthGuard); AuthService.validateOrCreateUser logs Prisma upsert errors.
- **Redaction:** JWT_SECRET and GITHUB_CLIENT_SECRET read from env, never logged. GitHub access_token discarded after user info fetch.

## Inputs

- `backend/package.json` — existing dependencies to extend
- `backend/src/config/configuration.ts` — existing config factory to extend with auth/gemini keys
- `backend/src/app.module.ts` — existing module imports to extend
- `backend/prisma/schema.prisma` — User model definition (githubId unique, username, email, avatarUrl, deployCount)
- `backend/src/prisma/prisma.service.ts` — PrismaService to inject for user upsert
- `backend/src/main.ts` — bootstrap with global prefix, CORS, pipes, filters, interceptors

## Expected Output

- `backend/package.json` — updated with 8 new npm dependencies
- `backend/src/config/configuration.ts` — extended with github, jwt, gemini config sections
- `backend/src/app.module.ts` — AuthModule added to imports
- `backend/src/auth/auth.module.ts` — AuthModule with Passport + JWT configuration
- `backend/src/auth/auth.service.ts` — validateOrCreateUser + login methods
- `backend/src/auth/auth.controller.ts` — 3 auth endpoints
- `backend/src/auth/github.strategy.ts` — GitHub OAuth Passport strategy
- `backend/src/auth/jwt.strategy.ts` — JWT Passport strategy
- `backend/src/auth/jwt-auth.guard.ts` — JWT AuthGuard export
- `backend/src/auth/github-auth.guard.ts` — GitHub AuthGuard
- `backend/src/auth/dto/auth-response.dto.ts` — response type
- `backend/test/auth.service.spec.ts` — AuthService unit tests
- `backend/test/auth.controller.spec.ts` — AuthController unit tests
