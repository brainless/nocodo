package com.nocodo.manager.ui.screens.projects

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.nocodo.manager.data.repository.ProjectRepository
import com.nocodo.manager.domain.model.ProjectsUiState
import dagger.hilt.android.lifecycle.HiltViewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import javax.inject.Inject

@HiltViewModel
class ProjectsViewModel @Inject constructor(
    private val projectRepository: ProjectRepository
) : ViewModel() {

    private val _uiState = MutableStateFlow<ProjectsUiState>(ProjectsUiState.Disconnected)
    val uiState: StateFlow<ProjectsUiState> = _uiState.asStateFlow()

    fun loadProjects() {
        viewModelScope.launch {
            _uiState.value = ProjectsUiState.Loading
            projectRepository.getProjects().fold(
                onSuccess = { projects ->
                    _uiState.value = if (projects.isEmpty()) {
                        ProjectsUiState.Empty
                    } else {
                        ProjectsUiState.Success(projects)
                    }
                },
                onFailure = { error ->
                    _uiState.value = ProjectsUiState.Error(
                        error.message ?: "Failed to load projects"
                    )
                }
            )
        }
    }

    fun refresh() {
        loadProjects()
    }
}
