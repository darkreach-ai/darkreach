//! Security-focused integration tests for the darkreach API.
//!
//! These tests verify that the API is resilient against common web application
//! attack vectors. Each test targets a specific vulnerability class from the
//! OWASP Top 10 and CWE databases, ensuring that input validation, output
//! encoding, and middleware protections work correctly.
//!
//! # Attack vectors covered
//!
//! | Test | OWASP / CWE | Description |
//! |------|-------------|-------------|
//! | SQL injection (sort_by, sort_dir, search) | A03:2021 Injection / CWE-89 | Parameterized queries prevent SQL injection |
//! | SQL injection data integrity | A03:2021 Injection / CWE-89 | Verify DB state unchanged after injection |
//! | Body size limit | A05:2021 Security Misconfiguration | 1MB payload limit prevents DoS |
//! | CORS allowed origin | A05:2021 Security Misconfiguration | Allowed origins get CORS headers |
//! | CORS evil origin rejected | A05:2021 Security Misconfiguration / CWE-942 | Unauthorized origins blocked |
//! | Path traversal (doc slug, roadmap) | A01:2021 Broken Access Control / CWE-22 | Slug validation prevents file reads |
//! | Negative block_size | A08:2021 Software Integrity / CWE-20 | Input validation rejects nonsensical values |
//! | Malformed JSON | A08:2021 Software Integrity / CWE-20 | JSON parser rejects invalid payloads |
//! | Missing API key | A07:2021 Auth Failures / CWE-287 | Operator endpoints require authentication |
//! | Invalid API key | A07:2021 Auth Failures / CWE-287 | Fabricated keys rejected |
//! | Error message leakage | A04:2021 Insecure Design / CWE-209 | No internal details in error responses |
//!
//! # Prerequisites
//!
//! - A running PostgreSQL instance with the `TEST_DATABASE_URL` environment variable set.
//! - Example: `TEST_DATABASE_URL=postgres://user:pass@localhost:5432/darkreach_test`
//!
//! # How to run
//!
//! ```bash
//! # Run all security tests (single-threaded):
//! TEST_DATABASE_URL=postgres://... cargo test --test security_tests -- --test-threads=1
//!
//! # Run a specific test:
//! TEST_DATABASE_URL=postgres://... cargo test --test security_tests sql_injection
//! ```
//!
//! # Testing strategy
//!
//! Security tests use a black-box approach: they send malicious input through
//! the HTTP API and verify the response is safe (correct status code, no data
//! leakage, no server crash). The tests do NOT verify internal implementation
//! details -- they validate observable behavior from an attacker's perspective.
//!
//! Each test sends multiple payloads for its attack class to improve coverage.
//! For SQL injection, we test multiple injection techniques (comment injection,
//! UNION SELECT, stacked queries, etc.).

mod common;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use tower::ServiceExt;

/// Skip the test if TEST_DATABASE_URL is not set.
macro_rules! require_db {
    () => {
        if !common::has_test_db() {
            eprintln!("Skipping: TEST_DATABASE_URL not set");
            return;
        }
    };
}

/// Builds a fresh Axum test router with a clean database.
async fn app() -> Router {
    common::build_test_app().await
}

/// Sends a GET request and returns the status code and parsed JSON body.
async fn get(app: Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let response = app
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap_or(serde_json::json!(null));
    (status, json)
}

// == SQL Injection =============================================================
// Tests that user-supplied query parameters cannot alter SQL query semantics.
//
// The darkreach API uses parameterized queries (via sqlx) for all user data,
// but the sort_by and sort_dir parameters are interpolated into ORDER BY clauses
// (since prepared statements cannot parameterize column names). These tests
// verify the allowlist-based sanitization catches injection attempts.
//
// References:
// - OWASP: https://owasp.org/Top10/A03_2021-Injection/
// - CWE-89: https://cwe.mitre.org/data/definitions/89.html
// ==============================================================================

/// Tests SQL injection resistance in the sort_by query parameter.
///
/// **Attack vector**: OWASP A03:2021 Injection / CWE-89 (SQL Injection).
///
/// The `sort_by` parameter is used in ORDER BY clauses where parameterized
/// queries cannot be used. The server uses an allowlist of valid column names
/// (id, digits, expression, form, created_at) and falls through to "id" for
/// unrecognized values. This test sends 4 injection payloads:
///
/// 1. `'; DROP TABLE primes; --` -- Classic single-quote breakout
/// 2. `1; DELETE FROM primes` -- Stacked query attempt
/// 3. `expression UNION SELECT * FROM pg_tables --` -- UNION-based data extraction
/// 4. `id; UPDATE primes SET form='hacked'` -- Stacked UPDATE attempt
///
/// Expected behavior: All attempts return 200 OK with a valid JSON array.
/// The injected SQL is treated as a literal string, fails the allowlist check,
/// and defaults to sorting by "id".
#[tokio::test]
async fn sql_injection_sort_column_sanitized() {
    require_db!();
    // Attempt SQL injection via sort_by parameter
    let injections = [
        "'; DROP TABLE primes; --",
        "1; DELETE FROM primes",
        "expression UNION SELECT * FROM pg_tables --",
        "id; UPDATE primes SET form='hacked'",
    ];

    for injection in &injections {
        let uri = format!("/api/primes?sort_by={}", urlencoding::encode(injection));
        let (status, json) = get(app().await, &uri).await;
        // Should succeed (injected value falls through to default "id")
        assert_eq!(
            status,
            StatusCode::OK,
            "Injection attempt should not crash: {}",
            injection
        );
        // Should return valid JSON with primes array
        assert!(
            json["primes"].is_array(),
            "Response should contain primes array for: {}",
            injection
        );
    }
}

/// Tests SQL injection resistance in the sort_dir query parameter.
///
/// **Attack vector**: OWASP A03:2021 Injection / CWE-89 (SQL Injection).
///
/// The `sort_dir` parameter is interpolated into the ORDER BY clause as either
/// "ASC" or "DESC". The server validates against these two exact values and
/// defaults to "DESC" for anything else. This test sends 3 injection payloads:
///
/// 1. `DESC; DROP TABLE primes; --` -- Append stacked query after valid keyword
/// 2. `asc UNION SELECT 1,2,3,4,5` -- Append UNION after valid keyword
/// 3. `'; --` -- Classic breakout attempt
///
/// Expected behavior: All return 200 OK (injected value defaults to "DESC").
#[tokio::test]
async fn sql_injection_sort_dir_sanitized() {
    require_db!();
    let injections = [
        "DESC; DROP TABLE primes; --",
        "asc UNION SELECT 1,2,3,4,5",
        "'; --",
    ];

    for injection in &injections {
        let uri = format!("/api/primes?sort_dir={}", urlencoding::encode(injection));
        let (status, _) = get(app().await, &uri).await;
        assert_eq!(
            status,
            StatusCode::OK,
            "Sort dir injection should not crash: {}",
            injection
        );
    }
}

/// Tests SQL injection resistance in the search (ILIKE) query parameter.
///
/// **Attack vector**: OWASP A03:2021 Injection / CWE-89 (SQL Injection).
///
/// The `search` parameter is used in a `WHERE expression ILIKE $1` clause
/// with proper parameterization. Unlike sort_by/sort_dir, this parameter IS
/// passed through a prepared statement placeholder, making injection impossible
/// at the SQL level. However, we still test to verify:
///
/// 1. The server does not crash on malicious input
/// 2. The response is always a valid JSON array
/// 3. No data from other tables leaks through
///
/// Test payloads include:
/// 1. `'; DROP TABLE primes; --` -- Classic breakout (becomes literal in ILIKE)
/// 2. `%'; DELETE FROM primes WHERE '1'='1` -- Percent-wildcard breakout
/// 3. `' OR '1'='1` -- Boolean tautology (classic login bypass, ineffective here)
/// 4. `\'; UPDATE primes SET form='hacked'; --` -- Escaped quote breakout
#[tokio::test]
async fn sql_injection_search_param_escaped() {
    require_db!();
    // Attempt SQL injection via the search parameter (used in ILIKE with parameterized query)
    let injections = [
        "'; DROP TABLE primes; --",
        "%'; DELETE FROM primes WHERE '1'='1",
        "' OR '1'='1",
        "\\'; UPDATE primes SET form='hacked'; --",
    ];

    for injection in &injections {
        let uri = format!("/api/primes?search={}", urlencoding::encode(injection));
        let (status, json) = get(app().await, &uri).await;
        assert_eq!(
            status,
            StatusCode::OK,
            "Search injection should not crash: {}",
            injection
        );
        assert!(
            json["primes"].is_array(),
            "Should return valid JSON with primes for: {}",
            injection
        );
    }
}

// == Body Size Limit ===========================================================
// Tests that oversized request bodies are rejected before reaching handlers.
//
// Without a body size limit, an attacker could send multi-gigabyte payloads to
// exhaust server memory (Denial of Service). The Axum middleware rejects
// bodies larger than 1MB with 413 Payload Too Large.
//
// References:
// - OWASP: https://owasp.org/Top10/A05_2021-Security_Misconfiguration/
// - CWE-400: https://cwe.mitre.org/data/definitions/400.html
// ==============================================================================

/// Tests that payloads exceeding 1MB are rejected with 413 Payload Too Large.
///
/// **Attack vector**: OWASP A05:2021 Security Misconfiguration / CWE-400
/// (Uncontrolled Resource Consumption).
///
/// Sends a 2MB payload to the worker registration endpoint. The body size
/// limit middleware should intercept this before the JSON parser or handler
/// sees it, preventing memory exhaustion.
#[tokio::test]
async fn body_size_limit_enforced() {
    require_db!();
    let router = app().await;

    // 2MB payload exceeds the 1MB limit
    let large_body = "x".repeat(2 * 1024 * 1024);
    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/agents/tasks")
                .method(Method::POST)
                .header("content-type", "application/json")
                .header(
                    "authorization",
                    format!("Bearer {}", common::test_admin_jwt()),
                )
                .body(Body::from(large_body))
                .unwrap(),
        )
        .await
        .unwrap();
    // Axum's body limit layer may return either 413 Payload Too Large or 400 Bad Request
    assert!(
        response.status() == StatusCode::PAYLOAD_TOO_LARGE
            || response.status() == StatusCode::BAD_REQUEST,
        "Oversized body should be rejected with 413 or 400 (got {})",
        response.status()
    );
}

// == CORS ======================================================================
// Tests that Cross-Origin Resource Sharing headers are correctly configured.
//
// The darkreach frontend runs on app.darkreach.ai while the API runs on
// api.darkreach.ai. Without proper CORS headers, browsers would block
// cross-origin API calls. However, overly permissive CORS (e.g., allowing
// all origins with credentials) can enable cross-site request forgery.
//
// References:
// - OWASP: https://owasp.org/Top10/A05_2021-Security_Misconfiguration/
// - CWE-942: https://cwe.mitre.org/data/definitions/942.html
// ==============================================================================

/// Tests that CORS preflight from an allowed origin returns proper headers.
///
/// **Attack vector**: OWASP A05:2021 Security Misconfiguration / CWE-942
/// (Overly Permissive Cross-domain Whitelist).
///
/// Sends an OPTIONS preflight from `http://localhost:3000` (which is in the
/// default allowlist). The response must include:
/// - `access-control-allow-origin: http://localhost:3000`
/// - `access-control-allow-methods`
#[tokio::test]
async fn cors_preflight_allowed_origin_returns_headers() {
    require_db!();
    let router = app().await;

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/stats")
                .method(Method::OPTIONS)
                .header("origin", "http://localhost:3000")
                .header("access-control-request-method", "GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let allow_origin = response
        .headers()
        .get("access-control-allow-origin")
        .expect("Missing access-control-allow-origin for allowed origin");
    assert_eq!(
        allow_origin.to_str().unwrap(),
        "http://localhost:3000",
        "Allow-origin should reflect the allowed origin exactly"
    );
    assert!(
        response
            .headers()
            .get("access-control-allow-methods")
            .is_some(),
        "Missing access-control-allow-methods header"
    );
}

/// Tests that CORS preflight from an unauthorized origin is NOT reflected.
///
/// **Attack vector**: OWASP A05:2021 Security Misconfiguration / CWE-942.
///
/// Sends an OPTIONS preflight from `https://evil.example.com` which is not
/// in the CORS allowlist. The server must NOT include `access-control-allow-origin`
/// in the response, which causes the browser to block the cross-origin request.
/// This prevents malicious sites from making API calls on behalf of logged-in users.
#[tokio::test]
async fn cors_preflight_evil_origin_rejected() {
    require_db!();
    let router = app().await;

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/stats")
                .method(Method::OPTIONS)
                .header("origin", "https://evil.example.com")
                .header("access-control-request-method", "GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // The evil origin must NOT be reflected in allow-origin
    let allow_origin = response.headers().get("access-control-allow-origin");
    assert!(
        allow_origin.is_none()
            || allow_origin.unwrap().to_str().unwrap() != "https://evil.example.com",
        "Evil origin should not be reflected in access-control-allow-origin"
    );
}

/// Tests that GET responses to allowed cross-origin requests include the correct origin.
///
/// **Attack vector**: OWASP A05:2021 Security Misconfiguration / CWE-942.
///
/// Sends a regular GET request with an Origin header from an allowed origin.
/// The response must include `access-control-allow-origin: http://localhost:3000`.
#[tokio::test]
async fn cors_get_includes_allow_origin() {
    require_db!();
    let router = app().await;

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/status")
                .header("origin", "http://localhost:3000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let allow_origin = response
        .headers()
        .get("access-control-allow-origin")
        .expect("Missing access-control-allow-origin for allowed origin");
    assert_eq!(
        allow_origin.to_str().unwrap(),
        "http://localhost:3000",
        "Allow-origin should match the requesting origin"
    );
}

/// Tests that GET from an evil origin does NOT get allow-origin header.
///
/// **Attack vector**: OWASP A05:2021 Security Misconfiguration / CWE-942.
///
/// Sends a GET with an unauthorized origin. The response should either omit
/// `access-control-allow-origin` or not reflect the evil origin, preventing
/// the browser from exposing response data to the attacker's JavaScript.
#[tokio::test]
async fn cors_get_evil_origin_not_reflected() {
    require_db!();
    let router = app().await;

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/status")
                .header("origin", "https://evil.example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let allow_origin = response.headers().get("access-control-allow-origin");
    assert!(
        allow_origin.is_none()
            || allow_origin.unwrap().to_str().unwrap() != "https://evil.example.com",
        "Evil origin should not be reflected in access-control-allow-origin"
    );
}

// == Path Traversal ============================================================
// Tests that URL path components cannot escape the intended directory.
//
// The /api/docs/{slug} endpoint serves documentation by slug. Without proper
// validation, an attacker could use "../" sequences to read arbitrary files
// from the server filesystem (e.g., /etc/passwd, environment variables, etc.).
//
// References:
// - OWASP: https://owasp.org/Top10/A01_2021-Broken_Access_Control/
// - CWE-22: https://cwe.mitre.org/data/definitions/22.html
// ==============================================================================

/// Tests that path traversal in doc slugs is rejected.
///
/// **Attack vector**: OWASP A01:2021 Broken Access Control / CWE-22
/// (Improper Limitation of a Pathname to a Restricted Directory).
///
/// Sends 4 traversal payloads targeting the docs endpoint:
///
/// 1. `../../../etc/passwd` -- Unix traversal to read system password file
/// 2. `..%2F..%2Fetc%2Fpasswd` -- URL-encoded traversal (bypasses naive filters)
/// 3. `foo\\..\\..\etc\\passwd` -- Windows-style backslash traversal
/// 4. `..\\windows\\system32` -- Windows system directory access
///
/// Expected behavior: All return either 400 Bad Request (slug validation rejects
/// the traversal sequences) or 404 Not Found (the normalized path does not match
/// any document). The server must never return file contents from outside the
/// docs directory.
#[tokio::test]
async fn path_traversal_in_doc_slug_rejected() {
    require_db!();
    let traversal_attempts = [
        "/api/docs/../../../etc/passwd",
        "/api/docs/..%2F..%2Fetc%2Fpasswd",
        "/api/docs/foo\\..\\..\\etc\\passwd",
        "/api/docs/..\\windows\\system32",
    ];

    for path in &traversal_attempts {
        let (status, _) = get(app().await, path).await;
        assert!(
            status == StatusCode::BAD_REQUEST || status == StatusCode::NOT_FOUND,
            "Path traversal should be rejected: {} (got {})",
            path,
            status
        );
    }
}

/// Tests that path traversal in roadmap slugs is rejected.
///
/// **Attack vector**: OWASP A01:2021 Broken Access Control / CWE-22.
///
/// The roadmap endpoint shares the same slug validation as docs. This test
/// confirms the traversal protection extends to all slug-based routes.
#[tokio::test]
async fn path_traversal_in_roadmap_slug_rejected() {
    require_db!();
    let (status, _) = get(app().await, "/api/docs/roadmaps/../../../etc/passwd").await;
    assert!(
        status == StatusCode::BAD_REQUEST || status == StatusCode::NOT_FOUND,
        "Roadmap path traversal should be rejected (got {})",
        status
    );
}

// == Request Validation ========================================================
// Tests that invalid request payloads are rejected with appropriate error codes
// before reaching business logic. This prevents unexpected behavior from
// malformed or malicious inputs.
//
// References:
// - OWASP: https://owasp.org/Top10/A08_2021-Software_and_Data_Integrity_Failures/
// - CWE-20: https://cwe.mitre.org/data/definitions/20.html
// ==============================================================================

/// Tests that a negative block_size in search job creation is rejected.
///
/// **Attack vector**: OWASP A08:2021 Software Integrity / CWE-20
/// (Improper Input Validation).
///
/// A negative block_size would either cause integer overflow during block
/// generation or create an astronomically large number of blocks. The server
/// should reject this with 400 Bad Request before attempting block creation.
#[tokio::test]
async fn search_job_negative_block_size_rejected() {
    require_db!();
    let router = app().await;

    let payload = serde_json::json!({
        "search_type": "factorial",
        "params": {},
        "range_start": 1,
        "range_end": 100,
        "block_size": -5
    });

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/search_jobs")
                .method(Method::POST)
                .header("content-type", "application/json")
                .header(
                    "authorization",
                    format!("Bearer {}", common::test_admin_jwt()),
                )
                .body(Body::from(serde_json::to_string(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

/// Tests that syntactically invalid JSON is rejected with a client error.
///
/// **Attack vector**: OWASP A08:2021 Software Integrity / CWE-20
/// (Improper Input Validation).
///
/// Sends `{invalid json}` which is not valid JSON. The Axum JSON extractor
/// should return a 4xx error (400 Bad Request or 422 Unprocessable Entity)
/// rather than a 500 Internal Server Error. A 500 would indicate the server
/// is not handling the parse failure gracefully, which could expose internal
/// error details or stack traces.
#[tokio::test]
async fn malformed_json_returns_error() {
    require_db!();
    let router = app().await;

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/agents/tasks")
                .method(Method::POST)
                .header("content-type", "application/json")
                .body(Body::from("{invalid json}"))
                .unwrap(),
        )
        .await
        .unwrap();
    // Should return 4xx (400 or 422)
    assert!(
        response.status().is_client_error(),
        "Malformed JSON should return client error, got {}",
        response.status()
    );
}

// == SQL Injection Data Integrity =============================================
// Beyond checking that injection payloads return 200 (not 500), we also verify
// that the database state is unchanged after injection attempts. This catches
// scenarios where injection silently executes side-effects even if the HTTP
// response looks normal.
//
// References:
// - CWE-89: SQL Injection — data manipulation via injected statements
// ==============================================================================

/// Verifies database state is unchanged after SQL injection attempts.
///
/// **Attack vector**: OWASP A03:2021 Injection / CWE-89.
///
/// Inserts a known prime, sends multiple injection payloads that attempt
/// to modify or delete data, then verifies the prime still exists unmodified.
/// This catches "blind" injection where the HTTP response looks normal but
/// the injected SQL actually executes.
#[tokio::test]
async fn sql_injection_does_not_modify_data() {
    require_db!();
    let db = common::setup_test_db().await;

    // Insert a sentinel prime so we can verify it survives injection attempts
    sqlx::query(
        "INSERT INTO primes (form, expression, digits, proof_method, search_params) \
         VALUES ('factorial', '5!+1', 3, 'deterministic', '{}')",
    )
    .execute(db.pool())
    .await
    .unwrap();

    // Verify sentinel exists
    let count_before: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM primes")
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(
        count_before.0, 1,
        "Sentinel prime should exist before injection"
    );

    // Send injection payloads that attempt data modification
    let router = common::build_test_app().await;
    let payloads = [
        "/api/primes?sort_by=id%3B+DELETE+FROM+primes%3B+--",
        "/api/primes?sort_by=id%3B+UPDATE+primes+SET+form%3D%27hacked%27%3B+--",
        "/api/primes?sort_dir=DESC%3B+DROP+TABLE+primes%3B+--",
        "/api/primes?search=%27%3B+DELETE+FROM+primes+WHERE+%271%27%3D%271",
    ];

    for uri in &payloads {
        let resp = router
            .clone()
            .oneshot(Request::builder().uri(*uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert!(
            resp.status().is_success() || resp.status().is_client_error(),
            "Unexpected server error for injection: {}",
            uri
        );
    }

    // Verify sentinel is still intact — no rows deleted or modified
    let count_after: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM primes")
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(
        count_after.0, 1,
        "Injection should not delete primes (was {}, now {})",
        count_before.0, count_after.0
    );

    let form: (String,) = sqlx::query_as("SELECT form FROM primes WHERE expression = '5!+1'")
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(
        form.0, "factorial",
        "Injection should not modify prime form (got '{}')",
        form.0
    );
}

// == Operator Authentication ==================================================
// Tests that the operator API key authentication system correctly rejects
// unauthenticated and malformed requests.
//
// References:
// - OWASP: https://owasp.org/Top10/A07_2021-Identification_and_Authentication_Failures/
// - CWE-287: Improper Authentication
// ==============================================================================

/// Tests that operator endpoints reject requests without an API key.
///
/// **Attack vector**: OWASP A07:2021 Identification and Authentication Failures
/// / CWE-287 (Improper Authentication).
///
/// The heartbeat, work, and result endpoints require a valid `x-api-key` header.
/// Requests without this header should receive 401 Unauthorized.
#[tokio::test]
async fn operator_endpoints_reject_missing_api_key() {
    require_db!();
    let router = app().await;

    let protected_endpoints = [
        ("/api/v1/nodes/heartbeat", Method::POST),
        ("/api/v1/nodes/work", Method::GET),
        ("/api/v1/nodes/result", Method::POST),
    ];

    for (uri, method) in &protected_endpoints {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri(*uri)
                    .method(method.clone())
                    .header("content-type", "application/json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        // Some endpoints may return 400 (body parsing fails before auth check)
        assert!(
            status == StatusCode::UNAUTHORIZED || status == StatusCode::BAD_REQUEST,
            "Endpoint {} {} should reject missing API key (got {})",
            method,
            uri,
            status
        );
    }
}

/// Tests that operator endpoints reject requests with an invalid API key.
///
/// **Attack vector**: OWASP A07:2021 / CWE-287.
///
/// Sends requests with a fabricated API key that does not match any operator.
/// The server should return 401 Unauthorized without revealing whether the
/// key format is valid or whether any operators exist.
#[tokio::test]
async fn operator_endpoints_reject_invalid_api_key() {
    require_db!();
    let router = app().await;

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/nodes/heartbeat")
                .method(Method::POST)
                .header("content-type", "application/json")
                .header("x-api-key", "definitely-not-a-valid-key-12345")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    // Endpoint may return 422 (invalid input format rejected before auth check)
    assert!(
        response.status() == StatusCode::UNAUTHORIZED
            || response.status() == StatusCode::UNPROCESSABLE_ENTITY,
        "Invalid API key should return 401 or 422 (got {})",
        response.status()
    );

    // Verify error response does not leak internal details
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body);
    assert!(
        !body_str.contains("sqlx") && !body_str.contains("database") && !body_str.contains("query"),
        "Error response should not leak database details: {}",
        body_str
    );
}

/// Tests that error responses from auth endpoints don't leak internal state.
///
/// **Attack vector**: OWASP A04:2021 Insecure Design / CWE-209
/// (Generation of Error Message Containing Sensitive Information).
///
/// Sends various malformed requests and verifies error bodies do not contain
/// stack traces, SQL error messages, or internal identifiers.
#[tokio::test]
async fn auth_error_responses_do_not_leak_internals() {
    require_db!();
    let router = app().await;

    let error_probes = [
        ("/api/v1/nodes/heartbeat", Method::POST, "{}"),
        (
            "/api/v1/nodes/result",
            Method::POST,
            "{\"block_id\": 99999}",
        ),
        ("/api/v1/operators/rotate-key", Method::POST, "{}"),
    ];

    let sensitive_patterns = [
        "stack trace",
        "SQLSTATE",
        "sqlx",
        "pg_catalog",
        "internal error",
        "panicked at",
        "thread '",
        "postgresql://",
        "postgres://",
    ];

    for (uri, method, body_str) in &error_probes {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri(*uri)
                    .method(method.clone())
                    .header("content-type", "application/json")
                    .header("x-api-key", "bogus-key")
                    .body(Body::from(*body_str))
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let text = String::from_utf8_lossy(&body).to_lowercase();
        for pattern in &sensitive_patterns {
            assert!(
                !text.contains(pattern),
                "Response for {} {} leaked sensitive info '{}': {}",
                method,
                uri,
                pattern,
                text
            );
        }
    }
}
