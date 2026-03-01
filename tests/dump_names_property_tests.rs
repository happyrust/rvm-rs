use proptest::prelude::*;
use rvm_rs::store::geometry::{Cylinder, FacetGroup, GeometryKind};
use rvm_rs::store::{FileNode, GroupNode, ModelNode, NodeKind, Store};
use rvm_rs::visitor::dump_names::DumpNames;
use rvm_rs::visitor::traverse;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{self, Write};
use std::rc::Rc;

struct SharedBuffer(Rc<RefCell<Vec<u8>>>);

impl Write for SharedBuffer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn build_chain_store(
    file_info: &str,
    file_note: &str,
    file_date: &str,
    file_user: &str,
    file_encoding: &str,
    model_project: &str,
    model_name: &str,
    group_names: &[String],
    prim_counts: &[usize],
    facet_counts: &[usize],
) -> Store {
    let mut store = Store::new();

    let file_node = FileNode {
        info: store.intern_string(file_info),
        note: store.intern_string(file_note),
        date: store.intern_string(file_date),
        user: store.intern_string(file_user),
        encoding: store.intern_string(file_encoding),
        path: store.intern_string(""),
    };
    let file_id = store.new_node(NodeKind::File(file_node));
    store.push_node_context(file_id);

    let model_node = ModelNode {
        project: store.intern_string(model_project),
        name: store.intern_string(model_name),
        first_color: None,
    };
    let model_id = store.new_node(NodeKind::Model(model_node));
    store.push_node_context(model_id);

    for (idx, name) in group_names.iter().enumerate() {
        let group_node = GroupNode {
            name: store.intern_string(name),
            translation: glam::Vec3::ZERO,
            material: 0,
            transparency: 0,
            id: -1,
            bbox_world: rvm_rs::math::BBox3::new(),
            first_geometry: None,
            attributes: Vec::new(),
        };
        let group_id = store.new_node(NodeKind::Group(group_node));
        store.push_node_context(group_id);

        for _ in 0..prim_counts[idx] {
            let kind = GeometryKind::Cylinder(Cylinder {
                radius: 1.0,
                height: 1.0,
            });
            store.new_geometry(group_id, kind);
        }

        for _ in 0..facet_counts[idx] {
            let kind = GeometryKind::FacetGroup(FacetGroup {
                polygons: Vec::new(),
            });
            store.new_geometry(group_id, kind);
        }
    }

    store
}

fn dump_output(store: &mut Store) -> String {
    let buffer = Rc::new(RefCell::new(Vec::new()));
    let writer = SharedBuffer(buffer.clone());
    let mut visitor = DumpNames::new(Box::new(writer));
    traverse(store, &mut visitor);
    let bytes = buffer.borrow().clone();
    String::from_utf8(bytes).unwrap()
}

fn is_group_line(trimmed: &str) -> bool {
    if trimmed.starts_with("File:")
        || trimmed.starts_with("Model:")
        || trimmed.starts_with("info:")
        || trimmed.starts_with("note:")
        || trimmed.starts_with("date:")
        || trimmed.starts_with("user:")
        || trimmed.starts_with("encoding:")
        || trimmed.starts_with("project:")
        || trimmed.starts_with("name:")
        || trimmed.starts_with("pgeos=")
        || trimmed.starts_with("fgrps=")
    {
        return false;
    }

    true
}

fn parse_group_lines(output: &str) -> Vec<(usize, String)> {
    let mut groups = Vec::new();
    for line in output.lines() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || !is_group_line(trimmed) {
            continue;
        }
        let indent = line.chars().take_while(|c| *c == ' ').count();
        groups.push((indent, trimmed.to_string()));
    }
    groups
}

fn parse_counts(output: &str) -> HashMap<String, (Option<usize>, Option<usize>)> {
    let mut stack: Vec<String> = Vec::new();
    let mut counts: HashMap<String, (Option<usize>, Option<usize>)> = HashMap::new();

    for line in output.lines() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            continue;
        }

        let indent = line.chars().take_while(|c| *c == ' ').count();

        if is_group_line(trimmed) {
            let depth = indent / 4 + 1;
            while stack.len() >= depth {
                stack.pop();
            }
            stack.push(trimmed.to_string());
            continue;
        }

        if trimmed.starts_with("pgeos=") {
            let depth = indent / 4;
            if depth == 0 || depth > stack.len() {
                continue;
            }
            let group = stack[depth - 1].clone();
            let value = trimmed["pgeos=".len()..].trim();
            if let Ok(parsed) = value.parse::<usize>() {
                counts.entry(group).or_insert((None, None)).0 = Some(parsed);
            }
        } else if trimmed.starts_with("fgrps=") {
            let depth = indent / 4;
            if depth == 0 || depth > stack.len() {
                continue;
            }
            let group = stack[depth - 1].clone();
            let value = trimmed["fgrps=".len()..].trim();
            if let Ok(parsed) = value.parse::<usize>() {
                counts.entry(group).or_insert((None, None)).1 = Some(parsed);
            }
        }
    }

    counts
}

prop_compose! {
    fn group_counts_strategy()(group_count in 1usize..6)
        (prim_counts in prop::collection::vec(0usize..4, group_count),
         facet_counts in prop::collection::vec(0usize..4, group_count),
         group_count in Just(group_count))
        -> (usize, Vec<usize>, Vec<usize>) {
        (group_count, prim_counts, facet_counts)
    }
}

prop_compose! {
    fn meta_string()(s in "[A-Za-z0-9_]{0,12}") -> String {
        s
    }
}

prop_compose! {
    fn file_meta_strategy()
        (info in meta_string(),
         note in meta_string(),
         date in meta_string(),
         user in meta_string(),
         encoding in meta_string())
        -> (String, String, String, String, String) {
        (info, note, date, user, encoding)
    }
}

prop_compose! {
    fn model_meta_strategy()
        (project in meta_string(), name in meta_string())
        -> (String, String) {
        (project, name)
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 100, ..ProptestConfig::default() })]
    // Feature: auxiliary-tools-implementation, Property 18: Name Dump Traversal Completeness
    #[test]
    fn prop_dump_names_traversal_completeness((group_count, prim_counts, facet_counts) in group_counts_strategy()) {
        let group_names: Vec<String> = (0..group_count).map(|i| format!("G{}", i)).collect();
        let mut store = build_chain_store(
            "info",
            "note",
            "date",
            "user",
            "utf8",
            "project",
            "model",
            &group_names,
            &prim_counts,
            &facet_counts,
        );

        let output = dump_output(&mut store);
        let groups: Vec<String> = parse_group_lines(&output)
            .into_iter()
            .map(|(_, name)| name)
            .collect();

        prop_assert_eq!(groups, group_names);
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 100, ..ProptestConfig::default() })]
    // Feature: auxiliary-tools-implementation, Property 19: Name Dump File Metadata
    #[test]
    fn prop_dump_names_file_metadata((info, note, date, user, encoding) in file_meta_strategy()) {
        let group_names = vec!["G0".to_string()];
        let mut store = build_chain_store(
            &info,
            &note,
            &date,
            &user,
            &encoding,
            "project",
            "model",
            &group_names,
            &[0],
            &[0],
        );

        let output = dump_output(&mut store);
        let expected_info = format!("    info:     \"{}\"", info);
        let expected_note = format!("    note:     \"{}\"", note);
        let expected_date = format!("    date:     \"{}\"", date);
        let expected_user = format!("    user:     \"{}\"", user);
        let expected_encoding = format!("    encoding: \"{}\"", encoding);

        prop_assert!(output.contains(&expected_info));
        prop_assert!(output.contains(&expected_note));
        prop_assert!(output.contains(&expected_date));
        prop_assert!(output.contains(&expected_user));
        prop_assert!(output.contains(&expected_encoding));
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 100, ..ProptestConfig::default() })]
    // Feature: auxiliary-tools-implementation, Property 20: Name Dump Model Information
    #[test]
    fn prop_dump_names_model_information((project, name) in model_meta_strategy()) {
        let group_names = vec!["G0".to_string()];
        let mut store = build_chain_store(
            "info",
            "note",
            "date",
            "user",
            "utf8",
            &project,
            &name,
            &group_names,
            &[0],
            &[0],
        );

        let output = dump_output(&mut store);
        let expected_project = format!("    project:  \"{}\"", project);
        let expected_name = format!("    name:     \"{}\"", name);

        prop_assert!(output.contains(&expected_project));
        prop_assert!(output.contains(&expected_name));
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 100, ..ProptestConfig::default() })]
    // Feature: auxiliary-tools-implementation, Property 21: Name Dump Indentation Consistency
    #[test]
    fn prop_dump_names_indentation_consistency((group_count, prim_counts, facet_counts) in group_counts_strategy()) {
        let group_names: Vec<String> = (0..group_count).map(|i| format!("G{}", i)).collect();
        let mut store = build_chain_store(
            "info",
            "note",
            "date",
            "user",
            "utf8",
            "project",
            "model",
            &group_names,
            &prim_counts,
            &facet_counts,
        );

        let output = dump_output(&mut store);
        let groups = parse_group_lines(&output);
        prop_assert_eq!(groups.len(), group_names.len());

        for (idx, (indent, name)) in groups.iter().enumerate() {
            prop_assert_eq!(name, &group_names[idx]);
            prop_assert_eq!(*indent, idx * 4);
            prop_assert_eq!(indent % 4, 0);
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 100, ..ProptestConfig::default() })]
    // Feature: auxiliary-tools-implementation, Property 22: Name Dump Geometry Counts
    #[test]
    fn prop_dump_names_geometry_counts((group_count, prim_counts, facet_counts) in group_counts_strategy()) {
        let group_names: Vec<String> = (0..group_count).map(|i| format!("G{}", i)).collect();
        let mut store = build_chain_store(
            "info",
            "note",
            "date",
            "user",
            "utf8",
            "project",
            "model",
            &group_names,
            &prim_counts,
            &facet_counts,
        );

        let output = dump_output(&mut store);
        let counts = parse_counts(&output);

        for (idx, name) in group_names.iter().enumerate() {
            let (pgeos, fgrps) = counts.get(name).cloned().unwrap_or((None, None));

            if prim_counts[idx] > 0 {
                prop_assert_eq!(pgeos, Some(prim_counts[idx]));
            } else {
                prop_assert_eq!(pgeos, None);
            }

            if facet_counts[idx] > 0 {
                prop_assert_eq!(fgrps, Some(facet_counts[idx]));
            } else {
                prop_assert_eq!(fgrps, None);
            }
        }
    }
}
