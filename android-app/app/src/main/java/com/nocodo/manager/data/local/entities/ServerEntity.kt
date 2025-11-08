package com.nocodo.manager.data.local.entities

import androidx.room.Entity
import androidx.room.PrimaryKey
import com.nocodo.manager.domain.model.Server

@Entity(tableName = "servers")
data class ServerEntity(
    @PrimaryKey(autoGenerate = true)
    val id: Long = 0,
    val host: String,
    val user: String,
    val keyPath: String? = null,
    val port: Int = 22
) {
    fun toDomain(): Server = Server(
        id = id,
        host = host,
        user = user,
        keyPath = keyPath,
        port = port
    )

    companion object {
        fun fromDomain(server: Server): ServerEntity = ServerEntity(
            id = server.id,
            host = server.host,
            user = server.user,
            keyPath = server.keyPath,
            port = server.port
        )
    }
}
