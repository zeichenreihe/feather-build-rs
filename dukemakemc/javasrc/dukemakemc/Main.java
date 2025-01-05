package dukemakemc;

import dukemakemc.compilation.DiagnosticListenerImpl;
import dukemakemc.compilation.FileManagerImpl;
import dukemakemc.compilation.JavaInputFileImpl;
import dukemakemc.packet.Connection;
import dukemakemc.packet.Packet;
import dukemakemc.util.FileObjectPath;

import javax.tools.*;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.HashMap;

public class Main {
	public static void main(String[] args) {
		// we use a trick with an empty argument
		if (args.length == 3 && args[0].isEmpty() && "run".equals(args[1]) && !args[2].isEmpty()) {
			try {
				connectAndRun(args[2]);
				System.exit(0);
			} catch (Throwable t) {
				t.printStackTrace(System.err);
				System.exit(64);
			}
		} else {
			System.err.println("got the following arguments: " + Arrays.toString(args));
			System.err.println("this program compiles java class files if instructed so by a unix domain socket");
			System.exit(128);
		}
	}

	static void connectAndRun(String pathName) {
		try (Connection connection = new Connection(pathName)) {
			try {
				String msg = connection.recvString();

				System.out.println("java says \"" + msg + "\"");

				connection.sendString("greetings from java");

				Packet.MultiFile files = (Packet.MultiFile) connection.recvPacket();

				run(connection, files);
			} catch (Throwable throwable) {
				connection.sendPacket(new Packet.Crashed(throwable));
			} finally {
				connection.sendPacket(new Packet.Exit());
			}
		} catch (IOException e) {
			throw new RuntimeException("produced an exception", e);
		}
	}

	static void run(Connection connection, Packet.MultiFile files) throws IOException {
		HashMap<FileObjectPath, JavaInputFileImpl> inputs = new HashMap<>();
		for (Packet.File file : files.files()) {
			var fop = FileObjectPath.inputFile(file.name());
			var input = new JavaInputFileImpl(fop, file.content());
			inputs.put(fop, input);
		}

		DiagnosticListenerImpl diagnosticListener = new DiagnosticListenerImpl();

		JavaCompiler javac = ToolProvider.getSystemJavaCompiler();
		StandardJavaFileManager javacStandardFileManager = javac.getStandardFileManager(
				diagnosticListener,
				null, // use default locale
				StandardCharsets.UTF_8
		);

		try (
			FileManagerImpl fileManager = new FileManagerImpl(javacStandardFileManager, inputs);
			DiagnosticListenerImpl.DiagnosticForwardingWriter diagnosticListenerWriter =
					new DiagnosticListenerImpl.DiagnosticForwardingWriter(diagnosticListener);
		) {
			Iterable<? extends JavaFileObject> compilationUnits = fileManager.inputs();

			Iterable<String> options = null; // TODO: this needs input

			JavaCompiler.CompilationTask task = javac.getTask(
					diagnosticListenerWriter,
					fileManager,
					diagnosticListener,
					options,
					null,
					compilationUnits
			);

			if (task.call()) {
				System.out.println("success");

				fileManager.debugResults();

				connection.sendPacket(new Packet.Message("success"));
				//Packet.File back = new Packet.File("name back", output);
				//connection.sendPacket(back);

			} else {
				System.out.println("failure");
			}
		}
	}

	public static void stop() {
		throw new RuntimeException("stop for debug reasons");
	}
}
