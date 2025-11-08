package com.nocodo.manager.domain.model

sealed class ProjectsUiState {
    object Disconnected : ProjectsUiState()
    object Connecting : ProjectsUiState()
    object Loading : ProjectsUiState()
    object Empty : ProjectsUiState()
    data class Success(val projects: List<Project>) : ProjectsUiState()
    data class Error(val message: String) : ProjectsUiState()
}
