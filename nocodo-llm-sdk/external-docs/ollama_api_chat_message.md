> ## Documentation Index
> Fetch the complete documentation index at: https://docs.ollama.com/llms.txt
> Use this file to discover all available pages before exploring further.

# Generate a chat message

> Generate the next chat message in a conversation between a user and an assistant.



## OpenAPI

````yaml openapi.yaml post /api/chat
openapi: 3.1.0
info:
  title: Ollama API
  version: 0.1.0
  license:
    name: MIT
    url: https://opensource.org/licenses/MIT
  description: |
    OpenAPI specification for the Ollama HTTP API
servers:
  - url: http://localhost:11434
    description: Ollama
security: []
paths:
  /api/chat:
    post:
      summary: Generate a chat message
      description: >-
        Generate the next chat message in a conversation between a user and an
        assistant.
      operationId: chat
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/ChatRequest'
      responses:
        '200':
          description: Chat response
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ChatResponse'
              example:
                model: gemma3
                created_at: '2025-10-17T23:14:07.414671Z'
                message:
                  role: assistant
                  content: Hello! How can I help you today?
                done: true
                done_reason: stop
                total_duration: 174560334
                load_duration: 101397084
                prompt_eval_count: 11
                prompt_eval_duration: 13074791
                eval_count: 18
                eval_duration: 52479709
            application/x-ndjson:
              schema:
                $ref: '#/components/schemas/ChatStreamEvent'
components:
  schemas:
    ChatRequest:
      type: object
      required:
        - model
        - messages
      properties:
        model:
          type: string
          description: Model name
        messages:
          type: array
          description: >-
            Chat history as an array of message objects (each with a role and
            content)
          items:
            $ref: '#/components/schemas/ChatMessage'
        tools:
          type: array
          description: Optional list of function tools the model may call during the chat
          items:
            $ref: '#/components/schemas/ToolDefinition'
        format:
          oneOf:
            - type: string
              enum:
                - json
            - type: object
          description: Format to return a response in. Can be `json` or a JSON schema
        options:
          $ref: '#/components/schemas/ModelOptions'
        stream:
          type: boolean
          default: true
        think:
          oneOf:
            - type: boolean
            - type: string
              enum:
                - high
                - medium
                - low
          description: >-
            When true, returns separate thinking output in addition to content.
            Can be a boolean (true/false) or a string ("high", "medium", "low")
            for supported models.
        keep_alive:
          oneOf:
            - type: string
            - type: number
          description: >-
            Model keep-alive duration (for example `5m` or `0` to unload
            immediately)
        logprobs:
          type: boolean
          description: Whether to return log probabilities of the output tokens
        top_logprobs:
          type: integer
          description: >-
            Number of most likely tokens to return at each token position when
            logprobs are enabled
    ChatResponse:
      type: object
      properties:
        model:
          type: string
          description: Model name used to generate this message
        created_at:
          type: string
          format: date-time
          description: Timestamp of response creation (ISO 8601)
        message:
          type: object
          properties:
            role:
              type: string
              enum:
                - assistant
              description: Always `assistant` for model responses
            content:
              type: string
              description: Assistant message text
            thinking:
              type: string
              description: Optional deliberate thinking trace when `think` is enabled
            tool_calls:
              type: array
              items:
                $ref: '#/components/schemas/ToolCall'
              description: Tool calls requested by the assistant
            images:
              type: array
              items:
                type: string
              description: Optional base64-encoded images in the response
        done:
          type: boolean
          description: Indicates whether the chat response has finished
        done_reason:
          type: string
          description: Reason the response finished
        total_duration:
          type: integer
          description: Total time spent generating in nanoseconds
        load_duration:
          type: integer
          description: Time spent loading the model in nanoseconds
        prompt_eval_count:
          type: integer
          description: Number of tokens in the prompt
        prompt_eval_duration:
          type: integer
          description: Time spent evaluating the prompt in nanoseconds
        eval_count:
          type: integer
          description: Number of tokens generated in the response
        eval_duration:
          type: integer
          description: Time spent generating tokens in nanoseconds
        logprobs:
          type: array
          items:
            $ref: '#/components/schemas/Logprob'
          description: >-
            Log probability information for the generated tokens when logprobs
            are enabled
    ChatStreamEvent:
      type: object
      properties:
        model:
          type: string
          description: Model name used for this stream event
        created_at:
          type: string
          format: date-time
          description: When this chunk was created (ISO 8601)
        message:
          type: object
          properties:
            role:
              type: string
              description: Role of the message for this chunk
            content:
              type: string
              description: Partial assistant message text
            thinking:
              type: string
              description: Partial thinking text when `think` is enabled
            tool_calls:
              type: array
              items:
                $ref: '#/components/schemas/ToolCall'
              description: Partial tool calls, if any
            images:
              type: array
              items:
                type: string
              description: Partial base64-encoded images, when present
        done:
          type: boolean
          description: True for the final event in the stream
    ChatMessage:
      type: object
      required:
        - role
        - content
      properties:
        role:
          type: string
          enum:
            - system
            - user
            - assistant
            - tool
          description: Author of the message.
        content:
          type: string
          description: Message text content
        images:
          type: array
          items:
            type: string
            description: Base64-encoded image content
          description: Optional list of inline images for multimodal models
        tool_calls:
          type: array
          items:
            $ref: '#/components/schemas/ToolCall'
          description: Tool call requests produced by the model
    ToolDefinition:
      type: object
      required:
        - type
        - function
      properties:
        type:
          type: string
          enum:
            - function
          description: Type of tool (always `function`)
        function:
          type: object
          required:
            - name
            - parameters
          properties:
            name:
              type: string
              description: Function name exposed to the model
            description:
              type: string
              description: Human-readable description of the function
            parameters:
              type: object
              description: JSON Schema for the function parameters
    ModelOptions:
      type: object
      description: Runtime options that control text generation
      properties:
        seed:
          type: integer
          description: Random seed used for reproducible outputs
        temperature:
          type: number
          format: float
          description: Controls randomness in generation (higher = more random)
        top_k:
          type: integer
          description: Limits next token selection to the K most likely
        top_p:
          type: number
          format: float
          description: Cumulative probability threshold for nucleus sampling
        min_p:
          type: number
          format: float
          description: Minimum probability threshold for token selection
        stop:
          oneOf:
            - type: string
            - type: array
              items:
                type: string
          description: Stop sequences that will halt generation
        num_ctx:
          type: integer
          description: Context length size (number of tokens)
        num_predict:
          type: integer
          description: Maximum number of tokens to generate
      additionalProperties: true
    ToolCall:
      type: object
      properties:
        function:
          type: object
          required:
            - name
          properties:
            name:
              type: string
              description: Name of the function to call
            description:
              type: string
              description: What the function does
            arguments:
              type: object
              description: JSON object of arguments to pass to the function
    Logprob:
      type: object
      description: Log probability information for a generated token
      properties:
        token:
          type: string
          description: The text representation of the token
        logprob:
          type: number
          description: The log probability of this token
        bytes:
          type: array
          items:
            type: integer
          description: The raw byte representation of the token
        top_logprobs:
          type: array
          items:
            $ref: '#/components/schemas/TokenLogprob'
          description: Most likely tokens and their log probabilities at this position
    TokenLogprob:
      type: object
      description: Log probability information for a single token alternative
      properties:
        token:
          type: string
          description: The text representation of the token
        logprob:
          type: number
          description: The log probability of this token
        bytes:
          type: array
          items:
            type: integer
          description: The raw byte representation of the token

````
