import "@testing-library/jest-dom/vitest";

if (typeof globalThis.crypto === "undefined") {
  globalThis.crypto = (await import("node:crypto")).webcrypto as unknown as Crypto;
}
