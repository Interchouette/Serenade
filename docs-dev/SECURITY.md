# Security

AuthN/Z hooks, CSRF tokens, and how HTML apps stay safe. This is **not** a full OAuth/OIDC stack.

## Pieces

| Type | Role |
| --- | --- |
| `UserInterface` / `InMemoryUser` | Principal id + roles |
| `TokenInterface` / `UsernamePasswordToken` | Authenticated flag, optional user, credentials echo |
| `Voter` / `AccessDecisionManager` | Affirmative strategy (any `Grant` wins) |
| `Authenticator` | App-owned credential check |
| `FirewallMiddleware` | HTTP middleware: read header → authenticate → store token on request attributes |
| `SessionMiddleware` / `AsyncSessionMiddleware` | HTTP middleware: load/save session via `serenade-session` (see [SESSION.md](SESSION.md)) |
| `CsrfToken` / `CsrfTokenManager` / `HmacCsrfTokenManager` | Issue and validate CSRF tokens (stateless HMAC) |
| `PasswordHasher` / `Argon2idPasswordHasher` | Hash and verify passwords (Argon2id, PHC string) |
| `CSRF_FIELD_NAME` (`_token`) | Default HTML field name (Symfony habit) |

Request attribute key: `_security_token` (`TOKEN_ATTRIBUTE`). Helper: `request_token(&request)`.

HTML forms wire CSRF through **`serenade-form`** (see [FORMS.md](FORMS.md)): forms enable CSRF by default and call the token manager on bind/render.

## Bearer / API key plug-in

Apps own authenticators. Example pattern for an admin API key or bearer token:

1. Implement `Authenticator::authenticate` (parse `Authorization: Bearer …` or a dedicated header).
2. On success, return `UsernamePasswordToken::authenticated(InMemoryUser::new(…), credentials)`.
3. Register `FirewallMiddleware::new("Authorization", authenticator)` on `HttpKernel` (first middleware is outermost).
4. Controllers read `request_token` and optionally run `AccessDecisionManager` + `RoleVoter` for subjects like `admin.area`.

Package config scaffold remains `config/packages/security.toml` from the `security` recipe (`enabled = false` until the app wires authenticators).

## CSRF (HMAC)

`HmacCsrfTokenManager::new(secret)` signs tokens as `nonce.mac` for a given intention id (usually the form name). Validation recomputes the MAC; no server-side session store is required for CSRF v0.

Use a long random app secret. Rotate only with a coordinated cutover (old tokens become invalid).

Session stickiness (HTML apps, flash, later login token storage) is separate: register `SessionMiddleware` from `serenade-session` on the HTTP kernel ([SESSION.md](SESSION.md)). CSRF does not depend on that middleware.

## Password hashing

`Argon2idPasswordHasher` implements `PasswordHasher`:

```rust
use serenade_security::{Argon2idPasswordHasher, PasswordHasher};

let hasher = Argon2idPasswordHasher::new();
let hashed = hasher.hash("secret")?;
assert!(hasher.verify(&hashed, "secret")?);
assert!(!hasher.verify(&hashed, "wrong")?);
```

- Output is a PHC string (`$argon2id$…`)
- Empty plain passwords are rejected
- Malformed stored hashes return `SecurityError::Password`
- Apps own user rows and when to rehash after parameter changes

## XSS

Default HTML escaping for form render lives in **`serenade-form`** (`escape_html` / `escape_attr`). Controllers must not concatenate raw user input into HTML responses.

## Non-goals

- OAuth2 / OIDC providers
- Built-in user persistence
- Coupling CSRF to a server session (CSRF v0 stays HMAC-stateless; session is optional via `serenade-session`)
