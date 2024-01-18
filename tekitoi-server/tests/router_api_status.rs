use axum::body::Body;
use axum::http::{Request, StatusCode};
use similar_asserts::assert_eq;
use tower::ServiceExt;

#[tokio::test]
async fn should_return_no_content() {
    let config = tekitoi_server::Config::default();
    let app = config.build();

    let app = app.into_router();

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/status")
                .method("GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::NO_CONTENT);
}
