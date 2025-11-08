package com.nocodo.manager

import android.app.Application
import dagger.hilt.android.HiltAndroidApp
import org.bouncycastle.jce.provider.BouncyCastleProvider
import java.security.Security

@HiltAndroidApp
class NocodoApp : Application() {
    override fun onCreate() {
        super.onCreate()

        // Add BouncyCastle security provider for SSH key operations
        Security.removeProvider("BC")
        Security.addProvider(BouncyCastleProvider())
    }
}
