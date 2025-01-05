package dukemakemc.util;

import java.net.URI;
import java.net.URISyntaxException;

public record UriBox(Scheme scheme, URI uri) {
	public static UriBox of(Scheme scheme, String host, String path) {
		URI uri;
		try {
			uri = new URI(scheme.schemeString, host, path, null);
		} catch (URISyntaxException e) {
			throw new RuntimeException(e);
		}

		return new UriBox(scheme, uri);
	}
	public enum Scheme {
		JAVA_INPUT_FILE("java-input-file"),
		JAVA_OUTPUT_FILE("java-output-file");

		private final String schemeString;

		Scheme(String schemeString) {
			this.schemeString = schemeString;
		}
	}
}
