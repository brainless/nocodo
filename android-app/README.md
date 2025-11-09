# Nocodo Manager - Android App

Native Android application for accessing the Nocodo Manager from Android smartphones and tablets.

## Overview

This is a Kotlin-based Android app that provides mobile access to the Nocodo Manager via SSH tunneling. The app establishes secure SSH connections to remote servers and communicates with the manager API through forwarded ports.

## Quick Start (Windows)

1. Make sure Android Studio is installed
2. Open PowerShell and navigate to the `android-app` directory
3. Run the build script:
   ```powershell
   .\build.ps1
   ```
4. Find your APK at: `app\build\outputs\apk\debug\app-debug.apk`

That's it! See [BUILD_INSTRUCTIONS.md](BUILD_INSTRUCTIONS.md) for more details.

## Features

- **SSH Connection Management**: Establish and maintain SSH tunnels to remote servers
- **Server Management**: Save and manage multiple SSH server configurations
- **Project Browsing**: View and browse projects from the manager API
- **Secure Authentication**: JWT-based authentication with encrypted credential storage
- **Background Service**: Persistent SSH tunnel with automatic reconnection
- **Material 3 Design**: Modern UI following Material Design 3 guidelines

## Technology Stack

- **Language**: Kotlin 100%
- **Build System**: Gradle with Kotlin DSL
- **Min SDK**: API 26 (Android 8.0 Oreo)
- **Target SDK**: API 34 (Android 14)
- **UI Framework**: Jetpack Compose with Material 3
- **Architecture**: MVVM with Hilt dependency injection
- **Database**: Room for local data persistence
- **Networking**: Retrofit + OkHttp
- **SSH**: SSHJ library with BouncyCastle

## Prerequisites

- **Android Studio** (latest stable version) - includes Android SDK and JDK
  - Windows: Install from [developer.android.com](https://developer.android.com/studio)
  - The bundled JDK (JBR) will be automatically used by the build scripts
- **Android SDK** with API 34 (installed via Android Studio SDK Manager)
- **Git** (for cloning the repository)
- **PowerShell** or **Command Prompt** (Windows) / **Bash** (Linux/macOS)

**Note for Windows users**: The build scripts (`build.ps1` / `build.bat`) handle JAVA_HOME configuration automatically. No manual JDK setup required!

## Building the App

### Windows (PowerShell) - Recommended

We provide convenient build scripts for Windows:

```powershell
# Build debug APK (default)
.\build.ps1

# Build release APK
.\build.ps1 assembleRelease

# Clean build
.\build.ps1 clean
```

Or using Command Prompt:
```cmd
build.bat
build.bat assembleRelease
```

The scripts automatically configure JAVA_HOME and use the Gradle distribution from Android Studio.

**Output**: `app\build\outputs\apk\debug\app-debug.apk`

### Linux/macOS

Debug build:
```bash
./gradlew assembleDebug
```

Release build:
```bash
./gradlew assembleRelease
```

### Manual Build (All Platforms)

If you prefer to use Gradle directly:

**PowerShell:**
```powershell
$env:JAVA_HOME = "C:\Program Files\Android\Android Studio\jbr"
.\gradlew assembleDebug
```

**Bash (Git Bash on Windows):**
```bash
export JAVA_HOME="/c/Program Files/Android/Android Studio/jbr"
./gradlew assembleDebug
```

See [BUILD_INSTRUCTIONS.md](BUILD_INSTRUCTIONS.md) for detailed Windows build instructions and troubleshooting.

## Running the App

### Install on Connected Device

**Windows (PowerShell):**
```powershell
.\build.ps1 installDebug
# Or manually:
adb install app\build\outputs\apk\debug\app-debug.apk
adb shell am start -n com.nocodo.manager/.ui.screens.MainActivity
```

**Linux/macOS:**
```bash
./gradlew installDebug
adb shell am start -n com.nocodo.manager/.ui.screens.MainActivity
```

### Using Android Studio

1. Open the `android-app` folder in Android Studio
2. Connect your Android device or start an emulator
3. Click the "Run" button (green triangle) or press Shift+F10
4. Select your target device

## Architecture

The app follows MVVM architecture with:
- Jetpack Compose for UI
- Hilt for dependency injection
- Room for local database
- Retrofit for API communication
- SSHJ for SSH tunneling

See GitHub issue #174 for full specifications.

## Troubleshooting

### Windows Build Issues

**"JAVA_HOME is not set" error:**
- Make sure Android Studio is installed at `C:\Program Files\Android\Android Studio`
- If installed elsewhere, edit the `$AndroidStudioPath` variable in `build.ps1`

**"Gradle not found" error:**
- The script will automatically fall back to using the `gradlew` wrapper
- Make sure you're running the script from the `android-app` directory

**"gradlew: Permission denied":**
```powershell
# In PowerShell:
.\build.ps1
# This automatically handles permissions
```

**PowerShell execution policy error:**
```powershell
# Run PowerShell as Administrator and execute:
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser

# Or run the script with bypass:
powershell.exe -ExecutionPolicy Bypass -File build.ps1
```

### General Build Issues

**Build fails with compilation errors:**
- Make sure you have the latest version from git
- Try cleaning the build: `.\build.ps1 clean` then rebuild

**Out of memory during build:**
- Close other applications
- Increase Gradle memory in `gradle.properties`:
  ```properties
  org.gradle.jvmargs=-Xmx2048m
  ```

For more help, see [BUILD_INSTRUCTIONS.md](BUILD_INSTRUCTIONS.md) or open an issue on GitHub.
