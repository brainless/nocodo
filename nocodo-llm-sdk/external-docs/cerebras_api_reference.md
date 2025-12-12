# Chat Completions

## Request

<ParamField path="logprobs" type="bool">
  Whether to return log probabilities of the output tokens or not.

  Default: `False`
</ParamField>

<ParamField path="max_completion_tokens" type="integer | null">
  The maximum number of tokens that can be generated in the completion, including reasoning tokens. The total length of input tokens and generated tokens is limited by the model's context length.

  Default settings: `qwen-3-32b` = 40k | `llama-3.3-70b` = 64k.
</ParamField>

<ParamField path="messages" type="object[]" required="true">
  A list of messages comprising the conversation so far.

  **Note**: System prompts must be passed to the `messages` parameter as a string. Support for other object types will be added in future releases.
</ParamField>

<ParamField path="model" type="string" required="true">
  Available options:

  * `llama3.1-8b`
  * `llama-3.3-70b`
  * `qwen-3-32b`
  * `qwen-3-235b-a22b-instruct-2507` (preview)
  * `gpt-oss-120b`
  * `zai-glm-4.6` (preview)
</ParamField>

<ParamField path="parallel_tool_calls" type="boolean | null">
  Whether to enable parallel function calling during tool use. When enabled (default), the model can request multiple tool calls simultaneously in a single response. When disabled, the model will only request one tool call at a time.

  Default: `true`
</ParamField>

<ParamField body="prediction" type="object | null">
  Configuration for a [Predicted Output](/capabilities/predicted-outputs), which can greatly speed up response times when large parts of the model response are known in advance. This is most common when you are regenerating a file with mostly minor changes to the content.

  <Expandable title="possible types">
    <br />

    ##### Static Content

    Static predicted output content, such as the content of a text or code file that is being regenerated.

    <Expandable title="properties">
      <ResponseField name="content" type="string | array" required>
        The content that should be matched when generating a model response. If continuous token sequences from the generated tokens match this content, the entire model response can be returned faster.

        <Expandable title="possible types">
          <br />

          ##### Text content `string`

          The content used for a given Predicted Output. Typically the text of a file you are regenerating with only minor changes. <br />

          <br />

          ##### Array of content parts `array`

          An array of content parts with a defined type. Supported options may differ based on the [model](/models/) that is used to generate the response. May contain text inputs.

          <Expandable title="possible types">
            <ResponseField name="text" type="string" required>
              The text content.
            </ResponseField>

            <ResponseField name="type" type="string" required>
              The type of content part. Always `text`.
            </ResponseField>
          </Expandable>

          <br />
        </Expandable>
      </ResponseField>

      <ResponseField name="type" type="string" required>
        The type of the predicted content you wish to provide. This type is currently always <code>content</code>.
      </ResponseField>
    </Expandable>

    <br />
  </Expandable>

  Visit our page on [Predicted Outputs](/capabilities/predicted-outputs) for more information and examples.
</ParamField>

<ParamField path="reasoning_effort" type="string | null">
  Controls the amount of reasoning the model performs. Available values:

  * `"low"` - Minimal reasoning, faster responses
  * `"medium"` - Moderate reasoning (default)
  * `"high"` - Extensive reasoning, more thorough analysis

  <Note>
    This flag is only available for [gpt-oss-120b](/models/openai-oss) model.
  </Note>
</ParamField>

<ParamField path="response_format" type="object | null">
  An object that controls the format of the model response.

  Setting to `{ "type": "json_schema", "json_schema": { "name": "schema_name", "strict": true, "schema": {...} } }` enforces schema compliance by ensuring that the model output conforms to your specified JSON schema. See [Structured Outputs](../capabilities/structured-outputs) for more information.

  Setting `{ "type": "json_object" }` enables the legacy JSON mode, ensuring that the model output is valid JSON. However, using `json_schema` is recommended for models that support it.

  <Expandable title="properties">
    <br />

    ##### Text `object`

    Default response format. Generates plain text responses.

    <Expandable title="properties">
      <ResponseField name="type" type="string" required>
        The type of response format being defined. Always `text`.
      </ResponseField>
    </Expandable>

    <br />

    ##### JSON schema `object`

    Generates structured JSON output that conforms to the specified schema. Use this format when you need the model to return structured JSON.

    <Expandable title="properties">
      <ParamField path="json_schema" type="object" required>
        Structured Outputs configuration options.
      </ParamField>

      <Expandable title="properties">
        <ParamField path="name" type="string" required>
          An optional name for your schema.
        </ParamField>

        <ParamField path="description" type="string" optional>
          A description of the response format’s purpose, used by the model to determine how to generate its response in that format.
        </ParamField>

        <ParamField path="schema" type="object">
          A valid [JSON Schema](https://json-schema.org/) object that defines the structure, types, and requirements for the response. Supports standard JSON Schema features including types (string, number, boolean, integer, object, array, enum, anyOf, null), nested structures, required fields, and additionalProperties (must be set to false).
        </ParamField>

        <ParamField path="strict" type="boolean">
          When set to `true`, enforces strict adherence to the schema. The model will only return fields defined in the schema and with the correct types. When `false`, behaves similar to JSON mode but uses the schema as a guide. Defaults to `false`.
        </ParamField>
      </Expandable>

      <ParamField path="type" type="string" required>
        The type of response format being defined. Always `json_schema`.
      </ParamField>
    </Expandable>

    <br />

    ##### JSON object `object`

    A legacy method for generating JSON responses. Using `json_schema` is recommended for models that support it. To use `json_object` remember to also include a system or user message to specify the desired format.

    <Expandable title="properties">
      <ResponseField name="type" type="string" required>
        The type of response format being defined. Always `json_object`.
      </ResponseField>
    </Expandable>

    <Note>
      When using JSON object, you must explicitly instruct the model to generate JSON through a system or user message. `json_object` is not compatible with streaming - `stream` must be set to `false`.
    </Note>

    <br />
  </Expandable>
</ParamField>

<ParamField path="seed" type="integer | null">
  If specified, our system will make a best effort to sample deterministically, such that repeated requests with the same `seed` and parameters should return the same result. Determinism is not guaranteed.
</ParamField>

<ParamField path="stop" type="string | null">
  Up to 4 sequences where the API will stop generating further tokens. The returned text will not contain the stop sequence.
</ParamField>

<ParamField path="stream" type="boolean | null">
  If set, partial message deltas will be sent.
</ParamField>

<ParamField path="temperature" type="number | null">
  What sampling temperature to use, between 0 and 1.5. Higher values like 0.8 will make the output more random, while lower values like 0.2 will make it more focused and deterministic. We generally recommend altering this or top\_p but not both.
</ParamField>

<ParamField path="top_logprobs" type="integer | null">
  An integer between 0 and 20 specifying the number of most likely tokens to return at each token position, each with an associated log probability.
  `logprobs` must be set to true if this parameter is used.
</ParamField>

<ParamField path="top_p" type="number | null">
  An alternative to sampling with temperature, called nucleus sampling, where the model considers the results of the tokens with top\_p probability mass. So, 0.1 means only the tokens comprising the top 10% probability mass are considered. We generally recommend altering this or temperature but not both.
</ParamField>

<ParamField path="tool_choice" type="string | object">
  Controls which (if any) tool is called by the model. `none` means the model will not call any tool and instead generates a message. `auto` means the model can pick between generating a message or calling one or more tools. required means the model must call one or more tools. Specifying a particular tool via `{"type": "function", "function": {"name": "my_function"}}` forces the model to call that tool.

  `none` is the default when no tools are present. `auto` is the default if tools are present.
</ParamField>

<ParamField path="tools" type="object | null">
  A list of tools the model may call. Currently, only functions are supported as a tool. Use this to provide a list of functions the model may generate JSON inputs for.

  Specifying tools consumes prompt tokens in the context. If too many are given, the model may perform poorly or you may hit context length limitations

  <Expandable title="properties">
    <ParamField path="tools.function.description" type="string">
      A description of what the function does, used by the model to choose when and how to call the function.
    </ParamField>

    <ParamField path="tools.function.name" type="string">
      The name of the function to be called. Must be a-z, A-Z, 0-9, or contain underscores and dashes, with a maximum length of 64.
    </ParamField>

    <ParamField path="tools.function.parameters" type="object">
      The parameters the functions accepts, described as a JSON Schema object. Omitting parameters defines a function with an empty parameter list.
    </ParamField>

    <ParamField path="tools.type" type="string">
      The type of the tool. Currently, only `function` is supported.
    </ParamField>
  </Expandable>
</ParamField>

<ParamField path="user" type="string | null">
  A unique identifier representing your end-user, which can help to monitor and detect abuse.
</ParamField>

## Response

<ResponseField name="id" type="string">
  A unique identifier for the chat completion.
</ResponseField>

<ResponseField name="choices" type="object[]">
  A list of chat completion choices. Can be more than one if `n` is greater than 1.

  <Expandable title="choice properties">
    <ResponseField name="finish_reason" type="string">
      The reason the model stopped generating tokens. Possible values: `stop`, `length`, `content_filter`, `tool_calls`.
    </ResponseField>

    <ResponseField name="index" type="integer">
      The index of the choice in the list of choices.
    </ResponseField>

    <ResponseField name="message" type="object">
      A chat completion message generated by the model.

      <Expandable title="message properties">
        <ResponseField name="content" type="string">
          The contents of the message.
        </ResponseField>

        <ResponseField name="role" type="string">
          The role of the author of this message (always `assistant`).
        </ResponseField>

        <ResponseField name="reasoning" type="string">
          The model's reasoning content when using reasoning models (e.g., `gpt-oss-120b` with `reasoning_effort` set).
        </ResponseField>
      </Expandable>
    </ResponseField>
  </Expandable>
</ResponseField>

<ResponseField name="created" type="integer">
  The Unix timestamp (in seconds) of when the chat completion was created.
</ResponseField>

<ResponseField name="model" type="string">
  The model used for the chat completion.
</ResponseField>

<ResponseField name="object" type="string">
  The object type, which is always `chat.completion`.
</ResponseField>

<ResponseField name="usage" type="object">
  Usage statistics for the completion request.

  <Expandable title="usage properties">
    <ResponseField name="prompt_tokens" type="integer">
      Number of tokens in the prompt.
    </ResponseField>

    <ResponseField name="completion_tokens" type="integer">
      Number of tokens in the generated completion.
    </ResponseField>

    <ResponseField name="total_tokens" type="integer">
      Total number of tokens used in the request (prompt + completion).
    </ResponseField>

    <ResponseField name="completion_tokens_details" type="object">
      Breakdown of completion tokens when using Predicted Outputs.

      <Expandable title="properties">
        <ResponseField name="accepted_prediction_tokens" type="integer">
          When using Predicted Outputs, the number of tokens in the prediction that appeared in the completion.
        </ResponseField>

        <ResponseField name="rejected_prediction_tokens" type="integer">
          When using Predicted Outputs, the number of tokens in the prediction that did not appear in the completion. Like reasoning tokens, these tokens are still counted in the total completion tokens for the purposes of billing, output, and context window limits.
        </ResponseField>
      </Expandable>
    </ResponseField>
  </Expandable>
</ResponseField>

<RequestExample>
  ```python Python theme={null}
  from cerebras.cloud.sdk import Cerebras
  import os 

  client = Cerebras(api_key=os.environ.get("CEREBRAS_API_KEY"),)

  chat_completion = client.chat.completions.create(
      model="gpt-oss-120b",
      messages=[
          {"role": "user", "content": "Hello!",}
      ],
  )
  print(chat_completion)
  ```

  ```javascript Node.js theme={null}
  import Cerebras from '@cerebras/cerebras_cloud_sdk';

  const client = new Cerebras({
    apiKey: process.env['CEREBRAS_API_KEY'],
  });

  async function main() {
    const completionCreateResponse = await client.chat.completions.create({
      messages: [{ role: 'user', content: 'Hello!' }],
      model: 'gpt-oss-120b',
    });

    console.log(completionCreateResponse);
  }
  main();
  ```

  ```cli cURL theme={null}
  curl --location 'https://api.cerebras.ai/v1/chat/completions' \
  --header 'Content-Type: application/json' \
  --header "Authorization: Bearer ${CEREBRAS_API_KEY}" \
  --data '{
    "model": "gpt-oss-120b",
    "stream": false,
    "messages": [{"content": "Hello!", "role": "user"}],
    "temperature": 0,
    "max_completion_tokens": -1,
    "seed": 0,
    "top_p": 1
  }'
  ```
</RequestExample>

<ResponseExample>
  ```json Response theme={null}
  {
    "id": "chatcmpl-292e278f-514e-4186-9010-91ce6a14168b",
    "choices": [
      {
        "finish_reason": "stop",
        "index": 0,
        "message": {
          "content": "Hello! How can I assist you today?",
          "reasoning": "The user is asking for a simple greeting to the world. This is a straightforward request that doesn't require complex analysis. I should provide a friendly, direct response.",
          "role": "assistant"
        }
      }
    ],
    "created": 1723733419,
    "model": "gpt-oss-120b",
    "system_fingerprint": "fp_70185065a4",
    "object": "chat.completion",
    "usage": {
      "prompt_tokens": 12,
      "completion_tokens": 10,
      "total_tokens": 22,
      "completion_tokens_details": {
        "accepted_prediction_tokens": 0,
        "rejected_prediction_tokens": 0
      }
    },
    "time_info": {
      "queue_time": 0.000073161,
      "prompt_time": 0.0010744798888888889,
      "completion_time": 0.005658071111111111,
      "total_time": 0.022224903106689453,
      "created": 1723733419
    }
  }
  ```
</ResponseExample>


---

> To find navigation and other pages in this documentation, fetch the llms.txt file at: https://inference-docs.cerebras.ai/llms.txt
