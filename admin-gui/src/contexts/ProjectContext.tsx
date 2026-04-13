import { createContext, useContext, createSignal, createEffect, batch, type Accessor, type Setter, type JSX } from 'solid-js';
import type { Project, ListProjectsResponse, CreateProjectRequest, CreateProjectResponse } from '../types/api';

const API_BASE_URL = '';  // Use relative URLs to leverage Vite proxy

interface ProjectContextValue {
  projects: Accessor<Project[]>;
  currentProject: Accessor<Project | null>;
  isLoading: Accessor<boolean>;
  error: Accessor<string | null>;
  setCurrentProject: Setter<Project | null>;
  loadProjects: () => Promise<void>;
  createProject: (name: string, path?: string) => Promise<Project | null>;
}

const ProjectContext = createContext<ProjectContextValue>();

export function ProjectProvider(props: { children: JSX.Element }) {
  const [projects, setProjects] = createSignal<Project[]>([]);
  const [currentProject, setCurrentProject] = createSignal<Project | null>(null);
  const [isLoading, setIsLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  // Load projects on mount
  const loadProjects = async () => {
    setIsLoading(true);
    setError(null);
    try {
      const response = await fetch(`${API_BASE_URL}/api/projects`);
      if (!response.ok) {
        throw new Error(`Failed to load projects: ${response.status}`);
      }
      const data = await response.json() as ListProjectsResponse;
      
      batch(() => {
        setProjects(data.projects);
        // If no current project is selected, select the first one
        if (data.projects.length > 0 && !currentProject()) {
          setCurrentProject(data.projects[0]);
        }
      });
    } catch (err) {
      console.error('Error loading projects:', err);
      setError(err instanceof Error ? err.message : 'Failed to load projects');
    } finally {
      setIsLoading(false);
    }
  };

  // Create a new project
  const createProject = async (name: string, path?: string): Promise<Project | null> => {
    setIsLoading(true);
    setError(null);
    try {
      const body: CreateProjectRequest = {
        name,
        path: path || null,
      };
      
      const response = await fetch(`${API_BASE_URL}/api/projects`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });
      
      if (!response.ok) {
        throw new Error(`Failed to create project: ${response.status}`);
      }
      
      const data = await response.json() as CreateProjectResponse;
      
      batch(() => {
        setProjects(prev => [data.project, ...prev]);
        setCurrentProject(data.project);
      });
      
      return data.project;
    } catch (err) {
      console.error('Error creating project:', err);
      setError(err instanceof Error ? err.message : 'Failed to create project');
      return null;
    } finally {
      setIsLoading(false);
    }
  };

  // Load projects on mount
  createEffect(() => {
    loadProjects();
  });

  const value: ProjectContextValue = {
    projects,
    currentProject,
    isLoading,
    error,
    setCurrentProject,
    loadProjects,
    createProject,
  };

  return (
    <ProjectContext.Provider value={value}>
      {props.children}
    </ProjectContext.Provider>
  );
}

export function useProject() {
  const context = useContext(ProjectContext);
  if (!context) {
    throw new Error('useProject must be used within a ProjectProvider');
  }
  return context;
}
