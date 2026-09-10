# src/lib/

## Responsibility

Utility re-exports for the frontend. Currently a single file, `utils.ts`, that re-exports `cn` (class-name merge helper) so imports can use the stable path `@/lib/utils` regardless of where the implementation actually lives.

## Design

`utils.ts` is one line:

```ts
export { cn } from "cn";
```

The implementation comes from the npm package **`cn`** — a standalone classname-merging package. This is explicitly **not** the conventional `clsx` + `tailwind-merge` combo that stock shadcn projects ship; the repo deliberately replaced it with the `cn` package (a project convention noted in AGENTS.md). No local logic, no clsx, no tailwind-merge.

## Flow

No data or control flow of its own. It is a pass-through: consumers call `cn(...)` and get the package's merged-class-string result directly.

## Integration

- Path alias `@` → `./src` (declared in both `tsconfig.json` and `vite.config.ts`), so consumers import via `@/lib/utils`.
- Single consumer: `src/components/color-field.tsx` (imports `{ cn } from "@/lib/utils"`). The shadcn components in `src/components/ui/` do not import it.
- If `cn` is needed in more components (e.g., new `ui/` primitives), extend this file's exports rather than adding new util paths.
