// A5
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) A5 contributors

use a5::core::hex::hex_to_u64;
use a5::core::migrate::migrate;
use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
struct MigrateTestCase {
    v0: String,
    v1: String,
}

#[test]
fn test_migrate_v0_to_v1() {
    let data = fs::read_to_string("tests/fixtures/migrate.json").expect("Failed to read fixtures");
    let fixtures: Vec<MigrateTestCase> =
        serde_json::from_str(&data).expect("Failed to parse fixtures");
    for fixture in fixtures {
        let v0 = hex_to_u64(&fixture.v0).unwrap();
        let v1 = hex_to_u64(&fixture.v1).unwrap();
        assert_eq!(migrate(v0).unwrap(), v1, "migrate({})", fixture.v0);
    }
}
