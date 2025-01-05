package dukemakemc.util;

import java.nio.ByteBuffer;

public class ResizingByteBuffer {
	private ByteBuffer buffer;

	public ResizingByteBuffer(int capacity) {
		this.buffer = ByteBuffer.allocate(capacity);
	}

	private void ensureSpace(int needed) {
		if (this.buffer.remaining() < needed) {
			int currentCapacity = this.buffer.capacity();
			int capacity = Math.max(2 * currentCapacity, currentCapacity + needed);
			this.buffer = ByteBuffer.allocate(capacity).put(this.buffer.flip());
		}
	}

	public void put(byte b) {
		ensureSpace(1);
		this.buffer.put(b);
	}

	public void put(byte[] src, int off, int len) {
		ensureSpace(len);
		this.buffer.put(src, off, len);
	}

	public String toString() {
		return this.getClass().getName() + "[pos=" + this.buffer.position()
				+ " lim=" + this.buffer.limit()
				+ " cap=" + this.buffer.capacity()
				+ "]";
	}

	/**
	 * Note that other locations could still mutate this buffer.
	 * @return a {@link ByteBuffer} created by {@link ByteBuffer#asReadOnlyBuffer()}
	 */
	public ByteBuffer getBuffer() {
		return this.buffer.asReadOnlyBuffer().flip();
	}
}
