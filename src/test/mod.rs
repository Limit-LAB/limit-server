use limit_deps::*;

use limit_test_utils::mock_config;

macro_rules! tokio_run {
    ($e:expr) => {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on($e)
    };
}

#[test]
fn integration_test() {
    tracing_subscriber::fmt::init();
    tracing::info!("⚠integration tests started⚠");

    mock_config();

    tokio_run!(async {
        let tasks = vec![
            // tokio::spawn(limit_server_auth_test::integration_test()),
            tokio::spawn(limit_server_event_test::integration_test()),
        ];
        futures::future::join_all(tasks).await
    })
    .into_iter()
    .for_each(|r| {
        if let Err(e) = r {
            tracing::error!("💥integration test failed💥: {}", e);
            panic!("💥integration test failed💥: {}", e);
        }
    });
    tracing::info!("🎉integration tests finished🎉");
}
