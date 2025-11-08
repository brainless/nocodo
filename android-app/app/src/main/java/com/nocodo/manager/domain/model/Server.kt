package com.nocodo.manager.domain.model

data class Server(
    val id: Long = 0,
    val host: String,
    val user: String,
    val keyPath: String? = null,
    val port: Int = 22
) {
    fun connectionString(): String = "$user@$host:$port"
}
