//! Persist authenticated identity in a [`Session`](serenade_session::Session).

use serenade_http::{
    AsyncMiddleware, AsyncNext, HttpError, Middleware, Request, RequestHandler, Response,
};
use serenade_session::{Session, request_session};

use crate::firewall::TOKEN_ATTRIBUTE;
use crate::user::{InMemoryUser, TokenInterface, UsernamePasswordToken};

/// Session attribute key for the serialized security identity.
pub const SECURITY_SESSION_KEY: &str = "_serenade.security_token";

/// Stores an authenticated token in `session` (user id + roles only; no credentials).
///
/// Anonymous or unauthenticated tokens clear the session key (same as [`logout`]).
pub fn login(session: &mut Session, token: &UsernamePasswordToken) {
    if !token.is_authenticated() {
        logout(session);
        return;
    }
    let Some(user) = token.user() else {
        logout(session);
        return;
    };
    session.set(
        SECURITY_SESSION_KEY,
        encode_identity(user.user_identifier(), user.roles()),
    );
}

/// Removes the security identity from `session`.
pub fn logout(session: &mut Session) {
    let _ = session.remove(SECURITY_SESSION_KEY);
}

/// Restores a token from `session` when a valid identity is stored.
#[must_use]
pub fn token_from_session(session: &Session) -> Option<UsernamePasswordToken> {
    let raw = session.get(SECURITY_SESSION_KEY)?;
    let (id, roles) = decode_identity(raw)?;
    Some(UsernamePasswordToken::authenticated(
        InMemoryUser::new(id, roles),
        String::new(),
    ))
}

/// Sync middleware: restore session identity onto `_security_token` when absent.
///
/// Push [`serenade_session::SessionMiddleware`] first (outermost), then this layer.
/// Does not overwrite a token already set (for example by the firewall).
#[derive(Clone, Copy, Debug, Default)]
pub struct SessionTokenMiddleware;

impl SessionTokenMiddleware {
    /// Restores the security token from the request session.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Middleware for SessionTokenMiddleware {
    fn process(
        &self,
        request: &mut Request,
        next: &dyn RequestHandler,
    ) -> Result<Response, HttpError> {
        restore_token_attribute(request);
        next.handle(request)
    }
}

/// Async variant of [`SessionTokenMiddleware`].
#[derive(Clone, Copy, Debug, Default)]
pub struct AsyncSessionTokenMiddleware;

impl AsyncSessionTokenMiddleware {
    /// Restores the security token from the request session.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl AsyncMiddleware for AsyncSessionTokenMiddleware {
    fn process<'a>(
        &'a self,
        request: &'a mut Request,
        next: AsyncNext<'a>,
    ) -> serenade_http::BoxFuture<'a, Result<Response, HttpError>> {
        Box::pin(async move {
            restore_token_attribute(request);
            next.run(request).await
        })
    }
}

fn restore_token_attribute(request: &mut Request) {
    if request
        .attributes()
        .get::<UsernamePasswordToken>(TOKEN_ATTRIBUTE)
        .is_some()
    {
        return;
    }
    let Some(session) = request_session(request) else {
        return;
    };
    let Some(token) = token_from_session(session) else {
        return;
    };
    request.attributes_mut().insert(TOKEN_ATTRIBUTE, token);
}

fn encode_identity(user_id: &str, roles: &[String]) -> String {
    let mut out = escape(user_id);
    out.push('\n');
    for (index, role) in roles.iter().enumerate() {
        if index > 0 {
            out.push('|');
        }
        out.push_str(&escape(role));
    }
    out
}

fn decode_identity(raw: &str) -> Option<(String, Vec<String>)> {
    let mut lines = raw.splitn(2, '\n');
    let id_raw = lines.next()?;
    let roles_raw = lines.next().unwrap_or("");
    let id = unescape(id_raw);
    if id.is_empty() {
        return None;
    }
    let roles = if roles_raw.is_empty() {
        Vec::new()
    } else {
        split_unescaped(roles_raw, '|')
            .into_iter()
            .map(|part| unescape(&part))
            .filter(|role| !role.is_empty())
            .collect()
    };
    Some((id, roles))
}

fn split_unescaped(value: &str, sep: char) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            current.push('\\');
            if let Some(next) = chars.next() {
                current.push(next);
            }
            continue;
        }
        if ch == sep {
            parts.push(std::mem::take(&mut current));
            continue;
        }
        current.push(ch);
    }
    parts.push(current);
    parts
}

fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '|' => out.push_str("\\|"),
            other => out.push(other),
        }
    }
    out
}

fn unescape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('|') => out.push('|'),
            Some('\\') | None => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serenade_http::{AsyncHttpKernel, HttpKernel, Method, Request, Response};
    use serenade_session::{
        AsyncSessionMiddleware, CookieSession, MemorySessionStore, Session, SessionMiddleware,
        request_session_mut,
    };

    use super::{
        AsyncSessionTokenMiddleware, SECURITY_SESSION_KEY, SessionTokenMiddleware, decode_identity,
        encode_identity, login, logout, token_from_session, unescape,
    };
    use crate::firewall::{TOKEN_ATTRIBUTE, request_token};
    use crate::user::{InMemoryUser, TokenInterface, UsernamePasswordToken};
    use crate::{Authenticator, FirewallMiddleware, SecurityError};

    #[test]
    fn login_logout_roundtrip_and_escape() {
        let mut session = Session::new("s1");
        let token = UsernamePasswordToken::authenticated(
            InMemoryUser::new(
                "alice",
                vec![String::from("ROLE_USER"), String::from("a|b")],
            ),
            "secret-should-not-persist",
        );
        login(&mut session, &token);
        let restored = token_from_session(&session).expect("token");
        assert!(restored.is_authenticated());
        assert_eq!(restored.user().expect("user").user_identifier(), "alice");
        assert_eq!(
            restored.user().expect("user").roles(),
            &[String::from("ROLE_USER"), String::from("a|b")]
        );
        assert_eq!(restored.credentials(), Some(""));
        logout(&mut session);
        assert!(token_from_session(&session).is_none());
        assert!(session.get(SECURITY_SESSION_KEY).is_none());
    }

    #[test]
    fn login_anonymous_clears_and_decode_rejects_empty_id() {
        let mut session = Session::new("s2");
        login(
            &mut session,
            &UsernamePasswordToken::authenticated(
                InMemoryUser::new("bob", vec![String::from("ROLE_ADMIN")]),
                "",
            ),
        );
        login(&mut session, &UsernamePasswordToken::anonymous());
        assert!(token_from_session(&session).is_none());
        assert!(decode_identity("\nROLE_USER").is_none());
        assert!(decode_identity("").is_none());
        assert_eq!(encode_identity("u", &[]), "u\n");
        assert_eq!(
            decode_identity("id\\nline\nrole\\|x|ok").expect("decode"),
            (
                String::from("id\nline"),
                vec![String::from("role|x"), String::from("ok")]
            )
        );
        assert_eq!(unescape("\\ztrail\\"), "\\ztrail\\");
        assert_eq!(unescape("\\\\"), "\\");
    }

    #[test]
    fn session_token_middleware_restores_without_overwriting() {
        let store = Arc::new(MemorySessionStore::new());
        let cookies = CookieSession::new(store);
        let mut seed = HttpKernel::new(|request: &mut Request| {
            let session = request_session_mut(request).expect("session");
            login(
                session,
                &UsernamePasswordToken::authenticated(
                    InMemoryUser::new("carol", vec![String::from("ROLE_USER")]),
                    "",
                ),
            );
            Ok(Response::text(200, "seed"))
        });
        seed.push_middleware(SessionMiddleware::new(cookies.clone()));
        let set_cookie = seed
            .handle(Request::new(Method::Get, "/"))
            .headers()
            .get("set-cookie")
            .expect("cookie")
            .to_owned();

        let mut kernel = HttpKernel::new(|request: &mut Request| {
            let token = request_token(request).expect("restored");
            assert_eq!(token.user().expect("user").user_identifier(), "carol");
            Ok(Response::text(200, "ok"))
        });
        kernel.push_middleware(SessionMiddleware::new(cookies.clone()));
        kernel.push_middleware(SessionTokenMiddleware::new());
        let response =
            kernel.handle(Request::new(Method::Get, "/").with_header("cookie", &set_cookie));
        assert_eq!(response.status(), 200);

        let mut overwrite = HttpKernel::new(|request: &mut Request| {
            let token = request_token(request).expect("firewall");
            assert_eq!(token.user().expect("user").user_identifier(), "dave");
            Ok(Response::text(200, "fw"))
        });
        overwrite.push_middleware(SessionMiddleware::new(cookies));
        overwrite.push_middleware(SessionTokenMiddleware);
        overwrite.push_middleware(FirewallMiddleware::new("Authorization", FixedAuthenticator));
        let response = overwrite.handle(
            Request::new(Method::Get, "/")
                .with_header("cookie", set_cookie)
                .with_header("Authorization", "yes"),
        );
        assert_eq!(response.status(), 200);
    }

    struct FixedAuthenticator;

    impl Authenticator for FixedAuthenticator {
        fn authenticate(
            &self,
            credentials: Option<&str>,
        ) -> Result<UsernamePasswordToken, SecurityError> {
            if credentials == Some("yes") {
                Ok(UsernamePasswordToken::authenticated(
                    InMemoryUser::new("dave", vec![String::from("ROLE_ADMIN")]),
                    "yes",
                ))
            } else {
                Err(SecurityError::Authentication {
                    message: String::from("no"),
                })
            }
        }
    }

    #[tokio::test]
    async fn async_session_token_middleware_restores() {
        let cookies = CookieSession::new(Arc::new(MemorySessionStore::new()));
        let mut session = Session::new("async");
        login(
            &mut session,
            &UsernamePasswordToken::authenticated(InMemoryUser::new("erin", Vec::new()), ""),
        );
        let set = cookies.commit(&session).expect("commit").expect("cookie");

        let mut kernel = AsyncHttpKernel::from_sync(|request: &mut Request| {
            assert!(request.attributes().contains(TOKEN_ATTRIBUTE));
            assert_eq!(
                request_token(request)
                    .expect("token")
                    .user()
                    .expect("user")
                    .user_identifier(),
                "erin"
            );
            Ok(Response::text(200, "ok"))
        });
        kernel.push_middleware(AsyncSessionMiddleware::new(cookies));
        kernel.push_middleware(AsyncSessionTokenMiddleware::new());
        let response = kernel
            .handle(Request::new(Method::Get, "/").with_header("cookie", set))
            .await;
        assert_eq!(response.status(), 200);
    }

    #[test]
    fn restore_skips_without_session_or_identity() {
        let mut request = Request::new(Method::Get, "/");
        super::restore_token_attribute(&mut request);
        assert!(request_token(&request).is_none());

        let mut kernel = HttpKernel::new(|request: &mut Request| {
            assert!(request_token(request).is_none());
            Ok(Response::text(200, "anon"))
        });
        let cookies = CookieSession::new(Arc::new(MemorySessionStore::new()));
        kernel.push_middleware(SessionMiddleware::new(cookies));
        kernel.push_middleware(SessionTokenMiddleware);
        assert_eq!(kernel.handle(Request::new(Method::Get, "/")).status(), 200);
    }

    #[test]
    fn login_authenticated_without_user_clears() {
        let mut session = Session::new("s3");
        login(
            &mut session,
            &UsernamePasswordToken::authenticated(InMemoryUser::new("keep", Vec::new()), ""),
        );
        login(
            &mut session,
            &UsernamePasswordToken::authenticated_without_user(),
        );
        assert!(token_from_session(&session).is_none());
    }

    #[test]
    fn encode_escapes_backslash_and_newline_in_id() {
        let encoded = encode_identity("a\\b\nc", &[String::from("r")]);
        assert_eq!(
            decode_identity(&encoded).expect("decode"),
            (String::from("a\\b\nc"), vec![String::from("r")])
        );
        assert_eq!(
            decode_identity("u\n|ROLE|").expect("empty roles filtered"),
            (String::from("u"), vec![String::from("ROLE")])
        );
        assert_eq!(
            decode_identity("u\ntrail\\").expect("trailing backslash"),
            (String::from("u"), vec![String::from("trail\\")])
        );
    }

    #[test]
    fn session_token_middleware_skips_when_token_already_set() {
        let cookies = CookieSession::new(Arc::new(MemorySessionStore::new()));
        let mut session = Session::new("pre");
        login(
            &mut session,
            &UsernamePasswordToken::authenticated(
                InMemoryUser::new("from-session", Vec::new()),
                "",
            ),
        );
        let set = cookies.commit(&session).expect("commit").expect("cookie");

        let mut kernel = HttpKernel::new(|request: &mut Request| {
            assert_eq!(
                request_token(request)
                    .expect("token")
                    .user()
                    .expect("user")
                    .user_identifier(),
                "dave"
            );
            Ok(Response::text(200, "ok"))
        });
        kernel.push_middleware(SessionMiddleware::new(cookies));
        kernel.push_middleware(FirewallMiddleware::new("Authorization", FixedAuthenticator));
        kernel.push_middleware(SessionTokenMiddleware);
        let response = kernel.handle(
            Request::new(Method::Get, "/")
                .with_header("cookie", set)
                .with_header("Authorization", "yes"),
        );
        assert_eq!(response.status(), 200);
    }

    #[test]
    fn middleware_constructors() {
        let _ = SessionTokenMiddleware::new();
        let _ = AsyncSessionTokenMiddleware::new();
        assert!(FixedAuthenticator.authenticate(None).is_err());
    }
}
