---
applyTo: "**/*.{ts,tsx}, **/types/**/*.ts, **/lib/**/*.ts, **/utils/**/*.ts, **/services/**/*.ts, **/api/**/*.ts, **/hooks/**/*.ts, **/helpers/**/*.ts"
---

## When to Use `any` (Rarely)

### 1. Gradual Migration from JavaScript

Use `any` temporarily when typing existing JavaScript codebases or unknown third-party libraries.

```
function parseValue(data: any) {
  // Replace `any` with a strict type later
  return data;
}
```

### 2. Prototyping or Experimental Code

When rapidly prototyping an idea, `any` can speed up iteration — but remove it before production.

### 3. Interoperability with Un‑Typed APIs

When an external API lacks TypeScript support, using `any` may be acceptable **only until proper types are defined**.

---

## Why to Avoid `any`

1. **Loss of Type Safety** — Fails to detect incorrect types at compile time.
2. **Reduced Code Clarity** — Hides data intent from both the compiler and developers.
3. **Difficult Maintenance** — Refactors become error-prone in large projects.
4. **Inconsistent Behavior** — No guarantees about method availability or data structure.

Example:

```
function processUser(user: any) {
  return user.name.toUpperCase(); // Might crash at runtime
}
```

---

## Recommended Alternatives

### 1. Use `unknown` Instead of `any`

Unlike `any`, `unknown` enforces type checks before use.

```
let input: unknown = getInput();
if (typeof input === "string") {
  console.log(input.toUpperCase());
}
```

### 2. Define Proper Interfaces or Type Aliases

```
interface User {
  name: string;
  email: string;
}

function greet(user: User) {
  console.log(`Hello, ${user.name}`);
}
```

### 3. Use Generics for Dynamic Data

```
function identity<T>(value: T): T {
  return value;
}
```

### 4. Use Type Assertions Strictly When Safe

```
const data = JSON.parse(json) as User; // Safe only if structure is guaranteed
```

---

## Example — Replacing `any` Step by Step

**Before**

```
function handleResponse(response: any) {
  console.log(response.data.id);
}
```

**After**

```
interface ApiResponse {
  data: { id: number };
}

function handleResponse(response: ApiResponse) {
  console.log(response.data.id);
}
```

---

## Configuration-Level Enforcement

Enable strict options in your `tsconfig.json`:

```
{
  "compilerOptions": {
    "strict": true,
    "noImplicitAny": true,
    "noUncheckedIndexedAccess": true,
    "exactOptionalPropertyTypes": true
  }
}
```

- `noImplicitAny`: Prevents accidental `any` usage.
- `strict`: Activates the strict type‑checking mode suite.
- `noUncheckedIndexedAccess`: Ensures array or object access checks for `undefined`.

---

## Common Anti‑Patterns

- Declaring functions with `(...args: any[])`
- Returning `any` from utility functions
- Casting with `as any` to silence the compiler
- Using `any` in deeply nested generics instead of proper constraints

---

## Summary: Best Practice Matrix

| Situation                   | Recommended Action                    | Example                    |
| --------------------------- | ------------------------------------- | -------------------------- |
| Existing JS migration       | Temporary `any`, replace later        | `function init(data: any)` |
| Unknown dynamic input       | Use `unknown`                         | `let result: unknown`      |
| Untyped third-party library | Create type declarations              | `declare module 'lib'`     |
| Generic dynamic return      | Use generic types                     | `<T>(value: T): T`         |
| Quick prototype             | Temporary `any`, remove in production | `const temp: any`          |

---

## Final Notes

- The `any` type should be treated as an **escape hatch**, not a design pattern.
- Use `unknown`, type guards, or generics to retain flexibility _and_ safety.
- Review code regularly for hidden `any` values using linters (`eslint-plugin‑@typescript-eslint/no-explicit-any`).

---
