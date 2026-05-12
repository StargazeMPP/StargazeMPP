import js from "@eslint/js";
import globals from "globals";

/**
 * Minimal flat-config ESLint setup for Next.js 16.
 *
 * `eslint-config-next` 16.x currently expects a Babel parser shim that
 * Next 16 no longer ships (`next lint` was removed — see the upgrade
 * guide). When `eslint-plugin-next` ships a working flat-config bundle,
 * swap this for `import next from "eslint-config-next"` + `next.flat()`.
 */
export default [
  {
    ignores: [
      ".next/**",
      "out/**",
      "build/**",
      "next-env.d.ts",
      "node_modules/**",
      "app/_landing.html",
      "playwright-report/**",
      "test-results/**",
    ],
  },
  js.configs.recommended,
  {
    languageOptions: {
      ecmaVersion: 2022,
      sourceType: "module",
      globals: { ...globals.browser, ...globals.node },
    },
    rules: {
      // The Lovable scaffold + dashboard component contain plenty of
      // mock-data unused vars; relax until the playground is wired to
      // the real session manager.
      "no-unused-vars": "off",
      "no-undef": "off",
      "no-empty": ["error", { allowEmptyCatch: true }],
    },
  },
];
