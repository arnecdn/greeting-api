use config::Config;
use dotenv::dotenv;
use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct Settings {
    pub db: Db,
    pub otel_collector: OtelCollector,
    pub kube: Kube
}

impl Settings {
    pub fn new() -> Self {
        dotenv().ok();

        let settings = Config::builder()
            .add_source(config::File::with_name("./res/server").required(false))
            .add_source(config::Environment::with_prefix("APP").separator("__"))
            .build()
            .unwrap();

        settings.try_deserialize().unwrap()
    }
}


#[derive(Deserialize)]
pub struct Db{
    pub database_url: String
}

#[derive(Deserialize)]
pub (crate) struct OtelCollector{
    pub (crate) oltp_endpoint: String
}

#[derive(Deserialize)]
pub (crate) struct Kube{
    pub (crate) my_pod_name: String
}