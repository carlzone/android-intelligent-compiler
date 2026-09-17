package dev.aic.host

object NativeCompiler {
    init { System.loadLibrary("aic_jni") }
    @JvmStatic external fun compile(source: String, assets: String, output: String, level: Int): String
    @JvmStatic external fun validate(source: String, assets: String, level: Int): String
}
