//! AuthN/Z hooks: users, tokens, voters, HTTP firewall, CSRF, password hashing,
//! and session login bridge.
//!
//! Full OAuth/OIDC is out of scope. Apps plug bearer or API-key authenticators
//! into [`FirewallMiddleware`]. CSRF uses [`HmacCsrfTokenManager`] (stateless HMAC).
//! Password hashing uses [`Argon2idPasswordHasher`]. Session stickiness for HTML
//! logins uses [`login`] / [`SessionTokenMiddleware`]. See `docs-dev/SECURITY.md`.

mod access;
mod csrf;
mod error;
mod firewall;
mod password;
mod session_bridge;
mod user;

pub use access::{AccessDecisionManager, RoleVoter, Subject, Vote, Voter};
pub use csrf::{CSRF_FIELD_NAME, CsrfToken, CsrfTokenManager, HmacCsrfTokenManager};
pub use error::SecurityError;
pub use firewall::{Authenticator, FirewallMiddleware, TOKEN_ATTRIBUTE, request_token};
pub use password::{Argon2idPasswordHasher, PasswordHasher};
pub use session_bridge::{
    AsyncSessionTokenMiddleware, SECURITY_SESSION_KEY, SessionTokenMiddleware, login, logout,
    token_from_session,
};
pub use user::{InMemoryUser, TokenInterface, UserInterface, UsernamePasswordToken};

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
