use gsearch::graph::Graph;
use gsearch::io::read_graph_file;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: gsearch <graph.json>")?;

    let file = read_graph_file(&path)?;
    let graph = Graph::from_file(&file)?;

    println!(
        "{}: {} nodes, {} edges",
        file.graph_id,
        graph.node_count(),
        graph.edge_count(),
    );
    println!("anchor radius: {:.4}", graph.radius_from(0));

    Ok(())
}
