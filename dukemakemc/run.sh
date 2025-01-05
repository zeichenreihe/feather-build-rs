# easier compilation of java for quick changes

JAVA_HOME=/usr/lib/jvm/java-23-openjdk/
$JAVA_HOME/bin/javac -d javalib $(find javasrc -type f) && cargo run -p dukemakemc -- $@
