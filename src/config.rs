use serde::{ Serialize, Deserialize };
use uuid::Uuid;
use std::fs;
use std::io::{ self, ErrorKind, Result as IoResult };

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub uuid: String,
    pub address: String,
}

impl Config {
    pub fn load(config_file: &str) -> IoResult<Self> {
        match fs::read_to_string(config_file) {
            Ok(contents) => {
                // Try to parse the config file
                match serde_json::from_str::<Config>(&contents) {
                    Ok(config) => Ok(config),
                    Err(_) => {
                        // If parsing fails, prompt for a new configuration
                        Config::create_new_config(config_file)
                    }
                }
            }
            Err(ref e) if e.kind() == ErrorKind::NotFound => {
                // If the file does not exist, create a new configuration
                Config::create_new_config(config_file)
            }
            Err(e) => Err(e),
        }
    }

    fn create_new_config(config_file: &str) -> IoResult<Self> {
        let new_uuid = Uuid::new_v4().to_string();
        let new_address = Config::prompt_for_address()?;
        let new_config = Config {
            uuid: new_uuid,
            address: new_address,
        };
        new_config.save(config_file)?;
        Ok(new_config)
    }

    fn save(&self, config_file: &str) -> IoResult<()> {
        let data = serde_json::to_string_pretty(self)?;
        fs::write(config_file, data)?;
        Ok(())
    }

    fn prompt_for_address() -> IoResult<String> {
        println!("Enter the node's address (e.g., 127.0.0.1:8080): ");
        let mut input_address = String::new();
        io::stdin().read_line(&mut input_address)?;
        Ok(input_address.trim().to_string())
    }
}
