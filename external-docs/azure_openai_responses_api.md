# Azure OpenAI Responses API - Complete Documentation

## Overview

The Responses API is a stateful API from Azure OpenAI that combines capabilities from chat completions and assistants APIs in a unified experience. It supports the `computer-use-preview` model for Computer use capabilities.

## API Requirements

- **API Version**: v1 API required for latest features
- **Authentication**: API Key or Microsoft Entra ID
- **Base URL**: `https://YOUR-RESOURCE-NAME.openai.azure.com/openai/v1/`

## Region Availability

australiaeast, brazilsouth, canadacentral, canadaeast, eastus, eastus2, francecentral, germanywestcentral, italynorth, japaneast, koreacentral, northcentralus, norwayeast, polandcentral, southafricanorth, southcentralus, southeastasia, southindia, spaincentral, swedencentral, switzerlandnorth, uaenorth, uksouth, westus, westus3

## Supported Models

- **GPT-5 Series**: gpt-5.1-codex-max, gpt-5.1, gpt-5.1-chat, gpt-5.1-codex, gpt-5.1-codex-mini, gpt-5-pro, gpt-5-codex, gpt-5, gpt-5-mini, gpt-5-nano, gpt-5-chat
- **GPT-4 Series**: gpt-4o (multiple versions), gpt-4o-mini, gpt-4.1, gpt-4.1-nano, gpt-4.1-mini
- **Image Models**: gpt-image-1, gpt-image-1-mini
- **Reasoning Models**: o1, o3-mini, o3, o4-mini
- **Special**: computer-use-preview

## Core Endpoints

### 1. Create Response

**POST** `/responses`

```python
import os
from openai import OpenAI

client = OpenAI(
    api_key=os.getenv("AZURE_OPENAI_API_KEY"),
    base_url="https://YOUR-RESOURCE-NAME.openai.azure.com/openai/v1/",
)

response = client.responses.create(
    model="gpt-4.1-nano",
    input="This is a test.",
)

print(response.model_dump_json(indent=2))
```

**Using Microsoft Entra ID:**

```python
from openai import OpenAI
from azure.identity import DefaultAzureCredential, get_bearer_token_provider

token_provider = get_bearer_token_provider(
    DefaultAzureCredential(), "https://cognitiveservices.azure.com/.default"
)

client = OpenAI(
    base_url="https://YOUR-RESOURCE-NAME.openai.azure.com/openai/v1/",
    api_key=token_provider,
)

response = client.responses.create(
    model="gpt-4.1-nano",
    input="This is a test"
)
```

**Response Format:**

```json
{
  "id": "resp_67cb32528d6881909eb2859a55e18a85",
  "created_at": 1741369938.0,
  "model": "gpt-4o-2024-08-06",
  "object": "response",
  "output": [
    {
      "id": "msg_67cb3252cfac8190865744873aada798",
      "content": [
        {
          "annotations": [],
          "text": "Great! How can I help you today?",
          "type": "output_text"
        }
      ],
      "role": "assistant",
      "type": "message"
    }
  ],
  "output_text": "Great! How can I help you today?",
  "status": "completed",
  "usage": {
    "input_tokens": 20,
    "output_tokens": 11,
    "total_tokens": 31
  }
}
```

### 2. Retrieve Response

**GET** `/responses/{response_id}`

```python
response = client.responses.retrieve("resp_67cb61fa3a448190bcf2c42d96f0d1a8")
print(response.model_dump_json(indent=2))
```

```bash
curl -X GET https://YOUR-RESOURCE-NAME.openai.azure.com/openai/v1/responses/{response_id} \
  -H "Content-Type: application/json" \
  -H "api-key: $AZURE_OPENAI_API_KEY"
```

### 3. Delete Response

**DELETE** `/responses/{response_id}`

```python
response = client.responses.delete("resp_67cb61fa3a448190bcf2c42d96f0d1a8")
print(response)
```

Default retention: 30 days

### 4. List Input Items

**GET** `/responses/{response_id}/input_items`

```python
response = client.responses.input_items.list("resp_67d856fcfba0819081fd3cffee2aa1c0")
print(response.model_dump_json(indent=2))
```

## Advanced Features

### Chaining Responses

```python
# Initial response
response = client.responses.create(
    model="gpt-4o",
    input="Define and explain the concept of catastrophic forgetting?"
)

# Chain using previous_response_id
second_response = client.responses.create(
    model="gpt-4o",
    previous_response_id=response.id,
    input=[{"role": "user", "content": "Explain this at a college freshman level"}]
)
```

### Streaming

```python
response = client.responses.create(
    input="This is a test",
    model="o4-mini",
    stream=True
)

for event in response:
    if event.type == 'response.output_text.delta':
        print(event.delta, end='')
```

### Function Calling

```python
response = client.responses.create(
    model="gpt-4o",
    tools=[
        {
            "type": "function",
            "name": "get_weather",
            "description": "Get the weather for a location",
            "parameters": {
                "type": "object",
                "properties": {
                    "location": {"type": "string"},
                },
                "required": ["location"],
            },
        }
    ],
    input=[{"role": "user", "content": "What's the weather in San Francisco?"}],
)

# Handle tool calls
input_items = []
for output in response.output:
    if output.type == "function_call":
        input_items.append({
            "type": "function_call_output",
            "call_id": output.call_id,
            "output": '{"temperature": "70 degrees"}',
        })

second_response = client.responses.create(
    model="gpt-4o",
    previous_response_id=response.id,
    input=input_items
)
```

### Code Interpreter

```python
response = client.responses.create(
    model="gpt-4.1",
    tools=[
        {
            "type": "code_interpreter",
            "container": {"type": "auto"}
        }
    ],
    instructions="You are a personal math tutor. When asked a math question, write and run code using the python tool to answer the question.",
    input="I need to solve the equation 3x + 11 = 14. Can you help me?",
)

print(response.output)
```

**Supported File Types**: .c, .cs, .cpp, .csv, .doc, .docx, .html, .java, .json, .md, .pdf, .php, .pptx, .py, .rb, .tex, .txt, .css, .js, .sh, .ts, .xlsx, .xml, .zip, .jpeg, .jpg, .gif, .pkl, .png, .tar

### Image Input

**URL-based:**
```python
response = client.responses.create(
    model="gpt-4o",
    input=[
        {
            "role": "user",
            "content": [
                {"type": "input_text", "text": "what is in this image?"},
                {"type": "input_image", "image_url": "<image_URL>"}
            ]
        }
    ]
)
```

**Base64-encoded:**
```python
import base64

def encode_image(image_path):
    with open(image_path, "rb") as image_file:
        return base64.b64encode(image_file.read()).decode("utf-8")

base64_image = encode_image("path_to_your_image.jpg")

response = client.responses.create(
    model="gpt-4o",
    input=[
        {
            "role": "user",
            "content": [
                {"type": "input_text", "text": "what is in this image?"},
                {"type": "input_image", "image_url": f"data:image/jpeg;base64,{base64_image}"}
            ]
        }
    ]
)
```

### PDF Input

**Base64-encoded:**
```python
import base64

with open("PDF-FILE-NAME.pdf", "rb") as f:
    data = f.read()

base64_string = base64.b64encode(data).decode("utf-8")

response = client.responses.create(
    model="gpt-4o-mini",
    input=[
        {
            "role": "user",
            "content": [
                {
                    "type": "input_file",
                    "filename": "PDF-FILE-NAME.pdf",
                    "file_data": f"data:application/pdf;base64,{base64_string}",
                },
                {"type": "input_text", "text": "Summarize this PDF"},
            ],
        },
    ]
)

print(response.output_text)
```

**File ID (Upload first):**
```python
# Upload file with purpose "assistants" (user_data not currently supported)
file = client.files.create(
    file=open("nucleus_sampling.pdf", "rb"),
    purpose="assistants"
)

file_id = file.id

# Use file in response
response = client.responses.create(
    model="gpt-4o-mini",
    input=[
        {
            "role": "user",
            "content": [
                {"type": "input_file", "file_id": file_id},
                {"type": "input_text", "text": "Summarize this PDF"},
            ],
        },
    ]
)

print(response.output_text)
```

### Model Context Protocol (MCP)

```python
response = client.responses.create(
    model="gpt-4.1",
    tools=[
        {
            "type": "mcp",
            "server_label": "github",
            "server_url": "https://contoso.com/Azure/azure-rest-api-specs",
            "require_approval": "never"
        },
    ],
    input="What transport protocols are supported in the MCP spec?",
)

print(response.output_text)
```

**With Authentication:**
```python
response = client.responses.create(
    model="gpt-4.1",
    input="What is this repo in 100 words?",
    tools=[
        {
            "type": "mcp",
            "server_label": "github",
            "server_url": "https://gitmcp.io/Azure/azure-rest-api-specs",
            "headers": {
                "Authorization": "Bearer $YOUR_API_KEY"
            }
        }
    ]
)
```

**Approvals (when required):**
```python
# First get approval request
response = client.responses.create(
    model="gpt-4.1",
    tools=[
        {
            "type": "mcp",
            "server_label": "github",
            "server_url": "https://contoso.com/Azure/azure-rest-api-specs",
            "require_approval": "always"
        }
    ],
    input="What is this repo?"
)

# Then approve and continue
approval_id = response.output[0].id  # mcp_approval_request

response = client.responses.create(
    model="gpt-4.1",
    tools=[...],
    previous_response_id=response.id,
    input=[{
        "type": "mcp_approval_response",
        "approve": True,
        "approval_request_id": approval_id
    }],
)
```

### Background Tasks

```python
# Start background task
response = client.responses.create(
    model="o3",
    input="Write me a very long story",
    background=True
)

print(response.status)  # "queued" or "in_progress"

# Poll for completion
from time import sleep

while response.status in {"queued", "in_progress"}:
    print(f"Current status: {response.status}")
    sleep(2)
    response = client.responses.retrieve(response.id)

print(f"Final status: {response.status}\nOutput:\n{response.output_text}")
```

**Cancel background task:**
```python
response = client.responses.cancel("resp_1234567890")
print(response.status)
```

**Stream background response:**
```python
stream = client.responses.create(
    model="o3",
    input="Write me a very long story",
    background=True,
    stream=True,
)

cursor = None
for event in stream:
    print(event)
    cursor = event["sequence_number"]
```

**Resume streaming:**
```python
# Resume from sequence number
stream = client.responses.create(
    # ... parameters ...
    stream=True,
    starting_after=42
)
```

### Image Generation (Preview)

```python
import base64
from openai import OpenAI
from azure.identity import DefaultAzureCredential, get_bearer_token_provider

token_provider = get_bearer_token_provider(
    DefaultAzureCredential(), "https://cognitiveservices.azure.com/.default"
)

client = OpenAI(
    base_url="https://YOUR-RESOURCE-NAME.openai.azure.com/openai/v1/",
    api_key=token_provider,
    default_headers={
        "x-ms-oai-image-generation-deployment": "gpt-image-1",
        "api_version": "preview"
    }
)

response = client.responses.create(
    model="o3",
    input="Generate an image of gray tabby cat hugging an otter with an orange scarf",
    tools=[{"type": "image_generation"}],
)

# Save image
image_data = [
    output.result
    for output in response.output
    if output.type == "image_generation_call"
]

if image_data:
    image_base64 = image_data[0]
    with open("otter.png", "wb") as f:
        f.write(base64.b64decode(image_base64))
```

**Note**: Only supported by gpt-image-1 series, but can be called from gpt-4o, gpt-4o-mini, gpt-4.1, gpt-4.1-mini, gpt-4.1-nano, o3, gpt-5, and gpt-5.1 series.

### Encrypted Reasoning Items (Stateless Mode)

```python
response = client.responses.create(
    model="o4-mini",
    reasoning={"effort": "medium"},
    input="What is the weather like today?",
    tools=[{...}],
    include=["reasoning.encrypted_content"]
)
```

## Installation

```bash
pip install --upgrade openai
```

## Known Limitations

- Compaction with `/responses/compact` not supported
- Image generation using multi-turn editing and streaming not supported
- Images cannot be uploaded as files and referenced as input
- PDF file uploads with `user_data` purpose not supported (use `assistants`)
- Performance issues with background mode and streaming (resolution in progress)

## Response Status Values

- `completed`: Task finished successfully
- `queued`: Waiting to be processed (background mode)
- `in_progress`: Currently processing (background mode)

## Source

Documentation fetched from: https://learn.microsoft.com/en-us/azure/ai-foundry/openai/how-to/responses?view=foundry-classic
