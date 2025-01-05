package dukemakemc.util;

import java.io.IOException;
import java.io.Reader;
import java.nio.CharBuffer;
import java.util.Objects;

/**
 * Calling read on this reader moves the position in the buffer!
 * So call {@link CharBuffer#slice()} to avoid this.
 */
public class CharBufferReader extends Reader {
	private final CharBuffer buffer;
	public CharBufferReader(CharBuffer buffer) {
		this.buffer = buffer;
	}

	@Override
	public int read() {
		return this.buffer.hasRemaining() ? this.buffer.get() : -1;
	}

	@Override
	public int read(char[] chars, int off, int len) {
		Objects.checkFromIndexSize(off, len, chars.length);
		int amount = Math.min(len, this.buffer.remaining());
		this.buffer.get(chars, off, amount);
		return amount;
	}

	@Override
	public void close() throws IOException {

	}
}
