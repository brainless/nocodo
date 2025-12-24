# Other Tools

> Methods for using the GLM Coding Plan in other tools

You can easily integrate the **GLM-4.6** model into any tool that supports the **OpenAI API protocol**. Simply replace **the default API endpoint** with the one provided by [GLM Coding Plan](https://z.ai/subscribe?utm_source=zai\&utm_medium=link\&utm_term=devpack-integration\&utm_campaign=Platform_Ops&_channel_track_key=w3mNdY8g), and unlock the full power of Z.AI.

<Tip>
  **Black Friday**: Enjoy 50% off your first GLM Coding Plan purchase, plus an extra 20%/30% off! [Subscribe](https://z.ai/subscribe?utm_source=z.ai\&utm_medium=link\&utm_term=glm-devpack\&utm_campaign=Platform_Ops&_channel_track_key=jFgqJREK) now.
</Tip>

<Warning>
  Using the GLM Coding Plan, you need to configure the dedicated Coding API [https://api.z.ai/api/coding/paas/v4](https://api.z.ai/api/coding/paas/v4) instead of the General API [https://api.z.ai/api/paas/v4](https://api.z.ai/api/paas/v4)
</Warning>

<Warning>
  For users who have used the service before 2025-09-30: \
  The default model has been upgraded to GLM-4.6. Please update your config accordingly.
</Warning>

## Step 1: Supported Tools

Any tool supporting the **OpenAI Protocol** can run on **GLM-4.6**.Simply replace the default OpenAI API URL and set your API key.

Below are some common and popular tools supporting the **OpenAI Protocol** that can integrate `GLM-4.6` using the same approach:

* **Cursor**
* **Gemini CLI**
* **Cherry studio**
* ...

## Step 2: Install and Config

> Core Steps:
>
> 1. Select an OpenAI-compatible provider.
> 2. **Add/Replace the OpenAI Base URL with `https://api.z.ai/api/coding/paas/v4`.**
> 3. **Enter your Z.AI API Key and select `GLM-4.6`, `GLM-4.5` or `GLM-4.5-air`.**

Using **Cursor** as an example (Note: Custom configuration is only supported in Cursor Pro and higher versions), the following steps demonstrate how to integrate the `GLM-4.6` model via the OpenAI protocol. Similarly, other tools supporting the OpenAI protocol can adopt the same configuration approach.

### 1. Install Cursor

Download and install Cursor from the official website.

### 2. Create a New Provider/Model

In Cursor, navigate to the "**Models**" section and click the "**Add Custom Model**".

![Description](https://cdn.bigmodel.cn/markdown/176032216013820251013-100735.jpeg?attname=20251013-100735.jpeg)

* Select the **OpenAI Protocol**.
* Configure the **OpenAI API Key** (obtained from the Z.AI).
* In **Override OpenAI Base URL**, replace the default URL with `https://api.z.ai/api/coding/paas/v4`.
* Enter the model you wish to use, such as `GLM-4.6`, `GLM-4.5` or `GLM-4.5-air`.
* Note: In Cursor, the model name must be entered in uppercase, such as `GLM-4.6`.

![Description](https://cdn.bigmodel.cn/markdown/176032218295020251013-100740.jpeg?attname=20251013-100740.jpeg)

### 3. Save and Switch Models

After configuration, save your settings and select the newly created **GLM-4.6 Provider** on the homepage.

### 4. Get Started

With this setup, you can begin using the **GLM-4.6** model for code generation, debugging, task analysis, and more.

![Description](https://cdn.bigmodel.cn/markdown/176032221518820251013-100745.jpeg?attname=20251013-100745.jpeg)

## Step 3: Replacing the API URL

1. **Locate the API configuration section in your tool**:

For example, in **Goose**, this is typically where you set the API address in the configuration file; In **VS Code** plugins or **IntelliJ IDEA** plugins, configuration is usually done through the plugin's settings interface.

2. **Replace the OpenAI Base URL**:

Replace the default OpenAI API URL with `https://api.z.ai/api/coding/paas/v4`.

3. **Enter API Key and Select Model**:

* Enter your **Z.ai API Key**.
* Select `GLM-4.6`(standard, complex tasks) or `GLM-4.5-air`(lightweight, faster response) based on your requirements.

## Summary

By following these steps, you can integrate the **GLM-4.6** model into any tool supporting the **OpenAI protocol**. Simply replace the API endpoint and enter the corresponding API key to leverage the **GLM-4.6** model for powerful code generation, debugging, and analysis tasks within these tools. Integration with **GLM-4.6** is straightforward for any tool supporting the OpenAI protocol.


---

> To find navigation and other pages in this documentation, fetch the llms.txt file at: https://docs.z.ai/llms.txt
