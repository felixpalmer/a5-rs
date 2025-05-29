use crate::coordinate_systems::Radians;

#[test]
fn test_coord_primitives() {
    assert_eq!(Radians::new_unchecked(1.23).0, 1.23)
}
