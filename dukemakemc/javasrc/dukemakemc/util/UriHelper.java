package dukemakemc.util;

import java.util.regex.Pattern;

public class UriHelper {
	private static final Pattern PATTERN = Pattern.compile("[^a-zA-Z0-9]");

	public static String makeHostSafe(String hostUnsafe) {
		return PATTERN.matcher(hostUnsafe).replaceAll(".");
	}
}
