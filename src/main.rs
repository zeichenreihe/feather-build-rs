use anyhow::Result;
use petgraph::dot::Dot;
use crate::version_graph::VersionGraph;

mod reader;
mod version_graph;

fn main() -> Result<()> {
    println!("Hello, world!");

    let start = std::time::Instant::now();

    let v = VersionGraph::resolve()?;

    let elapsed = start.elapsed();

    println!("elapsed: {elapsed:?}");

    dbg!(Dot::new(&v.graph));

    Ok(())
}
