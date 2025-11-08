package com.nocodo.manager.data.repository

import com.nocodo.manager.data.local.ServerDao
import com.nocodo.manager.data.local.entities.ServerEntity
import com.nocodo.manager.domain.model.Server
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class ServerRepository @Inject constructor(
    private val serverDao: ServerDao
) {
    fun getAllServers(): Flow<List<Server>> =
        serverDao.getAllServers().map { entities ->
            entities.map { it.toDomain() }
        }

    suspend fun getServerById(id: Long): Server? =
        serverDao.getServerById(id)?.toDomain()

    suspend fun insertServer(server: Server): Long =
        serverDao.insertServer(ServerEntity.fromDomain(server))

    suspend fun deleteServer(server: Server) =
        serverDao.deleteServer(ServerEntity.fromDomain(server))

    suspend fun deleteServerById(id: Long) =
        serverDao.deleteServerById(id)
}
