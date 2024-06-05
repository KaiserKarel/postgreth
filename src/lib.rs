use pgrx::prelude::*;

pgrx::pg_module_magic!();

pub mod types {
    use pgrx::prelude::*;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, PostgresType)]
    #[repr(transparent)]
    pub struct Bloom(pub alloy_primitives::FixedBytes<256>);

    impl Bloom {
        pub fn into_filter(self) -> alloy_primitives::Bloom {
            alloy_primitives::Bloom::from(self.0)
        }
    }

    #[pg_extern]
    fn covers(left: Bloom, right: Bloom) -> bool {
        left.into_filter().covers(&right.into_filter())
    }

    #[derive(Serialize, Deserialize, PostgresType)]
    #[repr(transparent)]
    pub struct Address(pub alloy_primitives::FixedBytes<20>);

    impl Address {
        fn into_address(self) -> alloy_primitives::Address {
            alloy_primitives::Address::from(self.0)
        }
    }

    #[derive(Serialize, Deserialize, PostgresType)]
    #[repr(transparent)]
    pub struct B256(pub alloy_primitives::aliases::B256);

    #[pg_extern]
    pub fn contains_input(bloom: Bloom, input: &[u8]) -> bool {
        bloom
            .into_filter()
            .contains_input(alloy_primitives::BloomInput::Raw(input))
    }

    #[pg_extern]
    pub fn contains_input_hashed(bloom: Bloom, hash: &[u8]) -> bool {
        bloom
            .into_filter()
            .contains_input(alloy_primitives::BloomInput::Hash(
                hash.try_into()
                    .expect("hash did not have the correct length"),
            ))
    }

    #[pg_extern]
    pub fn m3_2048(bloom: Bloom, bytes: &[u8]) -> Bloom {
        let mut filter = bloom.into_filter();
        filter.m3_2048(bytes);
        Bloom(filter.0)
    }

    #[pg_extern]
    pub fn m3_2048_hashed(bloom: Bloom, hash: &[u8]) -> Bloom {
        let mut filter = bloom.into_filter();
        filter.m3_2048_hashed(
            hash.try_into()
                .expect("hash did not have the correct length"),
        );
        Bloom(filter.0)
    }

    #[pg_extern]
    fn contains_raw_log(bloom: Bloom, address: Address, topics: Vec<B256>) -> bool {
        let topics: Vec<_> = topics.into_iter().map(|t| t.0).collect();
        bloom
            .into_filter()
            .contains_raw_log(address.into_address(), &topics)
    }
}

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

    /// Parse any ethabi encoded object by providing the solidity type.
    ///
    /// ```sql
    /// SELECT item_to_jsonb('(bytes,bytes,(string,uint128)[])', '...'::bytea, true)
    /// ```
    ///
    /// The final argument, `prepend_magic_bytes`, is needed when decoding tuples.
    ///
    /// # Panics
    /// - When the provided `item` is invalid.
    /// - When the solidity type cannot decode the `value`.
    ///
    /// # Common errors
    /// - If `Err` value: SolTypes(Overrun)` is thrown, set prepend_magic_bytes to `true`
    #[pg_extern(immutable, parallel_safe)]
    pub fn item_to_jsonb(item: &str, value: &[u8], prepend_magic_bytes: bool) -> pgrx::JsonB {
        use alloy_dyn_abi::DynSolType;

        let mut v = vec![
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 32,
        ];

        let value = if prepend_magic_bytes {
            v.extend(value);
            &v
        } else {
            value
        };
        let my_type: DynSolType = item.parse().unwrap();
        let decoded = my_type.abi_decode(value).unwrap();
        pgrx::JsonB(alloy_dyn_parser::dyn_sol_to_json(decoded))
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use pgrx::prelude::*;

    #[pg_test]
    fn test_log_to_jsonb() {
        let abi = include_str!("../testdata/erc20.json");
        let log: &str = include_str!("../testdata/log.json");
        let expected = pgrx::JsonB(serde_json::from_str("{\"name\":\"Transfer\",\"data\":{\"from\":\"0x8a02604a33da84F492d161c8C9fc5068f368e352\",\"to\":\"0xA232a12C07681e067B8Da83bFC92A55DA831aD0D\",\"value\":\"100000000000000000000\"}}").expect("parsing json literal should work"));
        assert_eq!(expected.0, crate::parsing::log_to_jsonb(&abi, &log).0);
    }

    #[pg_test]
    fn test_item_to_jsonb() {
        use base64::prelude::*;
        use serde_json::Value;

        let item = crate::parsing::item_to_jsonb(
            "(bytes,bytes,(string,uint128)[])",
            &BASE64_STANDARD.decode("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAGAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAoAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAADgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSEeLN+mD9SDby119Oq2CdrgmMavQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAUqDOwPY7RIoxHkcv6sis+1XlUQp8AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAEAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAqMHg3Nzk4NzdhN2IwZDllODYwMzE2OWRkYmQ3ODM2ZTQ3OGI0NjI0Nzg5AAAAAAAAAAAAAAAAAAAAAAAAAAAAAA==").unwrap(),
            true
        );
        let expected: Value = serde_json::from_str(r#"["hHizfpg/Ug28tdfTqtgna4JjGr0=", "qDOwPY7RIoxHkcv6sis+1XlUQp8=", [["0x779877a7b0d9e8603169ddbd7836e478b4624789", "2"]]]"#).unwrap();
        assert_eq!(item.0, expected)
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
