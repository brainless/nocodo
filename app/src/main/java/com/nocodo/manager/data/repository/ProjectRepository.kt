package com.nocodo.manager.data.repository

import com.nocodo.manager.data.remote.ManagerApiService
import com.nocodo.manager.domain.model.Project
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class ProjectRepository @Inject constructor(
    private val apiService: ManagerApiService
) {
    suspend fun getProjects(): Result<List<Project>> {
        return try {
            val response = apiService.listProjects()
            if (response.isSuccessful) {
                val projects = response.body()?.projects?.map { it.toDomain() } ?: emptyList()
                Result.success(projects)
            } else {
                Result.failure(Exception("Failed to fetch projects: ${response.code()}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
}
