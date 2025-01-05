package dukemakemc.compilation;

import javax.tools.Diagnostic;
import javax.tools.DiagnosticListener;
import javax.tools.JavaFileObject;
import java.io.IOException;
import java.util.Arrays;
import java.util.Locale;

public class DiagnosticListenerImpl implements DiagnosticListener<JavaFileObject> {

	@Override
	public void report(Diagnostic<? extends JavaFileObject> diagnostic) {
		// TODO: impl
		System.out.println("from report: " + diagnostic.getMessage(Locale.getDefault()));
	}

	private void fromForwardingWriter(char[] input) {
		// TODO: impl
		System.out.println("from forwarding writer: " + new String(input));
	}

	public static class DiagnosticForwardingWriter extends java.io.Writer {

		private final DiagnosticListenerImpl diagnosticListener;

		public DiagnosticForwardingWriter(DiagnosticListenerImpl diagnosticListener) {
			this.diagnosticListener = diagnosticListener;
		}

		@Override
		public void write(char[] buf, int off, int len) throws IOException {
			char[] copy = Arrays.copyOfRange(buf, off, off + len);
			this.diagnosticListener.fromForwardingWriter(copy);;
		}

		@Override
		public void flush() throws IOException {
			// nothing to flush
		}

		@Override
		public void close() throws IOException {
			// nothing to close
		}
	}
}
