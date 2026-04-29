import { describe, expect, it, vi, beforeEach, afterEach } from "vitest";
import { act, renderHook } from "@testing-library/react";
import { usePlaygroundStore } from "../index";

// Mock the execution manager
vi.mock("@/lib/execution-manager", () => ({
  executeComparison: vi.fn(() =>
    Promise.resolve({
      pythonOutput: "42",
      rustOutput: "42",
      pythonTime: 100,
      rustTime: 10,
      speedup: 10.0,
      memorySaved: 5.0,
      pythonExitCode: 0,
      rustExitCode: 0,
    })
  ),
}));

describe("PlaygroundStore Actions", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe("setPythonCode", () => {
    it("updates Python code", () => {
      const { result } = renderHook(() => usePlaygroundStore());
      
      act(() => {
        result.current.setPythonCode("print('Hello')");
      });
      
      expect(result.current.pythonCode).toBe("print('Hello')");
    });

    it("triggers debounced transpilation", async () => {
      const { result } = renderHook(() => usePlaygroundStore());
      const transpileSpy = vi.spyOn(result.current, "transpileCode");
      
      act(() => {
        result.current.setPythonCode("def test(): pass");
      });
      
      // Should not transpile immediately
      expect(transpileSpy).not.toHaveBeenCalled();
      
      // Fast-forward debounce timer
      await act(async () => {
        vi.advanceTimersByTime(300);
      });
      
      expect(transpileSpy).toHaveBeenCalledOnce();
    });
  });

  describe("transpileCode", () => {
    it("transpiles Python code successfully", async () => {
      const { result } = renderHook(() => usePlaygroundStore());
      
      act(() => {
        result.current.setPythonCode("def add(a, b): return a + b");
      });
      
      await act(async () => {
        await result.current.transpileCode();
      });
      
      expect(result.current.rustCode).toBe("fn main() {}");
      expect(result.current.transpileResult?.success).toBe(true);
      expect(result.current.errors).toHaveLength(0);
    });

    it("handles transpilation errors", async () => {
      const { result } = renderHook(() => usePlaygroundStore());
      
      act(() => {
        result.current.setPythonCode("def syntax_error(:");
      });
      
      await act(async () => {
        await result.current.transpileCode();
      });
      
      expect(result.current.rustCode).toBe("");
      expect(result.current.transpileResult?.success).toBe(false);
      expect(result.current.errors).toContain("SyntaxError: invalid syntax");
    });
  });

  describe("executeCode", () => {
    it("executes code comparison successfully", async () => {
      const { result } = renderHook(() => usePlaygroundStore());
      const executionManager = await import("@/lib/execution-manager");
      
      act(() => {
        result.current.setPythonCode("print(42)");
        result.current.setRustCode("fn main() { println!(\"{}\", 42); }");
      });
      
      await act(async () => {
        await result.current.executeCode();
      });
      
      expect(executionManager.executeComparison).toHaveBeenCalledWith(
        "print(42)",
        "fn main() { println!(\"{}\", 42); }"
      );
      
      expect(result.current.executionResult).toEqual({
        pythonOutput: "42",
        rustOutput: "42",
        pythonTime: 100,
        rustTime: 10,
        speedup: 10.0,
        memorySaved: 5.0,
        pythonExitCode: 0,
        rustExitCode: 0,
      });
    });

    it("requires both Python and Rust code", async () => {
      const { result } = renderHook(() => usePlaygroundStore());
      const executionManager = await import("@/lib/execution-manager");
      
      // Reset the store to clear any previous execution results
      act(() => {
        result.current.reset();
      });
      
      act(() => {
        result.current.setPythonCode("print(42)");
        // Explicitly set empty Rust code
        result.current.setRustCode("");
      });
      
      await act(async () => {
        await result.current.executeCode();
      });
      
      expect(executionManager.executeComparison).not.toHaveBeenCalled();
      expect(result.current.executionResult).toBeNull();
    });
  });

  describe("utility actions", () => {
    it("clears errors", async () => {
      const { result } = renderHook(() => usePlaygroundStore());
      
      // Generate errors by transpiling invalid code
      act(() => {
        result.current.setPythonCode("def syntax_error(:");
      });
      
      await act(async () => {
        await result.current.transpileCode();
      });
      
      expect(result.current.errors.length).toBeGreaterThan(0);
      
      act(() => {
        result.current.clearErrors();
      });
      
      expect(result.current.errors).toHaveLength(0);
      expect(result.current.warnings).toHaveLength(0);
    });

    it("resets to initial state", async () => {
      const { result } = renderHook(() => usePlaygroundStore());
      
      // Modify state
      act(() => {
        result.current.setPythonCode("def modified(): pass");
      });
      
      await act(async () => {
        await result.current.transpileCode();
      });
      
      // Verify state was modified
      expect(result.current.pythonCode).toBe("def modified(): pass");
      expect(result.current.rustCode).toBe("fn main() {}");
      
      // Reset
      act(() => {
        result.current.reset();
      });
      
      expect(result.current.pythonCode).toContain("calculate_fibonacci");
      expect(result.current.rustCode).toBe("");
      expect(result.current.errors).toHaveLength(0);
      expect(result.current.transpileResult).toBeNull();
      expect(result.current.executionResult).toBeNull();
    });
  });
});