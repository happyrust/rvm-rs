use rvm_rs::store::geometry::{Cylinder, GeometryKind};
use rvm_rs::store::{Attribute, GroupNode, NodeKind, Store};
use rvm_rs::visitor::colorizer::Colorizer;
use rvm_rs::visitor::traverse;

fn build_group_with_geometry(
    store: &mut Store,
    name: &str,
    material: u32,
    attrs: Vec<(&str, &str)>,
) -> rvm_rs::store::geometry::GeometryId {
    let attributes = attrs
        .into_iter()
        .map(|(key, value)| Attribute {
            key: store.intern_string(key),
            value: store.intern_string(value),
        })
        .collect::<Vec<_>>();

    let group_node = GroupNode {
        name: store.intern_string(name),
        translation: glam::Vec3::ZERO,
        material,
        transparency: 0,
        id: -1,
        bbox_world: rvm_rs::math::BBox3::new(),
        first_geometry: None,
        attributes,
    };
    let group_id = store.new_node(NodeKind::Group(group_node));

    store.new_geometry(
        group_id,
        GeometryKind::Cylinder(Cylinder {
            radius: 1.0,
            height: 1.0,
        }),
    )
}

#[test]
fn test_unknown_material_uses_default() {
    let mut store = Store::new();
    let geo_id = build_group_with_geometry(&mut store, "G0", 9999, Vec::new());

    let mut visitor = Colorizer::new(None);
    traverse(&mut store, &mut visitor);

    let geo = store.get_geometry(geo_id).unwrap();
    assert_eq!(geo.color, 0x787878);
    assert_eq!(geo.color_rgb, 0x787878);
    assert_eq!(store.get_string(geo.color_name.unwrap()), "Default");
}

#[test]
fn test_unknown_color_attribute_keeps_material_color() {
    let mut store = Store::new();
    let geo_id = build_group_with_geometry(&mut store, "G0", 2, vec![("Color", "NoSuchColor")]);

    let mut visitor = Colorizer::new(Some("Color"));
    traverse(&mut store, &mut visitor);

    let geo = store.get_geometry(geo_id).unwrap();
    assert_eq!(geo.color, 0xcc0000);
    assert_eq!(geo.color_rgb, 0xcc0000);
    assert_eq!(store.get_string(geo.color_name.unwrap()), "Red");
}

#[test]
fn test_default_color_application() {
    let mut store = Store::new();
    let geo_id = build_group_with_geometry(&mut store, "G0", 0, Vec::new());

    let mut visitor = Colorizer::new(None);
    traverse(&mut store, &mut visitor);

    let geo = store.get_geometry(geo_id).unwrap();
    assert_eq!(geo.color, 0x787878);
    assert_eq!(geo.color_rgb, 0x787878);
    assert_eq!(store.get_string(geo.color_name.unwrap()), "Default");
}
