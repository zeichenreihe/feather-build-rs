use std::fs::File;
use std::io::{BufReader, Cursor, Read};
use std::path::Path;
use anyhow::{anyhow, bail, Context, Result};
use bytes::Buf;
use petgraph::dot::Dot;
use petgraph::graph::NodeIndex;
use zip::read::ZipFile;
use zip::ZipArchive;
use crate::download::version_details::VersionDetails;
use crate::download::version_manifest::VersionManifest;
use crate::download::versions_manifest::VersionsManifest;
use crate::tiny::RemoveDummy;
use crate::tiny::tree::Mappings;
use crate::tree::mappings::TinyV2Mappings;
use crate::version_graph::{Version, VersionGraph};

mod tiny;
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

struct Downloader {
    versions_manifest: Option<VersionsManifest>,
}

impl Downloader {
    fn new() -> Downloader {
        Downloader {
            versions_manifest: None,
        }
    }
    async fn versions_manifest(&mut self) -> Result<&VersionsManifest> {
        if let Some(ref version_manifest) = self.versions_manifest {
            Ok(version_manifest)
        } else {
            let url = "https://skyrising.github.io/mc-versions/version_manifest.json";

            let body = download::get(&url).await?;

            let versions_manifest: VersionsManifest = serde_json::from_str(&body)?;

            Ok(self.versions_manifest.insert(versions_manifest))
        }
    }

    async fn wanted_version_manifest(&mut self, version: &Version) -> Result<VersionManifest> {
        let manifest = self.versions_manifest().await?;

        let manifest_version = manifest
            .versions
            .iter()
            .find(|it| &it.id == version);

        if let Some(manifest_version) = manifest_version {
            let url = &manifest_version.url;

            let body = download::get(&url).await?;

            let version_manifest: VersionManifest = serde_json::from_str(&body)
                .with_context(|| anyhow!("Failed to parse version manifest for version {:?} from {:?}", version, url))?;

            Ok(version_manifest)
        } else {
            bail!("No version data for Minecraft version {:?}", version);
        }
    }
    async fn version_details(&mut self, version: &Version, environment: &Environment) -> Result<VersionDetails> {
        let manifest = self.versions_manifest().await?;

        let manifest_version = manifest
            .versions
            .iter()
            .find(|it| &it.id == version);

        if let Some(manifest_version) = manifest_version {
            let url = &manifest_version.details;

            let body = download::get(&url).await?;

            let version_details: VersionDetails = serde_json::from_str(&body)
                .with_context(|| anyhow!("Failed to parse version details for version {:?} from {:?}", version, url))?;

            if version_details.shared_mappings {
                if &Environment::Merged != environment {
                    bail!("Minecraft version {:?} is only available as merged but was requested for {:?}", version, environment);
                }
            } else {
                match environment {
                    Environment::Merged => {
                        bail!("Minecraft version {:?} cannot be merged - please select either the client or server environment!", version);
                    },
                    Environment::Client if !version_details.client => {
                        bail!("Minecraft version {:?} does not have a client jar!", version);
                    },
                    Environment::Server if !version_details.server => {
                        bail!("Minecraft version {:?} does not have a server jar", version);
                    },
                    _ => {},
                }
            }

            Ok(version_details)
        } else {
            bail!("No version details for Minecraft version {:?}", version);
        }
    }

    #[deprecated]
    async fn calamus(&mut self, version: &Version) -> Result<String> {
        let url = format!("https://github.com/OrnitheMC/calamus/raw/main/mappings/{}.tiny", version.0);

        let body = download::get(&url).await?;

        Ok(body)
    }
    async fn calamus_v2(&mut self, version: &Version) -> Result<Mappings> {
        let url = format!("https://maven.ornithemc.net/releases/net/ornithemc/calamus-intermediary/{}/calamus-intermediary-{}-v2.jar", version.0, version.0);

        let response = reqwest::get(&url)
            .await?;

        if !response.status().is_success() {
            bail!("Got a \"{}\" for {:?}", response.status(), &url);
        }

        let body: &[u8] = &response.bytes().await?;

        let reader = Cursor::new(body);

        let mut zip = ZipArchive::new(reader)?;

        let mappings = zip.by_name("mappings/mappings.tiny")
            .with_context(|| anyhow!("Cannot find mappings in zip file from {:?}", url))?;

        tiny::v2::read(mappings)
            .with_context(|| anyhow!("Failed to read mappings from mappings/mappings.tiny of {:?}", url))
    }
    async fn mc_libs(&mut self, version: &Version) -> Result<Vec<Jar>> {
        let version_file = self.wanted_version_manifest(version).await?;

        let mut libs = Vec::new();

        for lib in version_file.libraries {
            if let Some(artifact) = lib.downloads.artifact {
                let url = &artifact.url;

                let jar = self.get_jar(&url).await?;

                libs.push(jar);
            }
        }

        Ok(libs)
    }

    async fn get_jar(&mut self, url: &str) -> Result<Jar> {
        // TODO: download + cache jar


        // from libs:
        // to a file given by the
        // part after the last slash into a libraries folder (ensuring that we don't overwrite a file)

        Ok(Jar)
    }
}

struct Build {
    version: Version,
    mappings: TinyV2Mappings,
}

impl Build {
    fn create(version_graph: &VersionGraph, node: NodeIndex) -> Result<Build> {
        let mut mappings = version_graph.apply_diffs(node)?;

        mappings.remove_dummy();

        let version = version_graph.get(node)?.clone();

        Ok(Build {
            version,
            mappings,
        })
    }

    async fn build_feather_tiny(&self) -> Result<TinyV2Mappings> {
        let calamus_jar = self.map_calamus_jar().await?;
        let separate_mappings_for_build = &self.mappings;

        // run MapSpecializedMethodsCommand with the arguments
        //  calamus_jar, "tinyv2", separate_mappings_for_build, output
        // and then return `output`

        // TODO: impl

        /*
		new MapSpecializedMethodsCommand().run(
				calamusJar.getAbsolutePath(),
				"tinyv2",
				separateMappingsForBuild.v2Output.getAbsolutePath(), // impl via field read
				"tinyv2:intermediary:named",
				v2Output.getAbsolutePath()
		)

         */

        Ok(self.mappings.clone()) // TODO: impl
    }

    async fn v2_unmerged_feather_jar(&self) -> Result<()> {
        // create a jar file called
        //  feather-FEATHERVERSION-v2.jar
        // with the file (in it)
        //  mappings/mappings.tiny
        // written from the output of the
        let mappings = self.build_feather_tiny().await?;
        // function
        // put that jar file into
        //  builds/libs
        // TODO: impl

        /*

task v2UnmergedFeatherJar(dependsOn: buildFeatherTiny, type: Jar) {
	def mappings = buildFeatherTiny.v2Output
	group = "mapping build"
	outputs.upToDateWhen { false }
	archiveFileName = "feather-${featherVersion}-v2.jar"

	from(file(mappings)) {
		rename mappings.name, "mappings/mappings.tiny"
	}
	destinationDirectory.set(file("build/libs"))
}
         */

        Ok(())
    }

    async fn merge_v2(&self) -> Result<()> {
        // take the output of
        self.build_feather_tiny().await?;
        // and merge it with the output of
        self.invert_calamus_v2().await?;
        // into the file
        //   TEMP/merged-v2.tiny
        // using CommandMergeTiny, with the commands:
        //  namespace, "official"

        // then as the output:
        // run CommandReorderTinyV2 on that file, producing the
        // output of this method. the arguments to it are:
        //  "official", namespace, "named"

        // TODO: impl

        /*

task mergeV2(dependsOn: ["v2UnmergedFeatherJar", "invertCalamusV2"], type: FileOutput) {
	def mergedV2 = new File(tempDir, "merged-v2.tiny")

	output = new File(tempDir, "merged-reordered-v2.tiny")
	outputs.upToDateWhen { false }

	doLast {
		logger.lifecycle(":merging feather and calamus v2")
		String[] args = [
				invertCalamusV2.output.getAbsolutePath(),
				// we can use this, since v2UnmergedFeatherJar depends on buildFeatherTiny
				buildFeatherTiny.v2Output.getAbsolutePath(),
				mergedV2.getAbsolutePath(),
				namespace,
				"official"
		]

		new CommandMergeTinyV2().run(args)

		//Reorder the mappings to match the output of loom
		args = [
				mergedV2.getAbsolutePath(),
				output.getAbsolutePath(),
				"official",
				namespace,
				"named"
		]
		new CommandReorderTinyV2().run(args)
	}
}
         */

        Ok(())
    }

    async fn v2_merged_feather_jar(&self) -> Result<()> {
        // take the output of
        self.merge_v2().await?;
        // and store it in the jar file
        //  feather-FEATHERVERSION-mergedv2.jar
        // // TODO: ask space if that missing - is wanted: merged-v2
        // in that jar file, use the path
        //  mappings/mappings.tiny
        // and put the jar file to
        //  build/libs

        // TODO: impl
        /*

task v2MergedFeatherJar(dependsOn: ["mergeV2"], type: Jar) {
	def mappings = mergeV2.output
	group = "mapping build"
	outputs.upToDateWhen { false }
	archiveFileName = "feather-${featherVersion}-mergedv2.jar"

	from(file(mappings)) {
		rename mappings.name, "mappings/mappings.tiny"
	}
	destinationDirectory.set(file("build/libs"))
}
         */

        Ok(())
    }

    async fn build(&self) -> Result<()> {
        // take the outputs of these two
        self.v2_merged_feather_jar().await?;
        self.v2_unmerged_feather_jar().await?;

        Ok(())
    }

    async fn main_jar(&self) -> Result<Jar> {
        let mut downloader = Downloader::new();

        let environment = Environment::parse(&self.version);

        let version_details = downloader.version_details(&self.version, &environment).await?;

        match environment {
            Environment::Merged => {
                let url = &version_details.downloads.client.url;

                let client = downloader.get_jar(&url).await?;

                let url = &version_details.downloads.server.url;

                let server = downloader.get_jar(&url).await?;

                // TODO: merge the jars
                // call the JarMerger
                // with the following args
                //  clientJar, serverJar, mergedJar
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

    async fn map_calamus_jar(&self) -> Result<Jar> {
        let mut downloader = Downloader::new();

        let main_jar = self.main_jar().await?;

        // and map it with the mappings from
        let mappings = downloader.calamus_v2(&self.version).await?;
        // and the libs from
        let libraries = downloader.mc_libs(&self.version).await?;
        // and the arguments
        //  "official", "intermediary"
        // where you call mapJar
        // which just call tiny remapper in some way

        /*
        // TODO: impl
        mapJar(calamusJar, mainJar, downloadCalamus.dest, libraries, "official", "intermediary")
         */

        let map_jar = || {

            /*
	static void mapJar(File output, File input, File mappings, File DIR libraries, String from, String to) {

		def remapper = TinyRemapper.newRemapper()
				.withMappings(TinyUtils.createTinyMappingProvider(mappings.toPath(), "official", "intermediary"))
				.renameInvalidLocals(true)
				.rebuildSourceFilenames(true)
				.build()

		try {
			def outputConsumerBuilder = new OutputConsumerPath.Builder(output.toPath())
			def outputConsumer = outputConsumerBuilder.build()
			outputConsumer.addNonClassFiles(input.toPath())
			remapper.readInputs(input.toPath())

	*/
            for library in libraries {
                /*
                remapper.readClassPath(library)
                 */
            }
            /*
			remapper.apply(outputConsumer)
			outputConsumer.close()
			remapper.finish()
		} catch (Exception e) {
			remapper.finish()
			throw new RuntimeException("Failed to remap jar", e)
		}
	}
			 */
        };

        Ok(Jar)
    }

    async fn invert_calamus_v2(&self) -> Result<()> {
        let mut downloader = Downloader::new();

        let mappings = downloader.calamus_v2(&self.version).await?;

        // TODO: impl
        /*


task invertCalamusV2(dependsOn: downloadCalamusV2, type: FileOutput) {
	group = buildMappingGroup
	def v2Input = new File(mappingsCacheDir, "${version_id}-calamus-v2.tiny")

	output = new File(mappingsCacheDir, "${version_id}-calamus-inverted-v2.tiny")
	outputs.file(output)

	outputs.upToDateWhen { false }

	doLast {
		logger.lifecycle(":building inverted calamus v2")

		String[] v2Args = [
				v2Input.getAbsolutePath(),
				output.getAbsolutePath(),
				namespace, "official"
		]

		new CommandReorderTinyV2().run(v2Args)
	}
}
         */

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut downloader = Downloader::new();

    let dir = Path::new("mappings/mappings");

    let start = std::time::Instant::now();

    let v = VersionGraph::resolve(dir)?;

    let elapsed = start.elapsed();
    println!("elapsed: {elapsed:?}");

    let node = v.get_node("1.12.2").unwrap();

    let build = Build::create(&v, node)?;
    build.build().await?;

    Ok(())
}