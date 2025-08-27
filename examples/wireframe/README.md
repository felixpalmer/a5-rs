# A5 Wireframe Example

This example generates A5 cell boundaries at a given resolution and outputs them as a GeoJSON FeatureCollection. This replicates the functionality of the Python wireframe example found in `git/a5-py/examples/wireframe/index.py`.

## Usage

```bash
cargo run --example wireframe <resolution> <output.json>
```

Where:
- `resolution`: A5 cell resolution (integer)
- `output.json`: Path to the output GeoJSON file

## Examples

Generate all cells at resolution 0 (12 cells covering the entire world):
```bash
cargo run --example wireframe 0 world-res0.json
```

Generate all cells at resolution 1 (60 cells):
```bash
cargo run --example wireframe 1 world-res1.json
```

Generate all cells at resolution 2 (240 cells):
```bash
cargo run --example wireframe 2 world-res2.json
```

## Output Format

The output is a GeoJSON FeatureCollection where each feature represents an A5 cell with:
- Polygon geometry containing the cell boundary coordinates
- Properties including the hexadecimal cell ID

## Testing

Run the example tests:
```bash
cargo test --example wireframe
```

## Verification

To verify the port is working correctly, you can compare the output with the Python version:

```bash
# Generate with Python (from a5-py directory)
cd /Users/work/git/a5-py/examples/wireframe
python3 index.py 0 python-output.json

# Generate with Rust
cd /Users/work/git/a5-rs
cargo run --example wireframe 0 rust-output.json

# Compare cell counts and cell IDs
```

Both implementations should generate the same number of cells with the same cell IDs at each resolution.