use pgrx::prelude::*;

pgrx::pg_module_magic!();

pub mod parsing {
    use alloy_json_abi::JsonAbi;
    use ethers::types::Log;
    use pgrx::prelude::*;

    /// Parse an ethabi encoded log [`ethers::types::Log`] into self-describing JSONB.
    ///
    /// # Panics
    /// - On any internal error encountered. It is expected that `pgrx` gracefully handles these.
    #[pg_extern(immutable, parallel_safe)]
    pub fn log_to_jsonb(input: &str, value: &str) -> pgrx::JsonB {
        let abi: JsonAbi = serde_json::from_str(input).expect("deserializing json abi failed");
        let parser = alloy_dyn_parser::Parser::new(&abi);
        let value: Log = serde_json::from_str(value).expect("could not parse into log");
        let result = parser
            .parse(&value)
            .expect("could not parse log into keyed data");
        pgrx::JsonB(serde_json::to_value(result).expect("could not convert keyed events to json"))
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use pgrx::prelude::*;

    #[pg_test]
    fn test_hello_postgreth() {
        let abi = include_str!("../testdata/erc20.json");
        let log: &str = include_str!("../testdata/log.json");
        let expected = pgrx::JsonB(serde_json::from_str("{\"name\":\"Transfer\",\"data\":{\"from\":\"0x8a02604a33da84F492d161c8C9fc5068f368e352\",\"to\":\"0xA232a12C07681e067B8Da83bFC92A55DA831aD0D\",\"value\":\"100000000000000000000\"}}").unwrap());
        assert_eq!(expected.0, crate::parsing::log_to_jsonb(&abi, &log).0);
    }
}

/// This module is required by `cargo pgrx test` invocations.
/// It must be visible at the root of your extension crate.
#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {
        // perform one-off initialization when the pg_test framework starts
    }

    pub fn postgresql_conf_options() -> Vec<&'static str> {
        // return any postgresql.conf settings that are required for your tests
        vec![]
    }
}
