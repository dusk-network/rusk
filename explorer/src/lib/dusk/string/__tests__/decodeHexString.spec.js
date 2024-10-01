import { beforeEach, describe, expect, it, vi } from "vitest";
import { decodeHexString } from "..";

describe("decodeHexString", () => {
  const mockTextDecoder = vi
    .spyOn(global, "TextDecoder")
    .mockImplementation(function () {
      return {
        decode: vi.fn((bytes) => {
          // Simulate decoding of bytes
          return String.fromCharCode(...bytes);
        }),
        encoding: "utf-8",
        fatal: false,
        ignoreBOM: true,
      };
    });

  beforeEach(() => {
    mockTextDecoder.mockClear();
  });

  it("should return a decoded string for a valid hex string", () => {
    expect(decodeHexString("48656c6c6f")).toBe("Hello");
  });

  it("should return formatted JSON for a valid hex string containing JSON data", () => {
    const jsonString = '{"name": "ChatGPT", "language": "JavaScript"}';
    const hexString = Buffer.from(jsonString).toString("hex"); // Convert JSON string to hex

    expect(decodeHexString(hexString)).toBe(
      JSON.stringify(JSON.parse(jsonString), null, 2)
    );
  });

  it("should return the input value if it's not a valid hex string", () => {
    expect(decodeHexString("invalidHex")).toBe("invalidHex");
  });

  it("should return the decoded string even if it's not valid JSON", () => {
    expect(decodeHexString("48656c6c6f21")).toBe("Hello!");
  });

  it("should return the input value if decoded string contains invalid characters", () => {
    expect(decodeHexString("F09F9880")).toBe("F09F9880");
  });

  it("should return an empty string if the input is an empty hex string", () => {
    expect(decodeHexString("")).toBe("");
  });
});
