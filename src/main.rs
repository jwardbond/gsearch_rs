use std::error::Error;
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use std::time::Instant;

use clap::Parser;
use indicatif::{MultiProgress, ProgressBar};
use indicatif_log_bridge::LogWrapper;
use serde::Serialize;

use gsearch::alignment::Alignment;
use gsearch::graph::Graph;
use gsearch::io::read_graph_file;
use gsearch::search::{DEFAULT_WEIGHT, SearchParams, run_gsearch};
use gsearch::spatial_index::SpatialIndex;

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
    results: &'a [Alignment],
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let level = if args.verbose { "debug" } else { "info" };
    let logger =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(level)).build();
    let max_level = logger.filter();
    // Log lines are printed above the progress bar, so they do not break it.
    let multi = MultiProgress::new();
    LogWrapper::new(multi.clone(), logger).try_init()?;
    log::set_max_level(max_level);

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
    let bar = multi.add(ProgressBar::new(0));
    let start = Instant::now();
    let outcome = run_gsearch(&query, &index, &params, |done, total| {
        bar.set_length(total as u64);
        bar.set_position(done as u64);
    });
    bar.finish_and_clear();

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

    if let Some(path) = &args.output {
        let report = Report {
            query: &query_file.graph_id,
            graph: &graph_file.graph_id,
            tol: args.tol,
            k: args.k,
            weight: args.weight,
            n_anchors: outcome.n_anchors,
            n_mappings: outcome.n_mappings,
            results: &outcome.alignments,
        };
        serde_json::to_writer(BufWriter::new(File::create(path)?), &report)?;
    }
    Ok(())
}
