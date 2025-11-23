use crate::math::BBox3;
use crate::store::geometry::GeometryId;
use crate::store::strings::StringId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub u32);

#[derive(Debug, Clone)]
pub enum NodeKind {
    File(FileNode),
    Model(ModelNode),
    Group(GroupNode),
}

#[derive(Debug, Clone)]
pub struct Node {
    pub kind: NodeKind,
    pub next: Option<NodeId>,
    pub first_child: Option<NodeId>,
    pub last_child: Option<NodeId>,
}

#[derive(Debug, Clone)]
pub struct FileNode {
    pub info: StringId,
    pub note: StringId,
    pub date: StringId,
    pub user: StringId,
    pub encoding: StringId,
    pub path: StringId,
}

#[derive(Debug, Clone)]
pub struct ModelNode {
    pub project: StringId,
    pub name: StringId,
    pub first_color: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct GroupNode {
    pub name: StringId,
    pub translation: glam::Vec3,
    pub material: u32,
    pub transparency: u32,
    pub bbox_world: BBox3,
    pub first_geometry: Option<GeometryId>,
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Clone)]
pub struct Attribute {
    pub key: StringId,
    pub value: StringId,
}
