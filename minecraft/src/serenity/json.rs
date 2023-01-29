use serde::{Deserialize, Serialize};

pub trait Json: Sized {
    type JsonType: Serialize + for<'de> Deserialize<'de>;

    fn new(path: String, data: Self::JsonType) -> Self;
    fn name() -> &'static str;
    fn empty_json_str() -> &'static str;
    fn empty_json() -> Self::JsonType;

    fn load(path: String) -> Self {
        let data = match std::fs::read_to_string(&path) {
            Ok(json) => serde_json::from_str::<Self::JsonType>(&json)
                .expect(format!("Error parsing {} config.", Self::name()).as_str()),
            Err(_) => {
                std::fs::write(&path, Self::empty_json_str())
                    .expect(format!("Error accessing {} config path.", Self::name()).as_str());
                Self::empty_json()
            }
        };

        Self::new(path, data)
    }

    fn save(path: &String, data: &Self::JsonType) {
        let json = serde_json::to_string(data)
            .expect(format!("Error serializing {} config.", Self::name()).as_str());
        std::fs::write(path, json)
            .expect(format!("Error accessing {} config path.", Self::name()).as_str());
    }
}
