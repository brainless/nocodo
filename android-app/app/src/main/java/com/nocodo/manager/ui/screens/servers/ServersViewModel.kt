package com.nocodo.manager.ui.screens.servers

import android.content.Context
import android.content.Intent
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.nocodo.manager.data.repository.ServerRepository
import com.nocodo.manager.domain.model.ConnectionState
import com.nocodo.manager.domain.model.Server
import com.nocodo.manager.service.SshConnectionService
import com.nocodo.manager.ssh.SshKeyManager
import dagger.hilt.android.lifecycle.HiltViewModel
import dagger.hilt.android.qualifiers.ApplicationContext
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch
import javax.inject.Inject

@HiltViewModel
class ServersViewModel @Inject constructor(
    @ApplicationContext private val context: Context,
    private val serverRepository: ServerRepository,
    private val sshKeyManager: SshKeyManager
) : ViewModel() {

    val servers: StateFlow<List<Server>> = serverRepository.getAllServers()
        .stateIn(
            scope = viewModelScope,
            started = SharingStarted.WhileSubscribed(5000),
            initialValue = emptyList()
        )

    private val _sshKeyInfo = MutableStateFlow<SshKeyManager.SshKeyInfo?>(null)
    val sshKeyInfo: StateFlow<SshKeyManager.SshKeyInfo?> = _sshKeyInfo.asStateFlow()

    init {
        loadSshKeyInfo()
    }

    private fun loadSshKeyInfo() {
        viewModelScope.launch {
            try {
                _sshKeyInfo.value = sshKeyManager.getOrCreateDefaultKey()
            } catch (e: Exception) {
                // Handle error
            }
        }
    }

    fun saveServer(server: Server) {
        viewModelScope.launch {
            serverRepository.insertServer(server)
        }
    }

    fun deleteServer(server: Server) {
        viewModelScope.launch {
            serverRepository.deleteServer(server)
        }
    }

    fun connectToServer(server: Server) {
        val intent = Intent(context, SshConnectionService::class.java).apply {
            action = SshConnectionService.ACTION_CONNECT
            putExtra(SshConnectionService.EXTRA_HOST, server.host)
            putExtra(SshConnectionService.EXTRA_PORT, server.port)
            putExtra(SshConnectionService.EXTRA_USERNAME, server.user)
            putExtra(SshConnectionService.EXTRA_KEY_PATH, server.keyPath)
        }
        context.startForegroundService(intent)
    }
}
