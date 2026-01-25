// Type declarations for Bun.YAML which is not in bun-types
declare global {
  namespace Bun {
    const YAML: {
      parse(content: string): unknown;
      stringify(value: unknown): string;
    };
  }
}

export {};
