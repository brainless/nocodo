package com.nocodo.manager.service

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.Intent
import android.os.Binder
import android.os.IBinder
import android.util.Log
import androidx.core.app.NotificationCompat
import com.nocodo.manager.R
import com.nocodo.manager.domain.model.ConnectionState
import com.nocodo.manager.ssh.SshManager
import dagger.hilt.android.AndroidEntryPoint
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancelChildren
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import javax.inject.Inject

@AndroidEntryPoint
class SshConnectionService : Service() {

    @Inject
    lateinit var sshManager: SshManager

    private val binder = LocalBinder()
    private val serviceScope = CoroutineScope(SupervisorJob() + Dispatchers.Main)
    private var healthCheckJob: Job? = null

    private val _connectionState = MutableStateFlow<ConnectionState>(ConnectionState.Disconnected)
    val connectionState: StateFlow<ConnectionState> = _connectionState.asStateFlow()

    private var currentParams: SshManager.SshConnectionParams? = null
    private var reconnectAttempts = 0
    private val maxReconnectAttempts = 2

    inner class LocalBinder : Binder() {
        fun getService(): SshConnectionService = this@SshConnectionService
    }

    override fun onCreate() {
        super.onCreate()
        createNotificationChannel()
    }

    override fun onBind(intent: Intent?): IBinder = binder

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_CONNECT -> {
                val host = intent.getStringExtra(EXTRA_HOST) ?: return START_NOT_STICKY
                val port = intent.getIntExtra(EXTRA_PORT, 22)
                val username = intent.getStringExtra(EXTRA_USERNAME) ?: return START_NOT_STICKY
                val keyPath = intent.getStringExtra(EXTRA_KEY_PATH)
                val remotePort = intent.getIntExtra(EXTRA_REMOTE_PORT, 8081)

                val params = SshManager.SshConnectionParams(
                    host = host,
                    port = port,
                    username = username,
                    keyPath = keyPath,
                    remotePort = remotePort
                )

                connect(params)
            }
            ACTION_DISCONNECT -> {
                disconnect()
            }
        }

        return START_STICKY
    }

    fun connect(params: SshManager.SshConnectionParams) {
        currentParams = params
        reconnectAttempts = 0

        serviceScope.launch {
            _connectionState.value = ConnectionState.Connecting(params.host)
            startForeground(NOTIFICATION_ID, createNotification("Connecting to ${params.host}"))

            val result = sshManager.connect(params)
            result.fold(
                onSuccess = { localPort ->
                    _connectionState.value = ConnectionState.Connected(params.host, localPort)
                    startForeground(NOTIFICATION_ID, createNotification("Connected to ${params.host}"))
                    startHealthChecks()
                    reconnectAttempts = 0
                },
                onFailure = { error ->
                    Log.e(TAG, "Connection failed", error)
                    _connectionState.value = ConnectionState.Error(error.message ?: "Connection failed")
                    stopForeground(STOP_FOREGROUND_REMOVE)
                    stopSelf()
                }
            )
        }
    }

    fun disconnect() {
        stopHealthChecks()
        sshManager.disconnect()
        _connectionState.value = ConnectionState.Disconnected
        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf()
    }

    private fun startHealthChecks() {
        stopHealthChecks()
        healthCheckJob = serviceScope.launch {
            while (isActive) {
                delay(30_000) // 30 seconds
                if (!sshManager.isConnected()) {
                    Log.w(TAG, "SSH connection lost, attempting to reconnect")
                    attemptReconnect()
                    break
                }
            }
        }
    }

    private fun stopHealthChecks() {
        healthCheckJob?.cancel()
        healthCheckJob = null
    }

    private suspend fun attemptReconnect() {
        val params = currentParams ?: return

        if (reconnectAttempts >= maxReconnectAttempts) {
            Log.e(TAG, "Max reconnect attempts reached")
            _connectionState.value = ConnectionState.Error("Connection lost")
            disconnect()
            return
        }

        reconnectAttempts++
        Log.d(TAG, "Reconnect attempt $reconnectAttempts of $maxReconnectAttempts")

        _connectionState.value = ConnectionState.Connecting(params.host)
        val result = sshManager.connect(params)

        result.fold(
            onSuccess = { localPort ->
                _connectionState.value = ConnectionState.Connected(params.host, localPort)
                startHealthChecks()
                reconnectAttempts = 0
            },
            onFailure = { error ->
                Log.e(TAG, "Reconnect failed", error)
                delay(5000) // Wait 5 seconds before next attempt
                attemptReconnect()
            }
        )
    }

    private fun createNotificationChannel() {
        val channel = NotificationChannel(
            CHANNEL_ID,
            getString(R.string.notification_channel_ssh),
            NotificationManager.IMPORTANCE_LOW
        ).apply {
            description = getString(R.string.notification_channel_ssh_desc)
        }

        val notificationManager = getSystemService(NotificationManager::class.java)
        notificationManager.createNotificationChannel(channel)
    }

    private fun createNotification(contentText: String): Notification {
        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle(getString(R.string.app_name))
            .setContentText(contentText)
            .setSmallIcon(android.R.drawable.ic_dialog_info)
            .setOngoing(true)
            .build()
    }

    override fun onDestroy() {
        super.onDestroy()
        stopHealthChecks()
        sshManager.disconnect()
        serviceScope.coroutineContext.cancelChildren()
    }

    companion object {
        private const val TAG = "SshConnectionService"
        private const val CHANNEL_ID = "ssh_connection_channel"
        private const val NOTIFICATION_ID = 1

        const val ACTION_CONNECT = "com.nocodo.manager.ACTION_CONNECT"
        const val ACTION_DISCONNECT = "com.nocodo.manager.ACTION_DISCONNECT"

        const val EXTRA_HOST = "extra_host"
        const val EXTRA_PORT = "extra_port"
        const val EXTRA_USERNAME = "extra_username"
        const val EXTRA_KEY_PATH = "extra_key_path"
        const val EXTRA_REMOTE_PORT = "extra_remote_port"
    }
}
