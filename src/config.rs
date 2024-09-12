use serde::{ Serialize, Deserialize };
use uuid::Uuid;
use std::fs;
use std::io::{ self, ErrorKind, Result as IoResult };

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub uuid: String,
    pub address: String,
    pub wallet_address: Option<String>,
}

impl Config {
    /// Load the configuration from the file or create a new one if it doesn't exist
    pub fn load(config_file: &str) -> IoResult<Self> {
        match fs::read_to_string(config_file) {
            Ok(contents) => {
                // Try to parse the config file
                match serde_json::from_str::<Config>(&contents) {
                    Ok(mut config) => {
                        // Check if wallet_address is missing and prompt if needed
                        if config.wallet_address.is_none() {
                            config.wallet_address = Some(Config::prompt_for_wallet_address()?);
                            config.save(config_file)?; // Save the updated config with the wallet address
                        }
                        Ok(config)
                    }
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

    /// Create a new configuration file
    fn create_new_config(config_file: &str) -> IoResult<Self> {
        let new_uuid = Uuid::new_v4().to_string();
        let new_address = Config::prompt_for_address()?;
        let new_wallet_address = Config::prompt_for_wallet_address()?;

        let new_config = Config {
            uuid: new_uuid,
            address: new_address,
            wallet_address: Some(new_wallet_address),
        };

        new_config.save(config_file)?; // Save the newly created config
        Ok(new_config)
    }

    /// Save the current configuration to a file
    pub fn save(&self, filename: &str) -> IoResult<()> {
        let config_data = serde_json
            ::to_string_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?; // Convert serde_json error to io::Error
        fs::write(filename, config_data)?;
        Ok(())
    }

    /// Prompt the user for the node's address
    fn prompt_for_address() -> IoResult<String> {
        println!("Enter the node's address (e.g., 127.0.0.1:8080): ");
        let mut input_address = String::new();
        io::stdin().read_line(&mut input_address)?;
        Ok(input_address.trim().to_string())
    }

    /// Prompt the user for the wallet address
    pub fn prompt_for_wallet_address() -> IoResult<String> {
        println!("Enter the wallet address: ");
        let mut wallet_address = String::new();
        io::stdin().read_line(&mut wallet_address)?;
        Ok(wallet_address.trim().to_string())
    }
}
