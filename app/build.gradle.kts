plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
}

val rustJniLibs = layout.buildDirectory.dir("rustJniLibs")

android {
    namespace = "app.naga"
    compileSdk = 36
    ndkVersion = "27.3.13750724"

    defaultConfig {
        applicationId = "app.naga"
        minSdk = 26
        targetSdk = 34
        versionCode = 1
        versionName = "1.0"
        // Rust 库只编了 arm64，其他架构的手机装上也会在加载时闪退，不如直接装不上
        ndk { abiFilters += "arm64-v8a" }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }
    buildFeatures {
        compose = true
    }
    sourceSets["main"].jniLibs.srcDir(rustJniLibs)
}

// 算法在 Rust（core/）。debug 包也用 release 编，未优化的 YIN 太慢
val cargoBuild by tasks.registering(Exec::class) {
    val core = rootProject.file("core")
    workingDir = core
    inputs.dir(core.resolve("src"))
    inputs.files(core.resolve("Cargo.toml"), core.resolve("Cargo.lock"), core.resolve(".cargo/config.toml"))
    outputs.dir(rustJniLibs)
    environment("ANDROID_NDK_HOME", androidComponents.sdkComponents.ndkDirectory.get().asFile.path)
    commandLine(
        "cargo", "ndk", "-t", "arm64-v8a", "-P", "26",
        "-o", rustJniLibs.get().asFile.path, "build", "--release",
    )
}
tasks.named("preBuild") { dependsOn(cargoBuild) }

dependencies {
    implementation(platform("androidx.compose:compose-bom:2024.12.01"))
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.activity:activity-compose:1.9.3")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.8.7")
    implementation("androidx.core:core-ktx:1.15.0")
}
