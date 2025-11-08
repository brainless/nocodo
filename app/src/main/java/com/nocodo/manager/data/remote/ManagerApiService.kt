package com.nocodo.manager.data.remote

import com.nocodo.manager.data.remote.dto.LoginRequest
import com.nocodo.manager.data.remote.dto.LoginResponse
import com.nocodo.manager.data.remote.dto.ProjectListResponse
import com.nocodo.manager.data.remote.dto.RegisterRequest
import com.nocodo.manager.data.remote.dto.ServerStatusDto
import com.nocodo.manager.data.remote.dto.UserResponse
import retrofit2.Response
import retrofit2.http.Body
import retrofit2.http.GET
import retrofit2.http.POST

interface ManagerApiService {
    @GET("/api/health")
    suspend fun healthCheck(): Response<ServerStatusDto>

    @POST("/api/auth/register")
    suspend fun register(@Body request: RegisterRequest): Response<UserResponse>

    @POST("/api/auth/login")
    suspend fun login(@Body request: LoginRequest): Response<LoginResponse>

    @GET("/api/projects")
    suspend fun listProjects(): Response<ProjectListResponse>
}
