import { describe, expect, it, vi } from "vitest";
import { act, renderHook } from "@testing-library/react";
import { usePlaygroundStore } from "../index";

// Mock execution manager
vi.mock("@/lib/execution-manager", () => ({
  executeComparison: vi.fn(() => Promise.resolve({
    python: { stdout: "test", stderr: "", executionTimeMs: 10, memoryUsageMb: 1 },
    rust: { stdout: "test", stderr: "", executionTimeMs: 5, memoryUsageMb: 0.5 },
    performance: { speedup: 2.0, memoryReduction: 0.5 },
    energy: { joules: 0.001, wattsAverage: 1.0, co2Grams: 0.0005 }
  })),
}));

describe("usePlaygroundStore Basic Tests", () => {
  it("store hook returns valid functions", () => {
    const { result } = renderHook(() => usePlaygroundStore());
    
    expect(result.current).toBeTruthy();
    expect(typeof result.current.setPythonCode).toBe('function');
    expect(typeof result.current.setRustCode).toBe('function');
    expect(typeof result.current.transpileCode).toBe('function');
    expect(typeof result.current.executeCode).toBe('function');
    expect(typeof result.current.clearErrors).toBe('function');
    expect(typeof result.current.reset).toBe('function');
  });

  it("can update Python code", () => {
    const { result } = renderHook(() => usePlaygroundStore());
    
    act(() => {
      result.current.setPythonCode("def hello(): pass");
    });
    
    expect(result.current.pythonCode).toBe("def hello(): pass");
  });

  it("can update Rust code", () => {
    const { result } = renderHook(() => usePlaygroundStore());
    
    act(() => {
      result.current.setRustCode("fn hello() {}");
    });
    
    expect(result.current.rustCode).toBe("fn hello() {}");
  });

  it("has initial arrays for errors and warnings", () => {
    const { result } = renderHook(() => usePlaygroundStore());
    
    expect(Array.isArray(result.current.errors)).toBe(true);
    expect(Array.isArray(result.current.warnings)).toBe(true);
  });

  it("can clear errors", () => {
    const { result } = renderHook(() => usePlaygroundStore());
    
    act(() => {
      result.current.clearErrors();
    });
    
    expect(result.current.errors).toEqual([]);
    expect(result.current.warnings).toEqual([]);
  });
});