# serenade-security

AuthN/Z hooks: users, tokens, voters, HTTP firewall middleware, CSRF tokens,
Argon2id password hashing, and session login bridge (`login` /
`SessionTokenMiddleware`).

Full OAuth/OIDC is out of scope. Apps plug bearer or API-key authenticators
into `FirewallMiddleware`. CSRF: `HmacCsrfTokenManager` (stateless HMAC).
See `docs-dev/SECURITY.md` and Forms (`serenade-form`).
