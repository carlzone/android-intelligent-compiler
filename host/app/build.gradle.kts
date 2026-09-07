plugins { id("com.android.application"); id("org.jetbrains.kotlin.android") }
android {
    namespace = "dev.aic.host"
    compileSdk = 35
    signingConfigs { getByName("debug") { storeFile = rootProject.file("../compiler/.aic/host-debug.keystore") } }
    defaultConfig {
        applicationId = "dev.aic.host"
        minSdk = 30
        targetSdk = 35
        versionCode = 3
        versionName = "0.8.0"
        ndk { abiFilters += "arm64-v8a" }
        testInstrumentationRunner = "dev.aic.host.HostInstrumentation"
    }
    // Exclude ignored bootstrap files even in a previously prepared checkout.
    androidResources { ignoreAssetsPattern = "bootstrap:!.svn:!.git:!*.scc:.*:!CVS:!thumbs.db:!picasa.ini:!*~" }
    packaging { jniLibs { useLegacyPackaging = true; excludes += "**/libaapt2.so" } }
    compileOptions { sourceCompatibility = JavaVersion.VERSION_17; targetCompatibility = JavaVersion.VERSION_17 }
    kotlinOptions { jvmTarget = "17" }
    testOptions { unitTests.isReturnDefaultValues = true }
}
dependencies {
    implementation("com.android.tools.build:apksig:8.10.1")
    testImplementation("junit:junit:4.13.2")
}
