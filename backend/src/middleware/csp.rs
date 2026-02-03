use axum::{
    body::Body,
    http::{HeaderName, HeaderValue, Request, Response},
    middleware::Next,
};

// Simple CSP middleware that adds a Content-Security-Policy header to all responses.
// Adjust the policy string as needed for your environment.
pub async fn csp_middleware(req: Request<Body>, next: Next) -> Response<Body> {
    let mut res = next.run(req).await;

    // Policy: allow self, telegram widget, and images from https. Disallow objects.
    // Update this string if you need to allow additional trusted domains.
    const CSP: &str = "default-src 'self'; script-src 'self' https://telegram.org; connect-src 'self' https://api.telegram.org; img-src 'self' https:; frame-src https://telegram.org; object-src 'none'; base-uri 'self'; form-action 'self'; frame-ancestors 'self'";

    // Insert header if not already present
    if res.headers().get("content-security-policy").is_none() {
        let val = HeaderValue::from_static(CSP);
        res.headers_mut()
            .insert(HeaderName::from_static("content-security-policy"), val);
    }

    // Also add Referrer-Policy and X-Content-Type-Options for extra security
    if res.headers().get("referrer-policy").is_none() {
        let val = HeaderValue::from_static("no-referrer");
        res.headers_mut()
            .insert(HeaderName::from_static("referrer-policy"), val);
    }
    if res.headers().get("x-content-type-options").is_none() {
        let val = HeaderValue::from_static("nosniff");
        res.headers_mut()
            .insert(HeaderName::from_static("x-content-type-options"), val);
    }

    res
}
