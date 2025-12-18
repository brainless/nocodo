# Ask User Tool Usage Example

The `ask_user` tool allows the LLM to ask the user a list of questions to gather information or confirm actions.

## Request Structure

```json
{
  "type": "ask_user",
  "prompt": "I need some information about your project setup",
  "questions": [
    {
      "id": "project_name",
      "question": "What is your project name?",
      "type": "text",
      "validation": {
        "min_length": 2,
        "max_length": 50
      }
    },
    {
      "id": "framework",
      "question": "Which framework do you want to use?",
      "type": "select",
      "options": ["React", "Vue", "Angular", "Svelte"],
      "default": "React"
    },
    {
      "id": "typescript",
      "question": "Do you want to use TypeScript?",
      "type": "boolean",
      "default": "true"
    },
    {
      "id": "package_manager",
      "question": "Which package manager do you prefer?",
      "type": "select",
      "options": ["npm", "yarn", "pnpm"]
    }
  ],
  "required": true,
  "timeout_secs": 300
}
```

## Question Types

### Text Input
```json
{
  "id": "name",
  "question": "What is your name?",
  "type": "text",
  "default": "John Doe",
  "validation": {
    "min_length": 2,
    "max_length": 100
  }
}
```

### Number Input
```json
{
  "id": "age",
  "question": "What is your age?",
  "type": "number",
  "validation": {
    "min_value": 0,
    "max_value": 150
  }
}
```

### Boolean (Yes/No)
```json
{
  "id": "confirm",
  "question": "Do you want to proceed?",
  "type": "boolean",
  "default": "false"
}
```

### Single Choice (Select)
```json
{
  "id": "color",
  "question": "Choose a color:",
  "type": "select",
  "options": ["Red", "Green", "Blue"],
  "default": "Blue"
}
```

### Multiple Choice (Multi-select)
```json
{
  "id": "features",
  "question": "Select features to include:",
  "type": "multiselect",
  "options": ["Authentication", "Database", "API", "UI"]
}
```

### Password Input
```json
{
  "id": "password",
  "question": "Enter your password:",
  "type": "password",
  "validation": {
    "min_length": 8
  }
}
```

### Email Input
```json
{
  "id": "email",
  "question": "What is your email?",
  "type": "email"
}
```

### File Path Input
```json
{
  "id": "config_path",
  "question": "Where is your config file located?",
  "type": "file_path",
  "default": "./config.json"
}
```

## Validation Options

All questions support optional validation:

```json
{
  "validation": {
    "min_length": 5,
    "max_length": 100,
    "min_value": 1,
    "max_value": 10,
    "pattern": "^[a-zA-Z0-9]+$",
    "error_message": "Only alphanumeric characters allowed"
  }
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
      "answer": "My Awesome Project",
      "valid": true,
      "validation_error": null
    },
    {
      "question_id": "framework",
      "answer": "React",
      "valid": true,
      "validation_error": null
    }
  ],
  "message": "All questions answered successfully",
  "response_time_secs": 15.3
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