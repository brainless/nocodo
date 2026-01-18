Text embedding models

# Text embedding models

The Voyage text embedding endpoint receives as input a string (or a list of strings) and other arguments such as the preferred model name, and returns a response containing a list of embeddings.

# OpenAPI definition

```json
{
  "openapi": "3.0.2",
  "info": {
    "title": "Voyage API",
    "description": "The VoyageAI REST API. Please see https://docs.voyageai.com/reference for more details.\n",
    "version": "1.1",
    "contact": {
      "name": "VoyageAI Support",
      "url": "https://docs.voyageai.com/docs/faq",
      "email": "contact@voyageai.com"
    },
    "license": {
      "name": "MIT",
      "url": "https://github.com/voyage-ai/voyage-openapi/blob/main/LICENSE"
    }
  },
  "servers": [
    {
      "url": "https://api.voyageai.com/v1"
    }
  ],
  "security": [
    {
      "ApiKeyAuth": []
    }
  ],
  "tags": [
    {
      "name": "Inference"
    }
  ],
  "paths": {
    "/embeddings": {
      "post": {
        "tags": [
          "Inference"
        ],
        "summary": "Text embedding models",
        "description": "The Voyage text embedding endpoint receives as input a string (or a list of strings) and other arguments such as the preferred model name, and returns a response containing a list of embeddings.",
        "operationId": "embeddings-api",
        "requestBody": {
          "content": {
            "application/json": {
              "schema": {
                "type": "object",
                "required": [
                  "input",
                  "model"
                ],
                "properties": {
                  "input": {
                    "type": "object",
                    "description": "A single text string, or a list of texts as a list of strings, such as `[\"I like cats\", \"I also like dogs\"]`. Currently, we have two constraints on the list: <ul>  <li> The maximum length of the list is 1,000. </li>  <li> The total number of tokens in the list is at most 1M for `voyage-4-lite` and `voyage-3.5-lite`; 320K for `voyage-4`, `voyage-3.5` and `voyage-2`; and 120K for `voyage-4-large`, `voyage-3-large`, `voyage-code-3`, `voyage-finance-2`, and `voyage-law-2`. </li> <ul>\n",
                    "oneOf": [
                      {
                        "type": "string"
                      },
                      {
                        "type": "array",
                        "items": {
                          "type": "string"
                        }
                      }
                    ]
                  },
                  "model": {
                    "type": "string",
                    "description": "Name of the model. Recommended options: `voyage-4-large`, `voyage-4`, `voyage-4-lite`, `voyage-3-large`, `voyage-3.5`, `voyage-3.5-lite`, `voyage-code-3`, `voyage-finance-2`, `voyage-law-2`.\n"
                  },
                  "input_type": {
                    "type": "string",
                    "description": "Type of the input text. Defaults to `null`. Other options: `query`, `document`. <ul> <li> When `input_type` is `null`, the embedding model directly converts the inputs into numerical vectors. For retrieval/search purposes, where a \"query\" is used to search for relevant information among a collection of data referred to as \"documents,\" we recommend specifying whether your inputs are intended as queries or documents by setting `input_type` to `query` or `document`, respectively. In these cases, Voyage automatically prepends a prompt to your `inputs` before vectorizing them, creating vectors more tailored for retrieval/search tasks. Embeddings generated with and without the `input_type` argument are compatible. </li> <li> For transparency, the following prompts are prepended to your input. </li>\n  <ul>\n    <li> For <code>query</code>, the prompt is <i>\"Represent the query for retrieving supporting documents: \".</i> </li>\n    <li> For <code>document</code>, the prompt is <i>\"Represent the document for retrieval: \".</i> </li>\n  </ul> \n<ul> <ul>\n",
                    "enum": [
                      null,
                      "query",
                      "document"
                    ],
                    "nullable": true,
                    "default": null
                  },
                  "truncation": {
                    "type": "boolean",
                    "description": "Whether to truncate the input texts to fit within the context length. Defaults to `true`. <ul>  <li> If `true`, an over-length input texts will be truncated to fit within the context length, before vectorized by the embedding model. </li>  <li> If `false`, an error will be raised if any given text exceeds the context length. </li>  </ul>\n",
                    "default": true
                  },
                  "output_dimension": {
                    "type": "integer",
                    "description": "The number of dimensions for resulting output embeddings. Defaults to `null`. <ul> <li> Most models only support a single default dimension, used when `output_dimension` is set to `null` (see output embedding dimensions <a href=\"https://docs.voyageai.com/docs/embeddings\" target=\"_blank\">here</a>). </li> <li> `voyage-4-large`, `voyage-4`, `voyage-4-lite`, `voyage-3-large`, `voyage-3.5`, and `voyage-3.5-lite`, and `voyage-code-3` support the following `output_dimension` values: 2048, 1024 (default), 512, and 256. </li> </ul>\n",
                    "nullable": true,
                    "default": null
                  },
                  "output_dtype": {
                    "type": "string",
                    "description": "The data type for the embeddings to be returned. Defaults to `float`. Other options: `int8`, `uint8`, `binary`, `ubinary`. `float` is supported for all models. `int8`, `uint8`, `binary`, and `ubinary` are supported by `voyage-4-large`, `voyage-4`, `voyage-4-lite`, `voyage-3-large`, `voyage-3.5`, and `voyage-3.5-lite`, and `voyage-code-3`. Please see our <a href=\"https://docs.voyageai.com/docs/flexible-dimensions-and-quantization#quantization\" target=\"_blank\">guide</a> for more details about output data types. <ul> <li> `float`: Each returned embedding is a list of 32-bit (4-byte) <a href=\"https://en.wikipedia.org/wiki/Single-precision_floating-point_format\" target=\"_blank\">single-precision floating-point</a> numbers. This is the default and provides the highest precision / retrieval accuracy. </li> <li> `int8` and `uint8`: Each returned embedding is a list of 8-bit (1-byte) integers ranging from -128 to 127 and 0 to 255, respectively. </li> <li> `binary` and `ubinary`: Each returned embedding is a list of 8-bit integers that represent bit-packed, quantized single-bit embedding values: `int8` for `binary` and `uint8` for `ubinary`. The length of the returned list of integers is 1/8 of `output_dimension` (which is the actual dimension of the embedding). The `binary` type uses the offset binary method. Please refer to our guide for details on <a href=\"https://docs.voyageai.com/docs/flexible-dimensions-and-quantization#offset-binary\" target=\"_blank\">offset binary</a> and <a href=\"https://docs.voyageai.com/docs/flexible-dimensions-and-quantization#quantization\" target=\"_blank\">binary embeddings</a>.  </ul>\n",
                    "enum": [
                      "float",
                      "int8",
                      "uint8",
                      "binary",
                      "ubinary"
                    ],
                    "default": "float"
                  },
                  "encoding_format": {
                    "type": "string",
                    "description": "Format in which the embeddings are encoded. Defaults to `null`. Other options: `base64`. <ul> <li> If `null`, each embedding is an array of float numbers when `output_dtype` is set to `float` and as an array of integers for all other values of `output_dtype` (`int8`, `uint8`, `binary`, and `ubinary`). <li> If `base64`, the embeddings are represented as a <a href=\"https://docs.python.org/3/library/base64.html\" target=\"_blank\">Base64-encoded</a> NumPy array of: </li>\n  <ul>\n    <li> Floating-point numbers (<a href=\"https:/umpy.org/doc/2.1/user/basics.types.html#numerical-data-types\" target=\"_blank\">numpy.float32</a>) for <code>output_dtype</code> set to <code>float</code>. </li>\n    <li> Signed integers (<a href=\"https:/umpy.org/doc/2.1/user/basics.types.html#numerical-data-types\" target=\"_blank\">numpy.int8</a>) for <code>output_dtype</code> set to <code>int8</code> or <code>binary</code>. </li>\n    <li> Unsigned integers (<a href=\"https:/umpy.org/doc/2.1/user/basics.types.html#numerical-data-types\" target=\"_blank\">numpy.uint8</a>) for <code>output_dtype</code> set to <code>uint8</code> or <code>ubinary</code>. </li>\n  </ul> \n</ul>\n",
                    "enum": [
                      null,
                      "base64"
                    ],
                    "nullable": true,
                    "default": null
                  }
                }
              }
            }
          }
        },
        "responses": {
          "200": {
            "description": "Success",
            "content": {
              "application/json": {
                "schema": {
                  "type": "object",
                  "properties": {
                    "object": {
                      "type": "string",
                      "description": "The object type, which is always `list`."
                    },
                    "data": {
                      "type": "array",
                      "description": "An array of embedding objects.",
                      "items": {
                        "type": "object",
                        "properties": {
                          "object": {
                            "type": "string",
                            "description": "The object type, which is always `embedding`."
                          },
                          "embedding": {
                            "type": "array",
                            "description": "Each embedding is a vector represented as an array of float numbers when `output_dtype` is set to `float` and as an array of integers for all other values of `output_dtype` (`int8`, `uint8`, `binary`, and `ubinary`). The length of this vector varies depending on the specific model, `output_dimension`, and `output_dtype`.\n",
                            "items": {
                              "type": "number"
                            }
                          },
                          "index": {
                            "type": "integer",
                            "description": "An integer representing the index of the embedding within the list of embeddings.\n"
                          }
                        }
                      }
                    },
                    "model": {
                      "type": "string",
                      "description": "Name of the model."
                    },
                    "usage": {
                      "type": "object",
                      "properties": {
                        "total_tokens": {
                          "type": "integer",
                          "description": "The total number of tokens used for computing the embeddings."
                        }
                      }
                    }
                  },
                  "x-readme-ref-name": "EmbeddingsObject"
                },
                "examples": {
                  "Success": {
                    "value": "{\"object\":\"list\",\"data\":[{\"object\":\"embedding\",\"embedding\":[-0.016709786,0.026996311,-0.027496673,\"...\",-0.012125067],\"index\":0}, {\"object\":\"embedding\",\"embedding\":[0.003613521,0.026428301,-0.009491397,\"...\",-0.028471239],\"index\":1}],\"model\":\"voyage-4-large\", \"usage\":{\"total_tokens\":8}}\n"
                  }
                }
              }
            }
          },
          "4XX": {
            "description": "Client error  <p> This indicates an issue with the request format or frequency. Please see our  [Error Codes](https://docs.voyageai.com/docs/error-codes) guide. </p>\n",
            "content": {
              "application/json": {
                "schema": {
                  "properties": {
                    "detail": {
                      "type": "string",
                      "description": "The error message."
                    }
                  }
                }
              }
            }
          },
          "5XX": {
            "description": "Server Error <p> This indicates our servers are experiencing high traffic or having an unexpected issue. Please see our  [Error Codes](https://docs.voyageai.com/docs/error-codes) guide. </p>\n"
          }
        },
        "x-readme": {
          "code-samples": [
            {
              "language": "shell",
              "code": "curl --request POST \\\n     --url https://api.voyageai.com/v1/embeddings \\\n     --header \"Authorization: Bearer $VOYAGE_API_KEY\" \\\n     --header \"content-type: application/json\" \\\n     --data '\n{\n  \"input\": [\n    \"Sample text 1\",\n    \"Sample text 2\"\n  ],\n  \"model\": \"voyage-4-large\"\n}\n'"
            }
          ],
          "samples-languages": [
            "shell"
          ]
        }
      }
    }
  },
  "components": {
    "securitySchemes": {
      "ApiKeyAuth": {
        "type": "apiKey",
        "in": "header",
        "name": "Authorization: Bearer",
        "x-default": "$VOYAGE_API_KEY"
      }
    }
  },
  "x-readme": {
    "headers": [],
    "explorer-enabled": false,
    "proxy-enabled": false,
    "samples-enabled": true
  },
  "x-readme-fauxas": true
}
```
