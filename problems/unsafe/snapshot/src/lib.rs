use thiserror::Error;

use std::{
    collections::HashMap,
    iter::{ExactSizeIterator, Iterator},
    marker::PhantomData,
};

////////////////////////////////////////////////////////////////////////////////

pub type ObjectId = i64;

////////////////////////////////////////////////////////////////////////////////

#[derive(PartialEq, Eq, Clone, Copy, Default, Debug)]
pub struct ResourceTotals {
    pub cpu: u64,
    pub memory: u64,
    pub disk_capacity: u64,
}

impl std::ops::AddAssign for ResourceTotals {
    fn add_assign(&mut self, other: Self) {
        self.cpu += other.cpu;
        self.memory += other.memory;
        self.disk_capacity += other.disk_capacity;
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub struct Dump {
    pub node_segments: Vec<NodeSegmentRecord>,
    pub nodes: Vec<NodeRecord>,
    pub pod_sets: Vec<PodSetRecord>,
    pub pods: Vec<PodRecord>,
}

pub struct NodeSegmentRecord {
    pub id: ObjectId,
}

pub struct NodeRecord {
    pub id: ObjectId,
    pub node_segment_id: ObjectId,
    pub resource_totals: ResourceTotals,
}

pub struct PodSetRecord {
    pub id: ObjectId,
    pub node_segment_id: ObjectId,
}

pub struct PodRecord {
    pub id: ObjectId,
    pub pod_set_id: ObjectId,
    pub node_id: Option<ObjectId>,
    pub resource_requests: ResourceTotals,
}

////////////////////////////////////////////////////////////////////////////////
#[derive(Default)]
pub struct NodeSegment<'a> {
    pub id: ObjectId,
    pub resource_usage: ResourceTotals,
    pub resource_requests: ResourceTotals,
    pub resource_totals: ResourceTotals,
    nodes: Vec<*const Node<'a>>,
    pod_sets: Vec<*const PodSet<'a>>,
    lifetime: PhantomData<&'a ()>,
}

impl<'a> NodeSegment<'a> {
    pub fn nodes(&self) -> impl Iterator<Item = &Node<'a>> {
        self.nodes.iter().map(|&n| unsafe { &*n })
    }

    pub fn pod_sets(&self) -> impl Iterator<Item = &PodSet<'a>> {
        self.pod_sets.iter().map(|&ps| unsafe { &*ps })
    }
}

////////////////////////////////////////////////////////////////////////////////
pub struct Node<'a> {
    pub id: ObjectId,
    pub resource_usage: ResourceTotals,
    pub resource_totals: ResourceTotals,
    pods: Vec<*const Pod<'a>>,
    node_segment: *const NodeSegment<'a>,
    lifetime: PhantomData<&'a ()>,
}

impl<'a> Node<'a> {
    pub fn pods(&self) -> impl Iterator<Item = &Pod<'a>> {
        self.pods.iter().map(|&p| unsafe { &*p })
    }

    pub fn node_segment(&self) -> &NodeSegment<'a> {
        unsafe { &*self.node_segment }
    }
}

////////////////////////////////////////////////////////////////////////////////
pub struct PodSet<'a> {
    pub id: ObjectId,
    pub resource_requests: ResourceTotals,
    pods: Vec<*const Pod<'a>>,
    node_segment: *const NodeSegment<'a>,
    lifetime: PhantomData<&'a ()>,
}

impl<'a> PodSet<'a> {
    pub fn pods(&self) -> impl Iterator<Item = &Pod<'a>> {
        self.pods.iter().map(|&p| unsafe { &*p })
    }

    pub fn node_segment(&self) -> &NodeSegment<'a> {
        unsafe { &*self.node_segment }
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct Pod<'a> {
    pub id: ObjectId,
    pub resource_requests: ResourceTotals,
    pod_set: *const PodSet<'a>,
    node: Option<*const Node<'a>>,
    lifetime: PhantomData<&'a ()>,
}

impl<'a> Pod<'a> {
    pub fn pod_set(&self) -> &PodSet<'a> {
        unsafe { &*self.pod_set }
    }

    pub fn node(&self) -> Option<&Node<'a>> {
        unsafe { self.node.map(|n| &*n) }
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub struct Snapshot<'a> {
    node_map: HashMap<ObjectId, Box<Node<'a>>>,
    node_segment_map: HashMap<ObjectId, Box<NodeSegment<'a>>>,
    pod_map: HashMap<ObjectId, Box<Pod<'a>>>,
    pod_set_map: HashMap<ObjectId, Box<PodSet<'a>>>,
}

impl<'a> Snapshot<'a> {
    pub fn new(dump: &Dump) -> Result<Self> {
        let mut snapshot: Snapshot = Default::default();
        // init node_segment_map
        for el in dump.node_segments.iter() {
            snapshot
                .node_segment_map
                .insert(
                    el.id,
                    Box::new(NodeSegment {
                        id: el.id,
                        ..Default::default()
                    }),
                )
                .map_or(Ok(()), |_| {
                    Err(Error::DuplicateObject {
                        ty: ObjectType::NodeSegment,
                        id: el.id,
                    })
                })?;
        }

        for el in dump.pod_sets.iter() {
            let node_segment =
                snapshot
                    .get_node_segment(&el.node_segment_id)
                    .ok_or(Error::MissingObject {
                        ty: ObjectType::NodeSegment,
                        id: el.node_segment_id,
                    })?;
            snapshot
                .pod_set_map
                .insert(
                    el.id,
                    Box::new(PodSet {
                        id: el.id,
                        node_segment: node_segment as *const NodeSegment,
                        resource_requests: ResourceTotals::default(),
                        pods: Vec::new(),
                        lifetime: PhantomData,
                    }),
                )
                .map_or(Ok(()), |_| {
                    Err(Error::DuplicateObject {
                        ty: ObjectType::PodSet,
                        id: el.id,
                    })
                })?;
        }

        for el in dump.nodes.iter() {
            let node_segment =
                snapshot
                    .get_node_segment(&el.node_segment_id)
                    .ok_or(Error::MissingObject {
                        ty: ObjectType::NodeSegment,
                        id: el.node_segment_id,
                    })?;
            snapshot
                .node_map
                .insert(
                    el.id,
                    Box::new(Node {
                        id: el.id,
                        node_segment: node_segment as *const NodeSegment,
                        resource_usage: ResourceTotals::default(),
                        resource_totals: el.resource_totals,
                        pods: Vec::new(),
                        lifetime: PhantomData,
                    }),
                )
                .map_or(Ok(()), |_| {
                    Err(Error::DuplicateObject {
                        ty: ObjectType::Node,
                        id: el.id,
                    })
                })?;
        }

        for el in dump.pods.iter() {
            let pod_set = snapshot
                .get_pod_set(&el.pod_set_id)
                .ok_or(Error::MissingObject {
                    ty: ObjectType::PodSet,
                    id: el.pod_set_id,
                })?;
            let mut node = None;
            if let Some(node_id) = el.node_id {
                let node_from_hmap = snapshot.get_node(&node_id).ok_or(Error::MissingObject {
                    ty: ObjectType::Node,
                    id: node_id,
                })?;
                node = Some(node_from_hmap as *const Node);
            }

            snapshot
                .pod_map
                .insert(
                    el.id,
                    Box::new(Pod {
                        id: el.id,
                        pod_set: pod_set as *const PodSet,
                        resource_requests: el.resource_requests,
                        node,
                        lifetime: PhantomData,
                    }),
                )
                .map_or(Ok(()), |_| {
                    Err(Error::DuplicateObject {
                        ty: ObjectType::Pod,
                        id: el.id,
                    })
                })?;
        }

        Ok(snapshot.fix_deps_and_res())
    }

    fn fix_deps_and_res(mut self) -> Self {
        self.pod_map.values().for_each(|p| {
            let ps_ref = p.pod_set();
            let ps = self.pod_set_map.get_mut(&ps_ref.id).unwrap();
            ps.pods.push(p.as_ref() as *const Pod);
            ps.resource_requests += p.resource_requests;
        });

        self.pod_map.values().for_each(|p| {
            if let Some(node_ref) = p.node() {
                let node = self.node_map.get_mut(&node_ref.id).unwrap();
                node.pods.push(p.as_ref() as *const Pod);
                node.resource_usage += p.resource_requests;
            }
        });

        self.pod_set_map.values().for_each(|ps| {
            let node_segment_ref = ps.node_segment();
            let node_segment = self.node_segment_map.get_mut(&node_segment_ref.id).unwrap();
            node_segment.pod_sets.push(ps.as_ref() as *const PodSet);
            node_segment.resource_requests += ps.resource_requests;
        });

        self.node_map.values().for_each(|n| {
            let node_segment_ref = n.node_segment();
            let node_segment = self.node_segment_map.get_mut(&node_segment_ref.id).unwrap();
            node_segment.nodes.push(n.as_ref() as *const Node);
            node_segment.resource_usage += n.resource_usage;
            node_segment.resource_totals += n.resource_totals;
        });

        self
    }

    pub fn nodes(&self) -> impl ExactSizeIterator<Item = &Node<'a>> {
        self.node_map.values().map(|n| n.as_ref())
    }
    pub fn node_segments(&self) -> impl ExactSizeIterator<Item = &NodeSegment<'a>> {
        self.node_segment_map.values().map(|ns| ns.as_ref())
    }

    pub fn pods(&self) -> impl ExactSizeIterator<Item = &Pod<'a>> {
        self.pod_map.values().map(|p| p.as_ref())
    }

    pub fn pod_sets(&self) -> impl ExactSizeIterator<Item = &PodSet<'a>> {
        self.pod_set_map.values().map(|ps| ps.as_ref())
    }

    pub fn get_node(&self, id: &ObjectId) -> Option<&Node<'a>> {
        self.node_map.get(id).map(|n| n.as_ref())
    }

    pub fn get_node_segment(&self, id: &ObjectId) -> Option<&NodeSegment<'a>> {
        self.node_segment_map.get(id).map(|ns| ns.as_ref())
    }

    pub fn get_pod(&self, id: &ObjectId) -> Option<&Pod<'a>> {
        self.pod_map.get(id).map(|p| p.as_ref())
    }

    pub fn get_pod_set(&self, id: &ObjectId) -> Option<&PodSet<'a>> {
        self.pod_set_map.get(id).map(|ps| ps.as_ref())
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, PartialEq, Eq)]
pub enum ObjectType {
    NodeSegment,
    Node,
    PodSet,
    Pod,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    #[error("snapshot references a non-existent object (type: {ty:?}, id: {id})")]
    MissingObject { ty: ObjectType, id: ObjectId },
    #[error("found duplicate object in snapshot (type: {ty:?}, id: {id})")]
    DuplicateObject { ty: ObjectType, id: ObjectId },
}

pub type Result<T> = std::result::Result<T, Error>;
