use gsearch::io::read_graph_file;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let path= std::env::args().nth(1).ok_or("usage: gsearch <graph.json>")?;

    let graph = read_graph_file(&path)?;

    println!(
        "{}: {} nodes, {} edges",
        graph.graph_id,
        graph.nodes.len(), 
        graph.edges.len(),
    );
    Ok(())
}
