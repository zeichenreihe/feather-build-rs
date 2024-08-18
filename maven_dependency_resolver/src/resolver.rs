use std::borrow::Cow;
use anyhow::{anyhow, bail, Context, Result};
use std::future::Future;
use log::trace;
use crate::coord::MavenCoord;
use crate::Downloader;
use crate::maven_pom::MavenPom;

/// Represents a maven repository.
#[derive(Debug, Clone, PartialEq)]
pub struct Resolver<'a> {
	pub name: Cow<'a, str>,
	/// The url of the maven repo.
	///
	/// Note that this may or may not end with a `/`. Care must be taken when using this value.
	pub maven: Cow<'a, str>,
}

impl Resolver<'_> {
	pub const fn new<'a>(name: &'a str, maven: &'a str) -> Resolver<'a> {
		Resolver { name: Cow::Borrowed(name), maven: Cow::Borrowed(maven) }
	}
}

/// Tries the given resolvers until one returns `Some(_)`.
pub(crate) async fn try_resolvers<'a, T, F: Future<Output = Result<Option<T>>>>(
	resolvers: &'a [Resolver<'a>],
	url_maker: impl Fn(&Resolver) -> String,
	downloader: impl Fn(String) -> F, // with HKT we'd use &str here, and we wouldn't need the .clone() below...
) -> Result<(&'a Resolver<'a>, T)> {
	for resolver in resolvers {
		let url = url_maker(resolver);

		trace!("trying resolver {:?} with {url:?}", resolver.name);
		if let Some(x) = downloader(url.clone()).await.with_context(|| anyhow!("failed to get artifact from {url:?}"))? {
			trace!("success");
			return Ok((resolver, x));
		} else {
			// try next resolver
		}
	}
	bail!("no file from any provider")
}

fn make_metadata_url(maven: &str, group: &str, artifact: &str) -> String {
	format!("{maven}{maven_slash}{group}/{artifact}/maven-metadata.xml",
		maven_slash = if maven.ends_with('/') { "" } else { "/" },
		group = group.replace('.', "/")
	)
}

pub(crate) async fn try_get_pom_for<'a>(downloader: &impl Downloader, resolvers: &'a [Resolver<'a>], coord: &MavenCoord)
		-> Result<(&'a Resolver<'a>, MavenPom)> {
	try_resolvers(
		resolvers,
		|resolver| coord.make_pom_url(resolver),
		|url| async move {
			downloader.get_maven_pom(&url).await?
				.map(|pom| {
					if pom.model_version == "4.0.0" {
						Ok(pom)
					} else {
						bail!("expected maven pom with `model_version=4.0.0`, got {:?}", pom.model_version)
					}
				})
				.transpose()
		}
	).await
}

#[cfg(test)]
mod testing {
	use pretty_assertions::assert_eq;
	use crate::Resolver;

	/*#[test]
	fn resolver_appends_missing_slash() {
		assert_eq!(Resolver::new("test", "https://maven.example.org").maven, "https://maven.example.org/");
		assert_eq!(Resolver::new("test", "https://maven.example.org/").maven, "https://maven.example.org/");
	}*/
	// TODO: tests?
}