package dukemakemc.util;

import javax.tools.JavaFileManager;
import javax.tools.JavaFileObject;
import java.net.URI;

public record FileObjectPath(
		String path,
		URI uri
) {
	public static FileObjectPath inputFile(String path) {
		String pathWithSlash = path.startsWith("/") ? path : "/" + path;
		UriBox uriBox = UriBox.of(UriBox.Scheme.JAVA_INPUT_FILE, "foo", pathWithSlash);
		return new FileObjectPath(path, uriBox.uri());
	}
	public static FileObjectPath outputFile(JavaFileManager.Location location, String className, JavaFileObject.Kind kind) {
		String host = UriHelper.makeHostSafe(location.getName());

		String path = className.replace('.', '/') + kind.extension;
		String pathWithSlash = path.startsWith("/") ? path : "/" + path;

		UriBox uriBox = UriBox.of(UriBox.Scheme.JAVA_OUTPUT_FILE, host, pathWithSlash);
		return new FileObjectPath(path, uriBox.uri());
	}

	public JavaFileObject.Kind kind() {
		for (JavaFileObject.Kind k : JavaFileObject.Kind.values())
			if (this.path.endsWith(k.extension))
				return k;
		return JavaFileObject.Kind.OTHER;
	}

	public String userFriendlyName() {
		return this.path;
	}

	public boolean isNameCompatible(String simpleName, JavaFileObject.Kind kind) {
		// impl from SimpleJavaFileObject
		String baseName = simpleName + kind.extension;
		return kind.equals(kind())
				&& (baseName.equals(uri().getPath())
				|| uri().getPath().endsWith("/" + baseName));
	}
}
