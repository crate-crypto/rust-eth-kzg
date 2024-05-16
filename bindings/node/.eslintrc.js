module.exports = {
  root: true,
  env: {
    browser: true,
    es6: true,
    node: true,
    mocha: true,
  },
  globals: {
    BigInt: true,
  },
  parser: "@typescript-eslint/parser",
  parserOptions: {
    ecmaVersion: "latest",
    project: "./tsconfig.json",
    sourceType: "script",
  },
  plugins: ["@typescript-eslint", "eslint-plugin-import", "eslint-plugin-node", "prettier"],
  extends: [
    "eslint:recommended",
    "plugin:import/errors",
    "plugin:import/warnings",
    "plugin:import/typescript",
    "plugin:@typescript-eslint/recommended",
  ],
  rules: {
    "prettier/prettier": "error",
    "constructor-super": "off",
    "@typescript-eslint/await-thenable": "error",
    "@typescript-eslint/explicit-function-return-type": [
      "error",
      {
        allowExpressions: true,
      },
    ],
    "@typescript-eslint/func-call-spacing": "error",
    "@typescript-eslint/member-ordering": "error",
    "@typescript-eslint/no-explicit-any": "error",
    "@typescript-eslint/no-require-imports": "error",
    "@typescript-eslint/no-unused-vars": [
      "error",
      {
        varsIgnorePattern: "^_",
      },
    ],
    "@typescript-eslint/ban-ts-comment": "warn",
    "@typescript-eslint/no-use-before-define": "error",
    "@typescript-eslint/semi": "error",
    "@typescript-eslint/type-annotation-spacing": "error",
    "@typescript-eslint/no-floating-promises": "error",
    "@typescript-eslint/explicit-member-accessibility": ["error", {accessibility: "no-public"}],
    "@typescript-eslint/no-unsafe-call": "error",
    "@typescript-eslint/no-unsafe-return": "error",
    "import/no-extraneous-dependencies": [
      "error",
      {
        devDependencies: false,
        optionalDependencies: false,
        peerDependencies: false,
      },
    ],
    "node/no-deprecated-api": "error",
    "new-parens": "error",
    "no-caller": "error",
    "no-cond-assign": "error",
    "no-var": "error",
    "object-curly-spacing": ["error", "never"],
    "prefer-const": "error",
    quotes: ["error", "double"],
    // semi: "off",
    // "func-call-spacing": "off",
    // "import/no-duplicates": "off",
    // "no-bitwise": "off",
    // "no-consecutive-blank-lines": 0,
    // "no-console": "warn",
    // "object-literal-sort-keys": 0,
    // "no-prototype-builtins": 0,
  },
  // settings: {
  //   "import/core-modules": ["node:child_process", "node:crypto", "node:fs", "node:os", "node:path", "node:util"],
  // },
  // overrides: [
  //   {
  //     files: ["lib/index.mjs"],
  //     parserOptions: {
  //       sourceType: "module"
  //     },
  //     rules: {
  //       // The imports are all resolved via tsconfig.mjs.json but the root tsconfig that eslint uses
  //       // shows them as unresolved.
  //       "import/no-unresolved": "off",
  //     },
  //   }, 
  //   {
  //     files: ["test/**/*.ts"],
  //     rules: {
  //       "import/no-extraneous-dependencies": "off",
  //       "@typescript-eslint/no-explicit-any": "off",
  //     },
  //   }, 
  //   {
  //     // Is a dev file and squacks about chokidar being a devDependency
  //     files: ["scripts/watch.ts"],
  //     rules: {
  //       "import/no-extraneous-dependencies": "off"
  //     },
  //   },
  // ],
};
