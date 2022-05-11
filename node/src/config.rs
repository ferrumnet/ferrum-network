use std::{fs::File, io::BufReader, path::Path};

use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub initial_authourity_seed_list: Vec<String>,
    pub root_seed: String,
    pub endowed_accounts_seed_list: Vec<String>,
    pub address_list: Vec<String>,
}

pub fn read_config_from_file<P: AsRef<Path>>(path: P) -> Result<Config, String> {
    match File::open(path){
        Ok(file ) => {
            let reader = BufReader::new(file);

            match serde_json::from_reader(reader) {
                Ok(config) => return Ok(config),
                Err(err) => return Err(err.to_string())
            }
        },
        Err(err) => Err(err.to_string())
    }
}
