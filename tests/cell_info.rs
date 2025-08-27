use a5_rs::core::cell_info::{cell_area, get_num_cells, get_num_cells_bigint};
use num_bigint::BigInt;
use serde::Deserialize;

#[derive(Deserialize)]
struct NumCellsFixture {
    resolution: i32,
    count: u64,
    #[serde(rename = "countBigInt")]
    count_big_int: String,
}

#[derive(Deserialize)]
struct CellAreaFixture {
    resolution: i32,
    #[serde(rename = "areaM2")]
    area_m2: f64,
}

#[derive(Deserialize)]
struct CellInfoFixtures {
    #[serde(rename = "numCells")]
    num_cells: Vec<NumCellsFixture>,
    #[serde(rename = "cellArea")]
    cell_area: Vec<CellAreaFixture>,
}

fn load_cell_info_fixtures() -> CellInfoFixtures {
    let fixture_data = include_str!("../tests/fixtures/cell-info.json");
    serde_json::from_str(fixture_data).expect("Failed to parse cell-info fixtures")
}

#[test]
fn test_get_num_cells() {
    let fixtures = load_cell_info_fixtures();

    for fixture in fixtures.num_cells {
        // Test u64 version
        assert_eq!(
            get_num_cells(fixture.resolution),
            fixture.count,
            "get_num_cells failed for resolution {}",
            fixture.resolution
        );

        // Test BigInt version
        let resolution_bigint = BigInt::from(fixture.resolution);
        assert_eq!(
            get_num_cells_bigint(&resolution_bigint).to_string(),
            fixture.count_big_int,
            "get_num_cells_bigint failed for resolution {}",
            fixture.resolution
        );
    }
}

#[test]
fn test_cell_area() {
    let fixtures = load_cell_info_fixtures();

    for fixture in fixtures.cell_area {
        assert_eq!(
            cell_area(fixture.resolution),
            fixture.area_m2,
            "cell_area failed for resolution {}",
            fixture.resolution
        );
    }
}
