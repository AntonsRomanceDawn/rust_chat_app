use axum::{
    body::Body,
    http::{self, Request, StatusCode},
};
use dashmap::DashMap;
use http_body_util::BodyExt;
use server::{
    config::{AppState, Config},
    create_app,
    database::db::Db,
    dtos::{
        KeyCountRespDto, LoginReqDto, LoginRespDto, OneTimePreKeyDto, PreKeyBundleRespDto,
        RegisterReqDto, RegisterRespDto, SignedPreKeyDto, UploadKeysReqDto,
    },
    errors::error_codes,
};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

/// Helper struct to hold test state
struct TestApp {
    router: axum::Router,
}

impl TestApp {
    async fn new(pool: PgPool) -> Self {
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to migrate test database");

        let db = Db::new(pool);
        let config = Config {
            // URL is managed by sqlx::test machinery, so we don't need it here usually
            database_url: "postgres://test:test@localhost/test".to_string(),
            jwt_secret: "test_secret_key_12345".to_string(),
            port: 0,
            access_expiry: 3600,
            refresh_expiry: 86400,
        };

        let state = AppState {
            config: Arc::new(config),
            db,
            channels: Arc::new(DashMap::new()),
        };

        let router = create_app(state);
        Self { router }
    }

    async fn post<T: serde::Serialize>(&self, uri: &str, body: &T) -> (StatusCode, String) {
        let req_body = serde_json::to_string(body).unwrap();
        let req = Request::builder()
            .method(http::Method::POST)
            .uri(uri)
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(Body::from(req_body))
            .unwrap();

        let response = self.router.clone().oneshot(req).await.unwrap();

        let status = response.status();
        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();

        (status, body_str)
    }

    async fn post_auth<T: serde::Serialize>(
        &self,
        uri: &str,
        body: &T,
        token: &str,
    ) -> (StatusCode, String) {
        let req_body = serde_json::to_string(body).unwrap();
        let req = Request::builder()
            .method(http::Method::POST)
            .uri(uri)
            .header(http::header::CONTENT_TYPE, "application/json")
            .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
            .body(Body::from(req_body))
            .unwrap();

        let response = self.router.clone().oneshot(req).await.unwrap();

        let status = response.status();
        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();

        (status, body_str)
    }

    async fn get_auth(&self, uri: &str, token: &str) -> (StatusCode, String) {
        let req = Request::builder()
            .method(http::Method::GET)
            .uri(uri)
            .header(http::header::AUTHORIZATION, format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let response = self.router.clone().oneshot(req).await.unwrap();

        let status = response.status();
        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();

        (status, body_str)
    }

    fn assert_error(
        &self,
        (status, body): (StatusCode, String),
        expected_status: StatusCode,
        expected_code: &str,
    ) {
        assert_eq!(
            status, expected_status,
            "Expected status {}, got {}. Body: {}",
            expected_status, status, body
        );
        let json: serde_json::Value =
            serde_json::from_str(&body).expect("Failed to parse error response");
        let actual_code = json["errors"][0]["code"]
            .as_str()
            .expect("Error response missing code field");
        assert_eq!(actual_code, expected_code, "Error code mismatch");
    }

    fn assert_success<T: serde::de::DeserializeOwned>(
        &self,
        (status, body): (StatusCode, String),
    ) -> T {
        assert!(
            status.is_success(),
            "Expected success, got {}. Body: {}",
            status,
            body
        );
        serde_json::from_str(&body).expect("Failed to parse success response")
    }
}

fn random_username() -> String {
    let uuid = Uuid::new_v4().simple().to_string();
    format!("user_{}", &uuid[..8])
}

#[sqlx::test]
async fn test_register_and_login_flow(pool: PgPool) {
    let app = TestApp::new(pool).await;

    let username = random_username();
    let password = "StrongPassword123!";

    // 1. Register
    let register_dto = RegisterReqDto {
        username: username.clone(),
        password: password.to_string(),
        confirm_password: password.to_string(),
    };

    let register_resp: RegisterRespDto =
        app.assert_success(app.post("/api/register", &register_dto).await);
    assert_eq!(register_resp.username, username);

    // 2. Register Duplicate (Expect Failure)
    let res = app.post("/api/register", &register_dto).await;
    app.assert_error(
        res,
        StatusCode::CONFLICT,
        error_codes::USERNAME_ALREADY_EXISTS,
    );

    // 3. Login with wrong username (Expect Failure)
    let wrong_login_dto = LoginReqDto {
        username: format!("{}different", username.clone()),
        password: password.to_string(),
    };
    let res = app.post("/api/login", &wrong_login_dto).await;
    app.assert_error(
        res,
        StatusCode::UNAUTHORIZED,
        error_codes::WRONG_CREDENTIALS,
    );

    // 4. Login Correctly
    let login_dto = LoginReqDto {
        username: username.clone(),
        password: password.to_string(),
    };

    let login_resp: LoginRespDto = app.assert_success(app.post("/api/login", &login_dto).await);
    assert!(!login_resp.access_token.is_empty());
    assert!(!login_resp.refresh_token.is_empty());
}

#[sqlx::test]
async fn test_register_validation_failure(pool: PgPool) {
    let app = TestApp::new(pool).await;

    let invalid_dto = RegisterReqDto {
        username: "ab".to_string(),   // Too short
        password: "pass".to_string(), // Too short
        confirm_password: "pass".to_string(),
    };

    let (status, _body) = app.post("/api/register", &invalid_dto).await;
    assert!(status.is_client_error());
}

#[sqlx::test]
async fn test_health_check_404(pool: PgPool) {
    let app = TestApp::new(pool).await;

    let req = Request::builder()
        .uri("/api/this-does-not-exist")
        .body(Body::empty())
        .unwrap();

    let response = app.router.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn test_keys_flow(pool: PgPool) {
    let app = TestApp::new(pool).await;

    // --- User 1 Setup ---
    let username1 = random_username();
    let password = "StrongPassword123!";
    let _ = app
        .post(
            "/api/register",
            &RegisterReqDto {
                username: username1.clone(),
                password: password.to_string(),
                confirm_password: password.to_string(),
            },
        )
        .await;

    let login_resp1: LoginRespDto = app.assert_success(
        app.post(
            "/api/login",
            &LoginReqDto {
                username: username1.clone(),
                password: password.to_string(),
            },
        )
        .await,
    );
    let token1 = login_resp1.access_token;

    // 1. Upload Keys for User 1
    let upload_keys_dto = UploadKeysReqDto {
        identity_key: "identity_key_base64".to_string(),
        registration_id: 1234,
        signed_prekey: SignedPreKeyDto {
            key_id: 1,
            public_key: "signed_prekey_public".to_string(),
            signature: "signed_prekey_signature".to_string(),
        },
        one_time_prekeys: vec![
            OneTimePreKeyDto {
                key_id: 101,
                public_key: "otp_101".to_string(),
            },
            OneTimePreKeyDto {
                key_id: 102,
                public_key: "otp_102".to_string(),
            },
        ],
    };

    let (status, _) = app.post_auth("/api/keys", &upload_keys_dto, &token1).await;
    assert_eq!(status, StatusCode::OK);

    // 2. Check Key Count for User 1
    let count_resp: KeyCountRespDto =
        app.assert_success(app.get_auth("/api/keys/status/count", &token1).await);
    assert_eq!(count_resp.count, 2);

    // --- User 2 Setup ---
    let username2 = random_username();
    let _ = app
        .post(
            "/api/register",
            &RegisterReqDto {
                username: username2.clone(),
                password: password.to_string(),
                confirm_password: password.to_string(),
            },
        )
        .await;

    let login_resp2: LoginRespDto = app.assert_success(
        app.post(
            "/api/login",
            &LoginReqDto {
                username: username2.clone(),
                password: password.to_string(),
            },
        )
        .await,
    );
    let token2 = login_resp2.access_token;

    // 3. User 2 fetches PreKey Bundle for User 1
    let bundle_resp: PreKeyBundleRespDto = app.assert_success(
        app.get_auth(&format!("/api/keys/{}", username1), &token2)
            .await,
    );

    assert_eq!(bundle_resp.identity_key, "identity_key_base64");
    assert_eq!(bundle_resp.signed_prekey.public_key, "signed_prekey_public");
    assert!(bundle_resp.one_time_prekey.is_some());

    // 4. Verify One-Time Prekey Consumption
    // Fetch again - should get the second key
    let bundle_resp_2: PreKeyBundleRespDto = app.assert_success(
        app.get_auth(&format!("/api/keys/{}", username1), &token2)
            .await,
    );
    assert!(bundle_resp_2.one_time_prekey.is_some());
    assert_ne!(
        bundle_resp.one_time_prekey.unwrap().key_id,
        bundle_resp_2.one_time_prekey.unwrap().key_id
    );

    // Fetch again - should get None for one-time key (exhausted)
    let bundle_resp_3: PreKeyBundleRespDto = app.assert_success(
        app.get_auth(&format!("/api/keys/{}", username1), &token2)
            .await,
    );
    assert!(bundle_resp_3.one_time_prekey.is_none());
}
