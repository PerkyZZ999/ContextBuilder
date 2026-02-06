---
applyTo: "**/schemas/**/*.{ts,js}, **/validations/**/*.{ts,js}, **/validators/**/*.{ts,js}, **/*schema.{ts,js}, **/*Schema.{ts,js}, **/*validation.{ts,js}, **/*Validation.{ts,js}, **/types/**/*.{ts,js}, **/models/**/*.{ts,js}, **/forms/**/*.{ts,tsx,js,jsx}"
---

# Complete Guide for Best Practices, Standards and Guidelines for Zod 4

## Table of Contents

1. [Introduction to Zod 4](#introduction-to-zod-4)
2. [Installation and Setup](#installation-and-setup)
3. [Schema Definition Fundamentals](#schema-definition-fundamentals)
4. [Type Inference and TypeScript Integration](#type-inference-and-typescript-integration)
5. [String Validation Patterns](#string-validation-patterns)
6. [Number and Numeric Type Validation](#number-and-numeric-type-validation)
7. [Object Schema Design](#object-schema-design)
8. [Arrays, Tuples, and Collections](#arrays-tuples-and-collections)
9. [Union and Discriminated Union Patterns](#union-and-discriminated-union-patterns)
10. [Custom Refinements and Validation](#custom-refinements-and-validation)
11. [Transformations and Preprocessing](#transformations-and-preprocessing)
12. [Async Validation](#async-validation)
13. [Error Handling and Customization](#error-handling-and-customization)
14. [Metadata and Schema Registry](#metadata-and-schema-registry)
15. [JSON Schema Conversion](#json-schema-conversion)
16. [Branded Types for Type Safety](#branded-types-for-type-safety)
17. [Form Validation Patterns](#form-validation-patterns)
18. [Performance Optimization](#performance-optimization)
19. [Advanced Patterns](#advanced-patterns)
20. [Testing and Validation Strategies](#testing-and-validation-strategies)

---

## Introduction to Zod 4

Zod 4 is a **TypeScript-first schema validation library** with static type inference. It enables both compile-time and runtime type safety through declarative schema definitions. Zod 4 introduced significant performance improvements (up to 14x faster string parsing, 7x faster array parsing, and 6.5x faster object parsing), a new metadata system, and simplified error customization.

### Why Choose Zod 4?

- **TypeScript-First**: Schemas serve as the single source of truth for both runtime validation and compile-time types
- **Zero Dependencies**: Lightweight and self-contained
- **Powerful Type Inference**: Use `z.infer<typeof schema>` to automatically generate TypeScript types
- **Composable Schemas**: Build complex validations from simpler, reusable pieces
- **Excellent Error Messages**: Detailed validation errors with customization support
- **Performance**: Significant improvements over Zod 3 for production applications

---

## Installation and Setup

### Basic Installation

```bash
npm install zod
# or
yarn add zod
# or
pnpm add zod
```

### Using Zod Mini (Tree-Shakable Version)

For projects where bundle size is critical, use Zod Mini:

```bash
npm install zod-mini
```

Zod Mini is a tree-shakable version that reduces bundle size by 6.6x in the core. It's ideal for frontend applications where bundle size matters.

### TypeScript Configuration

Ensure your `tsconfig.json` includes:

```json
{
  "compilerOptions": {
    "strict": true,
    "target": "ES2020",
    "module": "ESNext",
    "lib": ["ES2020"],
    "skipLibCheck": true
  }
}
```

### Import Best Practices

```typescript
// Standard import (recommended)
import { z } from "zod";

// Namespace import (when you need many utilities)
import * as zod from "zod";
const schema = zod.string();

// Type-only import for using inferred types
import type { z } from "zod";
type User = z.infer<typeof userSchema>;
```

---

## Schema Definition Fundamentals

### Primitive Types

Define schemas for basic JavaScript types:

```typescript
import { z } from "zod";

// String
const stringSchema = z.string();

// Number
const numberSchema = z.number();

// Integer
const integerSchema = z.number().int();

// Boolean
const booleanSchema = z.boolean();

// BigInt
const bigintSchema = z.bigint();

// Date
const dateSchema = z.date();

// Null and undefined
const nullSchema = z.null();
const undefinedSchema = z.undefined();

// Special types
const neverSchema = z.never(); // Allows no values
const unknownSchema = z.unknown(); // Allows any value
const anySchema = z.any(); // Bypass type checking (avoid when possible)
```

### Parsing Data

Zod provides three methods to validate data:

```typescript
const schema = z.string();

// 1. .parse() - throws error on failure
try {
  const result = schema.parse("hello"); // => 'hello'
  schema.parse(123); // throws ZodError
} catch (error) {
  console.error(error);
}

// 2. .safeParse() - returns result object (recommended)
const result = schema.safeParse("hello");
if (result.success) {
  console.log(result.data); // => 'hello'
} else {
  console.error(result.error); // ZodError
}

// 3. .parseAsync() - asynchronous parsing
const asyncResult = await schema.parseAsync("hello");

// 4. .safeParseAsync() - safe async parsing
const asyncSafeResult = await schema.safeParseAsync("hello");
if (asyncSafeResult.success) {
  console.log(asyncSafeResult.data);
}
```

### Coercion

Convert values to the expected type:

```typescript
import { z } from "zod";

// Coerce strings to numbers
const numberSchema = z.coerce.number();
numberSchema.parse("123"); // => 123

// Coerce to boolean
const boolSchema = z.coerce.boolean();
boolSchema.parse("true"); // => true
boolSchema.parse(1); // => true

// Coerce to date
const dateSchema = z.coerce.date();
dateSchema.parse("2025-01-01"); // => Date object

// Chain coercion with validation
const positiveInt = z.coerce.number().int().positive();
positiveInt.parse("42"); // => 42
```

### Literals and Enums

```typescript
// Single value literals
const statusSchema = z.literal("active");
statusSchema.parse("active"); // ✓

// Multiple values
const directionSchema = z.literal("up", "down", "left", "right");

// Native enums
enum UserRole {
  Admin = "admin",
  User = "user",
}
const roleSchema = z.nativeEnum(UserRole);

// String enums (recommended)
const prioritySchema = z.enum(["low", "medium", "high"]);
type Priority = z.infer<typeof prioritySchema>; // 'low' | 'medium' | 'high'
```

---

## Type Inference and TypeScript Integration

### Basic Type Inference

Extract TypeScript types directly from Zod schemas:

```typescript
import { z } from "zod";

const userSchema = z.object({
  id: z.string().uuid(),
  email: z.string().email(),
  age: z.number().int().positive().optional(),
  role: z.enum(["admin", "user", "moderator"]),
});

// Infer the TypeScript type
type User = z.infer<typeof userSchema>;
// Equivalent to:
// type User = {
//   id: string;
//   email: string;
//   age?: number;
//   role: 'admin' | 'user' | 'moderator';
// }
```

### Output Types

By default, `z.infer` returns the **output type** (after transformations):

```typescript
const transformSchema = z.string().transform((val) => val.toLowerCase());

type Output = z.infer<typeof transformSchema>; // string

// Get the input type instead
type Input = z.input<typeof transformSchema>; // string
```

### Input and Output Types

```typescript
const schema = z
  .object({
    dateString: z.string().datetime(),
  })
  .transform((data) => ({
    ...data,
    dateString: new Date(data.dateString),
  }));

type Input = z.input<typeof schema>;
// { dateString: string }

type Output = z.output<typeof schema>;
// { dateString: Date }
```

---

## String Validation Patterns

### Basic String Validation

```typescript
const stringSchema = z
  .string()
  .min(3, "String must be at least 3 characters")
  .max(50, "String must be at most 50 characters")
  .trim() // Remove whitespace
  .toLowerCase(); // Transform to lowercase
```

### Email Validation

```typescript
// Built-in email validation
const emailSchema = z.string().email("Invalid email address");

// Custom email pattern
const customEmailSchema = z.string().email();

// For advanced email validation, combine with refine
const strictEmailSchema = z
  .string()
  .email()
  .refine(
    (email) => !email.endsWith("@disposable.com"),
    "Disposable email addresses are not allowed"
  );
```

### URL and URI Validation

```typescript
// URL validation
const urlSchema = z.string().url("Invalid URL");

// HTTP/HTTPS only
const httpUrlSchema = z
  .string()
  .url()
  .refine(
    (url) => url.startsWith("http://") || url.startsWith("https://"),
    "Must be HTTP or HTTPS"
  );

// URI component
const uriComponentSchema = z
  .string()
  .refine(
    (val) => encodeURIComponent(val) === val,
    "Must be valid URI component"
  );
```

### UUID and ID Validation

```typescript
// UUID validation (strict RFC 4122)
const uuidSchema = z.string().uuid();

// Nanoid validation
const nanoidSchema = z.string().regex(/^[a-zA-Z0-9_-]{21}$/);

// Custom ID format
const customIdSchema = z
  .string()
  .regex(/^[A-Z]{3}-\d{6}$/, "Invalid ID format");
```

### IP Address Validation

```typescript
// IPv4
const ipv4Schema = z.string().ipv4("Invalid IPv4 address");

// IPv6
const ipv6Schema = z.string().ipv6("Invalid IPv6 address");

// CIDR blocks
const cidrSchema = z.string().cidr({ version: "v4" });
```

### JWT and Hash Validation

```typescript
// JWT validation (basic format check)
const jwtSchema = z.string().jwt();

// Hash algorithms (SHA256, MD5, etc.)
const sha256Schema = z.string().sha256();
const md5Schema = z.string().md5();
```

### Regular Expressions

```typescript
const passwordSchema = z
  .string()
  .min(8)
  .regex(/[A-Z]/, "Must contain uppercase letter")
  .regex(/[a-z]/, "Must contain lowercase letter")
  .regex(/\d/, "Must contain digit")
  .regex(/[!@#$%^&*]/, "Must contain special character");
```

### Template Literal Types

```typescript
const countryCodeSchema = z
  .string()
  .refine((val) => /^[A-Z]{2}$/.test(val), "Must be 2-letter country code");

// For specific patterns
const phoneSchema = z
  .string()
  .refine((val) => /^\+?1?\d{9,15}$/.test(val), "Invalid phone format");
```

---

## Number and Numeric Type Validation

### Integer Validation

```typescript
import { z } from "zod";

// Basic integer
const intSchema = z.number().int("Must be an integer");

// Integer with range
const ageSchema = z
  .number()
  .int()
  .min(0, "Age must be non-negative")
  .max(150, "Age must be realistic");

// Safe integers only (up to 2^53 - 1)
const safeIntSchema = z.number().int().safe();
```

### Floating Point Validation

```typescript
// Basic float
const priceSchema = z.number();

// Specific decimal places
const currencySchema = z
  .number()
  .multipleOf(0.01, "Must have max 2 decimal places");

// Finite numbers (no Infinity or NaN)
const finiteSchema = z.number().finite("Must be a finite number");
```

### BigInt Validation

```typescript
const bigintSchema = z.bigint();
const largeNumberSchema = z.bigint().positive("Must be positive");

// Convert from string
const bigintFromString = z.coerce.bigint();
bigintFromString.parse("9007199254740991"); // OK
```

### Number Ranges

```typescript
const scoreSchema = z
  .number()
  .min(0, "Score cannot be negative")
  .max(100, "Score cannot exceed 100");

const percentageSchema = z.number().min(0).max(100);

const negativeSchema = z.number().negative();
const positiveSchema = z.number().positive();
const nonPositiveSchema = z.number().nonpositive();
const nonNegativeSchema = z.number().nonnegative();
```

---

## Object Schema Design

### Basic Object Schemas

```typescript
import { z } from "zod";

const userSchema = z.object({
  name: z.string(),
  email: z.string().email(),
  age: z.number().int().positive(),
});

// Parse data
const user = userSchema.parse({
  name: "John Doe",
  email: "john@example.com",
  age: 30,
});
```

### Optional and Nullable Fields

```typescript
const userSchema = z.object({
  name: z.string(),
  email: z.string().email(),
  middleName: z.string().optional(), // Excludes field if undefined
  phone: z.string().nullable(), // Allows null
  nickname: z.string().nullish(), // Allows null and undefined
  bio: z.string().default("No bio"), // Default value if missing
});
```

### Strict vs Loose Objects

```typescript
// Strict object - rejects extra properties
const strictSchema = z
  .object({
    name: z.string(),
  })
  .strict();

// Loose object - allows extra properties (default)
const looseSchema = z.object({
  name: z.string(),
});

// Pass-through object - keeps extra properties
const passthroughSchema = z
  .object({
    name: z.string(),
  })
  .passthrough();

// Catch-all for extra properties
const withCatchall = z
  .object({
    name: z.string(),
  })
  .catchall(z.any());
```

### Object Composition

```typescript
// Merge objects
const baseUserSchema = z.object({
  id: z.string().uuid(),
  name: z.string(),
});

const adminExtensionSchema = z.object({
  permissions: z.array(z.string()),
  role: z.literal("admin"),
});

const adminSchema = baseUserSchema.merge(adminExtensionSchema);

// Extend objects
const userWithTimestamps = baseUserSchema.extend({
  createdAt: z.date(),
  updatedAt: z.date(),
});

// Pick specific fields
const userPreviewSchema = baseUserSchema.pick({ id: true, name: true });

// Omit specific fields
const userWithoutIdSchema = baseUserSchema.omit({ id: true });

// Partial - all fields optional
const partialUserSchema = baseUserSchema.partial();

// Required - all fields required
const requiredUserSchema = baseUserSchema.required();
```

### Accessing Shape Properties

```typescript
const userSchema = z.object({
  name: z.string(),
  email: z.string(),
});

// Get individual field schema
const nameSchema = userSchema.shape.name;

// Get keys as enum
const userKeysSchema = userSchema.keyof();
// Type: ZodEnum<['name', 'email']>
```

### Recursive Objects

```typescript
import { z } from "zod";

interface Node {
  value: string;
  children: Node[];
}

const nodeSchema: z.ZodType<Node> = z.lazy(() =>
  z.object({
    value: z.string(),
    children: z.array(nodeSchema),
  })
);

// Parse nested structures
const tree = nodeSchema.parse({
  value: "root",
  children: [
    { value: "child1", children: [] },
    { value: "child2", children: [] },
  ],
});
```

---

## Arrays, Tuples, and Collections

### Array Validation

```typescript
import { z } from "zod";

// Basic array
const stringArraySchema = z.array(z.string());

// Array with constraints
const tagsSchema = z
  .array(z.string())
  .min(1, "At least one tag required")
  .max(5, "Maximum 5 tags allowed");

// Non-empty array
const nonEmptySchema = z.array(z.string()).nonempty();

// Array of objects
const usersSchema = z.array(
  z.object({
    id: z.string(),
    name: z.string(),
  })
);
```

### Tuples

```typescript
// Fixed-length tuple
const coordinateSchema = z.tuple([z.number(), z.number()]);
coordinateSchema.parse([10, 20]); // ✓

// Tuple with rest elements
const colorRGBSchema = z.tuple([z.number(), z.number(), z.number()]);

// Tuple with optional elements
const optionalTupleSchema = z.tuple([z.string(), z.number().optional()]);
```

### Records

```typescript
// String keys with specific value type
const configSchema = z.record(z.string(), z.string());
const config = configSchema.parse({
  apiUrl: "https://api.example.com",
  environment: "production",
});

// Specific key type
const scoresSchema = z.record(z.enum(["easy", "medium", "hard"]), z.number());
```

### Maps and Sets

```typescript
// Map validation
const mapSchema = z.map(z.string(), z.number());

// Set validation
const setSchema = z.set(z.string());

// Transform to array if needed
const stringSetSchema = z.array(z.string()).transform((arr) => new Set(arr));
```

---

## Union and Discriminated Union Patterns

### Basic Unions

```typescript
import { z } from "zod";

// Union of types
const idSchema = z.union([z.string().uuid(), z.number()]);

// Shorthand with pipe
const flexibleIdSchema = z.string().uuid().or(z.number());
```

### Discriminated Unions

Discriminated unions are the most performant way to validate unions:

```typescript
// Define schemas with discriminator field
const adminSchema = z.object({
  type: z.literal("admin"),
  permissions: z.array(z.string()),
});

const userSchema = z.object({
  type: z.literal("user"),
  email: z.string().email(),
});

const guestSchema = z.object({
  type: z.literal("guest"),
});

// Create discriminated union
const roleSchema = z.discriminatedUnion("type", [
  adminSchema,
  userSchema,
  guestSchema,
]);

type Role = z.infer<typeof roleSchema>;

// Parse with type safety
const role = roleSchema.parse({
  type: "admin",
  permissions: ["read", "write"],
});
```

### Nested Discriminated Unions

```typescript
// Shape type with discriminated union
const circleSchema = z.object({
  shape: z.literal("circle"),
  radius: z.number().positive(),
});

const squareSchema = z.object({
  shape: z.literal("square"),
  side: z.number().positive(),
});

const shape2DSchema = z.discriminatedUnion("shape", [
  circleSchema,
  squareSchema,
]);

// Cube inherits from square
const cubeSchema = squareSchema.extend({
  shape: z.literal("cube"),
  depth: z.number().positive(),
});

// Merge into parent union using .options
const shape3DSchema = z.discriminatedUnion("type", [
  ...shape2DSchema.options,
  cubeSchema,
]);
```

### Discriminated Unions with Advanced Types

```typescript
// Support unions and pipes in discriminated unions (Zod 4)
const responseSchema = z.discriminatedUnion("status", [
  z.object({
    status: z.literal("success"),
    data: z.object({ message: z.string() }),
  }),
  z.object({
    status: z.literal("error"),
    error: z.object({ code: z.string(), message: z.string() }),
  }),
  z.object({
    status: z.literal("pending"),
    progress: z.number(),
  }),
]);
```

---

## Custom Refinements and Validation

### Using .refine()

Single-field custom validation:

```typescript
import { z } from "zod";

// Simple refinement
const passwordSchema = z
  .string()
  .min(8)
  .refine((val) => /[A-Z]/.test(val), "Password must contain uppercase letter")
  .refine((val) => /\d/.test(val), "Password must contain number");

// Refinement with path (for objects)
const userFormSchema = z
  .object({
    password: z.string(),
    confirmPassword: z.string(),
  })
  .refine((data) => data.password === data.confirmPassword, {
    message: "Passwords do not match",
    path: ["confirmPassword"], // Attach error to specific field
  });
```

### Using .superRefine()

Cross-field validation with granular control:

```typescript
const registrationSchema = z
  .object({
    email: z.string().email(),
    password: z.string().min(8),
    confirmPassword: z.string(),
    termsAccepted: z.boolean(),
  })
  .superRefine((data, ctx) => {
    // Multiple validations with specific error targeting
    if (data.password !== data.confirmPassword) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["confirmPassword"],
        message: "Passwords do not match",
      });
    }

    if (!data.termsAccepted) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["termsAccepted"],
        message: "You must accept the terms",
      });
    }
  });
```

### Using .check()

Return multiple validation issues:

```typescript
const schema = z
  .object({
    username: z.string(),
  })
  .check((data) => {
    const issues = [];

    if (data.username.length < 3) {
      issues.push({
        code: z.ZodIssueCode.too_small,
        path: ["username"],
        message: "Username too short",
      });
    }

    if (!/[a-z]/.test(data.username)) {
      issues.push({
        code: z.ZodIssueCode.custom,
        path: ["username"],
        message: "Username must contain lowercase",
      });
    }

    return issues.length === 0 ? true : issues;
  });
```

---

## Transformations and Preprocessing

### Transform

Modify data after validation:

```typescript
import { z } from "zod";

// Simple transformation
const trimmedStringSchema = z.string().transform((val) => val.trim());

// Complex transformation
const userSchema = z
  .object({
    firstName: z.string(),
    lastName: z.string(),
  })
  .transform((data) => ({
    ...data,
    fullName: `${data.firstName} ${data.lastName}`,
  }));

type User = z.infer<typeof userSchema>;
// { firstName: string; lastName: string; fullName: string }
```

### Preprocess

Transform data before validation:

```typescript
// Coerce strings to numbers
const numberSchema = z.preprocess((val) => {
  if (typeof val === "string") {
    return Number(val);
  }
  return val;
}, z.number());

// Normalize strings
const emailSchema = z.preprocess((val) => {
  if (typeof val === "string") {
    return val.toLowerCase().trim();
  }
  return val;
}, z.string().email());

// Array preprocessing
const commaStringSchema = z.preprocess((val) => {
  if (typeof val === "string") {
    return val.split(",").map((s) => s.trim());
  }
  return val;
}, z.array(z.string()));
```

### Pipe (Chain Transformations)

```typescript
// Chain multiple schemas with transformations
const processedStringSchema = z.pipe(
  z.string(),
  z.string().min(1),
  z.string().transform((val) => val.trim()),
  z.string().transform((val) => val.toLowerCase())
);

// More complex example
const dateFromUnixSchema = z.pipe(
  z.number(),
  z.number().int().positive(),
  z.number().transform((ts) => new Date(ts * 1000))
);
```

### Defaults and Fallbacks

```typescript
// Default value
const userSchema = z.object({
  role: z.enum(["user", "admin"]).default("user"),
  createdAt: z.date().default(() => new Date()),
  preferences: z.object({ theme: z.string() }).default({ theme: "light" }),
});

// Catch - fallback on error
const numberOrZeroSchema = z.number().catch(0);

// Predefault - default before validation
const predefaultSchema = z.number().prefault(10); // Set value if undefined, then validate
```

---

## Async Validation

### Basic Async Validation

Use `parseAsync()` or `safeParseAsync()` for asynchronous validation:

```typescript
import { z } from "zod";

const userSchema = z.object({
  username: z.string().min(3),
  email: z.string().email(),
});

// Parse asynchronously
async function validateUser(data: unknown) {
  try {
    const user = await userSchema.parseAsync(data);
    console.log("Valid user:", user);
  } catch (error) {
    console.error("Validation error:", error);
  }
}

// Safe parse
async function safeValidateUser(data: unknown) {
  const result = await userSchema.safeParseAsync(data);
  if (result.success) {
    console.log("User:", result.data);
  } else {
    console.error("Errors:", result.error);
  }
}
```

### Async Refinements

Validate against external data sources:

```typescript
const uniqueUsernameSchema = z
  .string()
  .min(3)
  .refine(async (username) => {
    // Check if username exists in database
    const response = await fetch(`/api/check-username?username=${username}`);
    const data = await response.json();
    return !data.exists; // True if available
  }, "Username is already taken");

// Usage
const username = await uniqueUsernameSchema.parseAsync("john_doe");

// Multiple async checks
const signupSchema = z
  .object({
    username: z.string(),
    email: z.string().email(),
  })
  .superRefine(async (data, ctx) => {
    // Check username uniqueness
    const usernameTaken = await checkUsernameExists(data.username);
    if (usernameTaken) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["username"],
        message: "Username already taken",
      });
    }

    // Check email uniqueness
    const emailTaken = await checkEmailExists(data.email);
    if (emailTaken) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["email"],
        message: "Email already registered",
      });
    }
  });
```

### Concurrent Async Validation

```typescript
const validateWithMultipleSources = z
  .object({
    username: z.string(),
    email: z.string().email(),
  })
  .superRefine(async (data, ctx) => {
    // Run checks in parallel
    const [usernameTaken, emailTaken] = await Promise.all([
      checkUsernameExists(data.username),
      checkEmailExists(data.email),
    ]);

    if (usernameTaken) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["username"],
        message: "Username taken",
      });
    }

    if (emailTaken) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["email"],
        message: "Email registered",
      });
    }
  });
```

### Async Validation Organization

For Next.js applications, separate async validation from client-side schemas:

```typescript
// schemas.ts (client-safe, no async)
import { z } from "zod";

export const userRegistrationSchema = z.object({
  username: z.string().min(3).max(20),
  email: z.string().email(),
  password: z.string().min(8),
});

// server-schemas.ts (server-only, with async)
("use server");

import { z } from "zod";
import { userRegistrationSchema } from "./schemas";

export const serverUserRegistrationSchema = userRegistrationSchema.superRefine(
  async (data, ctx) => {
    // Async validation here
    const userExists = await db.user.findUnique({
      where: { email: data.email },
    });

    if (userExists) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["email"],
        message: "Email already registered",
      });
    }
  }
);

// Then use in Server Actions
export async function registerUser(data: unknown) {
  const result = await serverUserRegistrationSchema.safeParseAsync(data);
  if (!result.success) {
    return { error: result.error };
  }
  // Process validated data
}
```

---

## Error Handling and Customization

### Understanding Zod Errors

```typescript
import { z } from "zod";

const schema = z.object({
  email: z.string().email("Invalid email"),
  age: z.number().min(18, "Must be 18 or older"),
});

const result = schema.safeParse({
  email: "invalid-email",
  age: 15,
});

if (!result.success) {
  console.log(result.error); // ZodError object
  console.log(result.error.issues); // Array of validation issues
}
```

### Custom Error Messages

Set custom error messages at the schema level:

```typescript
const userSchema = z.object({
  email: z
    .string({
      // These apply when type checking fails
      errorMap: (issue, ctx) => {
        if (issue.code === z.ZodIssueCode.invalid_type) {
          return { message: "Email must be a string" };
        }
        return { message: ctx.defaultError };
      },
    })
    .email("Please enter a valid email"),

  age: z.number().min(18, "You must be 18 or older"),
});
```

### Per-Parse Error Customization

Customize errors at parse time:

```typescript
const schema = z.object({
  name: z.string(),
  email: z.string().email(),
});

const result = schema.safeParse(
  { name: "John", email: "invalid" },
  {
    errorMap: (issue, ctx) => {
      if (issue.code === z.ZodIssueCode.invalid_type) {
        return { message: `Expected ${issue.expected}` };
      }
      return { message: ctx.defaultError };
    },
  }
);
```

### Global Error Customization

Configure global error messages:

```typescript
z.config({
  customError: (issue, ctx) => {
    // Return default error for some codes
    if (issue.code === z.ZodIssueCode.too_small) {
      return { message: `Minimum ${issue.minimum} characters` };
    }
    // Fall back to default
    return { message: ctx.defaultError };
  },
});
```

### Error Formatting

Format errors for display:

```typescript
import { z } from "zod";

const schema = z.object({
  email: z.string().email(),
  password: z.string().min(8),
});

const result = schema.safeParse(data);

if (!result.success) {
  // Flatten errors by path
  const flattened = result.error.flatten();
  console.log(flattened.fieldErrors);
  // { email: ['Invalid email'], password: [...] }

  // Prettify errors (human-readable)
  const pretty = z.prettifyError(result.error);
  console.log(pretty);

  // Tree format
  const tree = z.treeifyError(result.error);
  console.log(tree);

  // Format with custom function
  const formatted = z.formatError(result.error, (issue) => issue.message);
}
```

### Error Precedence

Error customization precedence (highest to lowest):

1. Schema-level error (hardcoded in schema)
2. Per-parse error (passed to `.parse()`)
3. Global error (via `z.config()`)
4. Default Zod error

### Internationalization

Implement multi-language support:

```typescript
// i18n-config.ts
const messages = {
  en: {
    invalid_email: "Invalid email address",
    too_small: (min: number) => `Minimum ${min} characters`,
    too_large: (max: number) => `Maximum ${max} characters`,
  },
  es: {
    invalid_email: "Dirección de correo inválida",
    too_small: (min: number) => `Mínimo ${min} caracteres`,
    too_large: (max: number) => `Máximo ${max} caracteres`,
  },
};

// Create locale-aware schema builder
function createSchema(locale: string) {
  return z.object({
    email: z.string().email(messages[locale].invalid_email),
    name: z
      .string()
      .min(2, messages[locale].too_small(2))
      .max(50, messages[locale].too_large(50)),
  });
}

const enSchema = createSchema("en");
const esSchema = createSchema("es");
```

---

## Metadata and Schema Registry

### Basic Metadata

Attach descriptions to schemas:

```typescript
import { z } from "zod";

const userSchema = z.object({
  email: z.string().email().describe("User email address"),
  age: z.number().describe("User age in years"),
  role: z.enum(["admin", "user"]).describe("User role"),
});

// Access metadata
const emailDescription = userSchema.shape.email.getDescription();
```

### Metadata with .meta()

Attach structured metadata:

```typescript
const userSchema = z.object({
  id: z
    .string()
    .uuid()
    .meta({
      title: "User ID",
      description: "Unique identifier",
      examples: ["550e8400-e29b-41d4-a716-446655440000"],
    }),
  email: z
    .string()
    .email()
    .meta({
      title: "Email Address",
      format: "email",
      examples: ["user@example.com"],
    }),
});

// Access metadata
const metadata = userSchema.shape.id.meta();
console.log(metadata.title); // 'User ID'
```

### Using Registries

Create typed metadata registries:

```typescript
// Define metadata structure
type SchemaMeta = {
  title: string;
  description: string;
  examples: unknown[];
  deprecated?: boolean;
};

// Create registry
const registry = z.registry<SchemaMeta>();

// Add schemas to registry
const userIdSchema = z.string().uuid();
registry.add(userIdSchema, {
  title: "User ID",
  description: "Unique user identifier",
  examples: ["550e8400-e29b-41d4-a716-446655440000"],
});

// Check if schema exists
if (registry.has(userIdSchema)) {
  console.log("Schema found");
}

// Retrieve metadata
const metadata = registry.get(userIdSchema);
```

### Global Registry

Use the built-in global registry:

```typescript
const emailSchema = z.string().email();

// Register with metadata
z.globalRegistry.add(emailSchema, {
  title: "Email Address",
  description: "Valid email address",
  examples: ["user@example.com"],
});

// Access globally
const globalMeta = z.globalRegistry.get(emailSchema);

// Convenient shorthand with .register()
emailSchema.register(z.globalRegistry, {
  title: "Email Address",
  description: "Valid email address",
});
```

### Constrained Registries

Restrict registry to specific schema types:

```typescript
// Only accept string schemas
type StringMeta = { examples: string[] };
const stringRegistry = z.registry<StringMeta, z.ZodString>();

const stringSchema = z.string();
stringRegistry.add(stringSchema, { examples: ["hello", "world"] });

// This would cause a TypeScript error
const numberSchema = z.number();
// stringRegistry.add(numberSchema, ...); // ❌ Error
```

---

## JSON Schema Conversion

### Basic Conversion

Convert Zod schemas to JSON Schema:

```typescript
import { z, toJSONSchema } from "zod";

const userSchema = z.object({
  id: z.string().uuid(),
  name: z.string().min(1),
  email: z.string().email(),
  age: z.number().int().min(0).optional(),
});

const jsonSchema = z.toJSONSchema(userSchema);

// Output:
// {
//   "type": "object",
//   "properties": {
//     "id": { "type": "string", "format": "uuid" },
//     "name": { "type": "string", "minLength": 1 },
//     "email": { "type": "string", "format": "email" },
//     "age": { "type": "number", "minimum": 0 }
//   },
//   "required": ["id", "name", "email"]
// }
```

### With Metadata

Include metadata in JSON Schema output:

```typescript
const schemaWithMeta = z.object({
  username: z.string().meta({
    title: "Username",
    description: "Unique username for the user",
    examples: ["john_doe", "jane_smith"],
  }),
  email: z.string().email().meta({
    title: "Email",
    description: "Email address",
  }),
});

// Register metadata globally
z.globalRegistry.add(schemaWithMeta.shape.username, {
  title: "Username",
  description: "Unique username",
});

const jsonSchema = z.toJSONSchema(schemaWithMeta);
```

### Using Registries for Refs

Handle schema references with registries:

```typescript
const userIdSchema = z.string().uuid();
const userSchema = z.object({
  id: userIdSchema,
  name: z.string(),
});

// Create registry with IDs
const registry = z.registry<{ id: string }>();
registry.add(userIdSchema, { id: "UserId" });

// Convert with registry
const jsonSchema = z.toJSONSchema(userSchema, {
  registry,
  target: "draft2024-12",
});

// Output includes $ref
// {
//   "type": "object",
//   "properties": {
//     "id": { "$ref": "#/$defs/UserId" },
//     ...
//   },
//   "$defs": {
//     "UserId": { "type": "string", "format": "uuid" }
//   }
// }
```

### OpenAPI Integration

Generate OpenAPI-compatible schemas:

```typescript
const petSchema = z.object({
  id: z.number().int(),
  name: z.string(),
  status: z.enum(["available", "pending", "sold"]),
});

const openAPISchema = z.toJSONSchema(petSchema, {
  target: "openapi-3.1.0",
});

// Use in OpenAPI spec
const openAPISpec = {
  components: {
    schemas: {
      Pet: openAPISchema,
    },
  },
};
```

---

## Branded Types for Type Safety

### Creating Branded Types

Use branded types to distinguish between structurally similar types:

```typescript
import { z } from "zod";

// Brand type definition
type UserId = string & { readonly __brand: "UserId" };

// Zod schema for branding
const userIdSchema = z.string().uuid().brand<UserId>();

// At runtime, a UserId is just a string
// But TypeScript treats it as distinct
function getUser(userId: UserId): User {
  // Can only accept branded UserId
}

// Usage
const id = userIdSchema.parse("550e8400-e29b-41d4-a716-446655440000");
getUser(id); // ✓

getUser("550e8400-e29b-41d4-a716-446655440000"); // ❌ Type Error
```

### Multiple Brands

Brand the same type differently:

```typescript
type ProductId = string & { readonly __brand: "ProductId" };
type OrderId = string & { readonly __brand: "OrderId" };

const productIdSchema = z.string().uuid().brand<ProductId>();
const orderIdSchema = z.string().uuid().brand<OrderId>();

function addProduct(productId: ProductId, orderId: OrderId) {
  // Type-safe function that prevents mixing up IDs
}

const prodId = productIdSchema.parse(someId);
const ordId = orderIdSchema.parse(someId);

addProduct(prodId, ordId); // ✓
addProduct(ordId, prodId); // ❌ Type Error
```

### Branded Types with Validation

```typescript
// Positive integer brand
type PositiveInt = number & { readonly __brand: "PositiveInt" };
const positiveIntSchema = z.number().int().positive().brand<PositiveInt>();

// Email brand
type Email = string & { readonly __brand: "Email" };
const emailSchema = z.string().email().brand<Email>();

// Application codes
type OrgSlug = string & { readonly __brand: "OrgSlug" };
const orgSlugSchema = z
  .string()
  .regex(/^[a-z0-9-]+$/, "Invalid slug format")
  .brand<OrgSlug>();

// Type-safe function
function createOrganization(slug: OrgSlug, email: Email) {
  // Implementation
}
```

### Practical Example: IDs and Slugs

```typescript
type UUID = string & { readonly __brand: "UUID" };
type Slug = string & { readonly __brand: "Slug" };
type Email = string & { readonly __brand: "Email" };

const uuidSchema = z.string().uuid().brand<UUID>();
const slugSchema = z
  .string()
  .toLowerCase()
  .regex(/^[a-z0-9-]+$/)
  .brand<Slug>();
const emailSchema = z.string().email().brand<Email>();

// Create types with branded values
type User = {
  id: UUID;
  slug: Slug;
  email: Email;
  name: string;
};

// Safe schema with branded types
const userSchema = z
  .object({
    id: uuidSchema,
    slug: slugSchema,
    email: emailSchema,
    name: z.string(),
  })
  .strict();

type ParsedUser = z.infer<typeof userSchema>;
// All IDs, slugs, and emails are now branded types!
```

---

## Form Validation Patterns

### React Hook Form Integration

```typescript
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";

const schema = z
  .object({
    email: z.string().email("Invalid email"),
    password: z.string().min(8, "Password too short"),
    confirmPassword: z.string(),
  })
  .refine((data) => data.password === data.confirmPassword, {
    message: "Passwords do not match",
    path: ["confirmPassword"],
  });

export function LoginForm() {
  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm({
    resolver: zodResolver(schema),
  });

  return (
    <form onSubmit={handleSubmit(onSubmit)}>
      <input {...register("email")} />
      {errors.email && <span>{errors.email.message}</span>}

      <input {...register("password")} type="password" />
      {errors.password && <span>{errors.password.message}</span>}

      <button type="submit">Sign In</button>
    </form>
  );
}
```

### Server-Side Form Validation (Next.js)

```typescript
// app/actions.ts
"use server";

import { z } from "zod";

const contactSchema = z.object({
  name: z.string().min(1, "Name required"),
  email: z.string().email("Invalid email"),
  message: z.string().min(10, "Message too short"),
});

export async function submitContact(formData: FormData) {
  const data = {
    name: formData.get("name"),
    email: formData.get("email"),
    message: formData.get("message"),
  };

  const result = contactSchema.safeParse(data);

  if (!result.success) {
    return {
      error: result.error.flatten().fieldErrors,
    };
  }

  // Process validated data
  await saveToDatabase(result.data);
  return { success: true };
}

// app/contact.tsx
("use client");

import { submitContact } from "./actions";

export function ContactForm() {
  async function handleSubmit(e: React.FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const formData = new FormData(e.currentTarget);
    const result = await submitContact(formData);

    if (result.error) {
      // Display field errors
    } else {
      // Show success
    }
  }

  return (
    <form onSubmit={handleSubmit}>
      <input name="name" required />
      <input name="email" required />
      <textarea name="message" required></textarea>
      <button type="submit">Submit</button>
    </form>
  );
}
```

### Progressive Enhancement

Validate on both client and server:

```typescript
// lib/schemas.ts
export const signupSchema = z.object({
  email: z.string().email(),
  password: z.string().min(8),
  name: z.string().min(1),
});

// app/api/signup/route.ts
import { signupSchema } from "@/lib/schemas";

export async function POST(request: Request) {
  const body = await request.json();

  const result = signupSchema.safeParse(body);
  if (!result.success) {
    return Response.json({ errors: result.error.flatten() }, { status: 400 });
  }

  // Additional server-side validation
  const existingUser = await db.user.findUnique({
    where: { email: result.data.email },
  });

  if (existingUser) {
    return Response.json(
      { error: "Email already registered" },
      { status: 400 }
    );
  }

  // Create user
  const user = await db.user.create({ data: result.data });
  return Response.json(user);
}
```

---

## Performance Optimization

### Lazy Schema Definition

```typescript
import { z } from "zod";

// For recursive or circular schemas, use lazy
const categorySchema: z.ZodType = z.lazy(() =>
  z.object({
    name: z.string(),
    subcategories: z.array(categorySchema).optional(),
  })
);
```

### Reuse Schemas

```typescript
// Define once, reuse everywhere
const emailSchema = z.string().email();
const dateSchema = z.date();
const uuidSchema = z.string().uuid();

const userSchema = z.object({
  id: uuidSchema,
  email: emailSchema,
  createdAt: dateSchema,
});

const postSchema = z.object({
  id: uuidSchema,
  authorEmail: emailSchema,
  publishedAt: dateSchema,
});
```

### Avoid Unnecessary Transformations

```typescript
// Good - transform only when needed
const userSchema = z.object({
  email: z
    .string()
    .email()
    .transform((e) => e.toLowerCase()),
});

// Avoid - unnecessary transformations
const schema = z.object({
  name: z
    .string()
    .transform((v) => v.trim())
    .transform((v) => v.toLowerCase())
    .transform((v) => v.charAt(0).toUpperCase() + v.slice(1)),
  // Better: .trim().toLowerCase().pipe(...)
});
```

### Use Zod Mini for Bundle Size

```bash
npm install zod-mini
```

For frontend-only code where bundle size matters:

```typescript
import { z } from "zod-mini";

// Same API, smaller bundle
const schema = z.object({
  name: z.string(),
});
```

---

## Advanced Patterns

### Extending Schemas

```typescript
const baseSchema = z.object({
  id: z.string(),
  createdAt: z.date(),
});

// Extend with new properties
const userSchema = baseSchema.extend({
  name: z.string(),
  email: z.string().email(),
});

// Merge multiple schemas
const timestampsSchema = z.object({
  createdAt: z.date(),
  updatedAt: z.date(),
});

const articleSchema = z
  .object({
    title: z.string(),
    content: z.string(),
  })
  .merge(timestampsSchema);
```

### Conditional Schemas

```typescript
// Discriminated union for conditional validation
const userTypeSchema = z.discriminatedUnion("type", [
  z.object({
    type: z.literal("individual"),
    firstName: z.string(),
    lastName: z.string(),
  }),
  z.object({
    type: z.literal("company"),
    companyName: z.string(),
    taxId: z.string(),
  }),
]);

// Dependent fields
const addressSchema = z
  .object({
    country: z.enum(["US", "CA", "MX"]),
  })
  .superRefine((data, ctx) => {
    if (data.country === "US") {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["state"],
        message: "State required for US addresses",
      });
    }
  });
```

### Polymorphic Schemas

```typescript
// Create a factory for schemas
function createEntitySchema<T extends z.ZodRawShape>(shape: T) {
  return z.object({
    id: z.string().uuid(),
    createdAt: z.date(),
    updatedAt: z.date(),
    ...shape,
  });
}

const userSchema = createEntitySchema({
  name: z.string(),
  email: z.string().email(),
});

const productSchema = createEntitySchema({
  title: z.string(),
  price: z.number().positive(),
});
```

### Schema Composition

```typescript
// Address schema
const addressSchema = z.object({
  street: z.string(),
  city: z.string(),
  zipCode: z.string(),
  country: z.string(),
});

// Person schema using address
const personSchema = z.object({
  name: z.string(),
  email: z.string().email(),
  address: addressSchema,
  billingAddress: addressSchema.optional(),
});

// Company schema reusing address
const companySchema = z.object({
  name: z.string(),
  registrationNumber: z.string(),
  mainOffice: addressSchema,
  branches: z.array(addressSchema),
});
```

---

## Testing and Validation Strategies

### Unit Testing Schemas

```typescript
import { z } from "zod";
import { describe, it, expect } from "vitest";

const emailSchema = z.string().email();

describe("Email Schema", () => {
  it("validates correct emails", () => {
    expect(emailSchema.safeParse("user@example.com").success).toBe(true);
  });

  it("rejects invalid emails", () => {
    expect(emailSchema.safeParse("invalid-email").success).toBe(false);
  });

  it("provides error details", () => {
    const result = emailSchema.safeParse("invalid");
    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error.issues[0].code).toBe(z.ZodIssueCode.invalid_string);
    }
  });
});
```

### Testing Complex Schemas

```typescript
const userSchema = z.object({
  email: z.string().email(),
  age: z.number().int().min(18),
  role: z.enum(["admin", "user"]),
});

describe("User Schema", () => {
  it("validates valid user", () => {
    const user = {
      email: "user@example.com",
      age: 25,
      role: "user",
    };
    expect(userSchema.safeParse(user).success).toBe(true);
  });

  it("collects multiple errors", () => {
    const invalid = {
      email: "invalid",
      age: 15,
      role: "superuser",
    };
    const result = userSchema.safeParse(invalid);
    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error.issues).toHaveLength(3);
    }
  });
});
```

### Property-Based Testing

```typescript
import { fc, testProp } from "@fast-check/vitest";

const emailSchema = z.string().email();

testProp(
  "email schema accepts valid email format",
  [fc.emailAddress()],
  (email) => {
    expect(emailSchema.safeParse(email).success).toBe(true);
  }
);
```

### Snapshot Testing

```typescript
it("error snapshot matches", () => {
  const schema = z.object({
    name: z.string().min(1),
    age: z.number().int().positive(),
  });

  const result = schema.safeParse({ name: "", age: -5 });
  expect(result).toMatchSnapshot();
});
```

---

## Best Practices Summary

### Design Principles

1. **Single Source of Truth**: Define schemas once, infer types from them
2. **Fail Fast**: Validate early, provide clear error messages
3. **Type Safety**: Leverage TypeScript's type system with branded types
4. **Composability**: Build complex schemas from simple, reusable pieces
5. **Async When Needed**: Use async validation for external checks (uniqueness, availability)

### Schema Organization

1. **Group Related Schemas**: Keep similar schemas together
2. **Reuse Common Patterns**: Create utility schemas for repeated structures
3. **Separate Client/Server**: Put async validation in server-only files
4. **Version Schemas**: Track schema changes over time
5. **Document Schemas**: Use descriptions and metadata

### Error Handling

1. **Use Safe Parse**: Prefer `safeParse()` over `parse()`
2. **Customize Messages**: Provide context-specific error messages
3. **Format for Display**: Use `flatten()` for form errors
4. **Internationalize**: Support multiple languages
5. **Log Issues**: Track validation failures for debugging

### Performance

1. **Use Lazy for Recursion**: Avoid infinite loops with circular references
2. **Minimize Transformations**: Only transform when necessary
3. **Bundle Size**: Use Zod Mini for frontend code
4. **Cache Schemas**: Define schemas at module level, not in functions
5. **Profile Large Schemas**: Test performance with complex validations

### Type Safety

1. **Avoid `any`**: Use proper Zod types instead
2. **Brand Domain Types**: Distinguish semantic types (UserId vs UUID)
3. **Strict Objects**: Use `.strict()` to prevent extra fields
4. **Discriminated Unions**: Prefer over regular unions for performance
5. **Readonly Types**: Mark immutable data appropriately

---

## Key Zod 4 Features

### Performance Improvements

- **14x faster** string parsing
- **7x faster** array parsing
- **6.5x faster** object parsing
- **100x reduction** in TypeScript instantiations
- **2x reduction** in bundle size

### New in Zod 4

- **Unified Error Customization**: Single `error` param replacing `message`, `invalid_type_error`, and `required_error`
- **Metadata Registry**: Store and manage schema metadata separately
- **JSON Schema Conversion**: Built-in `toJSONSchema()` method
- **Improved Discriminated Unions**: Support for unions and pipes
- **Recursive Object Support**: Better handling of circular schemas
- **File Schemas**: Validation for File objects
- **Template Literal Types**: Support for template string patterns

---

## Common Pitfalls to Avoid

- Don't overuse `.any()` or `.unknown()` - use specific types
- Avoid excessive nested `.refine()` calls - use `.superRefine()` instead
- Don't define schemas inside functions - define at module level
- Avoid mixing sync and async validation without clear organization
- Don't ignore error messages - customize them for users
- Avoid recursive types without `.lazy()` - causes type issues
- Don't skip validation on the server - validate all inputs
- Avoid creating separate TypeScript types - use `z.infer<>`

---

## Resources

- **Official Documentation**: https://zod.dev
- **GitHub Repository**: https://github.com/colinhacks/zod
- **TypeScript Best Practices**: https://www.typescriptlang.org/docs
- **Form Integration**: React Hook Form, Formik, TanStack Form
- **Related Tools**: Zod-to-JSON-Schema, OpenAPI generators
