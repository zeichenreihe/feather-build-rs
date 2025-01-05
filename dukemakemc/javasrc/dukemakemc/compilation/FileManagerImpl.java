package dukemakemc.compilation;

import dukemakemc.util.FileObjectPath;
import dukemakemc.util.ResizingByteBuffer;

import javax.tools.*;
import java.io.IOException;
import java.nio.ByteBuffer;
import java.util.*;

public class FileManagerImpl implements JavaFileManager {
	private final StandardJavaFileManager standardJavaFileManager;

	private final Map<FileObjectPath, JavaInputFileImpl> inputs;
	private final Map<FileObjectPath, JavaOutputFileImpl> outputs = new HashMap<>();

	public FileManagerImpl(StandardJavaFileManager fileManager, Map<FileObjectPath, JavaInputFileImpl> inputs) {
		this.standardJavaFileManager = fileManager;
		this.inputs = inputs;
	}

	/** Returns all the file objects for the inputs this file manager knows. */
	public Iterable<? extends JavaFileObject> inputs() {
		return this.inputs.values();
	}

	public void debugResults() {
		System.out.println(this.outputs);
		for (JavaOutputFileImpl output : this.outputs.values()) {
			System.out.println("- " + output.getName());
			ByteBuffer buf = output.content().getBuffer();
			byte[] bytes = new byte[buf.remaining()];
			buf.get(bytes).flip();


			String hexString = HexFormat.of().withUpperCase().formatHex(bytes);

			System.out.println("  " + hexString);
		}
	}

	private static void checkPackageOrientedLocation(Location location) {
		if (location.isModuleOrientedLocation()) {
			throw new IllegalArgumentException("expected a package oriented location, got " + location.getClass() + " " + location.getName());
		}
	}

	private static <T> T todo() {
		throw new RuntimeException("not yet implemented");
	}

	@Override
	public ClassLoader getClassLoader(Location location) {
		System.out.println("wanted to get class loader for " + location + ", returning null");
		return null;
	}

	@Override
	public Iterable<JavaFileObject> list(Location location, String packageName, Set<JavaFileObject.Kind> kinds, boolean recurse) throws IOException {
		return switch (location) {
			case StandardLocation.CLASS_PATH -> {
				System.out.println("foo");
				System.out.println("called list with location: " + location + ", packageName: " + packageName + ", kinds: " + kinds + ", recurse: " + recurse);
				yield () -> new Iterator<>() {
					@Override
					public boolean hasNext() {
						return false;
					}

					@Override
					public JavaFileObject next() {
						return null;
					}
				};
			}
			default -> {
				String locationName = location.getName();
				if (locationName.startsWith("SYSTEM_MODULES[")) {
					System.out.println("we don't have system modules");
					System.out.println("called list with location: " + location + ", packageName: " + packageName + ", kinds: " + kinds + ", recurse: " + recurse);
					yield standardJavaFileManager.list(location, packageName, kinds, recurse);
				} else {
					System.out.println("called list with location: " + location + ", packageName: " + packageName + ", kinds: " + kinds + ", recurse: " + recurse);
					yield todo();
				}
			}
		};
	}

	@Override
	public String inferBinaryName(Location location, JavaFileObject file) {
		return switch (file) {
			case JavaInputFileImpl(FileObjectPath loc, ByteBuffer ignored) -> {
				System.out.println("unknown binary name of input obj at " + loc);
				yield todo();
			}
			case JavaOutputFileImpl(FileObjectPath loc, ResizingByteBuffer ignored) -> {
				System.out.println("unknown binary name of output obj at " + loc);
				yield todo();
			}
			default -> {
				String s = standardJavaFileManager.inferBinaryName(location, file);
				//System.out.println("infer binary name: location: " + location + ", file: " + file + " -> " + s);
				yield s;
			}
		};
	}

	@Override
	public boolean isSameFile(FileObject a, FileObject b) {
		boolean s = Objects.equals(a, b);
		System.out.println("is same file a: " + a + ", b: " + b + " -> " + s);
		return s;
	}

	@Override
	public boolean handleOption(String current, Iterator<String> remaining) {
		boolean s = standardJavaFileManager.handleOption(current, remaining);
		System.out.println("handle option: current: " + current + ", remaining: " + remaining + " -> " + s);
		// TODO: calls standard file manager
		return s;
		//return todo();
	}

	@Override
	public boolean hasLocation(Location location) {
		boolean s = standardJavaFileManager.hasLocation(location);
		System.out.println("has location: " + location.getClass() + " : " + location.getName() + " -> " + s);
		//return todo();
		// TODO: calls standard file manager
		return s;
	}

	@Override
	public JavaFileObject getJavaFileForInput(Location location, String className, JavaFileObject.Kind kind) throws IOException {
		return switch (location) {
			case StandardLocation.CLASS_PATH -> {
				System.out.println("get java file for input: location: " + location + ", class name: " + className + ", kind: " + kind);
				yield todo();
			}
			default -> {
				checkPackageOrientedLocation(location);
				String locationName = location.getName();
				if (locationName.startsWith("SYSTEM_MODULES[")) {
					System.out.println("get java file for input: location: " + location + ", class name: " + className + ", kind: " + kind);
					yield standardJavaFileManager.getJavaFileForInput(location, className, kind);
				} else {
					System.out.println("get java file for input: location: " + location + ", class name: " + className + ", kind: " + kind);
					yield todo();
				}
			}
		};
	}


	@Override
	public JavaFileObject getJavaFileForOutput(Location location, String className, JavaFileObject.Kind kind, FileObject sibling) throws IOException {
		checkPackageOrientedLocation(location);

		if (location != StandardLocation.CLASS_OUTPUT) {
			System.out.println("java file for output: location: " + location + ", class name: " + className + ", kind: " + kind + ", sibling: " + sibling);
			throw new RuntimeException("unknown location for java file output");
		}

		// TODO: properly create output files
		var loc = FileObjectPath.outputFile(location, className, kind);
		var jofi = new JavaOutputFileImpl(loc, new ResizingByteBuffer(32));
		System.out.println(loc);

		System.out.println("java file for output: location: " + location + ", class name: " + className + ", kind: " + kind + ", sibling: " + sibling);

		var old = this.outputs.put(loc, jofi);
		if (old != null) {
			throw new RuntimeException("whaaat"); // TODO
		}


		return jofi;
	}

	@Override
	public FileObject getFileForInput(Location location, String packageName, String relativeName) throws IOException {
		// TODO: fwds
		//return standardJavaFileManager.getFileForInput(location, packageName, relativeName);
		return todo();

	}

	@Override
	public FileObject getFileForOutput(Location location, String packageName, String relativeName, FileObject sibling) throws IOException {
		return todo();
	}

	@Override
	public Location getLocationForModule(Location location, JavaFileObject fo) throws IOException {
		// TODO: fwds
		//return standardJavaFileManager.getLocationForModule(location, fo);
		return todo();

	}

	@Override
	public Location getLocationForModule(Location location, String moduleName) throws IOException {
		// TODO: fwds
		//return standardJavaFileManager.getLocationForModule(location, moduleName);
		return todo();

	}

	@Override
	public <S> ServiceLoader<S> getServiceLoader(Location location, Class<S> service) throws IOException {
		// TODO: fwds
		//return standardJavaFileManager.getServiceLoader(location, service);
		return todo();

	}

	@Override
	public String inferModuleName(Location location) throws IOException {
		String s = standardJavaFileManager.inferModuleName(location);
		//System.out.println("infer module name: " + location + " -> " + s);
		// TODO: fwds
		return s;
		//return todo();

	}

	@Override
	public Iterable<Set<Location>> listLocationsForModules(Location location) throws IOException {
		System.out.println("list locations for modules: " + location);
		// TODO: fwds
		return standardJavaFileManager.listLocationsForModules(location);
		//return todo();

	}

	@Override
	public boolean contains(Location location, FileObject fo) throws IOException {
		// TODO: fwds
		//return standardJavaFileManager.contains(location, fo);
		return todo();

	}

	@Override
	public void flush() throws IOException {
		// TODO: fwd
		standardJavaFileManager.flush();
	}

	@Override
	public void close() throws IOException {

	}

	@Override
	public int isSupportedOption(String option) {
		System.out.println("is supported option: " + option);
		// TODO: forwards
		return standardJavaFileManager.isSupportedOption(option);
		//return todo();

	}
}
