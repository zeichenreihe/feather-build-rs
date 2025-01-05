package dukemakemc.util;

import java.io.IOException;
import java.io.OutputStream;

public class ByteBufferOutputStream extends OutputStream {
	private final ResizingByteBuffer buffer;

	public ByteBufferOutputStream(ResizingByteBuffer buffer) {
		this.buffer = buffer;
	}

	@Override
	public void write(int b) throws IOException {
		this.buffer.put((byte) b);
	}

	@Override
	public void write(byte[] src, int off, int len) throws IOException {
		this.buffer.put(src, off, len);
	}
}
