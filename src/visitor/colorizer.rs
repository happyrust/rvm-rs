use crate::store::{Attribute, Geometry, GeometryId, Node, NodeId, NodeKind, Store, StringId};
use crate::visitor::Visitor;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy)]
struct ColorState {
    color_name: StringId,
    color_rgb: u32,
    override_active: bool,
}

pub struct Colorizer {
    color_name_by_material: HashMap<u32, StringId>,
    color_by_name: HashMap<String, u32>,
    color_attribute_raw: Option<String>,
    color_attribute: Option<StringId>,
    default_name: Option<StringId>,
    default_rgb: u32,
    stack: Vec<ColorState>,
    warned_materials: HashSet<u32>,
    warned_names: HashSet<String>,
    initialized: bool,
}

impl Colorizer {
    pub fn new(color_attribute: Option<&str>) -> Self {
        Self {
            color_name_by_material: HashMap::new(),
            color_by_name: HashMap::new(),
            color_attribute_raw: color_attribute.map(str::to_string),
            color_attribute: None,
            default_name: None,
            default_rgb: 0x787878,
            stack: Vec::new(),
            warned_materials: HashSet::new(),
            warned_names: HashSet::new(),
            initialized: false,
        }
    }

    fn ensure_initialized(&mut self, store: &mut Store) {
        if !self.initialized {
            self.init_color_palette(store);
            self.initialized = true;
        }
    }

    fn init_color_palette(&mut self, store: &mut Store) {
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
        ];

        for (material, name) in MATERIAL_COLOR_NAMES {
            let name_id = store.intern_string(name);
            self.color_name_by_material.insert(*material, name_id);
        }

        for (name, rgb) in COLOR_NAME_RGB {
            self.color_by_name.insert(name.to_ascii_lowercase(), *rgb);
        }

        let default_name = store.intern_string("Default");
        self.default_name = Some(default_name);
        self.color_by_name
            .insert("default".to_string(), self.default_rgb);

        if let Some(raw) = self.color_attribute_raw.as_ref() {
            self.color_attribute = Some(store.intern_string(raw));
        }
    }

    fn lookup_color(&self, name: &str) -> Option<u32> {
        let key = name.to_ascii_lowercase();
        self.color_by_name.get(&key).copied()
    }

    fn warn_unknown_material(&mut self, material: u32) {
        if self.warned_materials.insert(material) {
            eprintln!("Warning: Unrecognized material id {}", material);
        }
    }

    fn warn_unknown_name(&mut self, name: &str) {
        let key = name.to_ascii_lowercase();
        if self.warned_names.insert(key) {
            eprintln!("Warning: Unrecognized color name {}", name);
        }
    }

    fn apply_attribute_override(
        &mut self,
        attrs: &[Attribute],
        store: &Store,
        state: &mut ColorState,
    ) {
        let Some(attr_id) = self.color_attribute else {
            return;
        };

        for attr in attrs {
            if attr.key != attr_id {
                continue;
            }

            let value = store.get_string(attr.value);
            if let Some(rgb) = self.lookup_color(value) {
                state.color_name = attr.value;
                state.color_rgb = rgb;
                state.override_active = true;
            } else {
                self.warn_unknown_name(value);
            }
        }
    }
}

impl Visitor for Colorizer {
    fn visit_node(&mut self, _node_id: NodeId, node: &Node, store: &mut Store) {
        self.ensure_initialized(store);

        let NodeKind::Group(group) = &node.kind else {
            return;
        };

        let default_name = match self.default_name {
            Some(name) => name,
            None => return,
        };

        let mut state = if let Some(parent) = self.stack.last() {
            *parent
        } else {
            ColorState {
                color_name: default_name,
                color_rgb: self.default_rgb,
                override_active: false,
            }
        };

        if !state.override_active {
            if group.material == 0 {
                state.color_name = default_name;
                state.color_rgb = self.default_rgb;
            } else if let Some(color_name) = self.color_name_by_material.get(&group.material) {
                let name = store.get_string(*color_name);
                if let Some(rgb) = self.lookup_color(name) {
                    state.color_name = *color_name;
                    state.color_rgb = rgb;
                } else {
                    self.warn_unknown_name(name);
                }
            } else {
                self.warn_unknown_material(group.material);
            }
        }

        self.apply_attribute_override(&group.attributes, store, &mut state);
        self.stack.push(state);
    }

    fn visit_geometry(&mut self, geometry_id: GeometryId, _geometry: &Geometry, store: &mut Store) {
        self.ensure_initialized(store);

        let Some(state) = self.stack.last() else {
            return;
        };

        if let Some(geometry) = store.get_geometry_mut(geometry_id) {
            geometry.color = state.color_rgb;
            geometry.color_rgb = state.color_rgb;
            geometry.color_name = Some(state.color_name);
        }
    }

    fn leave_node(&mut self, _node_id: NodeId, node: &Node, _store: &mut Store) {
        if let NodeKind::Group(_) = &node.kind {
            let _ = self.stack.pop();
        }
    }
}
