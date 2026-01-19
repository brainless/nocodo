# Ask User Tool Usage Example

The `ask_user` tool allows the LLM to ask the user a list of questions to gather information or confirm actions.

## Request Structure

```json
{
  "type": "ask_user",
  "questions": [
    {
      "id": "project_name",
      "question": "What is your project name?",
      "type": "text",
      "default": "My Project"
    },
    {
      "id": "description",
      "question": "Describe your project:",
      "type": "text",
      "description": "Provide a brief overview of your project"
    }
  ]
}
```

## Question Types

### Text Input
```json
{
  "id": "name",
  "question": "What is your name?",
  "type": "text",
  "default": "John Doe"
}
```

## Response Structure

```json
{
  "type": "ask_user",
  "completed": true,
  "responses": [
    {
      "question_id": "project_name",
      "answer": "My Awesome Project"
    },
    {
      "question_id": "description",
      "answer": "A web application for task management"
    }
  ],
  "message": "All questions answered successfully"
}
```

## Usage Examples

### Project Initialization
Ask the user for project details before creating a new project structure.

### Configuration Validation
Confirm critical configuration changes with the user before applying them.

### Feature Selection
Let users choose which features to include in generated code or setups.

### Confirmation Dialogs
Ask for confirmation before potentially destructive operations.
