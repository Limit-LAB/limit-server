use crate::{
    auth::{auth_layer::AuthLayer, http_layer::AuthHttpLayer, mock_token},
    test::dummy_services::DummyAuthService,
};
use axum::{routing::get_service, Router};
use hyper::StatusCode;
use tower::Layer;

pub async fn test_auth_http_service() {
    lazy_static::lazy_static! {
        static ref TOKENS_RESULT_CODE_TABLE : Vec<(String, StatusCode)> = vec![
            (mock_token(chrono::Duration::days(1)), StatusCode::OK),
            ("invalid".to_string(), StatusCode::UNAUTHORIZED),
            (mock_token(chrono::Duration::days(-10)), StatusCode::UNAUTHORIZED),
        ];
    }
    tracing::info!("💪 test {} started", module_path!());
    crate::test::intergration::do_with_port(|p| async move {
        tracing::info!("🚀 test {} on port {}", module_path!(), p);
        let app = Router::new().route(
            "/",
            get_service(AuthHttpLayer.layer(AuthLayer.layer(DummyAuthService))),
        );

        let server = tokio::spawn(async move {
            axum::Server::bind(&format!("0.0.0.0:{}", p).parse().unwrap())
                .serve(app.into_make_service())
                .await
                .unwrap();
        });
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;

        // without header
        let server_addr = format!("http://localhost:{}", p);
        let res = tokio::spawn(async move {
            let res = reqwest::Client::new()
                .get(&server_addr.clone())
                .send()
                .await
                .unwrap();
            res
        })
        .await
        .unwrap();
        tracing::info!(
            "received {:?}, expected {:?}",
            res.status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        println!("{:?}", res.text().await);

        // with header
        for (token, result_code) in TOKENS_RESULT_CODE_TABLE.iter() {
            let server_addr = format!("http://localhost:{}", p);
            let res = tokio::spawn(async move {
                let res = reqwest::Client::new()
                    .get(&server_addr.clone())
                    .bearer_auth(token)
                    .send()
                    .await
                    .unwrap();
                res
            })
            .await
            .unwrap();
            tracing::info!(
                "token: {:?}, received {:?}, expected {:?}",
                token,
                res.status(),
                result_code
            );
            assert_eq!(res.status(), result_code.clone());
            println!("{:?}", res.text().await);
        }
        server.abort();
        tracing::info!("🎉test {} finished🎉", module_path!());
    })
    .await
    .await;
}
