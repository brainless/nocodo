package com.nocodo.manager.data.remote.dto

import com.nocodo.manager.domain.model.Project

data class ProjectDto(
    val id: Long,
    val name: String,
    val path: String,
    val description: String? = null
) {
    fun toDomain(): Project = Project(
        id = id,
        name = name,
        path = path,
        description = description
    )
}

data class ProjectListResponse(
    val projects: List<ProjectDto>
)
