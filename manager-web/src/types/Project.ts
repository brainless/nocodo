export interface Project {
  id: string;
  name: string;
  path: string;
  language: string | null;
  framework: string | null;
  status: string;
  created_at: number;
  updated_at: number;
  technologies?: string | null; // JSON string of technologies array
}