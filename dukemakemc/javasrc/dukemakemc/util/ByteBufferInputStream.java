package dukemakemc.util;

import java.io.InputStream;
import java.nio.ByteBuffer;
import java.util.Objects;

/**
 * Calling read on this input stream moves the position in the buffer!
 * So call {@link ByteBuffer#asReadOnlyBuffer()} to avoid this.
 */
public class ByteBufferInputStream extends InputStream {
	private final ByteBuffer buffer;
	public ByteBufferInputStream(ByteBuffer buffer) {
		this.buffer = buffer;
	}

	@Override
	public int available() {
		return this.buffer.remaining();
	}

	@Override
	public int read() {
		return this.buffer.hasRemaining()
				? (this.buffer.get() & 0xFF)
				: -1;
	}

	@Override
	public int read(byte[] b, int off, int len) {
		Objects.checkFromIndexSize(off, len, b.length);
		int amount = Math.min(len, this.buffer.remaining());
		this.buffer.get(b, off, amount);
		return amount;
	}
}
