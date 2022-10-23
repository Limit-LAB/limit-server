pub mod auth;
pub mod user;

use crossbeam_channel::{Receiver, Sender};
use diesel::{r2d2::ConnectionManager, SqliteConnection};
use lazy_static::lazy_static;
use r2d2::Pool;
use std::io::Read;
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
fn integration_tests() {
    tracing_subscriber::fmt::init();
    tracing::info!("⚠integration tests started⚠");
    crate::config::mock::mock();
    let manager = ConnectionManager::<SqliteConnection>::new("test.sqlite");
    let pool = Pool::builder()
        .test_on_check_out(true)
        .build(manager)
        .expect("Could not build connection pool");
    tokio_run!(async move {
        let tasks = vec![
            // tokio::spawn(auth::http_layer::test_auth_http_service()),
            tokio::spawn(user::services::test_verify_and_auth_user(pool.clone())),
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

lazy_static! {
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

pub async fn do_with_port<F, T>(f: F) -> T
where
    F: FnOnce(u16) -> T,
{
    let port = get_available_port().await;
    let res = f(port);
    AVAILABLE_PORTS_CHANNEL.0.clone().send(port).unwrap();
    res
}
