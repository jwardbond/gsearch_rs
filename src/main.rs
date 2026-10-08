use gsearch::graph::Graph;
use gsearch::io::read_graph_file;
use gsearch::spatial_index::SpatialIndex;

use std::error::Error;
use std::time::Instant;

const TOL: f64 = 10.0; // TODO temporary tolerance for query radius

fn main() -> Result<(), Box<dyn Error>> {
    // Parse args
    let usage = "usage: gsearch <query.json> <graph.json>";
    let mut args = std::env::args().skip(1);
    let query_path = args.next().ok_or(usage)?;
    let graph_path = args.next().ok_or(usage)?;

    // Build query
    let query_file = read_graph_file(&query_path)?;
    let query = Graph::from_file(&query_file)?;
    let radius = query.radius_from(0) + TOL;
    println!(
        "{}: {} nodes, crop radius {:.4}",
        query_file.graph_id,
        query.node_count(),
        radius,
    );

    // Build graph and index
    let start = Instant::now();
    let graph_file = read_graph_file(&graph_path)?;
    let graph = Graph::from_file(&graph_file)?;
    let index = SpatialIndex::new(graph);
    eprintln!("load and index:  {:.3} s", start.elapsed().as_secs_f64());
    let graph = index.graph();
    println!(
        "{}: {} nodes, {} edges",
        graph_file.graph_id,
        graph.node_count(),
        graph.edge_count(),
    );

    // Crop graph
    let crop = index.make_crop(0, radius);
    println!(
        "crop around node {}: {} nodes, {} edges",
        graph.id(0),
        crop.node_count(),
        crop.edge_count(),
    );

    // Crop every node
    let start = Instant::now();
    let mut total = 0;
    for anchor in 0..graph.node_count() {
        total += index.make_crop(anchor, radius).node_count();
    }
    eprintln!("crop every node: {:.3} s", start.elapsed().as_secs_f64());
    println!(
        "mean crop size: {:.2} nodes",
        total as f64 / graph.node_count() as f64
    );
    Ok(())
}
