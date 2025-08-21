# Introduction

Welcome to the JunctionRS Book. This guide provides an in-depth exploration of JunctionRS: an experimental, async-first, type-safe Object Graph Mapper (OGM) for Rust focused on Nodes, Edges, and traversal-friendly queries.

## Why JunctionRS?
Traditional ORMs center on tables, rows, and joins. JunctionRS elevates graph semantics: entities (Nodes) and relationships (Edges) as first-class types with a fluent, strongly-typed DSL.

## Project Status
Work in progress. Expect missing features and breaking changes between minor versions while core concepts stabilize.

## Key Highlights
- Async-first design
- DB-agnostic core with pluggable adapters (SurrealDB available today under the `surreal` feature)
- Derive macros to eliminate boilerplate for Node & Edge models
- Typed query and expression DSL (compile-time safety)
- Serde-powered projections and column selection via generated schemas
- Early graph traversal API and path capture semantics

## Audience
Developers building Rust applications that benefit from graph modeling and type-safe query composition. Ideal if you want ergonomics beyond raw driver queries without committing to heavyweight ORMs.

## Reading Map
If you are new: start with Core Concepts Overview, then Getting Started. Experienced users can jump directly to Derive Macros or Graph Traversal chapters.

---
Enjoy exploring JunctionRS!
