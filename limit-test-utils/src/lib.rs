use limit_deps::*;

use crossbeam_channel::{Receiver, Sender};
use once_cell::sync::Lazy;

pub fn mock_config() -> limit_config::Config {
    use limit_config::*;
    use limit_server_auth::{JWTClaim, JWTSub};
    use uuid::Uuid;

    GLOBAL_CONFIG
        .get_or_init(|| {
            let (server_secret_key, server_public_key) = limit_am::create_random_secret().unwrap();
            Config {
                url: "127.0.0.1:1313".to_string(),
                database: Database::Sqlite {
                    path: "test.sqlite".to_string(),
                },
                jwt_secret: "mock".to_string(),
                database_pool_thread_count: 3,
                admin_jwt: jsonwebtoken::encode(
                    &jsonwebtoken::Header::default(),
                    &JWTClaim::new(
                        JWTSub {
                            id: Uuid::new_v4(),
                            device_id: Uuid::new_v4().to_string(),
                        },
                        chrono::Duration::days(1),
                    ),
                    &jsonwebtoken::EncodingKey::from_secret("mock_admin".as_bytes()),
                )
                .unwrap(),
                metrics: Metrics::Terminal,
                server_secret_key,
                server_public_key,
                per_user_message_on_the_fly_limit: 100,
            }
        })
        .clone()
}

pub static AVAILABLE_PORTS_CHANNEL: Lazy<(Sender<u16>, Receiver<u16>)> = Lazy::new(|| {
    let conf_str = std::fs::read_to_string("integration_test_conf.toml").unwrap();
    let conf: toml::Value = toml::from_str(&conf_str).unwrap();
    let ports = conf["ports"]["available"]
        .as_str()
        .unwrap()
        .split('-')
        .map(|number| number.parse::<u16>().unwrap())
        .collect::<Vec<u16>>();
    let (s, r) = crossbeam_channel::bounded((ports[1] - ports[0]) as _);
    (ports[0]..ports[1]).for_each(|p| s.send(p).unwrap());
    (s, r)
});

pub async fn get_available_port() -> u16 {
    let r = AVAILABLE_PORTS_CHANNEL.1.clone();
    loop {
        if let Ok(p) = r.try_recv() {
            return p;
        } else {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        }
    }
}

#[macro_export]
macro_rules! test_tasks {
    ($port: expr, $($task:expr),+ $(,)?) => {
        vec![$(Box::pin($task($port)) as Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send>>),+];
    }
}

#[macro_export]
macro_rules! init_test_client {
    ($client_type:ty, $client_builder:ty) => {
        use limit_test_utils::get_available_port;
        use std::net::SocketAddr;

        let addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
        <$client_builder>::new(format!("{}", module_path!()))
            .address(addr)
            .build()
    };
}

#[macro_export]
macro_rules! test_service {
    ($port: expr, $server: expr, $tasks: expr) => {
        use limit_test_utils::get_available_port;
        use std::net::SocketAddr;
        tracing::info!("💪 test {} started", module_path!());

        tracing::info!("🚀 test {} on port {}", module_path!(), $port);
        let addr: SocketAddr = format!("127.0.0.1:{}", $port).parse().unwrap();
        let addr = addr.into();
        let server = tokio::spawn(async move {
            let server = $server.serve(addr).await.unwrap();
        });
        tokio::time::sleep(std::time::Duration::from_millis(3000)).await;
        futures::future::join_all($tasks)
            .await
            .into_iter()
            .for_each(|r| {
                if let Err(e) = r {
                    tracing::error!("💥integration test failed💥: {}", e);
                    panic!("💥integration test failed💥: {}", e);
                }
            });
        server.abort();

        tracing::info!("🎉test {} finished🎉", module_path!());
    };
}

pub async fn do_with_port<F, T: Send>(f: F) -> T
where
    F: FnOnce(u16) -> T,
{
    let port = get_available_port().await;
    let res = f(port);
    AVAILABLE_PORTS_CHANNEL.0.clone().send(port).unwrap();
    res
}

#[macro_export]
macro_rules! do_with_port_m {
    ($f: expr) => {{
        use limit_test_utils::get_available_port;
        use limit_test_utils::AVAILABLE_PORTS_CHANNEL;
        let port = get_available_port().await;
        let res = $f(port);
        AVAILABLE_PORTS_CHANNEL.0.clone().send(port).unwrap();
        res
    }};
}
