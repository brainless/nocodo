---
title: "Building Better Software with AI: My Prompting Approach"
description: "Learn high-level prompting techniques for structuring projects when building with large language models and coding agents."
tags: ["AI development", "prompting", "project structure", "software architecture"]
youtubeVideoId: "44-xXzLclYE"
publishDate: 2025-12-20
---

Good morning! Today I want to share something I've been discussing with a few people - my approach to prompting feels different. It's more nuanced yet simpler once the concept clicks. I think this is the perfect time to discuss the high-level prompting techniques I use for structuring projects.

We're talking about project-level structuring today, not specific modules or features. I hope you find this useful when building fullstack applications. The focus is on how to structure entire projects when building with large language models and coding agents.

## Naming Conventions Matter

The first thing I want to emphasize is keeping your names consistent - folder names, file names, everything. When you mention folders, you're likely creating backend and frontend, which are well-established terms. Mobile app, backend, frontend, desktop app - these are industry-standard terms.

Keep using these established terms. If you're confused about terminology, you can always search or ask Claude separately. For example, if you have an idea for a banking industry API integration, you can ask for examples of common naming conventions.

We should name things in ways that are easy for us to discuss constantly. Through my years as an engineer, I've developed common terms that help when thinking through processes and explaining to AI what I want to build next.

For example, when you return to a project after a few days and say "I forgot where this API integration exists, can you scan my codebase?" - clear naming helps immensely. If you have clearly differentiated backend, frontend, desktop app, mobile app folders, it's much easier to locate things.

## Breaking Down into Modules

The next step is thinking about sub-modules and sub-features. I encourage breaking them apart. In my project, if you look at nocodo, you'll see I've been dividing the backend into more and more sub-modules. This is important because it keeps whatever happens within a module extremely constrained and focused on solving that specific problem.

For example, nocodo talks to many large language models and AI providers. I realized it made sense for this to be an SDK. I created a separate video about building that SDK with detailed prompts and features. Today I'm not discussing the SDK specifics, but rather why I break things apart.

My backend is called "manager." Now I'm conflicting with what I just said about clear terms - "manager" isn't very clear. To me, "manager" is the backend. This works only because I consistently refer to the backend as "manager" in every prompt. Consistency comes first - as long as you stick to your convention, it works.

## The Nomenclature Challenge

You'll notice I have a mix of "manager-something" and "nocodo-something" naming. Some confusion has entered, and I'm still thinking through this. Naming conventions and nomenclature really matter because as you build your project, you'll make more subdivisions.

In my case, I have `manager-models` which is shared code between the desktop app and the backend. Between frontend and backend, there's shared code because they communicate with each other. In my project, it makes sense to have shared code where the API gets generated from.

Could I have called it `nocodo-models`? In my mind, that doesn't work. Maybe I'll rename this to `shared-models` or `shared-structs` to clarify it's about sharing actual Rust structures. But `manager-models` feels right given that "models" can also refer to AI models, which becomes confusing.

The way you talk is part of your process, and your process is how you talk - they're intertwined. Naming conventions come up repeatedly in how you describe projects, talk with large language models, and interact with coding agents.

It doesn't matter which coding agent you use - Cursor, Lovable for UI work, anything else. If you stick to conventions you've built and remain consistent, your prompts will be much better.

## Module Structure Benefits

When breaking modules apart, ensure you think through the naming. In my case, `nocodo-tools` contains all the tools that agents use. For example: talking to APIs, local files, listing files, searching files - all these are specific tools.

All source code is open source, so feel free to check the repository. Within tools, everything is in its own folder. Everything related to the file system - reading files, searching files, listing files - is in the `filesystem` folder.

This also relates to `manager-models`, the shared structures between backend and frontend. If you open the folders, you'll see that inside tools there's also `filesystem`. The file system of shared code between desktop and backend is the same file system that's also the file system tools because shared code goes into the `manager-models` folder.

This may seem overly complicated, but the reason is the amount of functionality in nocodo. Nocodo is a fairly complicated project by nature - it's a project that writes other projects. It's a code building system, a coding agent, and now becoming a full agentic platform that can take your idea and build agents, do evaluations, run tests, take inputs and API keys, and deploy agents to your cloud.

All of this is generated code, which means I have to be much more careful about architecture. I'm talking about high-level patterns of how I prompt, not specific prompt details. The structuring of folders, creating sub-models, and ensuring related functionality stays together.

You can do this in any programming language - TypeScript, Python, Ruby. What matters is latching onto common conventions from industry and software engineering, but also merging them with how you think. Both have to come together because if you don't, you'll either prompt in industry-standard ways you don't like, or use terms that are very different from conventional software project thinking, which can hurt AI prompting.

## Agent Architecture

As you can see, I have `nocodo-tools` and then `nocodo-agents`. Agents is a completely new sub-project I'm building. What are agents? Agents are code that has autonomy - it can create its own memory, keep track of resources, and use tools and AI models to run in a loop.

For example, waiting for customer messages: an agent comes in, looks at the customer, figures out their ID, checks the database, makes decisions with AI about whether to answer immediately, gather extra data, or call the manager.

My agents inside nocodo are simpler because I'm creating lots of agents focused on specific goals. They have autonomy to figure things out and make decisions with AI models, but they have very specific goals. I want to mix and match these agents to build bigger agents or multi-agent systems.

Agents is its own separate library - in Rust we call them separate crates. All these crates are referenced in the main `Cargo.toml` file, which is the configuration file where all modules are referenced. Your programming language will be different, and you can ask AI about the structure.

## The LLM SDK

I also have the `nocodo-llm-sdk` which is easier to understand - I connect to multiple models and providers. If you look at the source, you'll see I have Claude, GLM, Grok, OpenAI models. Some models can connect to different providers - Grok is provided by both XAI (originally Twitter's AI team) and Zen. Zen is a model provider behind OpenCode, and they also provide GLM.

If you go to GLM, you'll see I have ZAI as a provider (the original company behind GLM), Zen as a provider, and Cerebras as a provider because Cerebras also allows running GLM models.

The reason this entire software, the LLM SDK that talks to large language models, exists is that while there are existing libraries, my communication with LLMs is so deep and needs so many nuances that I chose to build the SDK. For me, that wasn't a tough job and will provide better benefits. You can use an existing SDK - that's not what we're discussing.

What matters is whether it makes sense to separate something into its own library. If you're integrating with a bank API and using that output in your web app after talking to an AI model, the bank API integration makes sense to be separate so you can test it, maintain it as the API changes, and fix issues without touching the rest of your codebase.

## The Benefits of Modularity

The more you take out these parts and make them independent components, modules, or crates (depending on your programming language), the more it helps. The added advantage is that I can go to my project and drop into just the LLM SDK to ask Claude specifically about it if something goes wrong.

I have tests. The way I run tests is `cargo run --bin test_runner` which runs only the tests for the LLM. There's a specific LLM test runner. Different sub-projects have different test runners using a configuration file for API keys.

The added benefit is that if I want to fix something and a test is failing, I can drop into this particular folder and ask my coding agent to fix that specific thing. It needs no context from the rest of my product - what's happening in the backend or frontend. It won't even touch those folders.

In some cases, your folders do have library sharing because you're sharing something from a different folder. But if this sub-project is self-contained - in my case, this is API integration to large language models and doesn't need to care about any other sub-projects - it makes much more sense to drop into a shell.

## Context Management

As the project grows, you want to be able to focus and constrain how much access the model has to all files. The more you constrain, the better results you'll get. The clearer and lesser the context, the better the model can do what you want.

Because I'm in the SDK folder, it can easily figure out that I already have support for GPT 4.1 codex model and just needs to add 4.2. It doesn't see that it's referenced in documentation because my documentation is older than the 4.2 release.

## Testing Strategy

Now I want to come to one last point before ending: tests. This is something I'm learning as I go. I come from a startup background where we generally write end-to-end tests because you care about the outcome of the entire process.

But at the scale where I generate code - tons of code every day, you can see in my commits there are mountains of code changes because I refactor a lot in this early stage - if I don't have unit tests, some of the end outcomes and APIs change. This is a constant, tiring process, but early stage that's going to be the case because your product isn't set in stone.

With unit tests, I can easily isolate what will fail and what I expect to change, then adapt the code accordingly. When you see all my tests passing, it gives more granularity into specific business logic that needs to be tested exhaustively. I'd rather focus on that and write five or six tests (the agent writes on my behalf).

Then I'd put two or three concrete integration tests where I know I'm going to send this prompt and expect these specific things to happen. That's maybe 1, 2, 3, 4 integration tests. The rest are more unit tests.

When building APIs and backends, I use helper functions. The API handler expects some state from your framework or database, but there's certain business logic the API handler does. I take out that specific helper code and stick it into another helper function or file.

Then I can inject parameters I want - fake user, fake web state, fake backend state - and say "do you do what I expect you to do?" That becomes a unit test that's much easier to test continuously and much faster to run.

Then you stick that helper function back into your API handler, and the rest of the API handlers become much simpler - basically take the current user, check authentication, check authorization, and call the helpers. All the helpers have the unique business logic, but the API handlers don't have much logic.

This is similar to the compartmentalization of the model-view-controller system. Your models, views, and controllers - views are like your handlers (earlier days views produced HTML, now we produce JSON, but views are still views - basically API handlers). Controllers were the real business logic, so controllers to me are helpers, and I unit test the helpers.

## Conclusion

These are very high-level concepts I'm learning that are giving me fantastic results. This is relative - it's my opinion. Please validate with others you're talking to or building with.

If you feel my way of doing things is valuable and you'd like to get on a call to show me your project if you're getting stuck, I'm definitely helping out. I'm doing more and more of these calls with founders who are getting stuck with coding with agents, and I'm happy to help.
