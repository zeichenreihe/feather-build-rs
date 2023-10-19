use std::path::Path;
use anyhow::Result;
use petgraph::dot::Dot;
use crate::version_graph::VersionGraph;

mod reader;
mod version_graph;

fn main() -> Result<()> {
    println!("Hello, world!");

    let dir = Path::new("mappings/mappings");

    let start = std::time::Instant::now();

    let v = VersionGraph::resolve(dir)?;

    let elapsed = start.elapsed();
    println!("elapsed: {elapsed:?}");

    let versions = vec![
        "1.3.1",
        "1.3.2",
        "1.12.2",
        "12w30e-client",
        "1.3-pre-07261249",
    ];

    for version in versions {
        let a = v.get(version)?;

        let start = std::time::Instant::now();

        let m = v.apply_diffs(a)?;

        let elapsed = start.elapsed();
        println!("{version} elapsed: {elapsed:?}");
    }

    //reader::tiny_v2::write(&m);

    //dbg!(Dot::new(&v.graph));

    Ok(())
}
