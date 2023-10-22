use std::fs::File;
use std::path::Path;
use anyhow::Result;
use petgraph::dot::Dot;
use petgraph::graph::NodeIndex;
use crate::tiny::RemoveDummy;
use crate::tiny::v2::Mappings;
use crate::version_graph::VersionGraph;

mod tiny;
mod reader;
mod version_graph;
mod writer;

#[derive(Debug)]
enum Environment {
    Merged,
    Client,
    Server,
}

impl Environment {
    fn is_merged(&self) -> bool {
        matches!(self, Self::Merged)
    }
    fn is_client(&self) -> bool {
        matches!(self, Environment::Merged) || matches!(self, Environment::Client)
    }
    fn is_server(&self) -> bool {
        matches!(self, Environment::Merged) || matches!(self, Environment::Server)
    }

    fn parse(id: &str) -> Environment {
        if id.ends_with("-client") {
            return Environment::Client;
        }
        if id.ends_with("-server") {
            return Environment::Server;
        }
        Environment::Merged
    }

    fn parse_version(&self, id: &str) -> String {
        match self {
            Environment::Merged => id.to_owned(),
            Environment::Client => todo!("remove the `-client` suffix"),
            Environment::Server => todo!("remove the `-server` suffix"),
        }
    }
}

struct Downloader {

}

impl Downloader {
    fn versions_manifest() {
        // create a file
        //  TMP/version_manifest_v2.json
        // download from
        //  https://skyrising.github.io/mc-versions/version_manifest.json
        // that is the file that is returned
        // do not cache that file
        todo!()
    }
    fn wanted_version_manifest() {
        // get the manifest version from
        Self::versions_manifest();
        // this also needs the mc version we want
        // Note for doing this we look at the mc version in the
        // manifest json we want, and look at the first entry for
        // getting the date
        let manifest_version = 0;

        // we cache based on the releaseTime???

        // give back

        // TODO: write docs


        todo!()
    }
    fn version_details() {
        Self::versions_manifest();
        todo!()
    }
    fn mc_jars() {
        Self::version_details();
        todo!()
    }
    fn calamus() {
        // download from (replacing version_id and namespace)
        let url = "https://github.com/OrnitheMC/calamus/raw/main/mappings/${version_id}.tiny";
        // but escape it before doing that
        // then return the contents
        todo!()
    }
    fn calamus_v2() {
        // download from (replacing version_id and namespace)
        let url = "https://maven.ornithemc.net/releases/net/ornithemc/calamus-${namespace}/${version_id}/calamus-${namespace}-${version_id}-v2.jar";
        // but escape it before doing that
        // then go in the jar to
        //  mappings/mappings.tiny
        // and extract that out and return it
        todo!()
    }
    fn mc_libs() {
        // pare the
        Self::wanted_version_manifest();
        // as a json. then take the "libraries" tag and iterate
        // on all elements
        // and try to get ".downloads" from it
        // if that exists, and the artifact ".download.artifact" != null
        // then download from url ".download.artifact.url", to a file given by the
        // part after the last slash into a libraries folder (ensuring that we don't overwrite a file)
        todo!()
    }
}

struct Build {
    mappings: Mappings,
}

impl Build {
    fn create(version_graph: &VersionGraph, version: NodeIndex) -> Result<Build> {
        let mut mappings = version_graph.apply_diffs(version)?;

        mappings.remove_dummy();

        Ok(Build {
            mappings,
        })
    }

    fn separate_mappings_for_build(&self) {
        // just return self.mappings
    }

    fn build_feather_tiny(&self) {
        let calamus_jar = map_calamus_jar();
        let separate_mappings_for_build = self.separate_mappings_for_build();

        // run MapSpecializedMethodsCommand with the arguments
        //  calamus_jar, "tinyv2", separate_mappings_for_build, output
        // and then return `output`

        todo!()
    }

    fn v2_unmerged_feather_jar(&self) {
        // create a jar file called
        //  feather-FEATHERVERSION-v2.jar
        // with the file (in it)
        //  mappings/mappings.tiny
        // written from the output of the
        self.build_feather_tiny();
        // function
        // put that jar file into
        //  builds/libs
    }

    fn merge_v2(&self) {
        // take the output of
        self.build_feather_tiny();
        // and merge it with the output of
        invert_calamus_v2();
        // into the file
        //   TEMP/merged-v2.tiny
        // using CommandMergeTiny, with the commands:
        //  namespace, "official"

        // then as the output:
        // run CommandReorderTinyV2 on that file, producing the
        // output of this method. the arguments to it are:
        //  "official", namespace, "named"

        todo!("see mergeV2")
    }

    fn v2_merged_feather_jar(&self) {
        // take the output of
        self.merge_v2();
        // and store it in the jar file
        //  feather-FEATHERVERSION-mergedv2.jar
        // // TODO: ask space if that missing - is wanted: merged-v2
        // in that jar file, use the path
        //  mappings/mappings.tiny
        // and put the jar file to
        //  build/libs

        todo!("see v2MergedFeatherJar");
    }

    fn build(&self) {
        // take the outputs of these two
        self.v2_merged_feather_jar();
        self.v2_unmerged_feather_jar();
    }
}

fn merge_jars() {
    Downloader::mc_jars();

    // if the env is merged, call the JarMerger
    // with the following args
    //  clientJar, serverJar, mergedJar
    // and return mergedJar

    // note that clientJar and serverJar are set in downloadMcJars
    todo!()
}

fn map_calamus_jar() {
    // take the merged minecraft jar from
    merge_jars();
    // (take the client if client, server if server, this one if merged)
    // and map it with the mappings from
    Downloader::calamus();
    // and the libs from
    Downloader::mc_libs();
    // and the arguments
    //  "official", namespace
    // where you call mapJar
    // which just call tiny remapper in some way

    todo!("read mapJar code!")
}

fn invert_calamus_v2() {
    Downloader::calamus_v2();

    todo!("see invertCalamusV2")
}


fn main() -> Result<()> {
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

    let start = std::time::Instant::now();

    let mut iter = v.versions();
    let mut iter = versions.iter().map(|s| v.get_node(s).unwrap());

    for node in iter {
        fn try_version(v: &VersionGraph, node: NodeIndex) -> Result<()> {
            let version = v.get(node)?;

            let start = std::time::Instant::now();

            let build = Build::create(v, node)?;

            let elapsed = start.elapsed();
            println!("{version:?} elapsed: {elapsed:?}");

            if version.name == "12w30e-client" {
                let mut file = File::create("/tmp/12w30e-client.tiny").unwrap();
                build.mappings.write(&mut file).unwrap();
            }

            Ok(())
        }
        if let Err(e) = try_version(&v, node) {
            println!("{e:?}")
        }
    }

    let elapsed = start.elapsed();
    println!("elapsed: {elapsed:?}");

    Ok(())
}
