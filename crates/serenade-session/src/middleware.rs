//! HTTP middleware that loads and saves a [`Session`](crate::Session) per request.

use serenade_http::{
    AsyncMiddleware, AsyncNext, HttpError, Middleware, Request, RequestHandler, Response,
};

use crate::{CookieSession, Session, SessionError};

/// Request attribute key where the session middleware stores the [`Session`].
pub const SESSION_ATTRIBUTE: &str = "_serenade_session";

/// Loads a session before the handler and commits it afterward (sync kernel).
pub struct SessionMiddleware {
    cookies: CookieSession,
}

impl SessionMiddleware {
    /// Middleware bound to `cookies` (store + cookie options).
    #[must_use]
    pub const fn new(cookies: CookieSession) -> Self {
        Self { cookies }
    }

    /// Cookie session helper used by this middleware.
    #[must_use]
    pub const fn cookies(&self) -> &CookieSession {
        &self.cookies
    }
}

impl Middleware for SessionMiddleware {
    fn process(
        &self,
        request: &mut Request,
        next: &dyn RequestHandler,
    ) -> Result<Response, HttpError> {
        open_into_request(&self.cookies, request)?;
        let mut response = next.handle(request)?;
        commit_from_request(&self.cookies, request, &mut response)?;
        Ok(response)
    }
}

/// Async [`AsyncMiddleware`] variant for [`serenade_http::AsyncHttpKernel`].
pub struct AsyncSessionMiddleware {
    cookies: CookieSession,
}

impl AsyncSessionMiddleware {
    /// Middleware bound to `cookies` (store + cookie options).
    #[must_use]
    pub const fn new(cookies: CookieSession) -> Self {
        Self { cookies }
    }

    /// Cookie session helper used by this middleware.
    #[must_use]
    pub const fn cookies(&self) -> &CookieSession {
        &self.cookies
    }
}

impl AsyncMiddleware for AsyncSessionMiddleware {
    fn process<'a>(
        &'a self,
        request: &'a mut Request,
        next: AsyncNext<'a>,
    ) -> serenade_http::BoxFuture<'a, Result<Response, HttpError>> {
        Box::pin(async move {
            open_into_request(&self.cookies, request)?;
            let mut response = next.run(request).await?;
            commit_from_request(&self.cookies, request, &mut response)?;
            Ok(response)
        })
    }
}

/// Returns the session from request attributes when present.
#[must_use]
pub fn request_session(request: &Request) -> Option<&Session> {
    request.attributes().get::<Session>(SESSION_ATTRIBUTE)
}

/// Returns a mutable session from request attributes when present.
pub fn request_session_mut(request: &mut Request) -> Option<&mut Session> {
    request
        .attributes_mut()
        .get_mut::<Session>(SESSION_ATTRIBUTE)
}

fn open_into_request(cookies: &CookieSession, request: &mut Request) -> Result<(), HttpError> {
    let cookie_header = request.headers().get("cookie");
    let session = cookies
        .open(cookie_header)
        .map_err(|err| session_http_error(&err))?;
    request.attributes_mut().insert(SESSION_ATTRIBUTE, session);
    Ok(())
}

fn commit_from_request(
    cookies: &CookieSession,
    request: &mut Request,
    response: &mut Response,
) -> Result<(), HttpError> {
    let Some(session) = request
        .attributes_mut()
        .remove::<Session>(SESSION_ATTRIBUTE)
    else {
        return Ok(());
    };
    if let Some(set_cookie) = cookies
        .commit(&session)
        .map_err(|err| session_http_error(&err))?
    {
        response.headers_mut().insert("set-cookie", set_cookie);
    }
    Ok(())
}

fn session_http_error(err: &SessionError) -> HttpError {
    HttpError::failed(err.to_string())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serenade_http::{AsyncHttpKernel, HttpKernel, Method, Request, Response};

    use super::{
        AsyncSessionMiddleware, SESSION_ATTRIBUTE, SessionMiddleware, request_session,
        request_session_mut,
    };
    use crate::{
        CookieSession, DEFAULT_SESSION_COOKIE, MemorySessionStore, SessionStore, parse_cookie_value,
    };

    #[test]
    fn session_middleware_loads_saves_and_sets_cookie() {
        let store = Arc::new(MemorySessionStore::new());
        let cookies = CookieSession::new(Arc::clone(&store) as Arc<dyn SessionStore>);
        let mut kernel = HttpKernel::new(|request: &mut Request| {
            assert!(request.attributes().contains(SESSION_ATTRIBUTE));
            let session = request_session_mut(request).expect("session");
            session.set("user", "42");
            session.flash().add("success", "Saved");
            Ok(Response::text(200, "ok"))
        });
        kernel.push_middleware(SessionMiddleware::new(cookies.clone()));
        let response = kernel.handle(Request::new(Method::Get, "/"));
        assert_eq!(response.status(), 200);
        let set_cookie = response.headers().get("set-cookie").expect("set-cookie");
        assert!(set_cookie.contains(DEFAULT_SESSION_COOKIE));
        let id = parse_cookie_value(set_cookie, DEFAULT_SESSION_COOKIE).expect("id");
        let attrs = store.load(&id).expect("load").expect("attrs");
        assert_eq!(attrs.get("user").map(String::as_str), Some("42"));
        assert!(attrs.contains_key("_serenade.flashes"));

        let mut kernel = HttpKernel::new(|request: &mut Request| {
            assert_eq!(
                request_session(request).expect("session").get("user"),
                Some("42")
            );
            let session = request_session_mut(request).expect("session");
            assert_eq!(session.flash().peek("success"), vec![String::from("Saved")]);
            Ok(Response::text(200, "again"))
        });
        kernel.push_middleware(SessionMiddleware::new(cookies));
        let response = kernel.handle(
            Request::new(Method::Get, "/")
                .with_header("cookie", format!("{DEFAULT_SESSION_COOKIE}={id}")),
        );
        assert_eq!(response.status(), 200);
        assert!(response.headers().get("set-cookie").is_none());
    }

    #[test]
    fn session_middleware_skips_set_cookie_when_clean() {
        let store = Arc::new(MemorySessionStore::new());
        let cookies = CookieSession::new(store);
        let mut first = HttpKernel::new(|request: &mut Request| {
            request_session_mut(request).expect("session").set("n", "1");
            Ok(Response::text(200, "1"))
        });
        first.push_middleware(SessionMiddleware::new(cookies.clone()));
        let set = first
            .handle(Request::new(Method::Get, "/"))
            .headers()
            .get("set-cookie")
            .expect("cookie")
            .to_owned();
        let mut second = HttpKernel::new(|_request: &mut Request| Ok(Response::text(200, "2")));
        second.push_middleware(SessionMiddleware::new(cookies));
        let response = second.handle(Request::new(Method::Get, "/").with_header("cookie", set));
        assert!(response.headers().get("set-cookie").is_none());
    }

    #[test]
    fn middleware_accessors() {
        let cookies = CookieSession::new(Arc::new(MemorySessionStore::new()));
        assert_eq!(
            SessionMiddleware::new(cookies.clone())
                .cookies()
                .options()
                .name(),
            DEFAULT_SESSION_COOKIE
        );
        assert_eq!(
            AsyncSessionMiddleware::new(cookies)
                .cookies()
                .options()
                .name(),
            DEFAULT_SESSION_COOKIE
        );
    }

    #[test]
    fn session_middleware_commits_when_attribute_removed() {
        let cookies = CookieSession::new(Arc::new(MemorySessionStore::new()));
        let mut kernel = HttpKernel::new(|request: &mut Request| {
            let _ = request
                .attributes_mut()
                .remove::<crate::Session>(SESSION_ATTRIBUTE);
            Ok(Response::text(200, "gone"))
        });
        kernel.push_middleware(SessionMiddleware::new(cookies));
        let response = kernel.handle(Request::new(Method::Get, "/"));
        assert_eq!(response.status(), 200);
        assert!(response.headers().get("set-cookie").is_none());
    }

    #[test]
    fn session_middleware_maps_store_errors() {
        struct Boom;

        impl SessionStore for Boom {
            fn load(
                &self,
                _id: &str,
            ) -> Result<Option<std::collections::HashMap<String, String>>, crate::SessionError>
            {
                Err(crate::SessionError::Store {
                    message: String::from("load boom"),
                })
            }

            fn save(
                &self,
                _id: &str,
                _attributes: &std::collections::HashMap<String, String>,
            ) -> Result<(), crate::SessionError> {
                Err(crate::SessionError::Store {
                    message: String::from("save boom"),
                })
            }

            fn delete(&self, _id: &str) -> Result<(), crate::SessionError> {
                Ok(())
            }
        }

        let cookies = CookieSession::new(Arc::new(Boom));
        let mut open_fail = HttpKernel::new(|_: &mut Request| Ok(Response::text(200, "x")));
        open_fail.push_middleware(SessionMiddleware::new(cookies.clone()));
        let response = open_fail.handle(
            Request::new(Method::Get, "/").with_header("cookie", "SERENADE_SESSION=abc"),
        );
        assert_eq!(response.status(), 500);
        assert!(response.body_str().unwrap_or("").contains("load boom"));

        let mut save_fail = HttpKernel::new(|request: &mut Request| {
            request_session_mut(request)
                .expect("session")
                .set("k", "v");
            Ok(Response::text(200, "x"))
        });
        save_fail.push_middleware(SessionMiddleware::new(cookies));
        let response = save_fail.handle(Request::new(Method::Get, "/"));
        assert_eq!(response.status(), 500);
        assert!(response.body_str().unwrap_or("").contains("save boom"));
    }

    #[tokio::test]
    async fn async_session_middleware_roundtrip() {
        let cookies = CookieSession::new(Arc::new(MemorySessionStore::new()));
        let mut kernel = AsyncHttpKernel::from_sync(|request: &mut Request| {
            request_session_mut(request)
                .expect("session")
                .set("async", "1");
            Ok(Response::text(200, "ok"))
        });
        kernel.push_middleware(AsyncSessionMiddleware::new(cookies));
        let response = kernel.handle(Request::new(Method::Get, "/")).await;
        assert_eq!(response.status(), 200);
        assert!(response.headers().get("set-cookie").is_some());
    }
}
