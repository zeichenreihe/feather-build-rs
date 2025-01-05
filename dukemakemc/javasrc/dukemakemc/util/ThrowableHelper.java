package dukemakemc.util;

import java.io.PrintWriter;
import java.io.StringWriter;

public class ThrowableHelper {
	public static String throwableToString(Throwable throwable) {
		StringWriter stringWriter = new StringWriter();
		throwable.printStackTrace(new PrintWriter(stringWriter));
		return stringWriter.toString();
	}
}
