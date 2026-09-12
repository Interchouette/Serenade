# serenade-security

AuthN/Z hooks: users, tokens, voters, HTTP firewall middleware, CSRF tokens,
and Argon2id password hashing (`PasswordHasher` / `Argon2idPasswordHasher`).

Full OAuth/OIDC is out of scope. Apps plug bearer or API-key authenticators
into `FirewallMiddleware`. CSRF: `HmacCsrfTokenManager` (stateless HMAC).
See `docs-dev/SECURITY.md` and Forms (`serenade-form`).
