package com.nocodo.manager.domain.model

data class Project(
    val id: Long,
    val name: String,
    val path: String,
    val description: String? = null
)
