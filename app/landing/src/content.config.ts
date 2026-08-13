import { defineCollection } from "astro:content";
import { z } from "astro/zod";
import { glob } from "astro/loaders";

const channelSchema = z.object({
  slug: z.string(),
  name: z.string(),
  icon: z.string(),
  blurb: z.string(),
  status: z.enum(["available", "coming-soon"]),
});

const featureSchema = z.object({
  slug: z.string(),
  title: z.string(),
  description: z.string(),
  icon: z.string(),
});

const testimonialSchema = z.object({
  slug: z.string(),
  quote: z.string(),
  name: z.string(),
  role: z.string(),
});

const faqSchema = z.object({
  slug: z.string(),
  question: z.string(),
  answer: z.string(),
});

const pricingSchema = z.object({
  slug: z.string(),
  name: z.string(),
  tagline: z.string(),
  priceMonthly: z.number(),
  cta: z.string(),
  popular: z.boolean().default(false),
  features: z.array(z.string()),
});

const changelogEntrySchema = z.object({
  type: z.enum(["added", "changed", "fixed", "removed"]),
  detail: z.string(),
});

const changelogSchema = z.object({
  slug: z.string(),
  title: z.string(),
  date: z.coerce.date(),
  release: z.string().optional(),
  changes: z.array(changelogEntrySchema).default([]),
});

const blogSchema = z.object({
  title: z.string(),
  description: z.string(),
  pubDate: z.coerce.date(),
  draft: z.boolean().default(false),
  tags: z.array(z.string()).default([]),
  ogImage: z.string().optional(),
});

const channels = defineCollection({
  loader: glob({ pattern: "**/*.json", base: "./src/content/channels" }),
  schema: channelSchema,
});

const features = defineCollection({
  loader: glob({ pattern: "**/*.json", base: "./src/content/features" }),
  schema: featureSchema,
});

const testimonials = defineCollection({
  loader: glob({ pattern: "**/*.json", base: "./src/content/testimonials" }),
  schema: testimonialSchema,
});

const faqs = defineCollection({
  loader: glob({ pattern: "**/*.json", base: "./src/content/faqs" }),
  schema: faqSchema,
});

const pricing = defineCollection({
  loader: glob({ pattern: "**/*.json", base: "./src/content/pricing" }),
  schema: pricingSchema,
});

const changelog = defineCollection({
  loader: glob({ pattern: "**/*.json", base: "./src/content/changelog" }),
  schema: changelogSchema,
});

const blog = defineCollection({
  loader: glob({ pattern: "**/*.mdx", base: "./src/content/blog" }),
  schema: blogSchema,
});

export { channelSchema, featureSchema, faqSchema, pricingSchema };
export const collections = { channels, features, testimonials, faqs, pricing, changelog, blog };