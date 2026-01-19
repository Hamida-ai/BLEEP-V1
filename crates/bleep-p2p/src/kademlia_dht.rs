use std::collections::HashMap;
use std::net::SocketAddr;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeId(String);

impl NodeId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn random() -> Self {
        use rand::Rng;
        let id: u64 = rand::thread_rng().gen();
        Self(id.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct Kademlia {
    nodes: HashMap<NodeId, SocketAddr>,
}

impl Kademlia {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, id: NodeId, addr: SocketAddr) {
        self.nodes.insert(id, addr);
    }

    pub fn find_node(&self, id: &NodeId) -> Option<&SocketAddr> {
        self.nodes.get(id)
    }
}
