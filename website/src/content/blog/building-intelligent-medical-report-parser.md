---
title: "Building an Intelligent Medical Report Parser with AI Agents"
description: "How we're building a context-aware autonomous agent to extract and normalize medical data from PDF reports."
tags: ["AI agents", "OCR", "medical data", "Tesseract", "automation", "AI development"]
publishDate: 2025-12-28
---

I'm building an autonomous agent for processing medical reports, and I want to share an interesting approach that demonstrates what it truly means to build "agentic" software. This agent is being developed for a medical reports app where users can submit their medical reports for analysis. The example I'll use is based on an actual personal medical report.

The challenge we're tackling: different labs across India (and globally) structure their reports differently. While PDF-to-text extraction is fairly straightforward with modern tools, the real complexity lies in handling extraction errors and building intelligence over time.

## The Technical Foundation: Tesseract OCR

For PDF text extraction, we're using Tesseract, a powerful open-source OCR library. The basic workflow is:

1. Convert PDF pages to images
2. Run Tesseract OCR on each image
3. Extract text with preserved spacing

Tesseract offers various options for preserving spacing and structure, which helps maintain the tabular format of medical reports. For example, a typical blood test report has columns for:
- Test name
- Result value
- Units
- Reference interval

The library works remarkably well and runs fast—processing each page quickly even for a 48-page report.

## The Problem: When OCR Gets Creative

Here's where things get interesting. Consider this line from a blood test report:

```
Platelet Count    223    10×10³/µL    150-450
```

The unit here contains an exponent: `10³`. But Tesseract might extract it as:
- `10104³` (interpreting the exponent as number 4)
- `10*10³` (using an asterisk instead of ×)
- Or other variations depending on the PDF rendering

These mistakes are understandable and expected with OCR. The question is: how do we handle them systematically?

## The Agentic Approach

This is where building an agent makes a real difference. Instead of trying to fix OCR settings or writing complex regex patterns for every edge case, we can build a system that:

1. **Processes data granularly** - Line by line, field by field
2. **Detects ambiguity** - Identifies when extracted data looks suspicious
3. **Uses AI selectively** - Only calls an LLM when needed
4. **Builds reference data** - Stores cleaned results for future matching

Here's an example prompt I used with Gemini Flash 2.0:

```
I have extracted text from a medical report for blood sample.
Each line has test name, result, units, reference.

The actual extracted value is: "10104³"

What should this be? Provide the corrected value.
```

The model correctly identified this should be `10³` (10 to the power of 3). This works because medical nomenclature and units are well-represented in training data—even cheaper models handle this well.

## Building Intelligence Over Time

Here's the key insight: you don't want to call an LLM for every single line in every report. Instead:

1. **First time**: When you encounter ambiguous data (like "Platelet Count" with weird units), send it to the LLM
2. **Store the result**: Save the cleaned data with metadata:
   - Test name: "Platelet Count"
   - Expected units: "10³/µL"
   - Reference interval: "150-450"
3. **Next time**: When you see similar data from a different lab, check your reference data first
4. **Only if no match**: Then call the LLM or flag for human review

This creates a self-improving system. The agent learns from each extraction, building an internal knowledge base. Eventually, most common tests are recognized automatically, regardless of how the OCR mangles the formatting.

## Best Practices for Agent Design

Based on this work, here are key principles for building effective agents:

### 1. Don't Dump Everything to the LLM

Resist the temptation to grab all the data, pass it to a model, and hope for the best. That approach:
- Wastes tokens and money
- Loses context and precision
- Doesn't improve over time

### 2. Process Granularly with Context

Instead, break data into the smallest meaningful units (like individual test results), then provide focused context:

```
Context: Medical report, blood sample test
Task: Clean this specific line
Data: [single line of extracted text]
```

In-context, granular prompts are far more effective than large, unfocused ones.

### 3. Build Decision Logic

The agent should decide:
- When to use cached reference data
- When to call an LLM
- When to flag for human review
- How confident it is in each decision

This is what makes it truly "agentic"—it's not just running a pipeline, it's making intelligent decisions about its own workflow.

### 4. Design for Improvement

Every interaction should potentially improve the system. Store cleaned data, track confidence levels, and use past successes to handle future edge cases.

## Conclusion

Building agentic systems isn't about maximizing LLM usage—it's about minimizing it through intelligent design. The agent we're building for nocodo demonstrates this: it uses AI selectively, builds knowledge over time, and focuses on granular, context-rich interactions.

This approach creates a system that gets smarter with use, handles edge cases gracefully, and remains cost-effective at scale. That's the future of practical AI agents: goal-focused, context-aware, and continuously improving.

---

*This agent is being built as part of nocodo, our platform for building autonomous agents. The principles outlined here apply broadly to any domain where you need to process messy, real-world data with varying formats and quality.*
