pub mod auth;

use lazy_static::lazy_static;
use std::io::Read;
use std::sync::{Arc, Mutex};
lazy_static! {
    pub static ref AVAILABLE_PORTS: Arc<Mutex<Vec<u16>>> = Arc::new(Mutex::new({
        let mut conf_file = std::fs::File::open("intergration_test_conf.toml").unwrap();
        let mut conf_str = String::new();
        conf_file.read_to_string(&mut conf_str).unwrap();
        let conf: toml::Value = toml::from_str(&conf_str).unwrap();
        let ports = conf["ports"]["available"]
            .as_str()
            .unwrap()
            .split('-')
            .map(|number| number.parse::<u16>().unwrap())
            .collect::<Vec<u16>>();
        (ports[0]..ports[1]).collect()
    }));
}

pub async fn get_available_port() -> u16 {
    loop {
        if let Some(p) = AVAILABLE_PORTS.clone().lock().unwrap().pop() {
            break p;
        } else {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            continue;
        }
    }
}

pub async fn do_with_port<F, T>(f: F) -> T
where
    F: FnOnce(u16) -> T,
{
    let port = get_available_port().await;
    let res = f(port);
    AVAILABLE_PORTS.clone().lock().unwrap().push(port);
    res
}
