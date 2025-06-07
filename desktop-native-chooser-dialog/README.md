# Desktop Native Chooser Dialog

A Rust library that provides native file chooser dialog functionality for Windows and macOS. 

While it can be used directly as a rust library, it is primarily designed to be used as a JNI library for the Kotlin 
library in the outer directory. This feature is optional and is named `java`. 

To build the library with the JNI feature, the JAWT library must be installed in the `lib` project directory.