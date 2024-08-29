use serde::{ Serialize, Deserialize };
use uuid::Uuid;
use std::fs;
use std::io::{ self, ErrorKind };

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub uuid: String,
    pub address: String,
}

impl Config {
    pub fn load(config_file: &str) -> Result<Self, io::Error> {
        match fs::read_to_string(config_file) {
            Ok(contents) => {
                serde_json
                    ::from_str(&contents)
                    .map_err(|e| io::Error::new(ErrorKind::InvalidData, e))
            }
            Err(ref e) if e.kind() == ErrorKind::NotFound => {
                // Generate a new UUID and prompt for an address if the config file does not exist
                let new_uuid = Uuid::new_v4().to_string();
                let new_address = Config::prompt_for_address()?;
                let new_config = Config {
                    uuid: new_uuid,
                    address: new_address,
                };
                new_config.save(config_file)?;
                Ok(new_config)
            }
            Err(e) => Err(e),
        }
    }

    fn save(&self, config_file: &str) -> Result<(), io::Error> {
        let data = serde_json::to_string_pretty(self)?;
        fs::write(config_file, data)?;
        Ok(())
    }

    fn prompt_for_address() -> Result<String, io::Error> {
        // Prompt the user to input the node's address
        println!("Enter the node's address (e.g., 127.0.0.1:8080): ");
        let mut input_address = String::new();
        io::stdin().read_line(&mut input_address)?;
        Ok(input_address.trim().to_string())
    }
}
