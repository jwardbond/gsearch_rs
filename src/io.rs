use serde::{Deserialize, Serialize};

use std::error::Error;
use std::fs::{File, read_to_string};
use std::io::BufWriter;
use std::path::Path;

pub type NodeId = u64;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeRecord {
    pub id: NodeId,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeRecord {
    pub source: NodeId,
    pub target: NodeId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphFile {
    pub graph_id: String,
    pub nodes: Vec<NodeRecord>,
    pub edges: Vec<EdgeRecord>,
}

fn parse_graph(json: &str) -> Result<GraphFile, serde_json::Error> {
    serde_json::from_str(json)
}

pub fn read_graph_file(path: &impl AsRef<Path>) -> Result<GraphFile, Box<dyn Error>> {
    let path = path.as_ref();
    let text = read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let graph = parse_graph(&text).map_err(|e| format!("cannot parse {}: {e}", path.display()))?;
    Ok(graph)
}

pub fn write_graph_file(path: &impl AsRef<Path>, file: &GraphFile) -> Result<(), Box<dyn Error>> {
    let writer = BufWriter::new(File::create(path)?);
    serde_json::to_writer(writer, file)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRIANGLE: &str = r#"{
        "graph_id": "tri",
        "nodes": [
            {"id": 10, "x": 0.0, "y": 0.0},
            {"id": 20, "x": 3.0, "y": 0.0},
            {"id": 30, "x": 0.0, "y": 4.0}
        ],
        "edges": [
            {"source": 10, "target": 20},
            {"source": 20, "target": 30}
        ]
    }"#;

    #[test]
    fn parses_nodes_and_edges() {
        let graph = parse_graph(TRIANGLE).unwrap();

        // every node and edge in the file is read
        assert_eq!(graph.graph_id, "tri");
        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.edges.len(), 2);
        // fields land in the right struct members
        assert_eq!(graph.nodes[1].id, 20);
        assert_eq!(graph.nodes[2].y, 4.0);
        assert_eq!(graph.edges[1].target, 30);
    }

    #[test]
    fn written_file_reads_back_the_same() {
        let file = parse_graph(TRIANGLE).unwrap();
        let path = std::env::temp_dir().join("gsearch_written_file_reads_back.json");

        write_graph_file(&path, &file).unwrap();
        let read_back = read_graph_file(&path).unwrap();
        std::fs::remove_file(&path).unwrap();

        assert_eq!(read_back, file);
    }

    #[test]
    fn rejects_node_without_coordinates() {
        let json = r#"{"graph_id": "bad", "nodes": [{"id": 1, "x": 0.0}], "edges": []}"#;

        let result = parse_graph(json);

        // a missing "y" is an error, not a silent default
        assert!(result.is_err());
    }
}
