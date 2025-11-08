package com.nocodo.manager.ssh

import android.content.Context
import android.util.Base64
import net.schmizz.sshj.SSHClient
import net.schmizz.sshj.common.SecurityUtils
import net.schmizz.sshj.userauth.keyprovider.KeyProvider
import java.io.File
import java.io.FileWriter
import java.security.KeyPairGenerator
import java.security.MessageDigest
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class SshKeyManager @Inject constructor(
    private val context: Context
) {
    private val sshDir: File by lazy {
        File(context.filesDir, ".ssh").apply {
            if (!exists()) mkdirs()
        }
    }

    data class SshKeyInfo(
        val publicKey: String,
        val fingerprint: String,
        val keyPath: String
    )

    fun getOrCreateDefaultKey(): SshKeyInfo {
        // Check for existing keys in order of preference
        val keyTypes = listOf("id_ed25519", "id_rsa", "id_ecdsa")
        for (keyType in keyTypes) {
            val privateKeyFile = File(sshDir, keyType)
            val publicKeyFile = File(sshDir, "$keyType.pub")

            if (privateKeyFile.exists() && publicKeyFile.exists()) {
                val publicKey = publicKeyFile.readText().trim()
                val fingerprint = calculateFingerprint(publicKey)
                return SshKeyInfo(publicKey, fingerprint, privateKeyFile.absolutePath)
            }
        }

        // No existing key found, generate ED25519 key
        return generateEd25519Key()
    }

    private fun generateEd25519Key(): SshKeyInfo {
        val privateKeyFile = File(sshDir, "id_ed25519")
        val publicKeyFile = File(sshDir, "id_ed25519.pub")

        try {
            // Generate ED25519 key pair
            val keyPairGenerator = KeyPairGenerator.getInstance("Ed25519", "BC")
            val keyPair = keyPairGenerator.generateKeyPair()

            // Save private key in OpenSSH format
            val privateKeyContent = SecurityUtils.formatPrivateKey(keyPair.private, "Ed25519")
            FileWriter(privateKeyFile).use { it.write(privateKeyContent) }
            privateKeyFile.setReadable(false, false)
            privateKeyFile.setReadable(true, true)

            // Save public key in OpenSSH format
            val publicKeyContent = SecurityUtils.formatPublicKey(keyPair.public, "Ed25519")
            FileWriter(publicKeyFile).use { it.write(publicKeyContent) }

            val fingerprint = calculateFingerprint(publicKeyContent)
            return SshKeyInfo(publicKeyContent, fingerprint, privateKeyFile.absolutePath)
        } catch (e: Exception) {
            throw RuntimeException("Failed to generate SSH key: ${e.message}", e)
        }
    }

    fun calculateFingerprint(publicKey: String): String {
        try {
            // Extract the base64 part of the public key
            val parts = publicKey.trim().split(" ")
            if (parts.size < 2) {
                throw IllegalArgumentException("Invalid public key format")
            }

            val keyData = Base64.decode(parts[1], Base64.NO_WRAP)
            val digest = MessageDigest.getInstance("SHA-256")
            val hash = digest.digest(keyData)

            // Format as SHA256:base64
            val base64Hash = Base64.encodeToString(hash, Base64.NO_WRAP or Base64.NO_PADDING)
            return "SHA256:$base64Hash"
        } catch (e: Exception) {
            return "Unknown"
        }
    }

    fun loadKeyProvider(keyPath: String?): KeyProvider? {
        val sshClient = SSHClient()
        return try {
            when {
                keyPath != null && File(keyPath).exists() -> {
                    sshClient.loadKeys(keyPath)
                }
                else -> {
                    // Try default keys
                    val keyTypes = listOf("id_ed25519", "id_rsa", "id_ecdsa")
                    for (keyType in keyTypes) {
                        val file = File(sshDir, keyType)
                        if (file.exists()) {
                            return sshClient.loadKeys(file.absolutePath)
                        }
                    }
                    null
                }
            }
        } catch (e: Exception) {
            null
        }
    }
}
