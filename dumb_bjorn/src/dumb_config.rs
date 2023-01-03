use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct DumbConfig {
    prefix: String,
    smart_bjorn_file_path: String,
}

impl DumbConfig {
    pub fn smart_bjorn_file_path(&self) -> &String {
        &self.smart_bjorn_file_path
    }

    pub fn prefix(&self) -> &String {
        &self.prefix
    }
}