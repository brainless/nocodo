package com.nocodo.manager.ssh

import android.content.Context
import android.util.Base64
import net.schmizz.sshj.SSHClient
import net.schmizz.sshj.common.SecurityUtils
import net.schmizz.sshj.userauth.keyprovider.KeyProvider
import net.schmizz.sshj.userauth.keyprovider.OpenSSHKeyFile
import java.io.File
import java.io.FileWriter
import java.security.KeyPair
import java.security.KeyPairGenerator
import java.security.MessageDigest
import java.security.interfaces.EdECPublicKey
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

            // Save private key in OpenSSH format using SSHJ's key file writer
            val keyFile = OpenSSHKeyFile()
            keyFile.init(privateKeyFile.absolutePath, null)
            // Note: SSHJ's OpenSSHKeyFile doesn't provide a direct way to generate and save keys
            // We'll use Java's key serialization instead
            privateKeyFile.writeText(formatPrivateKey(keyPair))
            privateKeyFile.setReadable(false, false)
            privateKeyFile.setReadable(true, true)

            // Save public key in OpenSSH format
            val publicKeyContent = formatPublicKey(keyPair)
            FileWriter(publicKeyFile).use { it.write(publicKeyContent) }

            val fingerprint = calculateFingerprint(publicKeyContent)
            return SshKeyInfo(publicKeyContent, fingerprint, privateKeyFile.absolutePath)
        } catch (e: Exception) {
            throw RuntimeException("Failed to generate SSH key: ${e.message}", e)
        }
    }

    private fun formatPrivateKey(keyPair: KeyPair): String {
        // For now, we'll use PEM format which is widely compatible
        // In production, you might want to use a proper OpenSSH key format library
        val privateKeyBytes = keyPair.private.encoded
        val base64Key = Base64.encodeToString(privateKeyBytes, Base64.DEFAULT)
        return """
            -----BEGIN PRIVATE KEY-----
            $base64Key-----END PRIVATE KEY-----
        """.trimIndent()
    }

    private fun formatPublicKey(keyPair: KeyPair): String {
        // Format Ed25519 public key in OpenSSH format
        val publicKey = keyPair.public
        val keyBytes = when (publicKey) {
            is EdECPublicKey -> {
                // Extract the raw key bytes
                val encoded = publicKey.encoded
                // Ed25519 public key is 32 bytes, extract from the DER encoding
                encoded.takeLast(32).toByteArray()
            }
            else -> publicKey.encoded
        }

        // Build OpenSSH public key format: "ssh-ed25519 <base64-encoded-key>"
        val keyType = "ssh-ed25519"
        val keyTypeBytes = keyType.toByteArray()

        // OpenSSH format: length + type + length + key
        val buffer = java.io.ByteArrayOutputStream()
        buffer.write(byteArrayOf(0, 0, 0, keyTypeBytes.size.toByte()))
        buffer.write(keyTypeBytes)
        buffer.write(byteArrayOf(0, 0, 0, keyBytes.size.toByte()))
        buffer.write(keyBytes)

        val base64Key = Base64.encodeToString(buffer.toByteArray(), Base64.NO_WRAP)
        return "$keyType $base64Key"
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
