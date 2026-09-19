use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct NodeVersion {

    pub node_id: String,
    pub version: String,

}

pub struct ProtocolVersionManager {

    pub current_version: String,
    pub nodes: HashMap<String, NodeVersion>,

}

impl ProtocolVersionManager {

    pub fn new(
        version: String,
    ) -> Self {

        Self {
            current_version: version,
            nodes: HashMap::new(),
        }

    }

    pub fn register_node(
        &mut self,
        node_id: String,
        version: String,
    ) {

        self.nodes.insert(
            node_id.clone(),
            NodeVersion {
                node_id,
                version,
            }
        );

    }

    pub fn compatible_nodes(
        &self,
    ) -> usize {

        self.nodes
            .values()
            .filter(
                |n|
                n.version
                ==
                self.current_version
            )
            .count()

    }

    pub fn incompatible_nodes(
        &self,
    ) -> usize {

        self.nodes
            .values()
            .filter(
                |n|
                n.version
                !=
                self.current_version
            )
            .count()

    }

    pub fn show(
        &self,
    ) {

        println!(
            "\n===== PROTOCOL VERSION ====="
        );

        println!(
            "Current Version: {}",
            self.current_version
        );

        for (_, node)
            in &self.nodes
        {

            println!(
                "{} => {}",
                node.node_id,
                node.version
            );

        }

    }

    pub fn readiness_report(
        &self,
    ) {

        println!(
            "\n===== UPGRADE READINESS ====="
        );

        println!(
            "Compatible Nodes: {}",
            self.compatible_nodes()
        );

        println!(
            "Incompatible Nodes: {}",
            self.incompatible_nodes()
        );

    }

}
