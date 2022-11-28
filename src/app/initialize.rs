use crate::config::{get_config, initialize_config, Config};
use crate::mure_error::Error;

pub fn init() -> Result<Config, Error> {
    let config = initialize_config()?;
    Ok(config)
}

pub fn get_config_or_initialize() -> Result<Config, Error> {
    match get_config() {
        Ok(config) => Ok(config),
        Err(_) => match init() {
            // if not found, create config
            // TODO: with dialog
            // TODO: care other error cases
            Ok(config) => Ok(config),
            Err(e) => Err(e),
        },
    }
}
