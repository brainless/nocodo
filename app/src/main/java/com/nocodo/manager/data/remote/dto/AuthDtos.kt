package com.nocodo.manager.data.remote.dto

import com.google.gson.annotations.SerializedName

data class RegisterRequest(
    val username: String,
    val password: String,
    val email: String? = null,
    @SerializedName("ssh_public_key")
    val sshPublicKey: String,
    @SerializedName("ssh_fingerprint")
    val sshFingerprint: String
)

data class LoginRequest(
    val username: String,
    val password: String,
    @SerializedName("ssh_fingerprint")
    val sshFingerprint: String
)

data class UserResponse(
    val id: Long,
    val username: String,
    val email: String?,
    @SerializedName("created_at")
    val createdAt: String?
)

data class LoginResponse(
    val token: String,
    val user: UserResponse
)
