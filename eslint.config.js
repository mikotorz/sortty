import js from "@eslint/js";
import tseslint from "typescript-eslint";
import svelte from "eslint-plugin-svelte";
import prettier from "eslint-config-prettier";
import globals from "globals";

export default tseslint.config(
  {
    ignores: [
      "build/",
      ".svelte-kit/",
      "src-tauri/target/",
      "node_modules/",
      "*.config.js",
      "*.config.ts",
    ],
  },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  ...svelte.configs.recommended,
  prettier,
  ...svelte.configs.prettier,
  {
    languageOptions: {
      globals: { ...globals.browser, ...globals.node },
    },
  },
  {
    files: ["**/*.svelte"],
    languageOptions: {
      parserOptions: {
        parser: tseslint.parser,
        extraFileExtensions: [".svelte"],
      },
    },
  },
  {
    // `.svelte.ts` files (Svelte 5 runes used outside a component, e.g.
    // scanSession.svelte.ts) are plain TypeScript as far as ESLint's static
    // parsing is concerned — the rune macros are ordinary generic function
    // calls syntactically — but eslint-plugin-svelte's recommended config
    // claims this glob for the Svelte parser, which doesn't handle bare
    // modules. Force it back to the TS parser.
    files: ["**/*.svelte.ts", "**/*.svelte.js"],
    languageOptions: {
      parser: tseslint.parser,
    },
  },
  {
    rules: {
      // Svelte 5's $props/$state/$derived/$effect runes are compiler
      // built-ins, not undeclared globals — this rule predates rune support.
      "no-undef": "off",
      // Intentionally-discarded destructured values (`const { [k]: _x, ...rest } = obj`).
      "@typescript-eslint/no-unused-vars": [
        "error",
        { argsIgnorePattern: "^_", varsIgnorePattern: "^_" },
      ],
    },
  },
);
