package dukemakemc.packet;

import java.io.Closeable;
import java.io.IOException;
import java.net.UnixDomainSocketAddress;
import java.nio.ByteBuffer;
import java.nio.channels.SocketChannel;
import java.nio.charset.StandardCharsets;

public class Connection implements Closeable {
	final SocketChannel socket;
	final ByteBuffer recvBuffer = ByteBuffer.allocate(4096);
	final ByteBuffer sendBuffer = ByteBuffer.allocate(4096);

	public Connection(String pathName) throws IOException {
		UnixDomainSocketAddress address = UnixDomainSocketAddress.of(pathName);
		this.socket = SocketChannel.open(address);
	}

	public String recvString() throws IOException {
		Packet packet = recvPacket();
		if (packet instanceof Packet.Message stringPacket) {
			return stringPacket.message();
		} else {
			throw new Error("got different packet " + packet);
		}
	}

	public void sendString(String message) throws IOException {
		this.sendPacket(new Packet.Message(message));
	}


	@Override
	public void close() throws IOException {
		this.socket.close();
	}

	// methods for recv

	public Packet recvPacket() throws IOException {
		return Packet.receive(this);
	}

	static String bufToString(ByteBuffer buffer) {
		return StandardCharsets.UTF_8.decode(buffer).toString();
	}

	int recvInt() throws IOException {
		int limit = this.recvBuffer.limit();
		this.recvBuffer.limit(this.recvBuffer.position() + 4);

		while (this.recvBuffer.hasRemaining())
			this.socket.read(this.recvBuffer);

		int value = this.recvBuffer.getInt(this.recvBuffer.position() - 4);
		this.recvBuffer.limit(limit);

		return value;
	}

	ByteBuffer recvBufferWithLen(int len) throws IOException {
		ByteBuffer recv = ByteBuffer.allocate(len);

		while (recv.hasRemaining())
			this.socket.read(recv);

		return recv.flip();
	}

	// methods for send
	public void sendPacket(Packet packet) throws IOException {
		packet.send(this);
	}

	static ByteBuffer stringToBuf(String string) {
		return StandardCharsets.UTF_8.encode(string);
	}

}

