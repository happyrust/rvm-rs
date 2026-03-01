use proptest::prelude::*;
use rvm_rs::store::geometry::{Cylinder, GeometryKind};
use rvm_rs::store::{Attribute, GroupNode, NodeId, NodeKind, Store};
use rvm_rs::visitor::colorizer::Colorizer;
use rvm_rs::visitor::traverse;

const MATERIAL_COLOR_NAMES: &[(u32, &str)] = &[
    (1, "Black"),
    (2, "Red"),
    (3, "Orange"),
    (4, "Yellow"),
    (5, "Green"),
    (6, "Cyan"),
    (7, "Blue"),
    (8, "Magenta"),
    (9, "Brown"),
    (10, "White"),
    (11, "Salmon"),
    (12, "LightGrey"),
    (13, "Grey"),
    (14, "Plum"),
    (15, "WhiteSmoke"),
    (16, "Maroon"),
    (17, "SpringGreen"),
    (18, "Wheat"),
    (19, "Gold"),
    (20, "RoyalBlue"),
    (21, "LightGold"),
    (22, "DeepPink"),
    (23, "ForestGreen"),
    (24, "BrightOrange"),
    (25, "Ivory"),
    (26, "Chocolate"),
    (27, "SteelBlue"),
    (28, "White"),
    (29, "Midnight"),
    (30, "NavyBlue"),
    (31, "Pink"),
    (32, "CoralRed"),
    (33, "Black"),
    (34, "Red"),
    (35, "Orange"),
    (36, "Yellow"),
    (37, "Green"),
    (38, "Cyan"),
    (39, "Blue"),
    (40, "Magenta"),
    (41, "Brown"),
    (42, "White"),
    (43, "Salmon"),
    (44, "LightGrey"),
    (45, "Grey"),
    (46, "Plum"),
    (47, "WhiteSmoke"),
    (48, "Maroon"),
    (49, "SpringGreen"),
    (50, "Wheat"),
    (51, "Gold"),
    (52, "RoyalBlue"),
    (53, "LightGold"),
    (54, "DeepPink"),
    (55, "ForestGreen"),
    (56, "BrightOrange"),
    (57, "Ivory"),
    (58, "Chocolate"),
    (59, "SteelBlue"),
    (60, "White"),
    (61, "Midnight"),
    (62, "NavyBlue"),
    (63, "Pink"),
    (64, "CoralRed"),
    (206, "Black"),
    (207, "White"),
    (208, "Grey"),
    (209, "LightGrey"),
    (210, "DarkGrey"),
    (211, "Beige"),
    (212, "Wheat"),
    (213, "Tan"),
    (214, "SandyBrown"),
    (215, "Brown"),
    (216, "DarkBrown"),
    (217, "Tomato"),
    (218, "OrangeRed"),
    (219, "Orange"),
    (220, "BrightOrange"),
    (221, "Red"),
    (222, "BrightRed"),
    (223, "DeepPink"),
    (224, "Pink"),
    (225, "Plum"),
    (226, "Yellow"),
    (227, "Gold"),
    (228, "LightYellow"),
    (229, "LightGold"),
    (230, "YellowGreen"),
    (231, "SpringGreen"),
    (232, "Green"),
    (233, "ForestGreen"),
    (234, "DarkGreen"),
    (235, "Cyan"),
    (236, "Turquoise"),
    (237, "Aquamarine"),
    (238, "Blue"),
    (239, "RoyalBlue"),
    (240, "NavyBlue"),
    (241, "PowderBlue"),
    (242, "Midnight"),
    (243, "SteelBlue"),
    (244, "Indigo"),
    (245, "Mauve"),
    (246, "Violet"),
    (247, "Magenta"),
    (248, "Beige"),
    (249, "Wheat"),
    (250, "Tan"),
    (251, "SandyBrown"),
    (252, "Brown"),
    (253, "Khaki"),
    (254, "Chocolate"),
    (255, "DarkBrown"),
];

const COLOR_NAME_RGB: &[(&str, u32)] = &[
    ("Blue", 0x0000cc),
    ("Pink", 0xcc919e),
    ("SteelBlue", 0x4782b5),
    ("SandyBrown", 0xf4a55e),
    ("Black", 0x000000),
    ("DarkGrey", 0x518c8c),
    ("RoyalBlue", 0x4775ff),
    ("White", 0xffffff),
    ("Brown", 0xcc2b2b),
    ("Ivory", 0xedede0),
    ("DarkGreen", 0x2d4f2d),
    ("Salmon", 0xf97f70),
    ("BrightOrange", 0xffa500),
    ("Chocolate", 0xed7521),
    ("BrightRed", 0xff0000),
    ("Plum", 0x8c668c),
    ("ForestGreen", 0x238e23),
    ("LightGold", 0xede8aa),
    ("CoralRed", 0xcc5b44),
    ("Indigo", 0x330066),
    ("BlueGrey", 0x687c93),
    ("Gold", 0xedc933),
    ("LightYellow", 0xededd1),
    ("PowderBlue", 0xafe0e5),
    ("LightGrey", 0xbfbfbf),
    ("Yellow", 0xcccc00),
    ("DarkBrown", 0x8c4414),
    ("DeepPink", 0xed1189),
    ("Mauve", 0x660099),
    ("Magenta", 0xdd00dd),
    ("Tomato", 0xff6347),
    ("Midnight", 0x2d2d4f),
    ("Orange", 0xed9900),
    ("YellowGreen", 0x99cc33),
    ("Aquamarine", 0x75edc6),
    ("DarkSlate", 0x2d4f4f),
    ("Red", 0xcc0000),
    ("Khaki", 0x9e9e5e),
    ("Wheat", 0xf4ddb2),
    ("Cyan", 0x00eded),
    ("Turquoise", 0x00bfcc),
    ("SpringGreen", 0x00ff7f),
    ("Grey", 0xa8a8a8),
    ("Green", 0x00cc00),
    ("Beige", 0xf4f4db),
    ("OrangeRed", 0xff7f00),
    ("Tan", 0xdb9370),
    ("WhiteSmoke", 0xf4f4f4),
    ("Maroon", 0x8e236b),
    ("NavyBlue", 0x00007f),
    ("Violet", 0xed82ed),
    ("Default", 0x787878),
];

fn color_rgb_by_name(name: &str) -> u32 {
    let key = name.to_ascii_lowercase();
    for (candidate, rgb) in COLOR_NAME_RGB {
        if candidate.to_ascii_lowercase() == key {
            return *rgb;
        }
    }
    panic!("unknown color name: {}", name);
}

fn build_group_with_geometry(
    store: &mut Store,
    name: &str,
    material: u32,
    attrs: Vec<(&str, &str)>,
) -> (NodeId, rvm_rs::store::geometry::GeometryId) {
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

    let geo_id = store.new_geometry(
        group_id,
        GeometryKind::Cylinder(Cylinder {
            radius: 1.0,
            height: 1.0,
        }),
    );

    (group_id, geo_id)
}

fn material_strategy() -> impl Strategy<Value = (u32, &'static str)> {
    prop::sample::select(MATERIAL_COLOR_NAMES.to_vec())
}

fn color_name_strategy() -> impl Strategy<Value = &'static str> {
    let names: Vec<&'static str> = COLOR_NAME_RGB.iter().map(|(name, _)| *name).collect();
    prop::sample::select(names)
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 100, ..ProptestConfig::default() })]
    // Feature: auxiliary-tools-implementation, Property 39: Material ID Color Mapping
    #[test]
    fn prop_colorizer_material_mapping((material, name) in material_strategy()) {
        let mut store = Store::new();
        let (_group_id, geo_id) = build_group_with_geometry(&mut store, "G0", material, Vec::new());

        let mut visitor = Colorizer::new(None);
        traverse(&mut store, &mut visitor);

        let geo = store.get_geometry(geo_id).unwrap();
        let expected_rgb = color_rgb_by_name(name);
        prop_assert_eq!(geo.color, expected_rgb);
        prop_assert_eq!(geo.color_rgb, expected_rgb);
        prop_assert_eq!(store.get_string(geo.color_name.unwrap()), name);
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 100, ..ProptestConfig::default() })]
    // Feature: auxiliary-tools-implementation, Property 40: Color Attribute Override
    #[test]
    fn prop_colorizer_attribute_override((material, material_name) in material_strategy(), override_name in color_name_strategy()) {
        prop_assume!(override_name != material_name);

        let mut store = Store::new();
        let (_group_id, geo_id) = build_group_with_geometry(
            &mut store,
            "G0",
            material,
            vec![("Color", override_name)],
        );

        let mut visitor = Colorizer::new(Some("Color"));
        traverse(&mut store, &mut visitor);

        let geo = store.get_geometry(geo_id).unwrap();
        let expected_rgb = color_rgb_by_name(override_name);
        prop_assert_eq!(geo.color, expected_rgb);
        prop_assert_eq!(geo.color_rgb, expected_rgb);
        prop_assert_eq!(store.get_string(geo.color_name.unwrap()), override_name);
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 100, ..ProptestConfig::default() })]
    // Feature: auxiliary-tools-implementation, Property 41: Color Inheritance
    #[test]
    fn prop_colorizer_inheritance(parent_name in color_name_strategy(), (child_material, _) in material_strategy()) {
        let mut store = Store::new();
        let (parent_id, _parent_geo) = build_group_with_geometry(
            &mut store,
            "Parent",
            0,
            vec![("Color", parent_name)],
        );
        store.push_node_context(parent_id);

        let (_child_id, child_geo) = build_group_with_geometry(
            &mut store,
            "Child",
            child_material,
            Vec::new(),
        );

        let mut visitor = Colorizer::new(Some("Color"));
        traverse(&mut store, &mut visitor);

        let geo = store.get_geometry(child_geo).unwrap();
        let expected_rgb = color_rgb_by_name(parent_name);
        prop_assert_eq!(geo.color, expected_rgb);
        prop_assert_eq!(geo.color_rgb, expected_rgb);
        prop_assert_eq!(store.get_string(geo.color_name.unwrap()), parent_name);
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 100, ..ProptestConfig::default() })]
    // Feature: auxiliary-tools-implementation, Property 42: Color Override Precedence
    #[test]
    fn prop_colorizer_override_precedence(parent_name in color_name_strategy(), child_name in color_name_strategy()) {
        prop_assume!(parent_name != child_name);

        let mut store = Store::new();
        let (parent_id, _parent_geo) = build_group_with_geometry(
            &mut store,
            "Parent",
            0,
            vec![("Color", parent_name)],
        );
        store.push_node_context(parent_id);

        let (_child_id, child_geo) = build_group_with_geometry(
            &mut store,
            "Child",
            0,
            vec![("Color", child_name)],
        );

        let mut visitor = Colorizer::new(Some("Color"));
        traverse(&mut store, &mut visitor);

        let geo = store.get_geometry(child_geo).unwrap();
        let expected_rgb = color_rgb_by_name(child_name);
        prop_assert_eq!(geo.color, expected_rgb);
        prop_assert_eq!(geo.color_rgb, expected_rgb);
        prop_assert_eq!(store.get_string(geo.color_name.unwrap()), child_name);
    }
}

prop_compose! {
    fn materials_strategy()(group_count in 1usize..6)
        (materials in prop::collection::vec(0u32..256, group_count))
        -> Vec<u32> {
        materials
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 100, ..ProptestConfig::default() })]
    // Feature: auxiliary-tools-implementation, Property 43: Color Storage Completeness
    #[test]
    fn prop_colorizer_storage_completeness(materials in materials_strategy()) {
        let mut store = Store::new();
        let mut geo_ids = Vec::new();

        for (idx, material) in materials.iter().enumerate() {
            let (_group_id, geo_id) = build_group_with_geometry(
                &mut store,
                &format!("G{}", idx),
                *material,
                Vec::new(),
            );
            geo_ids.push(geo_id);
        }

        let mut visitor = Colorizer::new(None);
        traverse(&mut store, &mut visitor);

        for geo_id in geo_ids {
            let geo = store.get_geometry(geo_id).unwrap();
            prop_assert!(geo.color_name.is_some());
            prop_assert_eq!(geo.color, geo.color_rgb);
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 100, ..ProptestConfig::default() })]
    // Feature: auxiliary-tools-implementation, Property 44: Color Name Case Insensitivity
    #[test]
    fn prop_colorizer_case_insensitive(name in color_name_strategy(), upper in any::<bool>()) {
        let variant = if upper {
            name.to_ascii_uppercase()
        } else {
            name.to_ascii_lowercase()
        };

        let mut store = Store::new();
        let (_group_id, geo_id) = build_group_with_geometry(
            &mut store,
            "G0",
            0,
            vec![("Color", &variant)],
        );

        let mut visitor = Colorizer::new(Some("Color"));
        traverse(&mut store, &mut visitor);

        let geo = store.get_geometry(geo_id).unwrap();
        let expected_rgb = color_rgb_by_name(name);
        prop_assert_eq!(geo.color, expected_rgb);
        prop_assert_eq!(geo.color_rgb, expected_rgb);
    }
}
