use crate::config::{Config, get_config, initialize_config, resolve_config_path};
use crate::mure_error::Error;

pub fn init() -> Result<Config, Error> {
    let config = initialize_config()?;
    Ok(config)
}

pub fn get_config_or_initialize() -> Result<Config, Error> {
    match get_config() {
        Ok(config) => Ok(config),
        Err(get_err) => {
            // Only auto-initialize when the config file is absent.
            // If the file exists but is unreadable or contains invalid TOML,
            // surface the original error instead of masking it with
            // "config file already exists".
            let config_exists = resolve_config_path().map(|p| p.exists()).unwrap_or(false);
            if config_exists {
                return Err(get_err);
            }
            init()
        }
    }
}
