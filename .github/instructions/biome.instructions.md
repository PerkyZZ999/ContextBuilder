---
description: BiomeJS linting and formatting conventions for Codze
applyTo: '**/*.{ts,tsx,js,jsx,json,css}'
---

# BiomeJS Configuration

Codze uses **BiomeJS** for linting, formatting, and import organization. BiomeJS replaces ESLint and Prettier with a single, fast tool written in Rust.

## Quick Reference

```bash
bun run lint          # Check for lint errors
bun run lint:fix      # Fix lint errors automatically
bun run format        # Format all files
bun run format:check  # Check formatting without writing
bun run check         # Run both lint and format checks
bun run check:fix     # Fix both lint and format issues
```

## Key Rules

### TypeScript/JavaScript
- **No `any`**: Use proper types instead of `any` (`noExplicitAny: error`)
- **No unused variables**: Remove or prefix with `_` (`noUnusedVariables: error`)
- **No unused imports**: Auto-removed on fix (`noUnusedImports: error`)
- **Use `const`**: Prefer `const` over `let` when not reassigned (`useConst: error`)
- **Import types**: Use `import type` for type-only imports (`useImportType: error`)

### React Hooks
- **Exhaustive deps**: Include all dependencies in useEffect/useMemo/useCallback
- **Hook rules**: Only call hooks at the top level of components

### Console & Debugging
- **No console**: Avoid `console.log` in production code (warning)
- **No debugger**: Never commit `debugger` statements

### Formatting (handled by Biome)
- **Indent**: 2 spaces
- **Line width**: 100 characters
- **Quotes**: Double quotes for strings and JSX
- **Semicolons**: Always
- **Trailing commas**: ES5 style
- **Line endings**: LF (Unix)

### Tailwind Class Sorting
BiomeJS sorts Tailwind classes in `className`, `clsx()`, `cn()`, and `cva()` calls.

```tsx
// ✅ Good - classes are sorted
<div className="flex items-center justify-center p-4 text-white" />

// ❌ Bad - unsorted classes
<div className="text-white p-4 flex justify-center items-center" />
```

## Import Organization

BiomeJS automatically organizes imports in this order:
1. React imports
2. External packages
3. Internal aliases (`@/...`)
4. Relative imports
5. Type imports (at the bottom of each group)

```tsx
// ✅ Properly organized imports
import { useState } from "react";

import { invoke } from "@tauri-apps/api/core";
import { useQuery } from "@tanstack/react-query";

import { Button } from "@/components/ui/button";
import { useEditorStore } from "@/stores/editor.store";

import type { FileEntry } from "@/types";
```

## Suppressing Rules

When you need to suppress a rule, use Biome's suppression comments:

```tsx
// biome-ignore lint/suspicious/noExplicitAny: Legacy API requires any
const legacyHandler = (data: any) => { ... };

// biome-ignore lint/correctness/useExhaustiveDependencies: Intentionally run once
useEffect(() => { ... }, []);
```

## VS Code Integration

Install the Biome VS Code extension for real-time linting and format-on-save:
- Extension ID: `biomejs.biome`
- Enable format on save in `.vscode/settings.json`

## Configuration

The Biome configuration is in `biome.json` at the project root. Key settings:
- Ignores: `node_modules`, `dist`, `src-tauri`
- Enables CSS linting and formatting
- Uses Tailwind class sorting
