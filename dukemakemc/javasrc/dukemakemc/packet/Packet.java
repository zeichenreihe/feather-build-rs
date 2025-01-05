package dukemakemc.packet;

import dukemakemc.util.ThrowableHelper;

import java.io.IOException;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;

public sealed interface Packet permits
		Packet.Crashed,
		Packet.Exit,
		Packet.File,
		Packet.MultiFile,
		Packet.Message
{
	int CRASHED = 4;
	int EXIT = 0;
	int FILE = 1;
	int MULTI_FILE = 2;
	int MESSAGE = 3;

	void debug();
	void send(Connection connection) throws IOException;
	static Packet receive(Connection connection) throws IOException {
		connection.recvBuffer.clear();

		// TODO: be smarter about read calls!
		int type = connection.recvInt();

		return switch (type) {
			case Packet.CRASHED -> Packet.Crashed.receive(connection);
			case Packet.EXIT -> Packet.Exit.receive(connection);
			case Packet.FILE -> Packet.File.receive(connection);
			case Packet.MULTI_FILE -> Packet.MultiFile.receive(connection);
			case Packet.MESSAGE -> Packet.Message.receive(connection);
			default -> throw new Error("unknown packet id " + type);
		};
	}

	record Crashed(Throwable throwable) implements Packet {

		@Override
		public void debug() {
			System.out.println("###### Packet.Crashed Start ######");
			this.throwable.printStackTrace(System.out);
			System.out.println("###### Packet.Crashed End ######");
		}

		@Override
		public void send(Connection connection) throws IOException {
			ByteBuffer messageBuffer = Connection.stringToBuf(
					ThrowableHelper.throwableToString(this.throwable));

			connection.sendBuffer.clear()
					.putInt(Packet.CRASHED)
					.putInt(messageBuffer.remaining())
					.flip();

			while (connection.sendBuffer.hasRemaining())
				connection.socket.write(connection.sendBuffer);

			while (messageBuffer.hasRemaining())
				connection.socket.write(messageBuffer);
		}

		static Packet.Crashed receive(Connection connection) throws IOException {
			throw new IllegalStateException("a crashed packet is only meant to be send from java, never received");
		}
	}

	record Exit() implements Packet {
		@Override
		public void debug() {
			System.out.println("###### Packet.Exit ######");
		}

		@Override
		public void send(Connection ignored) throws IOException {
			// no data
		}

		static Packet.Exit receive(Connection ignored) {
			// no data
			return new Packet.Exit();
		}
	}

	record File(
			String name,
			ByteBuffer content
	) implements Packet {
		@Override
		public void debug() {
			System.out.println("###### Packet.File Start ######");
			System.out.println("###### name: \"" + this.name + "\" ######");
			System.out.println(this.contentsAsString());
			System.out.println("###### Packet.File End ######");
		}

		@Override
		public void send(Connection connection) throws IOException {
			ByteBuffer nameBuffer = Connection.stringToBuf(this.name);
			ByteBuffer contentBuffer = this.content.asReadOnlyBuffer();

			connection.sendBuffer.clear()
					.putInt(Packet.FILE)
					.putInt(nameBuffer.remaining())
					.putInt(contentBuffer.remaining())
					.flip();

			while (connection.sendBuffer.hasRemaining())
				connection.socket.write(connection.sendBuffer);

			while (nameBuffer.hasRemaining())
				connection.socket.write(nameBuffer);

			while (contentBuffer.hasRemaining())
				connection.socket.write(contentBuffer);
		}

		static Packet.File receive(Connection connection) throws IOException {
			int nameLen = connection.recvInt();
			int fileLen = connection.recvInt();

			ByteBuffer nameBuffer = connection.recvBufferWithLen(nameLen);
			ByteBuffer contentBuffer = connection.recvBufferWithLen(fileLen);

			String name = Connection.bufToString(nameBuffer);

			return new Packet.File(name, contentBuffer);
		}

		public String contentsAsString() {
			return StandardCharsets.UTF_8.decode(this.content.asReadOnlyBuffer()).toString();
		}

		public ByteBuffer content() {
			return this.content.asReadOnlyBuffer();
		}
	}

	record MultiFile(ArrayList<File> files) implements Packet {
		@Override
		public void debug() {
			System.out.println("###### Packet.MultiFile Start ######");
			for (Packet.File file : this.files) {
				System.out.println("###### name: \"" + file.name() + "\" ######");
				System.out.println(file.contentsAsString());
			}
			System.out.println("###### Packet.MultiFile End ######");
		}

		@Override
		public void send(Connection connection) throws IOException {
			connection.sendBuffer.clear()
					.putInt(Packet.MULTI_FILE)
					.putInt(this.files.size())
					.flip();

			while (connection.sendBuffer.hasRemaining())
				connection.socket.write(connection.sendBuffer);

			for (Packet.File file : this.files) {
				ByteBuffer nameBuffer = Connection.stringToBuf(file.name());
				ByteBuffer contentBuffer = file.content();

				connection.sendBuffer.clear()
						.putInt(nameBuffer.remaining())
						.putInt(contentBuffer.remaining())
						.flip();

				while (connection.sendBuffer.hasRemaining())
					connection.socket.write(connection.sendBuffer);

				while (nameBuffer.hasRemaining())
					connection.socket.write(nameBuffer);

				while (contentBuffer.hasRemaining())
					connection.socket.write(contentBuffer);
			}
		}

		static Packet.MultiFile receive(Connection connection) throws IOException {
			int len = connection.recvInt();

			ArrayList<Packet.File> list = new ArrayList<>(len);
			for (int i = 0; i < len; i++) {

				int nameLen = connection.recvInt();
				int fileLen = connection.recvInt();

				ByteBuffer nameBuffer = connection.recvBufferWithLen(nameLen);
				ByteBuffer contentBuffer = connection.recvBufferWithLen(fileLen);

				String name = Connection.bufToString(nameBuffer);

				Packet.File filePacket = new Packet.File(name, contentBuffer);

				list.add(filePacket);
			}

			return new Packet.MultiFile(list);
		}
	}

	record Message(String message) implements Packet {
		@Override
		public void debug() {
			System.out.println("###### Packet.Message Start ######");
			System.out.println(this.message);
			System.out.println("###### Packet.Message End ######");
		}

		@Override
		public void send(Connection connection) throws IOException {
			ByteBuffer messageBuffer = Connection.stringToBuf(this.message);

			connection.sendBuffer.clear()
					.putInt(Packet.MESSAGE)
					.putInt(messageBuffer.remaining())
					.flip();

			while (connection.sendBuffer.hasRemaining())
				connection.socket.write(connection.sendBuffer);

			while (messageBuffer.hasRemaining())
				connection.socket.write(messageBuffer);
		}

		static Packet.Message receive(Connection connection) throws IOException {
			int len = connection.recvInt();

			ByteBuffer messageBuffer = connection.recvBufferWithLen(len);

			String message = Connection.bufToString(messageBuffer);

			return new Packet.Message(message);
		}
	}
}

