import { FlatCompat } from "@eslint/eslintrc";
import { fileURLToPath } from "url";
import path from "path";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

const compat = new FlatCompat({ baseDirectory: __dirname });

const config = [
  { ignores: ["node_modules/**", ".next/**", "test-results/**"] },
  ...compat.extends("next/core-web-vitals", "next/typescript"),
  {
    rules: {
      "react-hooks/set-state-in-effect": "off",
      "react-hooks/preserve-manual-memoization": "off",
      "react-hooks/purity": "off",
    },
  },
];

export default config;
