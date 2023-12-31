use std::io::Cursor;
use std::path::Path;
use anyhow::Result;
use crate::download::Downloader;
use crate::tree::mappings::Mappings;
use crate::version_graph::{Version, VersionGraph};

mod reader;
mod version_graph;
mod writer;
mod download;
mod tree;

#[derive(Debug)]
struct Jar;

#[derive(Debug, PartialEq)]
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

    fn parse(id: &Version) -> Environment {
        if id.0.ends_with("-client") {
            return Environment::Client;
        }
        if id.0.ends_with("-server") {
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

#[derive(Debug)]
struct Build {
    version: Version,
    mappings: Mappings<2>,
}

impl Build {
    fn create(version_graph: &VersionGraph, version: &Version) -> Result<Build> {
        let mut mappings = version_graph.apply_diffs(version)?;

        mappings.remove_dummy("named")?;

        Ok(Build {
            version: version.clone(),
            mappings,
        })
    }

    async fn build_feather_tiny(&self, downloader: &mut Downloader) -> Result<Mappings<2>> {
        let main_jar = self.main_jar(downloader).await?;
        let mappings = downloader.calamus_v2(&self.version).await?;
        let libraries = downloader.mc_libs(&self.version).await?;
        /*
        static void mapJar(Path input, File mappings, File DIR libraries, String from, String to) -> File output {
            let output = ...;

            def remapper = TinyRemapper.newRemapper()
                    .withMappings(TinyUtils.createTinyMappingProvider(mappings.toPath(), "official", "intermediary"))
                    .renameInvalidLocals(true)
                    .rebuildSourceFilenames(true)
                    .build()

            def outputConsumer = new OutputConsumerPath.Builder(output.toPath()).build()
            outputConsumer.addNonClassFiles(input)
            remapper.readInputs(input)

            libraries.eachFileRecurse(FileType.FILES) { file ->
                remapper.readClassPath(file.toPath())
            }
            remapper.apply(outputConsumer)
            outputConsumer.close()
            remapper.finish()

            return output;
        }
        calamusJar = mapJar(mainJar, downloadCalamus(v2).dest, libraries, "official", "intermediary")
        new MapSpecializedMethodsCommand().run([calamusJar, "tinyv2", self.mappings, "tinyv2:intermediary:named", _return_this_])
		*/
        // TODO: impl
        Ok(self.mappings.clone())
    }

    async fn v2_unmerged_feather_jar(&self, downloader: &mut Downloader) -> Result<Jar> {
        let mappings = self.build_feather_tiny(downloader).await?;

        // TODO: put this in a "jar" somehow

        let mut buf = Vec::new();

        writer::tiny_v2::write_zip_file(&mappings, &mut Cursor::new(&mut buf))?;

        let feather_version = "0.0.0";
        let file_name = format!("feather-{feather_version}-v2.jar");

        Ok(Jar)
    }

    async fn invert_calamus_v2(&self, downloader: &mut Downloader) -> Result<Mappings<2>> {
        downloader.calamus_v2(&self.version)
            .await?
            .reorder(["intermediary", "official"])
    }

    async fn merge_v2(&self, downloader: &mut Downloader) -> Result<Mappings<3>> {
        let mut mappings_a = self.build_feather_tiny(downloader).await?;
        let mappings_b = self.invert_calamus_v2(downloader).await?;

        mappings_a.rename_namespaces(["calamus", "named"], ["intermediary", "named"])?;

        let merged = Mappings::merge(&mappings_b, &mappings_a)?;

        let output = merged.reorder(["official", "intermediary", "named"])?;

        Ok(output)
    }

    async fn v2_merged_feather_jar(&self, downloader: &mut Downloader) -> Result<Jar> {
        let merge_v2 = self.merge_v2(downloader).await?;

        // TODO: put this in a "jar" somehow

        let mut buf = Vec::new();

        writer::tiny_v2::write_zip_file(&merge_v2, &mut Cursor::new(&mut buf))?;

        let feather_version = "0.0.0";
        let file_name = format!("feather-{feather_version}-mergedv2.jar"); // TODO: ask space if that missing - is wanted: merged-v2

        Ok(Jar)
    }

    async fn build(&self, downloader: &mut Downloader) -> Result<()> {
        self.v2_merged_feather_jar(downloader).await?;
        self.v2_unmerged_feather_jar(downloader).await?;
        // TODO: impl: be done with the results from both calls above!
        Ok(())
    }

    async fn main_jar(&self, downloader: &mut Downloader) -> Result<Jar> {
        let environment = Environment::parse(&self.version);

        let version_details = downloader.version_details(&self.version, &environment).await?;

        match environment {
            Environment::Merged => {
                let client = downloader.get_jar(&version_details.downloads.client.url).await?;
                let server = downloader.get_jar(&version_details.downloads.server.url).await?;

                // TODO: merge the jars
                // like this:
                //   def jarMerger = new JarMerger(clientJar, serverJar, mergedJar)
                //   jarMerger.merge()
                //   jarMerger.close()
                // and return mergedJar
                // note that clientJar and serverJar are set in downloadMcJars

                let jar = Jar; // merge here

                Ok(jar)
            },
            Environment::Client => {
                let url = &version_details.downloads.client.url;

                downloader.get_jar(&url).await
            },
            Environment::Server => {
                let url = &version_details.downloads.server.url;

                downloader.get_jar(&url).await
            },
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut downloader = Downloader::new();

    let dir = Path::new("mappings/mappings");

    let start = std::time::Instant::now();

    let v = VersionGraph::resolve(dir)?;

    let elapsed = start.elapsed();
    println!("version graph reading took: {elapsed:?}");

    let version = v.get("1.12.2").unwrap();

    let build = Build::create(&v, version)?;
    build.build(&mut downloader).await?;

    Ok(())
}