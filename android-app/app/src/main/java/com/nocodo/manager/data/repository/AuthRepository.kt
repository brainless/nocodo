package com.nocodo.manager.data.repository

import android.content.Context
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import com.nocodo.manager.data.remote.ManagerApiService
import com.nocodo.manager.data.remote.dto.LoginRequest
import com.nocodo.manager.data.remote.dto.LoginResponse
import com.nocodo.manager.data.remote.dto.RegisterRequest
import com.nocodo.manager.data.remote.dto.UserResponse
import com.nocodo.manager.data.remote.interceptors.AuthInterceptor
import dagger.hilt.android.qualifiers.ApplicationContext
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class AuthRepository @Inject constructor(
    @ApplicationContext private val context: Context,
    private val apiService: ManagerApiService,
    private val authInterceptor: AuthInterceptor
) {
    private val masterKey = MasterKey.Builder(context)
        .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
        .build()

    private val sharedPreferences = EncryptedSharedPreferences.create(
        context,
        "encrypted_prefs",
        masterKey,
        EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
        EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
    )

    fun getStoredToken(): String? {
        return sharedPreferences.getString("jwt_token", null)
    }

    fun saveToken(token: String) {
        sharedPreferences.edit().putString("jwt_token", token).apply()
        authInterceptor.setToken(token)
    }

    fun clearToken() {
        sharedPreferences.edit().remove("jwt_token").apply()
        authInterceptor.clearToken()
    }

    suspend fun register(
        username: String,
        password: String,
        email: String?,
        sshPublicKey: String,
        sshFingerprint: String
    ): Result<UserResponse> {
        return try {
            val request = RegisterRequest(
                username = username,
                password = password,
                email = email,
                sshPublicKey = sshPublicKey,
                sshFingerprint = sshFingerprint
            )
            val response = apiService.register(request)
            if (response.isSuccessful && response.body() != null) {
                Result.success(response.body()!!)
            } else {
                Result.failure(Exception("Registration failed: ${response.code()}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun login(
        username: String,
        password: String,
        sshFingerprint: String
    ): Result<LoginResponse> {
        return try {
            val request = LoginRequest(
                username = username,
                password = password,
                sshFingerprint = sshFingerprint
            )
            val response = apiService.login(request)
            if (response.isSuccessful && response.body() != null) {
                val loginResponse = response.body()!!
                saveToken(loginResponse.token)
                Result.success(loginResponse)
            } else {
                Result.failure(Exception("Login failed: ${response.code()}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun healthCheck(): Result<Boolean> {
        return try {
            val response = apiService.healthCheck()
            Result.success(response.isSuccessful)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
}
