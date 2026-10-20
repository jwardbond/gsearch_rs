use std::error::Error;
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use std::time::Instant;

use clap::Parser;
use indicatif::ProgressBar;
use serde::Serialize;

use gsearch::alignment::Alignment;
use gsearch::graph::Graph;
use gsearch::io::read_graph_file;
use gsearch::search::{DEFAULT_WEIGHT, SearchParams, run_gsearch};
use gsearch::spatial_index::SpatialIndex;
use gsearch::{path_scoring, procrustes};

/// Search a graph for the places that match a query shape.
#[derive(Parser)]
#[command(version)]
struct Args {
    /// The query graph (JSON).
    query: PathBuf,
    /// The graph to search (JSON).
    graph: PathBuf,
    /// The tolerance, in the units of the graph.
    #[arg(short = 'e', long, default_value_t = 10.0)]
    tol: f64,
    /// How many alignments to keep.
    #[arg(short, default_value_t = 1)]
    k: usize,
    /// The weight of the edge score in the score.
    #[arg(short, long, default_value_t = DEFAULT_WEIGHT)]
    weight: f64,
    /// Write the results to this file (JSON).
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// Log the work of every anchor.
    #[arg(short, long)]
    verbose: bool,
}

/// What `-o` writes.
#[derive(Serialize)]
struct Report<'a> {
    query: &'a str,
    graph: &'a str,
    tol: f64,
    k: usize,
    weight: f64,
    n_anchors: usize,
    n_mappings: usize,
    /// True while the transform or the routing is a placeholder: the scores mean nothing then.
    placeholder_scoring: bool,
    results: &'a [Alignment],
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let level = if args.verbose { "debug" } else { "info" };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(level)).init();

    let query_file = read_graph_file(&args.query)?;
    let query = Graph::from_file(&query_file)?;
    if query.node_count() == 0 {
        return Err("the query has no nodes".into());
    }
    let graph_file = read_graph_file(&args.graph)?;
    let index = SpatialIndex::new(Graph::from_file(&graph_file)?);

    let params = SearchParams {
        tol: args.tol,
        k: args.k,
        weight: args.weight,
    };
    let bar = ProgressBar::new(0);
    let start = Instant::now();
    let outcome = run_gsearch(&query, &index, &params, |done, total| {
        bar.set_length(total as u64);
        bar.set_position(done as u64);
    });
    bar.finish_and_clear();

    let placeholder_scoring = procrustes::IS_PLACEHOLDER || path_scoring::IS_PLACEHOLDER;
    println!(
        "{} in {}: {} anchors, {} mappings, {} alignments kept ({:.3} s)",
        query_file.graph_id,
        graph_file.graph_id,
        outcome.n_anchors,
        outcome.n_mappings,
        outcome.alignments.len(),
        start.elapsed().as_secs_f64(),
    );
    if let Some(best) = outcome.alignments.first() {
        println!("lowest score: {}", best.score);
    }
    if placeholder_scoring {
        println!("warning: the scores use a placeholder (see procrustes.rs, path_scoring.rs)");
    }

    if let Some(path) = &args.output {
        let report = Report {
            query: &query_file.graph_id,
            graph: &graph_file.graph_id,
            tol: args.tol,
            k: args.k,
            weight: args.weight,
            n_anchors: outcome.n_anchors,
            n_mappings: outcome.n_mappings,
            placeholder_scoring,
            results: &outcome.alignments,
        };
        serde_json::to_writer(BufWriter::new(File::create(path)?), &report)?;
    }
    Ok(())
}
