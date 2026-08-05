import { renderHook, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { useDebounce } from './useDebounce';

describe('useDebounce', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('executes the debounced function after the specified delay', () => {
    const fn = vi.fn();
    const { result } = renderHook(() => useDebounce(fn, 300));

    act(() => {
      result.current.run();
    });

    // Should NOT fire immediately
    expect(fn).not.toHaveBeenCalled();

    // Advance time by slightly less than the wait
    act(() => {
      vi.advanceTimersByTime(299);
    });
    expect(fn).not.toHaveBeenCalled();

    // Advance the final millisecond to trigger the timer
    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(fn).toHaveBeenCalledTimes(1);
  });

  it('only triggers once on rapid successive calls', () => {
    const fn = vi.fn();
    const { result } = renderHook(() => useDebounce(fn, 300));

    act(() => {
      result.current.run();
      // Advance 100ms and call again — the first timer should be cleared
      vi.advanceTimersByTime(100);
      result.current.run();
      // Advance another 100ms and call again
      vi.advanceTimersByTime(100);
      result.current.run();
    });

    // The third call's timer should now fire after the remaining delay
    act(() => {
      vi.advanceTimersByTime(299);
    });
    expect(fn).not.toHaveBeenCalled();

    act(() => {
      vi.advanceTimersByTime(1);
    });
    // Only one call — the first two were cancelled
    expect(fn).toHaveBeenCalledTimes(1);
  });

  it('respects a custom delay', () => {
    const fn = vi.fn();
    const { result } = renderHook(() => useDebounce(fn, 500));

    act(() => {
      result.current.run();
    });

    // Should not fire before 500ms
    act(() => {
      vi.advanceTimersByTime(499);
    });
    expect(fn).not.toHaveBeenCalled();

    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(fn).toHaveBeenCalledTimes(1);
  });

  it('cleans up the pending timer on unmount', () => {
    const fn = vi.fn();
    const { result, unmount } = renderHook(() => useDebounce(fn, 300));

    act(() => {
      result.current.run();
    });

    // Unmount the hook — the cleanup effect should clear the timer
    unmount();

    // Advance past the full delay — fn should NOT have been called
    act(() => {
      vi.advanceTimersByTime(300);
    });

    expect(fn).not.toHaveBeenCalled();
  });

  it('calls the latest callback when function reference changes', () => {
    const fn1 = vi.fn();
    const fn2 = vi.fn();

    const { result, rerender } = renderHook(
      ({ fn }: { fn: typeof fn1 }) => useDebounce(fn, 300),
      { initialProps: { fn: fn1 } },
    );

    act(() => {
      result.current.run();
    });

    // Before the timer fires, swap the callback
    rerender({ fn: fn2 });

    // Advance past the delay — should call fn2, NOT fn1
    act(() => {
      vi.advanceTimersByTime(300);
    });

    expect(fn1).not.toHaveBeenCalled();
    expect(fn2).toHaveBeenCalledTimes(1);
  });

  it('passes arguments through to the original function', () => {
    const fn = vi.fn();
    const { result } = renderHook(() => useDebounce(fn, 300));

    act(() => {
      result.current.run('arg1', 42, { key: 'value' });
    });

    act(() => {
      vi.advanceTimersByTime(300);
    });

    expect(fn).toHaveBeenCalledWith('arg1', 42, { key: 'value' });
  });

  it('resets the delay on each invocation', () => {
    const fn = vi.fn();
    const { result } = renderHook(() => useDebounce(fn, 300));

    act(() => {
      result.current.run(); // timer fires at t=300
    });

    // Advance 200ms — original timer has 100ms remaining
    act(() => {
      vi.advanceTimersByTime(200);
    });
    expect(fn).not.toHaveBeenCalled();

    // Call run() again — resets the timer so it now fires at t=500
    act(() => {
      result.current.run();
    });

    // Advance another 100ms (t=300) — the reset timer still has 200ms left
    act(() => {
      vi.advanceTimersByTime(100);
    });
    expect(fn).not.toHaveBeenCalled();

    // Advance the remaining 200ms (t=500) — timer fires exactly once
    act(() => {
      vi.advanceTimersByTime(200);
    });
    expect(fn).toHaveBeenCalledTimes(1);
  });
});
