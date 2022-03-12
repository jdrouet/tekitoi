pub mod authorize;
pub mod error;

#[cfg(test)]
mod tests {
    use crate::tests::TestServer;
    use actix_web::http::StatusCode;

    #[actix_web::test]
    async fn get_favicon() {
        let req = actix_web::test::TestRequest::get()
            .uri("/favicon.svg")
            .to_request();
        let srv = TestServer::from_simple();
        let res = srv.execute(req).await;
        assert_eq!(res.status(), StatusCode::OK);
    }
}
