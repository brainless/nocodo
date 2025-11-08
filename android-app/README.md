# Nocodo Manager - Android App

Native Android application for accessing the Nocodo Manager from Android smartphones and tablets.

## Overview

This is a Kotlin-based Android app that provides mobile access to the Nocodo Manager via SSH tunneling. The app establishes secure SSH connections to remote servers and communicates with the manager API through forwarded ports.

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

- Android Studio (latest stable version)
- JDK 17 or higher
- Android SDK with API 34
- Git

## Building the App

Debug build:
```bash
./gradlew assembleDebug
```

Release build:
```bash
./gradlew assembleRelease
```

## Running the App

Install on connected device:
```bash
./gradlew installDebug
adb shell am start -n com.nocodo.manager/.ui.screens.MainActivity
```

## Architecture

The app follows MVVM architecture with:
- Jetpack Compose for UI
- Hilt for dependency injection
- Room for local database
- Retrofit for API communication
- SSHJ for SSH tunneling

See GitHub issue #174 for full specifications.
