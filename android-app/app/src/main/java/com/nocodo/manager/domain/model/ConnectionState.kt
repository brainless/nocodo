package com.nocodo.manager.domain.model

sealed class ConnectionState {
    object Disconnected : ConnectionState()
    data class Connecting(val serverHost: String) : ConnectionState()
    data class Connected(val serverHost: String, val localPort: Int) : ConnectionState()
    data class Error(val message: String) : ConnectionState()
}
