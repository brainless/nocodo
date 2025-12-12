# OpenAI Responses API on AWS Bedrock - Complete Documentation

## Overview
The Responses API provides stateful conversation management with support for streaming, background processing, and multi-turn interactions. For complete API details, see the [OpenAI Responses documentation](https://platform.openai.com/docs/api-reference/responses).

## Endpoint
**POST** `/v1/responses`

## Supported Regions
Available endpoints:
- `bedrock-mantle.us-east-2.api.aws` (US East - Ohio)
- `bedrock-mantle.us-east-1.api.aws` (US East - N. Virginia)
- `bedrock-mantle.us-west-2.api.aws` (US West - Oregon)
- `bedrock-mantle.ap-southeast-3.api.aws` (Asia Pacific - Jakarta)
- `bedrock-mantle.ap-south-1.api.aws` (Asia Pacific - Mumbai)
- `bedrock-mantle.ap-northeast-1.api.aws` (Asia Pacific - Tokyo)
- `bedrock-mantle.eu-central-1.api.aws` (Europe - Frankfurt)
- `bedrock-mantle.eu-west-1.api.aws` (Europe - Ireland)
- `bedrock-mantle.eu-west-2.api.aws` (Europe - London)
- `bedrock-mantle.eu-south-1.api.aws` (Europe - Milan)
- `bedrock-mantle.eu-north-1.api.aws` (Europe - Stockholm)
- `bedrock-mantle.sa-east-1.api.aws` (South America - São Paulo)

## Prerequisites
- **Authentication**: Amazon Bedrock API key or AWS credentials
- **Environment Variables**:
  - `OPENAI_API_KEY` – Your Amazon Bedrock API key
  - `OPENAI_BASE_URL` – Regional endpoint (e.g., `https://bedrock-mantle.us-east-1.api.aws/v1`)

## Basic Request

### OpenAI SDK (Python)
```python
# Create a basic response using the OpenAI SDK
# Requires OPENAI_API_KEY and OPENAI_BASE_URL environment variables

from openai import OpenAI

client = OpenAI()

response = client.responses.create(
    model="openai.gpt-oss-120b",
    input=[
        {"role": "user", "content": "Hello! How can you help me today?"}
    ]
)

print(response)
```

### HTTP Request
```bash
# Create a basic response
# Requires OPENAI_API_KEY and OPENAI_BASE_URL environment variables

curl -X POST $OPENAI_BASE_URL/responses \
   -H "Content-Type: application/json" \
   -H "Authorization: Bearer $OPENAI_API_KEY" \
   -d '{
    "model": "openai.gpt-oss-120b",
    "input": [
        {"role": "user", "content": "Hello! How can you help me today?"}
    ]
}'
```

## Stream Responses

### OpenAI SDK (Python)
```python
# Stream response events incrementally using the OpenAI SDK
# Requires OPENAI_API_KEY and OPENAI_BASE_URL environment variables

from openai import OpenAI

client = OpenAI()

stream = client.responses.create(
    model="openai.gpt-oss-120b",
    input=[{"role": "user", "content": "Tell me a story"}],
    stream=True
)

for event in stream:
    print(event)
```

### HTTP Request
```bash
# Stream response events incrementally
# Requires OPENAI_API_KEY and OPENAI_BASE_URL environment variables

curl -X POST $OPENAI_BASE_URL/responses \
   -H "Content-Type: application/json" \
   -H "Authorization: Bearer $OPENAI_API_KEY" \
   -d '{
    "model": "openai.gpt-oss-120b",
    "input": [
        {"role": "user", "content": "Tell me a story"}
    ],
    "stream": true
}'
```

## Request Parameters
| Parameter | Type | Description |
|-----------|------|-------------|
| `model` | string | The model to use (e.g., `openai.gpt-oss-120b`) |
| `input` | array | Array of message objects with `role` and `content` |
| `stream` | boolean | Optional. Set to `true` for streaming responses |

## Key Features
- **Asynchronous inference** – Support for long-running inference workloads
- **Stateful conversation management** – Automatically rebuild context without manually passing conversation history
- **Flexible response modes** – Support for both streaming and non-streaming responses
- **Multi-turn interactions** – Manage stateful conversations across multiple turns

## Source

Documentation fetched from: https://docs.aws.amazon.com/bedrock/latest/userguide/bedrock-mantle.html
