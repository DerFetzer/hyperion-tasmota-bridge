use config::{Config, ConfigError, Environment, File};

#[derive(Debug, Deserialize, Clone)]
pub struct Wled {
    pub url: String,
    pub mappings: Vec<LedMapping>,
    pub number_of_leds: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LedMapping {
    pub source_start: u16,
    pub target_start: u16,
    pub length: Option<u16>,
    pub reverse: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub udp_bind_address: String,
    pub receive_buffer_size: Option<u32>,
    pub wleds: Vec<Wled>,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let s = Config::builder()
            .add_source(File::with_name("config.yml").required(false))
            .add_source(Environment::with_prefix("htb"))
            .build()?;

        let mut res: Settings = s.try_deserialize()?;

        for wled in res.wleds.iter_mut() {
            Settings::process_mappings(&mut wled.mappings)?;
        }

        Ok(res)
    }

    fn process_mappings(mappings: &mut [LedMapping]) -> Result<(), ConfigError> {
        if mappings.is_empty() {
            return Err(ConfigError::Message(
                "There has to be at least one mapping per device!".to_string(),
            ));
        }

        mappings.sort_by(|a, b| a.target_start.cmp(&b.target_start));
        // Check for overlapping target ranges
        let f = mappings.iter().fold(Some(0), |i, t| {
            i.and_then(|i| {
                if i > t.target_start {
                    None
                } else {
                    Some(t.target_start + t.length.unwrap_or(1))
                }
            })
        });

        if f.is_none() {
            return Err(ConfigError::Message(
                "Overlapping target ranges are not allowed!".to_string(),
            ));
        }

        Ok(())
    }
}
