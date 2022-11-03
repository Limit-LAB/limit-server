use crossbeam_channel::{Receiver, Sender};
use std::io::Read;

pub fn mock_config() -> limit_config::Config {
    use limit_config::*;
    use limit_server_auth::JWTClaim;
    use uuid::Uuid;

    GLOBAL_CONFIG
        .get_or_init(|| {
            let (server_secret_key, server_public_key) = limit_am::create_random_secret().unwrap();
            Config {
                database: Database::Sqlite {
                    path: "test.sqlite".to_string(),
                },
                jwt_secret: "mock".to_string(),
                database_pool_thread_count: 3,
                admin_jwt: jsonwebtoken::encode(
                    &jsonwebtoken::Header::default(),
                    &JWTClaim::new(Uuid::new_v4(), chrono::Duration::days(1)),
                    &jsonwebtoken::EncodingKey::from_secret("mock_admin".as_bytes()),
                )
                .unwrap(),
                metrics: Metrics::Terminal,
                server_secret_key,
                server_public_key,
            }
        })
        .clone()
}

lazy_static::lazy_static! {
    pub static ref AVAILABLE_PORTS_CHANNEL: (Sender<u16>, Receiver<u16>) = {
        let mut conf_file = std::fs::File::open("integration_test_conf.toml").unwrap();
        let mut conf_str = String::new();
        conf_file.read_to_string(&mut conf_str).unwrap();
        let conf: toml::Value = toml::from_str(&conf_str).unwrap();
        let ports = conf["ports"]["available"]
            .as_str()
            .unwrap()
            .split('-')
            .map(|number| number.parse::<u16>().unwrap())
            .collect::<Vec<u16>>();
        let (s, r) = crossbeam_channel::bounded((ports[1] - ports[0]) as _);
        (ports[0]..ports[1]).for_each(|p| s.clone().send(p).unwrap());
        (s, r)
    };
}

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
        vec![$(Box::pin($task($port)) as Pin<Box<dyn Future<Output = _> + Send>>),+];
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
        let addr = volo::net::Address::from(addr);
        let server = tokio::spawn(async move {
            let server = $server.run(addr).await.unwrap();
        });
        tokio::time::sleep(std::time::Duration::from_millis(3000)).await;
        futures::future::join_all($tasks).await;
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
