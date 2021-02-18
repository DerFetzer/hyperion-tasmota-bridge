use config::{Config, ConfigError, Environment, File};

#[derive(Debug, Deserialize)]
pub struct Mqtt {
    pub url: String,
    pub client_id: String,
    pub user: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Tasmota {
    pub mqtt_prefix: String,
    pub mappings: Vec<LedMapping>,
}

#[derive(Debug, Deserialize)]
pub struct LedMapping {
    pub source_start: u16,
    pub target_start: u16,
    pub length: Option<u16>,
    pub reverse: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub udp_bind_address: String,
    pub mqtt: Mqtt,
    pub receive_buffer_size: Option<u32>,
    pub tasmotas: Vec<Tasmota>,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = Config::new();

        s.merge(File::with_name("config.yml").required(false))?;
        s.merge(Environment::with_prefix("htb"))?;

        let mut res: Settings = s.try_into()?;

        for tasmota in res.tasmotas.iter_mut() {
            tasmota.mappings.sort_by(|a, b| a.target_start.cmp(&b.target_start));
            // Check for overlapping target ranges
            let f = tasmota.mappings.iter().fold(0_i32, |i, t| {
                if i == -1 || i > t.target_start as i32 {
                    -1
                }
                else {
                    (t.target_start + t.length.unwrap_or(1)) as i32
                }
            });

            if f == -1 {
                return Err(ConfigError::Message("Overlapping target ranges are not allowed!".to_string()));
            }
        }

        Ok(res)
    }
}
