# Android App Build Instructions

This directory contains build scripts to simplify building the nocodo Android app on Windows.

## Prerequisites

- Android Studio installed at `C:\Program Files\Android\Android Studio`
- Git Bash (for running gradlew wrapper if needed)

## Build Scripts

### Option 1: PowerShell Script (Recommended)

```powershell
# Build debug APK (default)
.\build.ps1

# Build release APK
.\build.ps1 assembleRelease

# Clean build
.\build.ps1 clean

# Clean and build
.\build.ps1 clean assembleDebug
```

### Option 2: Batch File

```cmd
# Build debug APK (default)
build.bat

# Build release APK
build.bat assembleRelease

# Clean build
build.bat clean
```

### Option 3: Direct Gradle Command

If the scripts don't work, you can use the manual command:

```powershell
# Set JAVA_HOME
$env:JAVA_HOME = "C:\Program Files\Android\Android Studio\jbr"

# Find your Gradle distribution
$GradleHome = "$env:USERPROFILE\.gradle\wrapper\dists\gradle-8.2-bin"
$GradleDir = Get-ChildItem -Path $GradleHome -Directory | Select-Object -First 1
$GradleBin = "$($GradleDir.FullName)\gradle-8.2\bin\gradle.bat"

# Run build
& $GradleBin assembleDebug
```

## Build Output

After a successful build, you can find the APK at:

- **Debug**: `app\build\outputs\apk\debug\app-debug.apk`
- **Release**: `app\build\outputs\apk\release\app-release.apk`

## Troubleshooting

### Java/JDK Not Found

If you see "JAVA_HOME is not set" error:

1. Make sure Android Studio is installed
2. Update the `ANDROID_STUDIO_PATH` variable in the build scripts to match your installation path

### Gradle Not Found

If Gradle distribution is not found:

1. The script will automatically fall back to using the `gradlew` wrapper
2. Make sure Git Bash is installed for running Unix-style scripts

### Different Android Studio Path

If Android Studio is installed in a different location, edit the build scripts and update:

```powershell
# In build.ps1
$AndroidStudioPath = "YOUR_CUSTOM_PATH"

# In build.bat
set ANDROID_STUDIO_PATH=YOUR_CUSTOM_PATH
```

## Common Build Tasks

- `assembleDebug` - Build debug APK
- `assembleRelease` - Build release APK (requires signing configuration)
- `clean` - Clean build artifacts
- `installDebug` - Build and install debug APK to connected device
- `test` - Run unit tests
- `connectedAndroidTest` - Run instrumentation tests

## Notes

- The scripts automatically set `JAVA_HOME` to use Android Studio's bundled JDK (JBR)
- Gradle 8.2 is used as specified in the gradle-wrapper.properties
- First build may take longer as dependencies are downloaded
