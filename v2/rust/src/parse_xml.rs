use chrono::NaiveDateTime;
use serde::de::Error;
use serde::{Deserialize, Deserializer};

#[derive(Deserialize)]
#[serde(rename_all = "UPPERCASE")]
enum Outcome {
    Fail,
    Pass,
    Skip,
    #[serde(rename = "NOT RUN")]
    NotRun,
}

fn parse_datetime<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = Deserialize::deserialize(deserializer)?;
    NaiveDateTime::parse_from_str(raw, "%Y%m%d %H:%M:%S.%f")
        .map_err(|_| Error::custom(format!("Invalid datetime: {}", raw)))
}

#[derive(Deserialize)]
struct Rebot {
    #[serde(rename = "@generator")]
    generator: String,
    #[serde(rename = "@generated", deserialize_with = "parse_datetime")]
    generated: NaiveDateTime,
    #[serde(rename = "@rpa")]
    rpa: bool,
    #[serde(rename = "@schemaversion")]
    schemaversion: usize,
    suite: Suite,
    errors: Errors,
}

#[derive(Deserialize)]
struct Suite {
    #[serde(rename = "@id")]
    id: String,
    #[serde(rename = "@name")]
    name: String,
    #[serde(default)]
    suite: Vec<Suite>,
    #[serde(default)]
    test: Vec<Test>,
}

#[derive(Deserialize)]
struct Status {
    #[serde(rename = "@status")]
    status: Outcome,
    #[serde(rename = "@starttime", deserialize_with = "parse_datetime")]
    starttime: NaiveDateTime,
    #[serde(rename = "@endtime", deserialize_with = "parse_datetime")]
    endtime: NaiveDateTime,
}

#[derive(Deserialize)]
struct Test {
    #[serde(rename = "@id")]
    id: String,
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "@line")]
    line: usize,
    status: Status,
}

#[derive(Deserialize)]
struct Errors {}

#[cfg(test)]
mod tests {
    use super::*;
    use quick_xml::de::from_str;
    use std::fs;

    #[test]
    fn test_parse_rebot() {
        let test_dir = format!("{}/../tests/rebot.xml", env!("CARGO_MANIFEST_DIR"));
        let xml = fs::read_to_string(&test_dir)
            .unwrap_or_else(|_| panic!("Missing test data! {}", test_dir));
        let rebot: Rebot = from_str(&xml).unwrap();
        assert_eq!(rebot.generator, "Rebot 6.1.1 (Python 3.11.4 on win32)");
        assert!(!rebot.rpa);
        assert_eq!(rebot.suite.suite.len(), 2);
    }
}
