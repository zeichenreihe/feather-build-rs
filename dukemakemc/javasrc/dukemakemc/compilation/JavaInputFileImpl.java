package dukemakemc.compilation;

import dukemakemc.util.ByteBufferInputStream;
import dukemakemc.util.CharBufferReader;
import dukemakemc.util.FileObjectPath;

import javax.lang.model.element.Modifier;
import javax.lang.model.element.NestingKind;
import javax.tools.JavaFileObject;
import java.io.*;
import java.net.URI;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;

public record JavaInputFileImpl(
		FileObjectPath loc,
		ByteBuffer content
) implements JavaFileObject {
	// Equality here is only based on the path, not on the content.
	// It would be stupid to compare an immutable byte buffer with a large size every time.
	@Override
	public boolean equals(Object obj) {
		if (obj instanceof JavaInputFileImpl(FileObjectPath otherLoc, ByteBuffer ignored)) {
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
		return new ByteBufferInputStream(this.content.asReadOnlyBuffer());
	}

	@Override
	public OutputStream openOutputStream() {
		throw new IllegalStateException("this is an input file");
	}

	@Override
	public Reader openReader(boolean ignoreEncodingErrors) {
		return new CharBufferReader(StandardCharsets.UTF_8.decode(this.content.asReadOnlyBuffer()));
	}

	@Override
	public CharSequence getCharContent(boolean ignoreEncodingErrors) {
		return StandardCharsets.UTF_8.decode(this.content.asReadOnlyBuffer());
	}

	@Override
	public Writer openWriter() {
		throw new IllegalStateException("this is an input file");
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
