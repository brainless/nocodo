package com.nocodo.manager.ssh

import android.util.Log
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import net.schmizz.sshj.SSHClient
import net.schmizz.sshj.connection.channel.direct.LocalPortForwarder
import net.schmizz.sshj.transport.verification.PromiscuousVerifier
import java.net.InetSocketAddress
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class SshManager @Inject constructor(
    private val sshKeyManager: SshKeyManager
) {
    private var sshClient: SSHClient? = null
    private var forwarder: LocalPortForwarder? = null
    private var localPort: Int = 0

    data class SshConnectionParams(
        val host: String,
        val port: Int,
        val username: String,
        val keyPath: String?,
        val remotePort: Int = 8081
    )

    suspend fun connect(params: SshConnectionParams): Result<Int> = withContext(Dispatchers.IO) {
        try {
            // Close existing connection if any
            disconnect()

            val client = SSHClient()
            client.addHostKeyVerifier(PromiscuousVerifier())

            Log.d(TAG, "Connecting to ${params.host}:${params.port}")
            client.connect(params.host, params.port)

            // Load SSH key
            val keyProvider = sshKeyManager.loadKeyProvider(params.keyPath)
                ?: return@withContext Result.failure(Exception("No SSH key found"))

            Log.d(TAG, "Authenticating with public key")
            client.authPublickey(params.username, keyProvider)

            // Set up port forwarding
            val localAddress = InetSocketAddress("127.0.0.1", 0)
            val remoteAddress = InetSocketAddress("127.0.0.1", params.remotePort)

            Log.d(TAG, "Setting up port forwarding to remote port ${params.remotePort}")
            val forwardingClient = client.newLocalPortForwarder(localAddress, remoteAddress)

            // The local port is determined from the server socket
            val boundPort = forwardingClient.localPort

            sshClient = client
            forwarder = forwardingClient
            localPort = boundPort

            Log.d(TAG, "SSH tunnel established. Local port: $boundPort")
            Result.success(boundPort)
        } catch (e: Exception) {
            Log.e(TAG, "SSH connection failed", e)
            disconnect()
            Result.failure(e)
        }
    }

    fun disconnect() {
        try {
            forwarder?.close()
            sshClient?.disconnect()
        } catch (e: Exception) {
            Log.e(TAG, "Error during disconnect", e)
        } finally {
            forwarder = null
            sshClient = null
            localPort = 0
        }
    }

    fun isConnected(): Boolean {
        return sshClient?.isConnected == true
    }

    fun getLocalPort(): Int = localPort

    companion object {
        private const val TAG = "SshManager"
    }
}
