import { defineCollection, z } from 'astro:content';

const playbook = defineCollection({
  type: 'content',
  schema: z.object({
    title: z.string().optional(),
    description: z.string().optional(),
    tags: z.array(z.string()).optional(),
  }),
});

const fundamentals = defineCollection({
  type: 'content',
  schema: z.object({
    title: z.string().optional(),
    description: z.string().optional(),
    tags: z.array(z.string()).optional(),
  }),
});

const aiDevelopment = defineCollection({
  type: 'content',
  schema: z.object({
    title: z.string().optional(),
    description: z.string().optional(),
    tags: z.array(z.string()).optional(),
  }),
});

const codingAgents = defineCollection({
  type: 'content',
  schema: z.object({
    title: z.string().optional(),
    description: z.string().optional(),
    tags: z.array(z.string()).optional(),
  }),
});

export const collections = {
  playbook,
  fundamentals,
  'ai-development': aiDevelopment,
  'coding-agents': codingAgents,
};