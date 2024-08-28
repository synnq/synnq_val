use serde::{ Serialize, Deserialize };
use uuid::Uuid;
use std::fs;
use std::io::ErrorKind;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub uuid: String,
}

impl Config {
    pub fn load(config_file: &str) -> Result<Self, std::io::Error> {
        match fs::read_to_string(config_file) {
            Ok(contents) =>
                serde_json
                    ::from_str(&contents)
                    .map_err(|e| std::io::Error::new(ErrorKind::InvalidData, e)),
            Err(ref e) if e.kind() == ErrorKind::NotFound => {
                // Generate a new UUID if the config file does not exist
                let new_config = Config {
                    uuid: Uuid::new_v4().to_string(),
                };
                new_config.save(config_file)?;
                Ok(new_config)
            }
            Err(e) => Err(e),
        }
    }

    fn save(&self, config_file: &str) -> Result<(), std::io::Error> {
        let data = serde_json::to_string_pretty(self)?;
        fs::write(config_file, data)?;
        Ok(())
    }
}
