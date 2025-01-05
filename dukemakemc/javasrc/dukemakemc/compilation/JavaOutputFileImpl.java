package dukemakemc.compilation;

import dukemakemc.util.ByteBufferOutputStream;
import dukemakemc.util.FileObjectPath;
import dukemakemc.util.ResizingByteBuffer;

import javax.lang.model.element.Modifier;
import javax.lang.model.element.NestingKind;
import javax.tools.JavaFileObject;
import java.io.*;
import java.net.URI;
import java.nio.charset.StandardCharsets;

public record JavaOutputFileImpl(
		FileObjectPath loc,
		ResizingByteBuffer content
) implements JavaFileObject {
	// Equality here is only based on the path, not on the content.
	// It would be stupid to compare an immutable byte buffer with a large size every time.
	@Override
	public boolean equals(Object obj) {
		if (obj instanceof JavaOutputFileImpl(FileObjectPath otherLoc, ResizingByteBuffer ignored)) {
			return this.loc.equals(otherLoc);
		} else {
			return false;
		}
	}
	@Override
	public int hashCode() {
		return loc.hashCode();
	}

	@Override
	public Kind getKind() {
		return this.loc.kind();
	}

	@Override
	public boolean isNameCompatible(String simpleName, Kind kind) {
		return this.loc.isNameCompatible(simpleName, kind);
	}

	@Override
	public NestingKind getNestingKind() {
		//noinspection ReturnOfNull
		return null; // unknown nesting kind
	}

	@Override
	public Modifier getAccessLevel() {
		//noinspection ReturnOfNull
		return null; // unknown access level
	}

	@Override
	public URI toUri() {
		return this.loc.uri();
	}

	@Override
	public String getName() {
		return this.loc.userFriendlyName();
	}

	@Override
	public InputStream openInputStream() {
		throw new IllegalStateException("this is an output file");
	}

	@Override
	public OutputStream openOutputStream() {
		return new ByteBufferOutputStream(this.content);
	}

	@Override
	public Reader openReader(boolean ignoreEncodingErrors) {
		throw new IllegalStateException("this is an output file");
	}

	@Override
	public CharSequence getCharContent(boolean ignoreEncodingErrors) {
		throw new IllegalStateException("this is an output file");
	}

	@Override
	public Writer openWriter() {
		return new OutputStreamWriter(this.openOutputStream(), StandardCharsets.UTF_8);
	}

	@Override
	public long getLastModified() {
		return 0; // the operation is not supported
	}

	@Override
	public boolean delete() {
		return false; // we're not able to delete ourselves
	}
}
