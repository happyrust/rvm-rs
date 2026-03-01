use crate::store::{Geometry, GeometryId, GeometryKind, Node, NodeId, NodeKind, Store};
use crate::visitor::Visitor;
use std::io::{self, Write};

pub struct DumpNames {
    output: Box<dyn Write>,
    stack: Vec<crate::store::StringId>,
    geometry_count: usize,
    facet_group_count: usize,
    pending_output: bool,
    error: Option<io::Error>,
}

impl DumpNames {
    pub fn new(output: Box<dyn Write>) -> Self {
        Self {
            output,
            stack: Vec::new(),
            geometry_count: 0,
            facet_group_count: 0,
            pending_output: false,
            error: None,
        }
    }

    fn print_group_tail(&mut self) {
        if !self.pending_output {
            return;
        }
        self.pending_output = false;

        self.write_str("\n");

        if self.geometry_count > 0 {
            let count = self.geometry_count;
            self.write_indent(self.stack.len());
            self.write_fmt(format_args!(" pgeos={}\n", count));
        }

        if self.facet_group_count > 0 {
            let count = self.facet_group_count;
            self.write_indent(self.stack.len());
            self.write_fmt(format_args!(" fgrps={}\n", count));
        }
    }

    fn write_indent(&mut self, levels: usize) {
        for _ in 0..levels {
            self.write_str("    ");
        }
    }

    fn write_str(&mut self, text: &str) {
        if self.error.is_some() {
            return;
        }
        if let Err(err) = self.output.write_all(text.as_bytes()) {
            self.error = Some(err);
        }
    }

    fn write_fmt(&mut self, args: std::fmt::Arguments<'_>) {
        if self.error.is_some() {
            return;
        }
        if let Err(err) = self.output.write_fmt(args) {
            self.error = Some(err);
        }
    }
}

impl Visitor for DumpNames {
    fn visit_node(&mut self, _node_id: NodeId, node: &Node, store: &mut Store) {
        if self.error.is_some() {
            return;
        }

        match &node.kind {
            NodeKind::File(file) => {
                self.write_str("File:\n");
                self.write_fmt(format_args!(
                    "    info:     \"{}\"\n",
                    store.get_string(file.info)
                ));
                self.write_fmt(format_args!(
                    "    note:     \"{}\"\n",
                    store.get_string(file.note)
                ));
                self.write_fmt(format_args!(
                    "    date:     \"{}\"\n",
                    store.get_string(file.date)
                ));
                self.write_fmt(format_args!(
                    "    user:     \"{}\"\n",
                    store.get_string(file.user)
                ));
                self.write_fmt(format_args!(
                    "    encoding: \"{}\"\n",
                    store.get_string(file.encoding)
                ));
            }
            NodeKind::Model(model) => {
                self.write_str("Model:\n");
                self.write_fmt(format_args!(
                    "    project:  \"{}\"\n",
                    store.get_string(model.project)
                ));
                self.write_fmt(format_args!(
                    "    name:     \"{}\"\n",
                    store.get_string(model.name)
                ));
            }
            NodeKind::Group(group) => {
                self.print_group_tail();
                self.geometry_count = 0;
                self.facet_group_count = 0;

                self.stack.push(group.name);
                self.write_indent(self.stack.len().saturating_sub(1));
                self.write_fmt(format_args!("{}", store.get_string(group.name)));
                self.pending_output = true;
            }
        }
    }

    fn visit_geometry(
        &mut self,
        _geometry_id: GeometryId,
        geometry: &Geometry,
        _store: &mut Store,
    ) {
        if self.error.is_some() {
            return;
        }

        match &geometry.kind {
            GeometryKind::FacetGroup(_) => self.facet_group_count += 1,
            _ => self.geometry_count += 1,
        }
    }

    fn leave_node(&mut self, _node_id: NodeId, node: &Node, _store: &mut Store) {
        if let NodeKind::Group(_) = &node.kind {
            self.print_group_tail();
            let _ = self.stack.pop();
        }
    }
}
